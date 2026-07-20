mod contract;
mod corpus;
mod runner;

use anyhow::{bail, ensure, Context, Result};
use clap::{Args, Subcommand};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::sync::mpsc::{sync_channel, RecvTimeoutError, TryRecvError};
use std::thread;
use std::time::{Duration, Instant};

use contract::{
    find_regressions, prepare_baseline_candidate, validate_denominator, RuleIdentity,
    ScoreboardBaseline, ScoreboardReport,
};

const DEFAULT_CORPUS: &str = "fixtures/scoreboard/svg-rect-path-v0/corpus.json";
const DEFAULT_REPORT: &str = "target/scoreboard/report-v0.json";
const DEFAULT_BASELINE: &str = "fixtures/scoreboard/svg-rect-path-v0/baseline-v0.json";
const DEFAULT_CANDIDATE: &str = "target/scoreboard/baseline-v0.candidate.json";

#[derive(Debug, Clone)]
struct ActiveRule {
    identity: RuleIdentity,
    prior_baseline_sha256: Option<String>,
}

#[derive(Args, Debug)]
pub(crate) struct ScoreboardArgs {
    #[command(subcommand)]
    command: ScoreboardCommand,
}

#[derive(Subcommand, Debug)]
enum ScoreboardCommand {
    /// Validate the fixed corpus, Chromium bake, and both parser seams.
    Check(CheckArgs),
    /// Render and score the fixed corpus (sealed until FLIP is ratified).
    Run(RunArgs),
    /// Derive a create-new baseline candidate from a valid report.
    Bless(BlessArgs),
}

#[derive(Args, Debug)]
struct CheckArgs {
    /// Fixed scoreboard corpus manifest.
    #[arg(long, default_value = DEFAULT_CORPUS)]
    corpus: PathBuf,
}

#[derive(Args, Debug)]
struct RunArgs {
    /// Fixed scoreboard corpus manifest.
    #[arg(long, default_value = DEFAULT_CORPUS)]
    corpus: PathBuf,
    /// Create-new report path.
    #[arg(long, default_value = DEFAULT_REPORT)]
    report: PathBuf,
    /// Prior committed baseline; the active configuration binds its absence or exact hash.
    #[arg(long, default_value = DEFAULT_BASELINE)]
    baseline: PathBuf,
}

#[derive(Args, Debug)]
struct BlessArgs {
    /// Fixed scoreboard corpus manifest used to validate report provenance.
    #[arg(long, default_value = DEFAULT_CORPUS)]
    corpus: PathBuf,
    /// Complete scoreboard report to project.
    #[arg(long, default_value = DEFAULT_REPORT)]
    report: PathBuf,
    /// Prior committed baseline; the active configuration binds its absence or exact hash.
    #[arg(long, default_value = DEFAULT_BASELINE)]
    baseline: PathBuf,
    /// Create-new baseline candidate path. Existing files are never replaced.
    #[arg(long, default_value = DEFAULT_CANDIDATE)]
    candidate: PathBuf,
}

pub(crate) fn run(args: ScoreboardArgs) -> Result<()> {
    match args.command {
        ScoreboardCommand::Check(args) => check(args),
        ScoreboardCommand::Run(args) => run_scoreboard(args),
        ScoreboardCommand::Bless(args) => bless(args),
    }
}

fn check(args: CheckArgs) -> Result<()> {
    let validated = corpus::validate(&args.corpus)?;
    println!(
        "scoreboard check: {} rows in {} ({})",
        validated.manifest.fixtures.len(),
        validated.manifest.corpus_id,
        validated.manifest.suite_id
    );
    for fixture in &validated.manifest.fixtures {
        println!("  include {}", fixture.id);
    }
    println!("excluded-family patrol ledger:");
    for family in &validated.manifest.excluded_families {
        println!(
            "  {} [{}] {}",
            family.path, family.reason_code, family.reason
        );
    }
    println!(
        "validated: source hashes, Chromium oracle hashes, 128x128 dimensions, legacy preflight, chassis preflight"
    );
    Ok(())
}

