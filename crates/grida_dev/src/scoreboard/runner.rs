use anyhow::{anyhow, bail, Context, Result};
use n0::paint::PaintCtx;
use skia_safe::{surfaces, Color, EncodedImageFormat};
use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};

use crate::reftest::args::BgColor;
use crate::reftest::compare::{compare_images, ScoringMask};
use crate::reftest::render::render_svg_to_png;

use super::contract::{
    ComparisonCell, ComparisonSet, CoverageSet, EngineCell, EngineCoverage, ProvenanceEvidence,
    RuleIdentity, RunEvidence, ScoreboardReport, ScoreboardRow, ScoringIdentity, SuiteReport,
    REPORT_KIND, SCHEMA_VERSION,
};
use super::corpus::{sha256_hex, EnginePreflight, ValidatedCorpus};

#[derive(Debug)]
pub(crate) struct RunBudget {
    started: Instant,
    limit: Duration,
}

impl RunBudget {
    pub(crate) fn new(limit: Duration) -> Result<Self> {
        if limit.is_zero() {
            bail!("scoreboard wall-clock budget must be positive");
        }
        Ok(Self {
            started: Instant::now(),
            limit,
        })
    }

    pub(crate) fn check(&self) -> Result<()> {
        if self.started.elapsed() > self.limit {
            bail!("scoreboard run exceeded its hard wall-clock budget");
        }
        Ok(())
    }

    pub(crate) fn deadline(&self) -> Result<Instant> {
        self.started
            .checked_add(self.limit)
            .ok_or_else(|| anyhow!("scoreboard wall-clock deadline is not representable"))
    }

    pub(crate) fn evidence(
        &self,
        runner_sha256: String,
        prior_baseline_sha256: Option<String>,
    ) -> Result<RunEvidence> {
        self.check()?;
        let budget_ms = u64::try_from(self.limit.as_millis())
            .map_err(|_| anyhow!("scoreboard wall-clock budget does not fit report schema"))?;
        let elapsed_ms = u64::try_from(self.started.elapsed().as_millis())
            .map_err(|_| anyhow!("scoreboard elapsed time does not fit report schema"))?;
        Ok(RunEvidence {
            runner_sha256,
            prior_baseline_sha256,
            budget_ms,
            elapsed_ms,
            complete: true,
        })
    }
}

