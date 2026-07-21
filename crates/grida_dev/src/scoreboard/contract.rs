use anyhow::{bail, ensure, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub(crate) const REPORT_KIND: &str = "grida-consolidation-scoreboard-report";
pub(crate) const BASELINE_KIND: &str = "grida-consolidation-scoreboard-baseline";
pub(crate) const SCHEMA_VERSION: u32 = 0;
const SCORING_PIXELS: u64 = 128 * 128;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RuleIdentity {
    pub rule_id: String,
    pub version: String,
    pub ratified: bool,
    pub owner_decision: String,
    pub budget_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ArtifactIdentity {
    pub id: String,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ScoringIdentity {
    pub method_id: String,
    pub threshold: String,
    pub detect_anti_aliasing: bool,
    pub background: String,
    pub mask: String,
}

impl Default for ScoringIdentity {
    fn default() -> Self {
        Self {
            method_id: "dify-threshold-zero-aa-v0".into(),
            threshold: "0".into(),
            detect_anti_aliasing: true,
            background: "#ffffff".into(),
            mask: "none".into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RunEvidence {
    pub runner_sha256: String,
    pub prior_baseline_sha256: Option<String>,
    pub budget_ms: u64,
    pub elapsed_ms: u64,
    pub complete: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ProvenanceEvidence {
    pub corpus_validated: bool,
    pub oracle_bake_validated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum EngineCell {
    Scored { rgba_sha256: String },
    Unsupported { reason_code: String, detail: String },
    Error { reason_code: String, detail: String },
}

impl EngineCell {
    fn is_scored(&self) -> bool {
        matches!(self, Self::Scored { .. })
    }

    fn validate(&self, context: &str) -> Result<()> {
        match self {
            Self::Scored { rgba_sha256 } => validate_sha256(rgba_sha256, context),
            Self::Unsupported {
                reason_code,
                detail,
            }
            | Self::Error {
                reason_code,
                detail,
            } => validate_reason(reason_code, detail, context),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum ComparisonCell {
    Scored {
        different_pixels: u64,
        scoring_pixels: u64,
    },
    Unavailable {
        reason_code: String,
        detail: String,
    },
}

impl ComparisonCell {
    fn validate(&self, context: &str) -> Result<()> {
        match self {
            Self::Scored {
                different_pixels,
                scoring_pixels,
            } => {
                ensure!(
                    *scoring_pixels == SCORING_PIXELS,
                    "{context}: scoring_pixels must equal the fixed 128x128 denominator"
                );
                ensure!(
                    different_pixels <= scoring_pixels,
                    "{context}: different_pixels exceeds scoring_pixels"
                );
            }
            Self::Unavailable {
                reason_code,
                detail,
            } => validate_reason(reason_code, detail, context)?,
        }
        Ok(())
    }

    fn different_pixels(&self) -> Option<u64> {
        match self {
            Self::Scored {
                different_pixels, ..
            } => Some(*different_pixels),
            Self::Unavailable { .. } => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComparisonSet {
    pub legacy_vs_oracle: ComparisonCell,
    pub chassis_vs_oracle: ComparisonCell,
    pub legacy_vs_chassis: ComparisonCell,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ScoreboardRow {
    pub fixture_id: String,
    pub legacy: EngineCell,
    pub chassis: EngineCell,
    pub comparisons: ComparisonSet,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct EngineCoverage {
    pub included: u64,
    pub scored: u64,
    pub unsupported: u64,
    pub error: u64,
}

impl EngineCoverage {
    pub(crate) fn from_rows<'a>(cells: impl Iterator<Item = &'a EngineCell>) -> Self {
        let mut coverage = Self::default();
        for cell in cells {
            coverage.included += 1;
            match cell {
                EngineCell::Scored { .. } => coverage.scored += 1,
                EngineCell::Unsupported { .. } => coverage.unsupported += 1,
                EngineCell::Error { .. } => coverage.error += 1,
            }
        }
        coverage
    }

    fn validate(&self, context: &str) -> Result<()> {
        ensure!(
            self.included == self.scored + self.unsupported + self.error,
            "{context}: coverage partition does not equal included rows"
        );
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CoverageSet {
    pub legacy: EngineCoverage,
    pub chassis: EngineCoverage,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SuiteReport {
    pub suite_id: String,
    pub coverage: CoverageSet,
    pub rows: Vec<ScoreboardRow>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ScoreboardReport {
    pub schema_version: u32,
    pub kind: String,
    pub rule: RuleIdentity,
    pub corpus: ArtifactIdentity,
    pub oracle_bake: ArtifactIdentity,
    pub scoring: ScoringIdentity,
    pub provenance: ProvenanceEvidence,
    pub run: RunEvidence,
    pub suites: Vec<SuiteReport>,
}

impl ScoreboardReport {
    pub(crate) fn validate_contract(&self) -> Result<()> {
        ensure!(
            self.schema_version == SCHEMA_VERSION,
            "unsupported report schema"
        );
        ensure!(self.kind == REPORT_KIND, "unsupported report kind");
        validate_identity(&self.corpus, "corpus")?;
        validate_identity(&self.oracle_bake, "oracle bake")?;
        validate_rule(&self.rule)?;
        ensure!(
            self.rule.ratified,
            "scoreboard reports require a ratified flip rule"
        );
        validate_scoring(&self.scoring)?;
        ensure!(
            self.provenance.corpus_validated && self.provenance.oracle_bake_validated,
            "scoreboard report provenance is invalid"
        );
        validate_budget_state(&self.run)?;
        ensure!(
            self.run.budget_ms == self.rule.budget_ms,
            "run budget does not match the ratified rule"
        );
        validate_sha256(&self.run.runner_sha256, "scoreboard runner")?;
        if let Some(hash) = &self.run.prior_baseline_sha256 {
            validate_sha256(hash, "scoreboard prior baseline")?;
        }
        ensure!(
            !self.suites.is_empty(),
            "report must contain at least one suite"
        );

        let mut previous_suite = None::<&str>;
        for suite in &self.suites {
            ensure!(!suite.suite_id.is_empty(), "suite id must not be empty");
            if let Some(previous) = previous_suite {
                ensure!(
                    previous < suite.suite_id.as_str(),
                    "suite ids must be sorted and unique"
                );
            }
            previous_suite = Some(&suite.suite_id);
            validate_suite(suite)?;
        }
        Ok(())
    }
}

pub(crate) fn validate_denominator(
    report: &ScoreboardReport,
    suite_id: &str,
    fixture_ids: &[String],
) -> Result<()> {
    report.validate_contract()?;
    ensure!(
        report.suites.len() == 1,
        "report must contain exactly one fixed suite"
    );
    let suite = &report.suites[0];
    ensure!(
        suite.suite_id == suite_id,
        "report suite does not match the corpus"
    );
    ensure!(
        suite.rows.len() == fixture_ids.len(),
        "report row count does not match the fixed corpus denominator"
    );
    for (row, expected_id) in suite.rows.iter().zip(fixture_ids) {
        ensure!(
            row.fixture_id == *expected_id,
            "report rows do not match the fixed ordered corpus denominator"
        );
    }
    let expected_count = u64::try_from(fixture_ids.len())
        .map_err(|_| anyhow::anyhow!("corpus denominator exceeds report count domain"))?;
    ensure!(
        suite.coverage.legacy.included == expected_count
            && suite.coverage.chassis.included == expected_count,
        "report coverage does not use the fixed corpus denominator"
    );
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct BaselineSuite {
    pub suite_id: String,
    pub coverage: CoverageSet,
    pub rows: Vec<ScoreboardRow>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ScoreboardBaseline {
    pub schema_version: u32,
    pub kind: String,
    pub rule: RuleIdentity,
    pub corpus: ArtifactIdentity,
    pub oracle_bake: ArtifactIdentity,
    pub scoring: ScoringIdentity,
    pub suites: Vec<BaselineSuite>,
}

impl ScoreboardBaseline {
    pub(crate) fn validate_contract(&self) -> Result<()> {
        ensure!(
            self.schema_version == SCHEMA_VERSION,
            "unsupported baseline schema"
        );
        ensure!(self.kind == BASELINE_KIND, "unsupported baseline kind");
        validate_identity(&self.corpus, "corpus")?;
        validate_identity(&self.oracle_bake, "oracle bake")?;
        validate_rule(&self.rule)?;
        ensure!(
            self.rule.ratified,
            "scoreboard baselines require a ratified flip rule"
        );
        validate_scoring(&self.scoring)?;
        ensure!(
            !self.suites.is_empty(),
            "baseline must contain at least one suite"
        );
        let mut previous_suite = None::<&str>;
        for suite in &self.suites {
            if let Some(previous) = previous_suite {
                ensure!(
                    previous < suite.suite_id.as_str(),
                    "suite ids must be sorted and unique"
                );
            }
            previous_suite = Some(&suite.suite_id);
            validate_suite_parts(&suite.suite_id, &suite.coverage, &suite.rows)?;
        }
        Ok(())
    }
}

pub(crate) fn validate_budget_state(evidence: &RunEvidence) -> Result<()> {
    ensure!(evidence.complete, "scoreboard run is incomplete");
    ensure!(
        evidence.budget_ms > 0,
        "scoreboard run has no wall-clock budget"
    );
    ensure!(
        evidence.elapsed_ms <= evidence.budget_ms,
        "scoreboard run exceeded its wall-clock budget"
    );
    Ok(())
}

pub(crate) fn prepare_baseline_candidate(
    report: &ScoreboardReport,
    previous: Option<&ScoreboardBaseline>,
) -> Result<ScoreboardBaseline> {
    report.validate_contract()?;
    ensure!(report.rule.ratified, "cannot bless an unratified flip rule");
    ensure!(
        report.provenance.corpus_validated && report.provenance.oracle_bake_validated,
        "cannot bless a provenance-invalid report"
    );
    validate_budget_state(&report.run)?;

    match (previous, &report.run.prior_baseline_sha256) {
        (None, None) => {}
        (None, Some(_)) => {
            bail!("report identifies a prior baseline, but none was supplied for validation")
        }
        (Some(_), None) => {
            bail!("report does not identify the prior baseline used for regression checks")
        }
        (Some(previous), Some(_)) => {
            let regressions = find_regressions(previous, report)?;
            if !regressions.is_empty() {
                bail!(
                    "baseline candidate contains regressions: {}",
                    regressions.join("; ")
                );
            }
        }
    }

    Ok(ScoreboardBaseline {
        schema_version: SCHEMA_VERSION,
        kind: BASELINE_KIND.into(),
        rule: report.rule.clone(),
        corpus: report.corpus.clone(),
        oracle_bake: report.oracle_bake.clone(),
        scoring: report.scoring.clone(),
        suites: report
            .suites
            .iter()
            .map(|suite| BaselineSuite {
                suite_id: suite.suite_id.clone(),
                coverage: suite.coverage.clone(),
                rows: suite.rows.iter().map(project_row).collect(),
            })
            .collect(),
    })
}

fn project_row(row: &ScoreboardRow) -> ScoreboardRow {
    ScoreboardRow {
        fixture_id: row.fixture_id.clone(),
        legacy: project_engine_cell(&row.legacy),
        chassis: project_engine_cell(&row.chassis),
        comparisons: ComparisonSet {
            legacy_vs_oracle: project_comparison_cell(&row.comparisons.legacy_vs_oracle),
            chassis_vs_oracle: project_comparison_cell(&row.comparisons.chassis_vs_oracle),
            legacy_vs_chassis: project_comparison_cell(&row.comparisons.legacy_vs_chassis),
        },
    }
}

fn project_engine_cell(cell: &EngineCell) -> EngineCell {
    match cell {
        EngineCell::Scored { rgba_sha256 } => EngineCell::Scored {
            rgba_sha256: rgba_sha256.clone(),
        },
        EngineCell::Unsupported { reason_code, .. } => EngineCell::Unsupported {
            reason_code: reason_code.clone(),
            detail: "detail retained in the source report".into(),
        },
        EngineCell::Error { reason_code, .. } => EngineCell::Error {
            reason_code: reason_code.clone(),
            detail: "detail retained in the source report".into(),
        },
    }
}

fn project_comparison_cell(cell: &ComparisonCell) -> ComparisonCell {
    match cell {
        ComparisonCell::Scored {
            different_pixels,
            scoring_pixels,
        } => ComparisonCell::Scored {
            different_pixels: *different_pixels,
            scoring_pixels: *scoring_pixels,
        },
        ComparisonCell::Unavailable { reason_code, .. } => ComparisonCell::Unavailable {
            reason_code: reason_code.clone(),
            detail: "detail retained in the source report".into(),
        },
    }
}

pub(crate) fn find_regressions(
    baseline: &ScoreboardBaseline,
    report: &ScoreboardReport,
) -> Result<Vec<String>> {
    baseline.validate_contract()?;
    report.validate_contract()?;
    ensure!(
        baseline.rule == report.rule,
        "baseline rule identity mismatch"
    );
    ensure!(
        baseline.corpus == report.corpus,
        "baseline corpus identity mismatch"
    );
    ensure!(
        baseline.oracle_bake == report.oracle_bake,
        "baseline oracle-bake identity mismatch"
    );
    ensure!(
        baseline.scoring == report.scoring,
        "baseline scoring identity mismatch"
    );
    ensure!(
        baseline.suites.len() == report.suites.len(),
        "baseline suite set mismatch"
    );

    let mut regressions = Vec::new();
    for (old_suite, new_suite) in baseline.suites.iter().zip(&report.suites) {
        ensure!(
            old_suite.suite_id == new_suite.suite_id,
            "baseline suite set mismatch"
        );
        ensure!(
            old_suite.rows.len() == new_suite.rows.len(),
            "baseline row set mismatch"
        );
        for (old, new) in old_suite.rows.iter().zip(&new_suite.rows) {
            ensure!(
                old.fixture_id == new.fixture_id,
                "baseline row set mismatch"
            );
            check_engine_transition(
                &old_suite.suite_id,
                &old.fixture_id,
                "legacy",
                &old.legacy,
                &new.legacy,
                &mut regressions,
            );
            check_engine_transition(
                &old_suite.suite_id,
                &old.fixture_id,
                "chassis",
                &old.chassis,
                &new.chassis,
                &mut regressions,
            );
            check_oracle_comparison(
                &old_suite.suite_id,
                &old.fixture_id,
                "legacy",
                &old.comparisons.legacy_vs_oracle,
                &new.comparisons.legacy_vs_oracle,
                &mut regressions,
            );
            check_oracle_comparison(
                &old_suite.suite_id,
                &old.fixture_id,
                "chassis",
                &old.comparisons.chassis_vs_oracle,
                &new.comparisons.chassis_vs_oracle,
                &mut regressions,
            );
        }
    }
    Ok(regressions)
}

fn validate_suite(suite: &SuiteReport) -> Result<()> {
    validate_suite_parts(&suite.suite_id, &suite.coverage, &suite.rows)
}

fn validate_suite_parts(
    suite_id: &str,
    coverage: &CoverageSet,
    rows: &[ScoreboardRow],
) -> Result<()> {
    ensure!(!suite_id.is_empty(), "suite id must not be empty");
    ensure!(!rows.is_empty(), "suite {suite_id}: rows must not be empty");
    coverage
        .legacy
        .validate(&format!("suite {suite_id} legacy"))?;
    coverage
        .chassis
        .validate(&format!("suite {suite_id} chassis"))?;
    ensure!(
        coverage.legacy == EngineCoverage::from_rows(rows.iter().map(|row| &row.legacy)),
        "suite {suite_id}: legacy coverage is not derived from rows"
    );
    ensure!(
        coverage.chassis == EngineCoverage::from_rows(rows.iter().map(|row| &row.chassis)),
        "suite {suite_id}: chassis coverage is not derived from rows"
    );

    let mut previous = None::<&str>;
    let mut seen = BTreeSet::new();
    for row in rows {
        ensure!(
            !row.fixture_id.is_empty(),
            "suite {suite_id}: fixture id is empty"
        );
        ensure!(
            seen.insert(&row.fixture_id),
            "suite {suite_id}: duplicate row {}",
            row.fixture_id
        );
        if let Some(previous) = previous {
            ensure!(
                previous < row.fixture_id.as_str(),
                "suite {suite_id}: rows must be sorted"
            );
        }
        previous = Some(&row.fixture_id);
        validate_row(suite_id, row)?;
    }
    Ok(())
}

fn validate_row(suite_id: &str, row: &ScoreboardRow) -> Result<()> {
    let context = format!("suite {suite_id} row {}", row.fixture_id);
    row.legacy.validate(&format!("{context} legacy"))?;
    row.chassis.validate(&format!("{context} chassis"))?;
    row.comparisons
        .legacy_vs_oracle
        .validate(&format!("{context} legacy-vs-oracle"))?;
    row.comparisons
        .chassis_vs_oracle
        .validate(&format!("{context} chassis-vs-oracle"))?;
    row.comparisons
        .legacy_vs_chassis
        .validate(&format!("{context} legacy-vs-chassis"))?;

    validate_comparison_availability(
        row.legacy.is_scored(),
        &row.comparisons.legacy_vs_oracle,
        &format!("{context} legacy-vs-oracle"),
    )?;
    validate_comparison_availability(
        row.chassis.is_scored(),
        &row.comparisons.chassis_vs_oracle,
        &format!("{context} chassis-vs-oracle"),
    )?;
    validate_comparison_availability(
        row.legacy.is_scored() && row.chassis.is_scored(),
        &row.comparisons.legacy_vs_chassis,
        &format!("{context} legacy-vs-chassis"),
    )?;
    Ok(())
}

fn validate_comparison_availability(
    endpoints_scored: bool,
    comparison: &ComparisonCell,
    context: &str,
) -> Result<()> {
    ensure!(
        endpoints_scored == matches!(comparison, ComparisonCell::Scored { .. }),
        "{context}: comparison availability does not match its engine endpoints"
    );
    Ok(())
}

fn validate_identity(identity: &ArtifactIdentity, context: &str) -> Result<()> {
    ensure!(!identity.id.is_empty(), "{context} identity is empty");
    validate_sha256(&identity.sha256, context)
}

fn validate_rule(rule: &RuleIdentity) -> Result<()> {
    ensure!(!rule.rule_id.is_empty(), "rule id is empty");
    ensure!(!rule.version.is_empty(), "rule version is empty");
    if rule.ratified {
        ensure!(
            !rule.owner_decision.is_empty(),
            "ratified rule has no owner decision"
        );
    }
    ensure!(
        rule.budget_ms > 0,
        "rule wall-clock budget must be positive"
    );
    Ok(())
}

fn validate_scoring(scoring: &ScoringIdentity) -> Result<()> {
    let expected = ScoringIdentity::default();
    ensure!(scoring == &expected, "unsupported scoring identity");
    Ok(())
}

fn validate_sha256(value: &str, context: &str) -> Result<()> {
    ensure!(
        value.len() == 64
            && value
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)),
        "{context}: sha256 must be 64 lowercase hexadecimal characters"
    );
    Ok(())
}

fn validate_reason(code: &str, detail: &str, context: &str) -> Result<()> {
    ensure!(
        !code.is_empty()
            && code
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_'),
        "{context}: reason_code must be non-empty snake_case ASCII"
    );
    ensure!(
        !detail.trim().is_empty(),
        "{context}: reason detail must not be empty"
    );
    Ok(())
}

fn check_engine_transition(
    suite: &str,
    fixture: &str,
    engine: &str,
    old: &EngineCell,
    new: &EngineCell,
    regressions: &mut Vec<String>,
) {
    let regressed = matches!(old, EngineCell::Scored { .. }) && !new.is_scored()
        || matches!(old, EngineCell::Unsupported { .. }) && matches!(new, EngineCell::Error { .. });
    if regressed {
        regressions.push(format!("{suite}/{fixture}: {engine} coverage regressed"));
    }
}

fn check_oracle_comparison(
    suite: &str,
    fixture: &str,
    engine: &str,
    old: &ComparisonCell,
    new: &ComparisonCell,
    regressions: &mut Vec<String>,
) {
    if let Some(old_pixels) = old.different_pixels() {
        match new.different_pixels() {
            Some(new_pixels) if new_pixels > old_pixels => regressions.push(format!(
                "{suite}/{fixture}: {engine}-vs-oracle differing pixels increased from {old_pixels} to {new_pixels}"
            )),
            None => regressions.push(format!(
                "{suite}/{fixture}: {engine}-vs-oracle comparison became unavailable"
            )),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scored(hash_byte: char) -> EngineCell {
        EngineCell::Scored {
            rgba_sha256: std::iter::repeat_n(hash_byte, 64).collect(),
        }
    }

    fn comparison(different_pixels: u64) -> ComparisonCell {
        ComparisonCell::Scored {
            different_pixels,
            scoring_pixels: 16_384,
        }
    }

    fn report() -> ScoreboardReport {
        let rows = vec![ScoreboardRow {
            fixture_id: "rect-solid".into(),
            legacy: scored('a'),
            chassis: scored('b'),
            comparisons: ComparisonSet {
                legacy_vs_oracle: comparison(2),
                chassis_vs_oracle: comparison(0),
                legacy_vs_chassis: comparison(2),
            },
        }];
        ScoreboardReport {
            schema_version: SCHEMA_VERSION,
            kind: REPORT_KIND.into(),
            rule: RuleIdentity {
                rule_id: "svg-rect-path-flip".into(),
                version: "1".into(),
                ratified: true,
                owner_decision: "gridaco/nothing#49".into(),
                budget_ms: 120_000,
            },
            corpus: ArtifactIdentity {
                id: "svg-rect-path-v0".into(),
                sha256: "c".repeat(64),
            },
            oracle_bake: ArtifactIdentity {
                id: "chromium-149".into(),
                sha256: "d".repeat(64),
            },
            scoring: ScoringIdentity::default(),
            provenance: ProvenanceEvidence {
                corpus_validated: true,
                oracle_bake_validated: true,
            },
            run: RunEvidence {
                runner_sha256: "f".repeat(64),
                prior_baseline_sha256: None,
                budget_ms: 120_000,
                elapsed_ms: 1_000,
                complete: true,
            },
            suites: vec![SuiteReport {
                suite_id: "svg-rect-path".into(),
                coverage: CoverageSet {
                    legacy: EngineCoverage::from_rows(rows.iter().map(|row| &row.legacy)),
                    chassis: EngineCoverage::from_rows(rows.iter().map(|row| &row.chassis)),
                },
                rows,
            }],
        }
    }

    #[test]
    fn report_requires_derived_coverage_and_all_three_comparisons() {
        let mut value = report();
        value.validate_contract().unwrap();
        value.suites[0].coverage.chassis.scored = 0;
        assert!(value
            .validate_contract()
            .unwrap_err()
            .to_string()
            .contains("coverage"));

        let mut value = report();
        value.suites[0].rows[0].comparisons.legacy_vs_chassis = ComparisonCell::Unavailable {
            reason_code: "missing_endpoint".into(),
            detail: "synthetic".into(),
        };
        assert!(value
            .validate_contract()
            .unwrap_err()
            .to_string()
            .contains("availability"));
    }

    #[test]
    fn report_denominator_must_equal_the_exact_ordered_corpus_rows() {
        let value = report();
        validate_denominator(&value, "svg-rect-path", &["rect-solid".into()]).unwrap();

        assert!(validate_denominator(
            &value,
            "svg-rect-path",
            &["rect-solid".into(), "unexpected-extra".into()]
        )
        .unwrap_err()
        .to_string()
        .contains("row count"));
        assert!(
            validate_denominator(&value, "wrong-suite", &["rect-solid".into()])
                .unwrap_err()
                .to_string()
                .contains("suite")
        );
    }

    #[test]
    fn unsupported_requires_a_machine_reason_and_unavailable_comparisons() {
        let mut value = report();
        value.suites[0].rows[0].chassis = EngineCell::Unsupported {
            reason_code: "".into(),
            detail: "not in the chassis profile".into(),
        };
        value.suites[0].rows[0].comparisons.chassis_vs_oracle = ComparisonCell::Unavailable {
            reason_code: "unsupported".into(),
            detail: "not scored".into(),
        };
        value.suites[0].rows[0].comparisons.legacy_vs_chassis = ComparisonCell::Unavailable {
            reason_code: "unsupported".into(),
            detail: "not scored".into(),
        };
        value.suites[0].coverage.chassis =
            EngineCoverage::from_rows(value.suites[0].rows.iter().map(|row| &row.chassis));
        assert!(value
            .validate_contract()
            .unwrap_err()
            .to_string()
            .contains("reason_code"));
    }

    #[test]
    fn baseline_projection_is_deterministic_and_strips_run_evidence() {
        let mut first = report();
        first.suites[0].rows[0].chassis = EngineCell::Unsupported {
            reason_code: "not_in_profile".into(),
            detail: "/random/scratch/one: parser diagnostic".into(),
        };
        first.suites[0].rows[0].comparisons.chassis_vs_oracle = ComparisonCell::Unavailable {
            reason_code: "engine_unavailable".into(),
            detail: "/random/scratch/one: no chassis pixels".into(),
        };
        first.suites[0].rows[0].comparisons.legacy_vs_chassis = ComparisonCell::Unavailable {
            reason_code: "engine_unavailable".into(),
            detail: "/random/scratch/one: no chassis pixels".into(),
        };
        first.suites[0].coverage.chassis =
            EngineCoverage::from_rows(first.suites[0].rows.iter().map(|row| &row.chassis));
        let mut second = first.clone();
        second.run.elapsed_ms = 9_999;
        if let EngineCell::Unsupported { detail, .. } = &mut second.suites[0].rows[0].chassis {
            *detail = "/different/host/two: parser diagnostic".into();
        }
        if let ComparisonCell::Unavailable { detail, .. } =
            &mut second.suites[0].rows[0].comparisons.chassis_vs_oracle
        {
            *detail = "/different/host/two: no chassis pixels".into();
        }
        if let ComparisonCell::Unavailable { detail, .. } =
            &mut second.suites[0].rows[0].comparisons.legacy_vs_chassis
        {
            *detail = "/different/host/two: no chassis pixels".into();
        }
        let first = prepare_baseline_candidate(&first, None).unwrap();
        let second = prepare_baseline_candidate(&second, None).unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn baseline_rejects_identity_mismatch_and_per_row_regression() {
        let original = report();
        let baseline = prepare_baseline_candidate(&original, None).unwrap();

        let mut subsequent = original.clone();
        subsequent.run.prior_baseline_sha256 = Some("9".repeat(64));
        assert!(find_regressions(&baseline, &subsequent).unwrap().is_empty());

        let mut mismatch = original.clone();
        mismatch.corpus.sha256 = "e".repeat(64);
        assert!(find_regressions(&baseline, &mismatch)
            .unwrap_err()
            .to_string()
            .contains("corpus identity"));

        let mut regression = original;
        regression.suites[0].rows[0].comparisons.chassis_vs_oracle = comparison(1);
        let regressions = find_regressions(&baseline, &regression).unwrap();
        assert_eq!(regressions.len(), 1);
        assert!(regressions[0].contains("chassis-vs-oracle"));
    }

    #[test]
    fn bless_refuses_unratified_incomplete_over_budget_and_invalid_provenance() {
        let mut value = report();
        value.rule.ratified = false;
        value.rule.owner_decision.clear();
        assert!(prepare_baseline_candidate(&value, None).is_err());

        let mut value = report();
        value.run.complete = false;
        assert!(prepare_baseline_candidate(&value, None).is_err());

        let mut value = report();
        value.run.elapsed_ms = value.run.budget_ms + 1;
        assert!(prepare_baseline_candidate(&value, None).is_err());

        let mut value = report();
        value.provenance.oracle_bake_validated = false;
        assert!(prepare_baseline_candidate(&value, None).is_err());

        let mut value = report();
        value.run.prior_baseline_sha256 = Some("8".repeat(64));
        assert!(prepare_baseline_candidate(&value, None).is_err());
    }
}