fn run_scoreboard(args: RunArgs) -> Result<()> {
    with_ratified_rule(active_rule(), |active| {
        let report_path = repository_path(&args.report)?;
        let baseline_path = repository_path(&args.baseline)?;
        let limit = Duration::from_millis(active.identity.budget_ms);
        let budget = runner::RunBudget::new(limit)?;
        let corpus_path = args.corpus;
        let prior_baseline_sha256 = active.prior_baseline_sha256;
        let (report, regressions) = run_with_hard_budget(budget, move |budget| {
            let validated = corpus::validate_for_report(&corpus_path)?;
            let previous = load_prior_baseline(&baseline_path, prior_baseline_sha256.as_deref())?;
            let mut report =
                runner::produce(&validated, active.identity, prior_baseline_sha256, budget)?;
            let regressions = if let Some(baseline) = &previous {
                find_regressions(baseline, &report)?
            } else {
                Vec::new()
            };
            report.run = budget.evidence(
                report.run.runner_sha256.clone(),
                report.run.prior_baseline_sha256.clone(),
            )?;
            report.validate_contract()?;
            Ok((report, regressions))
        })?;
        write_json_create_new(&report_path, &report)?;
        if !regressions.is_empty() {
            bail!(
                "scoreboard report contains baseline regressions: {}",
                regressions.join("; ")
            );
        }
        println!("scoreboard report: {}", report_path.display());
        Ok(())
    })
}

fn bless(args: BlessArgs) -> Result<()> {
    with_ratified_rule(active_rule(), |active| {
        let validated = corpus::validate_for_report(&args.corpus)?;
        let report_path = repository_path(&args.report)?;
        let report: ScoreboardReport = read_json(&report_path)?;
        report.validate_contract()?;
        ensure!(
            report.rule == active.identity,
            "report does not use the active ratified rule"
        );
        ensure!(
            report.run.prior_baseline_sha256 == active.prior_baseline_sha256,
            "report does not use the active prior-baseline identity"
        );
        ensure!(
            report.corpus == validated.corpus_identity,
            "report corpus provenance does not match current committed inputs"
        );
        ensure!(
            report.oracle_bake == validated.oracle_bake_identity,
            "report oracle-bake provenance does not match current committed inputs"
        );
        let fixture_ids = validated
            .manifest
            .fixtures
            .iter()
            .map(|fixture| fixture.id.clone())
            .collect::<Vec<_>>();
        validate_denominator(&report, &validated.manifest.suite_id, &fixture_ids)?;

        let baseline_path = repository_path(&args.baseline)?;
        let previous =
            load_prior_baseline(&baseline_path, active.prior_baseline_sha256.as_deref())?;
        let candidate = prepare_baseline_candidate(&report, previous.as_ref())?;
        let candidate_path = repository_path(&args.candidate)?;
        validate_candidate_path(&candidate_path, &baseline_path, &report_path)?;
        prepare_candidate_parent(&candidate_path)?;
        write_json_create_new_in_existing_parent(&candidate_path, &candidate)?;
        println!("baseline candidate: {}", candidate_path.display());
        Ok(())
    })
}

/// There is intentionally no environment variable, CLI flag, or local bypass.
/// Owner ratification on gridaco/nothing#49 must be recorded in the WG rule and
/// then represented here as one explicit identity-changing code review.
fn active_rule() -> Option<ActiveRule> {
    None
}

fn with_ratified_rule<T>(
    rule: Option<ActiveRule>,
    operation: impl FnOnce(ActiveRule) -> Result<T>,
) -> Result<T> {
    let Some(rule) = rule else {
        bail!(
            "scoreboard scoring is sealed: FLIP is unratified in gridaco/nothing#49; no render or report was opened"
        );
    };
    ensure!(
        rule.identity.ratified,
        "active scoreboard rule is not ratified"
    );
    ensure!(
        !rule.identity.owner_decision.is_empty(),
        "active rule lacks its owner decision"
    );
    operation(rule)
}