/// Produce a complete report for a rule that the caller has already proved is
/// active and ratified. The public CLI guard currently makes this function
/// unreachable: keeping the implementation behind that guard lets the render
/// seam compile without allowing a pre-ratification score to exist.
pub(crate) fn produce(
    corpus: &ValidatedCorpus,
    rule: RuleIdentity,
    prior_baseline_sha256: Option<String>,
    budget: &RunBudget,
) -> Result<ScoreboardReport> {
    budget.check()?;
    let scratch = tempfile::tempdir().context("create scoreboard scratch directory")?;
    let mut rows = Vec::with_capacity(corpus.manifest.fixtures.len());

    for (fixture, input) in corpus.manifest.fixtures.iter().zip(&corpus.inputs) {
        budget.check()?;
        let row_dir = scratch.path().join(&fixture.id);
        fs::create_dir(&row_dir)
            .with_context(|| format!("create scratch directory for {}", fixture.id))?;
        let source_path = row_dir.join("source.svg");
        let oracle_path = row_dir.join("oracle.png");
        fs::write(&source_path, input.source.as_bytes())
            .with_context(|| format!("stage exact validated source for {}", fixture.id))?;
        fs::write(&oracle_path, &input.oracle_png)
            .with_context(|| format!("stage exact validated oracle for {}", fixture.id))?;

        let (legacy, legacy_image) = render_legacy_twice(
            &input.legacy_preflight,
            &source_path,
            &row_dir,
            corpus.manifest.viewport.width,
            corpus.manifest.viewport.height,
        );
        budget.check()?;
        let (chassis, chassis_image) = render_chassis_twice(
            &input.chassis_preflight,
            &source_path,
            &fixture.source,
            &row_dir,
            corpus.manifest.viewport.width,
            corpus.manifest.viewport.height,
        );
        budget.check()?;

        let legacy_vs_oracle = compare_optional(
            legacy_image.as_deref(),
            Some(&oracle_path),
            &legacy,
            None,
            "legacy_vs_oracle",
        )?;
        let chassis_vs_oracle = compare_optional(
            chassis_image.as_deref(),
            Some(&oracle_path),
            &chassis,
            None,
            "chassis_vs_oracle",
        )?;
        let legacy_vs_chassis = compare_optional(
            legacy_image.as_deref(),
            chassis_image.as_deref(),
            &legacy,
            Some(&chassis),
            "legacy_vs_chassis",
        )?;

        rows.push(ScoreboardRow {
            fixture_id: fixture.id.clone(),
            legacy,
            chassis,
            comparisons: ComparisonSet {
                legacy_vs_oracle,
                chassis_vs_oracle,
                legacy_vs_chassis,
            },
        });
    }
    budget.check()?;

    let coverage = CoverageSet {
        legacy: EngineCoverage::from_rows(rows.iter().map(|row| &row.legacy)),
        chassis: EngineCoverage::from_rows(rows.iter().map(|row| &row.chassis)),
    };
    let executable = std::env::current_exe().context("locate scoreboard runner executable")?;
    let runner_sha256 = sha256_hex(
        &fs::read(&executable)
            .with_context(|| format!("read scoreboard runner {}", executable.display()))?,
    );
    let report = ScoreboardReport {
        schema_version: SCHEMA_VERSION,
        kind: REPORT_KIND.into(),
        rule,
        corpus: corpus.corpus_identity.clone(),
        oracle_bake: corpus.oracle_bake_identity.clone(),
        scoring: ScoringIdentity::default(),
        provenance: ProvenanceEvidence {
            corpus_validated: true,
            oracle_bake_validated: true,
        },
        run: budget.evidence(runner_sha256, prior_baseline_sha256)?,
        suites: vec![SuiteReport {
            suite_id: corpus.manifest.suite_id.clone(),
            coverage,
            rows,
        }],
    };
    report.validate_contract()?;
    Ok(report)
}

fn render_legacy_twice(
    preflight: &EnginePreflight,
    source: &Path,
    output: &Path,
    width: u32,
    height: u32,
) -> (EngineCell, Option<std::path::PathBuf>) {
    if let Some(result) = unsupported_preflight_result(preflight) {
        return result;
    }
    let first = output.join("legacy-1.png");
    let second = output.join("legacy-2.png");
    let result = (|| -> Result<String> {
        render_svg_to_png(source, &first, Some((width, height)))?;
        render_svg_to_png(source, &second, Some((width, height)))?;
        deterministic_rgba_hash(&first, &second)
    })();
    match result {
        Ok(rgba_sha256) => (EngineCell::Scored { rgba_sha256 }, Some(first)),
        Err(error) => (engine_error(error), None),
    }
}

fn render_chassis_twice(
    preflight: &EnginePreflight,
    source: &Path,
    source_identity: &str,
    output: &Path,
    width: u32,
    height: u32,
) -> (EngineCell, Option<std::path::PathBuf>) {
    if let Some(result) = unsupported_preflight_result(preflight) {
        return result;
    }
    let first = output.join("chassis-1.png");
    let second = output.join("chassis-2.png");
    let result = (|| -> Result<String> {
        let source_text = fs::read_to_string(source)
            .with_context(|| format!("read chassis source {}", source.display()))?;
        compile_and_render_chassis_png(source_identity, &source_text, &first, width, height)?;
        compile_and_render_chassis_png(source_identity, &source_text, &second, width, height)?;
        deterministic_rgba_hash(&first, &second)
    })();
    match result {
        Ok(rgba_sha256) => (EngineCell::Scored { rgba_sha256 }, Some(first)),
        Err(error) => (engine_error(error), None),
    }
}

fn unsupported_preflight_result(
    preflight: &EnginePreflight,
) -> Option<(EngineCell, Option<std::path::PathBuf>)> {
    match preflight {
        EnginePreflight::Accepted => None,
        EnginePreflight::Unsupported {
            reason_code,
            detail,
        } => Some((
            EngineCell::Unsupported {
                reason_code: (*reason_code).into(),
                detail: detail.clone(),
            },
            None,
        )),
    }
}