/// Run every score-producing and baseline-reading stage behind a real
/// wall-clock watchdog. Cooperative checks still make ordinary overruns
/// precise, while this boundary returns even if an in-process renderer or
/// comparator never returns. The worker never receives the report path, so a
/// timed-out operation cannot publish a partial or over-budget report.
fn run_with_hard_budget<T>(
    budget: runner::RunBudget,
    operation: impl FnOnce(&runner::RunBudget) -> Result<T> + Send + 'static,
) -> Result<T>
where
    T: Send + 'static,
{
    let deadline = budget.deadline()?;
    let (sender, receiver) = sync_channel(1);
    let worker = thread::Builder::new()
        .name("scoreboard-v0-worker".into())
        .spawn(move || {
            let _ = sender.send(operation(&budget));
        })
        .context("spawn scoreboard watchdog worker")?;

    // Derive the wait after spawning so thread-start overhead consumes the
    // original RunBudget instead of silently extending it. At an already
    // reached deadline, accept only a result that is already in the channel;
    // its own RunEvidence still proves that the operation finished in time.
    let timeout = deadline.saturating_duration_since(Instant::now());
    let received = if timeout.is_zero() {
        match receiver.try_recv() {
            Ok(result) => Ok(result),
            Err(TryRecvError::Empty) => Err(RecvTimeoutError::Timeout),
            Err(TryRecvError::Disconnected) => Err(RecvTimeoutError::Disconnected),
        }
    } else {
        receiver.recv_timeout(timeout)
    };

    match received {
        Ok(result) => {
            worker
                .join()
                .map_err(|_| anyhow::anyhow!("scoreboard worker panicked"))?;
            result
        }
        Err(RecvTimeoutError::Timeout) => {
            // Dropping the handle detaches the blocked worker. This function
            // is private to the terminal CLI path: returning the error ends
            // the process, and the OS terminates the worker. No report path
            // is reachable from that worker.
            drop(worker);
            bail!("scoreboard run exceeded its hard wall-clock budget")
        }
        Err(RecvTimeoutError::Disconnected) => {
            let _ = worker.join();
            bail!("scoreboard worker terminated without a result")
        }
    }
}

fn repository_path(path: &Path) -> Result<PathBuf> {
    ensure!(
        !path.as_os_str().is_empty(),
        "scoreboard path must not be empty"
    );
    ensure!(
        path.components()
            .all(|component| !matches!(component, Component::CurDir | Component::ParentDir)),
        "scoreboard paths must be normalized and cannot contain `.` or `..`: {}",
        path.display()
    );
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        Ok(corpus::repo_root().join(path))
    }
}

fn load_prior_baseline(
    path: &Path,
    expected_sha256: Option<&str>,
) -> Result<Option<ScoreboardBaseline>> {
    match expected_sha256 {
        None => {
            ensure!(
                !path.exists(),
                "the ratified rule declares no prior baseline, but one exists at {}",
                path.display()
            );
            Ok(None)
        }
        Some(expected_hash) => {
            let bytes = fs::read(path).with_context(|| {
                format!(
                    "the ratified rule requires prior baseline {}, but it could not be read",
                    path.display()
                )
            })?;
            ensure!(
                corpus::sha256_hex(&bytes) == expected_hash,
                "prior baseline hash does not match the ratified rule"
            );
            let baseline = serde_json::from_slice(&bytes)
                .with_context(|| format!("parse prior baseline {}", path.display()))?;
            Ok(Some(baseline))
        }
    }
}

fn validate_candidate_path(candidate: &Path, baseline: &Path, report: &Path) -> Result<()> {
    ensure!(
        candidate != baseline,
        "baseline candidate cannot be the committed baseline path"
    );
    ensure!(
        candidate != report,
        "baseline candidate cannot replace the source report path"
    );
    let review_root = corpus::repo_root().join("target/scoreboard");
    ensure!(
        candidate.starts_with(&review_root),
        "baseline candidates must be written under {}",
        review_root.display()
    );
    ensure!(
        candidate.parent() == Some(review_root.as_path()),
        "baseline candidates must be direct files under {}",
        review_root.display()
    );
    ensure!(
        candidate.file_name().is_some(),
        "baseline candidate must name a file"
    );
    Ok(())
}

fn prepare_candidate_parent(candidate: &Path) -> Result<()> {
    let root = corpus::repo_root();
    let review_root = ensure_directory_without_symlinks(&root, Path::new("target/scoreboard"))?;
    ensure!(
        candidate.parent() == Some(review_root.as_path()),
        "baseline candidate parent changed during validation"
    );
    Ok(())
}

fn ensure_directory_without_symlinks(root: &Path, relative: &Path) -> Result<PathBuf> {
    ensure!(
        relative
            .components()
            .all(|component| matches!(component, Component::Normal(_))),
        "confined output directory must be a normalized relative path"
    );
    let canonical_root = fs::canonicalize(root)
        .with_context(|| format!("canonicalize output root {}", root.display()))?;
    let mut directory = root.to_path_buf();
    for component in relative.components() {
        let Component::Normal(component) = component else {
            unreachable!("relative path components were validated above")
        };
        directory.push(component);
        match fs::symlink_metadata(&directory) {
            Ok(metadata) => {
                ensure!(
                    !metadata.file_type().is_symlink(),
                    "confined output directory contains a symlink: {}",
                    directory.display()
                );
                ensure!(
                    metadata.is_dir(),
                    "confined output component is not a directory: {}",
                    directory.display()
                );
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                fs::create_dir(&directory)
                    .with_context(|| format!("create output directory {}", directory.display()))?;
            }
            Err(error) => {
                return Err(error)
                    .with_context(|| format!("inspect output directory {}", directory.display()))
            }
        }
        let canonical = fs::canonicalize(&directory)
            .with_context(|| format!("canonicalize output directory {}", directory.display()))?;
        ensure!(
            canonical.starts_with(&canonical_root),
            "confined output directory escapes the repository: {}",
            directory.display()
        );
    }
    Ok(directory)
}

fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T> {
    let bytes = fs::read(path).with_context(|| format!("read {}", path.display()))?;
    serde_json::from_slice(&bytes).with_context(|| format!("parse {}", path.display()))
}

fn write_json_create_new(path: &Path, value: &impl Serialize) -> Result<()> {
    let mut bytes = serde_json::to_vec_pretty(value).context("serialize scoreboard JSON")?;
    bytes.push(b'\n');
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("create output directory {}", parent.display()))?;
    }
    write_bytes_create_new(path, &bytes)
}

fn write_json_create_new_in_existing_parent(path: &Path, value: &impl Serialize) -> Result<()> {
    let mut bytes = serde_json::to_vec_pretty(value).context("serialize scoreboard JSON")?;
    bytes.push(b'\n');
    let parent = path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("scoreboard output has no parent: {}", path.display()))?;
    ensure!(
        parent.is_dir(),
        "scoreboard output parent is not an existing directory: {}",
        parent.display()
    );
    write_bytes_create_new(path, &bytes)
}