fn compile_and_render_chassis_png(
    source_identity: &str,
    source: &str,
    output: &Path,
    width: u32,
    height: u32,
) -> Result<()> {
    let compiled = n0::svg_animation_frame::compile_latest(source_identity, source)
        .map_err(|error| anyhow!("compile chassis source: {error}"))?;
    let paint = PaintCtx::default();
    let width_i32 = i32::try_from(width).context("chassis width exceeds i32")?;
    let height_i32 = i32::try_from(height).context("chassis height exceeds i32")?;
    let mut surface = surfaces::raster_n32_premul((width_i32, height_i32))
        .ok_or_else(|| anyhow!("could not allocate chassis raster surface"))?;
    surface.canvas().clear(Color::TRANSPARENT);
    n0::svg_animation_frame::render_base(surface.canvas(), &compiled, &paint)
        .map_err(|error| anyhow!("render chassis Base frame: {error}"))?;
    let data = surface
        .image_snapshot()
        .encode(None, EncodedImageFormat::PNG, None)
        .ok_or_else(|| anyhow!("encode chassis PNG"))?;
    fs::write(output, data.as_bytes())
        .with_context(|| format!("write chassis PNG {}", output.display()))?;
    Ok(())
}

fn deterministic_rgba_hash(first: &Path, second: &Path) -> Result<String> {
    let first = image::open(first)
        .context("decode first determinism render")?
        .to_rgba8()
        .into_raw();
    let second = image::open(second)
        .context("decode second determinism render")?
        .to_rgba8()
        .into_raw();
    if first != second {
        bail!("repeated renders produced different decoded RGBA bytes");
    }
    Ok(sha256_hex(&first))
}

fn engine_error(error: anyhow::Error) -> EngineCell {
    let detail = error.to_string();
    let reason_code = if detail.contains("different decoded RGBA bytes") {
        "nondeterministic_pixels"
    } else {
        "render_error"
    };
    EngineCell::Error {
        reason_code: reason_code.into(),
        detail,
    }
}

fn compare_optional(
    actual: Option<&Path>,
    expected: Option<&Path>,
    actual_cell: &EngineCell,
    expected_cell: Option<&EngineCell>,
    label: &str,
) -> Result<ComparisonCell> {
    let (Some(actual), Some(expected)) = (actual, expected) else {
        let detail = if let Some(expected_cell) = expected_cell {
            format!("comparison unavailable: actual={actual_cell:?}; expected={expected_cell:?}")
        } else {
            format!("comparison unavailable: actual={actual_cell:?}")
        };
        return Ok(ComparisonCell::Unavailable {
            reason_code: "engine_unavailable".into(),
            detail,
        });
    };
    let result = compare_images(
        actual,
        expected,
        None,
        0.0,
        true,
        BgColor::White,
        ScoringMask::None,
    )
    .with_context(|| format!("compare {label}"))?;
    if let Some(error) = result.error {
        bail!("{label}: comparator failed: {error}");
    }
    Ok(ComparisonCell::Scored {
        different_pixels: result.different_pixels,
        scoring_pixels: result.scoring_pixels,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_wall_clock_budget_is_rejected_before_work() {
        assert!(RunBudget::new(Duration::ZERO)
            .unwrap_err()
            .to_string()
            .contains("must be positive"));
    }

    #[test]
    fn rejected_preflight_is_unsupported_while_runtime_failure_is_error() {
        let rejected = EnginePreflight::Unsupported {
            reason_code: "chassis_preflight_rejected",
            detail: "outside the bounded profile".into(),
        };
        let (cell, image) = unsupported_preflight_result(&rejected).unwrap();
        assert_eq!(image, None);
        assert!(matches!(
            cell,
            EngineCell::Unsupported {
                reason_code,
                detail
            } if reason_code == "chassis_preflight_rejected"
                && detail == "outside the bounded profile"
        ));

        assert!(unsupported_preflight_result(&EnginePreflight::Accepted).is_none());
        assert!(matches!(
            engine_error(anyhow!("paint failed after accepted preflight")),
            EngineCell::Error { reason_code, .. } if reason_code == "render_error"
        ));
    }
}