fn write_bytes_create_new(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .with_context(|| {
            format!(
                "create new output {}; existing files are never replaced",
                path.display()
            )
        })?;
    file.write_all(bytes)
        .with_context(|| format!("write {}", path.display()))?;
    file.sync_all()
        .with_context(|| format!("sync {}", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    fn synthetic_rule(prior_baseline_sha256: Option<String>) -> ActiveRule {
        ActiveRule {
            identity: RuleIdentity {
                rule_id: "svg-rect-path-flip".into(),
                version: "1".into(),
                ratified: true,
                owner_decision: "gridaco/nothing#49".into(),
                budget_ms: 120_000,
            },
            prior_baseline_sha256,
        }
    }

    #[test]
    fn unratified_rule_refuses_before_the_render_operation() {
        let called = Cell::new(false);
        let error = with_ratified_rule(None, |_| {
            called.set(true);
            Ok(())
        })
        .unwrap_err();
        assert!(!called.get());
        assert!(error.to_string().contains("no render or report was opened"));
    }

    #[test]
    fn nominally_active_but_unratified_rule_also_refuses_before_work() {
        let called = Cell::new(false);
        let error = with_ratified_rule(
            Some(ActiveRule {
                identity: RuleIdentity {
                    rule_id: "proposal".into(),
                    version: "0".into(),
                    ratified: false,
                    owner_decision: String::new(),
                    budget_ms: 120_000,
                },
                prior_baseline_sha256: None,
            }),
            |_| {
                called.set(true);
                Ok(())
            },
        )
        .unwrap_err();
        assert!(!called.get());
        assert!(error.to_string().contains("not ratified"));
    }

    #[test]
    fn hard_budget_returns_before_a_blocked_stage_finishes() {
        let started = Instant::now();
        let error = run_with_hard_budget(
            runner::RunBudget::new(Duration::from_millis(20)).unwrap(),
            |_| {
                thread::sleep(Duration::from_secs(1));
                Ok(())
            },
        )
        .unwrap_err();

        assert!(error.to_string().contains("hard wall-clock budget"));
        assert!(
            started.elapsed() < Duration::from_millis(500),
            "watchdog waited for the blocked stage instead of preempting the command"
        );
    }

    #[test]
    fn report_and_candidate_writer_never_replaces_an_existing_file() {
        let directory = tempfile::tempdir().unwrap();
        let output = directory.path().join("candidate.json");
        write_json_create_new(&output, &serde_json::json!({ "first": true })).unwrap();
        let error =
            write_json_create_new(&output, &serde_json::json!({ "second": true })).unwrap_err();
        assert!(error.to_string().contains("never replaced"));
        assert_eq!(
            fs::read_to_string(output).unwrap(),
            "{\n  \"first\": true\n}\n"
        );
    }

    #[test]
    fn prior_baseline_absence_is_explicit_in_the_rule() {
        let directory = tempfile::tempdir().unwrap();
        let baseline = directory.path().join("baseline.json");
        assert!(load_prior_baseline(&baseline, None).unwrap().is_none());

        fs::write(&baseline, b"{}\n").unwrap();
        assert!(load_prior_baseline(&baseline, None)
            .unwrap_err()
            .to_string()
            .contains("declares no prior baseline"));

        let missing = directory.path().join("missing.json");
        let active = synthetic_rule(Some("a".repeat(64)));
        assert!(
            load_prior_baseline(&missing, active.prior_baseline_sha256.as_deref())
                .unwrap_err()
                .to_string()
                .contains("requires prior baseline")
        );
    }

    #[test]
    fn candidate_is_separate_and_confined_to_the_review_directory() {
        let root = corpus::repo_root();
        let baseline = root.join(DEFAULT_BASELINE);
        let report = root.join(DEFAULT_REPORT);
        let candidate = root.join(DEFAULT_CANDIDATE);
        validate_candidate_path(&candidate, &baseline, &report).unwrap();
        assert!(validate_candidate_path(&baseline, &baseline, &report).is_err());
        assert!(validate_candidate_path(&root.join("candidate.json"), &baseline, &report).is_err());
        assert!(validate_candidate_path(
            &root.join("target/scoreboard/nested/candidate.json"),
            &baseline,
            &report
        )
        .is_err());
    }

    #[cfg(unix)]
    #[test]
    fn confined_output_directory_rejects_a_symlink_component() {
        use std::os::unix::fs::symlink;

        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        fs::create_dir(root.path().join("target")).unwrap();
        symlink(outside.path(), root.path().join("target/scoreboard")).unwrap();
        let error = ensure_directory_without_symlinks(root.path(), Path::new("target/scoreboard"))
            .unwrap_err();
        assert!(error.to_string().contains("contains a symlink"));
    }
}
