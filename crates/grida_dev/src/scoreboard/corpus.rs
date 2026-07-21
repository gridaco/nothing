use anyhow::{bail, ensure, Context, Result};
use quick_xml::events::Event;
use quick_xml::Reader;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::fs;
use std::path::{Component, Path, PathBuf};

use super::contract::ArtifactIdentity;

pub(crate) const CORPUS_ID: &str = "svg-rect-path-v0";
pub(crate) const SUITE_ID: &str = "svg-rect-path";
pub(crate) const VIEWPORT_WIDTH: u32 = 128;
pub(crate) const VIEWPORT_HEIGHT: u32 = 128;

const EXPECTED_IDS: [&str; 14] = [
    "path-evenodd",
    "path-opacity",
    "path-solid",
    "path-transform",
    "rect-opacity",
    "rect-overlap-order",
    "rect-rounded-elliptical",
    "rect-rounded-uniform",
    "rect-solid",
    "rect-transform-matrix",
    "rect-transform-rotate",
    "rect-transform-scale",
    "rect-transform-skew",
    "rect-transform-translate",
];

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Viewport {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CorpusFixture {
    pub id: String,
    pub source: String,
    pub source_sha256: String,
    pub oracle: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CorpusExclusion {
    pub path: String,
    pub reason_code: String,
    pub reason: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CorpusManifest {
    pub schema_version: u32,
    pub corpus_id: String,
    pub suite_id: String,
    pub request: String,
    pub viewport: Viewport,
    pub background: String,
    pub oracle_bake: String,
    pub fixtures: Vec<CorpusFixture>,
    pub excluded_families: Vec<CorpusExclusion>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct CaptureContract {
    width: u32,
    height: u32,
    device_scale_factor: u32,
    omit_background: bool,
    source_transport: String,
    javascript_enabled: bool,
    network_enabled: bool,
    source_mutation: bool,
    style_injection: bool,
    animation_control: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct OracleFixture {
    id: String,
    source_sha256: String,
    oracle_sha256: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct OracleBakeManifest {
    schema_version: u32,
    corpus_id: String,
    corpus_manifest_sha256: String,
    kind: String,
    browser_version: String,
    bake_script_sha256: String,
    capture: CaptureContract,
    fixtures: Vec<OracleFixture>,
}

pub(crate) struct ValidatedCorpus {
    pub manifest: CorpusManifest,
    pub corpus_identity: ArtifactIdentity,
    pub oracle_bake_identity: ArtifactIdentity,
    pub inputs: Vec<ValidatedInput>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum EnginePreflight {
    Accepted,
    Unsupported {
        reason_code: &'static str,
        detail: String,
    },
}

pub(crate) struct ValidatedInput {
    pub source: String,
    pub oracle_png: Vec<u8>,
    pub legacy_preflight: EnginePreflight,
    pub chassis_preflight: EnginePreflight,
}

#[derive(Debug, Clone)]
struct FixturePreflight {
    fixture_id: String,
    legacy: EnginePreflight,
    chassis: EnginePreflight,
}

pub(crate) fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("grida_dev is a direct child of the repository crates directory")
        .to_path_buf()
}

#[cfg(test)]
fn default_corpus_path() -> PathBuf {
    repo_root().join("fixtures/scoreboard/svg-rect-path-v0/corpus.json")
}

pub(crate) fn validate(corpus_path: &Path) -> Result<ValidatedCorpus> {
    validate_with_preflight_policy(corpus_path, true)
}

/// Validate report provenance while retaining engine entry-point rejections
/// as per-row coverage evidence. The strict `scoreboard check` path continues
/// to use [`validate`] and requires every row in the fixed v0 corpus to be
/// accepted by both engines.
pub(crate) fn validate_for_report(corpus_path: &Path) -> Result<ValidatedCorpus> {
    validate_with_preflight_policy(corpus_path, false)
}

fn validate_with_preflight_policy(
    corpus_path: &Path,
    require_all_preflights: bool,
) -> Result<ValidatedCorpus> {
    let root = fs::canonicalize(repo_root()).context("canonicalize repository root")?;
    let corpus_path = if corpus_path.is_absolute() {
        corpus_path.to_path_buf()
    } else {
        root.join(corpus_path)
    };
    let corpus_bytes = fs::read(&corpus_path)
        .with_context(|| format!("read scoreboard corpus {}", corpus_path.display()))?;
    let manifest: CorpusManifest = serde_json::from_slice(&corpus_bytes)
        .with_context(|| format!("parse scoreboard corpus {}", corpus_path.display()))?;
    validate_manifest_shape(&manifest)?;
    let corpus_hash = sha256_hex(&corpus_bytes);

    validate_closed_source_directory(&root, &manifest)?;
    let mut sources = Vec::with_capacity(manifest.fixtures.len());
    let mut preflights = Vec::with_capacity(manifest.fixtures.len());
    for fixture in &manifest.fixtures {
        let source_path = resolve_existing_repo_path(&root, &fixture.source)?;
        let source_bytes =
            fs::read(&source_path).with_context(|| format!("read source for {}", fixture.id))?;
        ensure!(
            sha256_hex(&source_bytes) == fixture.source_sha256,
            "{}: source hash does not match the corpus manifest",
            fixture.id
        );
        let source = std::str::from_utf8(&source_bytes)
            .with_context(|| format!("{}: SVG source is not UTF-8", fixture.id))?;
        sources.push(source.to_owned());
        validate_static_source(source, &fixture.id)?;
        let legacy = match grida::import::svg::pack::from_svg_str(source) {
            Ok(_) => EnginePreflight::Accepted,
            Err(error) => EnginePreflight::Unsupported {
                reason_code: "legacy_preflight_rejected",
                detail: error,
            },
        };
        let chassis = match n0::svg_animation_frame::compile_latest(fixture.source.clone(), source)
        {
            Ok(compiled) => {
                let (width, height) = compiled.viewport();
                if width != manifest.viewport.width as f32
                    || height != manifest.viewport.height as f32
                {
                    EnginePreflight::Unsupported {
                        reason_code: "chassis_viewport_mismatch",
                        detail: format!(
                            "chassis viewport {width}x{height} does not match corpus {}x{}",
                            manifest.viewport.width, manifest.viewport.height
                        ),
                    }
                } else {
                    EnginePreflight::Accepted
                }
            }
            Err(error) => EnginePreflight::Unsupported {
                reason_code: "chassis_preflight_rejected",
                detail: error.to_string(),
            },
        };
        preflights.push(FixturePreflight {
            fixture_id: fixture.id.clone(),
            legacy,
            chassis,
        });
    }
    if require_all_preflights {
        ensure_all_preflights_accepted(&preflights)?;
    }

    let oracle_manifest_path = resolve_existing_repo_path(&root, &manifest.oracle_bake)?;
    let oracle_bytes = fs::read(&oracle_manifest_path)
        .with_context(|| format!("read oracle bake {}", oracle_manifest_path.display()))?;
    let oracle: OracleBakeManifest = serde_json::from_slice(&oracle_bytes)
        .with_context(|| format!("parse oracle bake {}", oracle_manifest_path.display()))?;
    let oracle_pngs = validate_oracle_bake(&root, &manifest, &corpus_hash, &oracle)?;
    let inputs = sources
        .into_iter()
        .zip(oracle_pngs)
        .zip(preflights)
        .map(|((source, oracle_png), preflight)| ValidatedInput {
            source,
            oracle_png,
            legacy_preflight: preflight.legacy,
            chassis_preflight: preflight.chassis,
        })
        .collect();

    Ok(ValidatedCorpus {
        corpus_identity: ArtifactIdentity {
            id: manifest.corpus_id.clone(),
            sha256: corpus_hash,
        },
        oracle_bake_identity: ArtifactIdentity {
            id: format!("chromium-{}", oracle.browser_version),
            sha256: sha256_hex(&oracle_bytes),
        },
        manifest,
        inputs,
    })
}

fn ensure_all_preflights_accepted(preflights: &[FixturePreflight]) -> Result<()> {
    let mut failures = Vec::new();
    for preflight in preflights {
        if let EnginePreflight::Unsupported { detail, .. } = &preflight.legacy {
            failures.push(format!(
                "{}: legacy preflight failed: {detail}",
                preflight.fixture_id
            ));
        }
        if let EnginePreflight::Unsupported { detail, .. } = &preflight.chassis {
            failures.push(format!(
                "{}: chassis preflight failed: {detail}",
                preflight.fixture_id
            ));
        }
    }
    ensure!(
        failures.is_empty(),
        "engine preflight failures:\n{}",
        failures.join("\n")
    );
    Ok(())
}

fn validate_manifest_shape(manifest: &CorpusManifest) -> Result<()> {
    ensure!(manifest.schema_version == 0, "unsupported corpus schema");
    ensure!(manifest.corpus_id == CORPUS_ID, "unexpected corpus id");
    ensure!(manifest.suite_id == SUITE_ID, "unexpected suite id");
    ensure!(
        manifest.request == "static_base",
        "unexpected corpus request"
    );
    ensure!(
        manifest.viewport.width == VIEWPORT_WIDTH && manifest.viewport.height == VIEWPORT_HEIGHT,
        "unexpected corpus viewport"
    );
    ensure!(
        manifest.background == "#ffffff",
        "unexpected corpus background"
    );
    ensure!(
        manifest.oracle_bake == "fixtures/scoreboard/svg-rect-path-v0/oracle-bake.json",
        "unexpected oracle-bake path"
    );
    ensure!(
        manifest.fixtures.len() == EXPECTED_IDS.len(),
        "corpus denominator changed"
    );
    for (fixture, expected_id) in manifest.fixtures.iter().zip(EXPECTED_IDS) {
        ensure!(
            fixture.id == expected_id,
            "corpus rows must equal the fixed sorted denominator"
        );
        ensure!(
            fixture.source == format!("fixtures/scoreboard/svg-rect-path-v0/svg/{expected_id}.svg"),
            "{expected_id}: unexpected source path"
        );
        ensure!(
            fixture.oracle
                == format!("fixtures/scoreboard/svg-rect-path-v0/chromium/{expected_id}.png"),
            "{expected_id}: unexpected oracle path"
        );
        validate_sha256(&fixture.source_sha256, &format!("{expected_id} source"))?;
    }
    ensure!(
        !manifest.excluded_families.is_empty(),
        "corpus excluded-family patrol ledger is empty"
    );
    let mut exclusion_paths = BTreeSet::new();
    for exclusion in &manifest.excluded_families {
        ensure!(
            exclusion.path.starts_with("fixtures/") && !exclusion.path.contains(".."),
            "unsafe exclusion path {}",
            exclusion.path
        );
        ensure!(
            exclusion_paths.insert(&exclusion.path),
            "duplicate exclusion path"
        );
        validate_reason(&exclusion.reason_code, &exclusion.reason)?;
    }
    Ok(())
}

fn validate_closed_source_directory(root: &Path, manifest: &CorpusManifest) -> Result<()> {
    let source_dir = resolve_existing_repo_path(root, "fixtures/scoreboard/svg-rect-path-v0/svg")?;
    let mut actual = fs::read_dir(&source_dir)
        .with_context(|| format!("read source directory {}", source_dir.display()))?
        .map(|entry| {
            let entry = entry?;
            ensure!(
                entry.file_type()?.is_file(),
                "source directory contains a non-file entry"
            );
            Ok(entry.file_name().to_string_lossy().into_owned())
        })
        .collect::<Result<Vec<_>>>()?;
    actual.sort();
    let expected = manifest
        .fixtures
        .iter()
        .map(|fixture| format!("{}.svg", fixture.id))
        .collect::<Vec<_>>();
    ensure!(
        actual == expected,
        "source directory does not equal the enumerated denominator"
    );
    Ok(())
}

fn validate_static_source(source: &str, fixture_id: &str) -> Result<()> {
    let mut reader = Reader::from_reader(source.as_bytes());
    reader.config_mut().trim_text(true);
    let mut buffer = Vec::new();
    loop {
        match reader.read_event_into(&mut buffer) {
            Ok(Event::Start(element)) | Ok(Event::Empty(element)) => {
                let name = element.local_name();
                if !matches!(name.as_ref(), b"svg" | b"rect" | b"path") {
                    bail!(
                        "{fixture_id}: element <{}> is outside the rect/path scoreboard boundary",
                        String::from_utf8_lossy(name.as_ref())
                    );
                }
            }
            Ok(Event::DocType(_)) | Ok(Event::PI(_)) => {
                bail!("{fixture_id}: declarations and processing instructions are forbidden")
            }
            Ok(Event::Eof) => break,
            Ok(_) => {}
            Err(error) => bail!("{fixture_id}: XML preflight failed: {error}"),
        }
        buffer.clear();
    }
    Ok(())
}

fn validate_oracle_bake(
    root: &Path,
    corpus: &CorpusManifest,
    corpus_hash: &str,
    oracle: &OracleBakeManifest,
) -> Result<Vec<Vec<u8>>> {
    ensure!(oracle.schema_version == 0, "unsupported oracle-bake schema");
    ensure!(
        oracle.corpus_id == corpus.corpus_id,
        "oracle-bake corpus id mismatch"
    );
    ensure!(
        oracle.corpus_manifest_sha256 == corpus_hash,
        "oracle-bake corpus hash mismatch"
    );
    ensure!(
        oracle.kind == "chromium",
        "scoreboard oracle is not Chromium"
    );
    ensure!(
        !oracle.browser_version.is_empty(),
        "oracle browser version is empty"
    );
    validate_sha256(&oracle.bake_script_sha256, "oracle bake script")?;
    let script =
        resolve_existing_repo_path(root, "crates/grida_dev/scripts/scoreboard_bake_chromium.ts")?;
    ensure!(
        sha256_hex(&fs::read(script).context("read Chromium bake script")?)
            == oracle.bake_script_sha256,
        "oracle bake script hash mismatch"
    );
    ensure!(
        oracle.capture.width == corpus.viewport.width
            && oracle.capture.height == corpus.viewport.height
            && oracle.capture.device_scale_factor == 1
            && oracle.capture.omit_background
            && oracle.capture.source_transport == "data-url-from-hashed-bytes"
            && !oracle.capture.javascript_enabled
            && !oracle.capture.network_enabled
            && !oracle.capture.source_mutation
            && !oracle.capture.style_injection
            && !oracle.capture.animation_control,
        "oracle capture contract mismatch"
    );
    validate_oracle_records(corpus, oracle)?;

    let oracle_dir =
        resolve_existing_repo_path(root, "fixtures/scoreboard/svg-rect-path-v0/chromium")?;
    let mut actual = fs::read_dir(&oracle_dir)
        .with_context(|| format!("read oracle directory {}", oracle_dir.display()))?
        .map(|entry| {
            let entry = entry?;
            ensure!(
                entry.file_type()?.is_file(),
                "oracle directory contains a non-file entry"
            );
            Ok(entry.file_name().to_string_lossy().into_owned())
        })
        .collect::<Result<Vec<_>>>()?;
    actual.sort();
    let expected = corpus
        .fixtures
        .iter()
        .map(|fixture| format!("{}.png", fixture.id))
        .collect::<Vec<_>>();
    ensure!(
        actual == expected,
        "oracle directory does not equal the enumerated denominator"
    );

    let mut oracle_pngs = Vec::with_capacity(corpus.fixtures.len());
    for (fixture, record) in corpus.fixtures.iter().zip(&oracle.fixtures) {
        let oracle_path = resolve_existing_repo_path(root, &fixture.oracle)?;
        let bytes =
            fs::read(&oracle_path).with_context(|| format!("read oracle for {}", fixture.id))?;
        ensure!(
            sha256_hex(&bytes) == record.oracle_sha256,
            "{}: oracle hash mismatch",
            fixture.id
        );
        ensure!(
            image::image_dimensions(&oracle_path)?
                == (corpus.viewport.width, corpus.viewport.height),
            "{}: oracle dimensions mismatch",
            fixture.id
        );
        oracle_pngs.push(bytes);
    }
    Ok(oracle_pngs)
}

fn validate_oracle_records(corpus: &CorpusManifest, oracle: &OracleBakeManifest) -> Result<()> {
    ensure!(
        oracle.fixtures.len() == corpus.fixtures.len(),
        "oracle-bake row set is incomplete"
    );
    for (fixture, record) in corpus.fixtures.iter().zip(&oracle.fixtures) {
        ensure!(
            record.id == fixture.id,
            "oracle-bake rows are missing, reordered, or duplicated"
        );
        ensure!(
            record.source_sha256 == fixture.source_sha256,
            "{}: oracle source hash mismatch",
            fixture.id
        );
        validate_sha256(&record.oracle_sha256, &format!("{} oracle", fixture.id))?;
    }
    Ok(())
}

fn resolve_existing_repo_path(root: &Path, value: impl AsRef<Path>) -> Result<PathBuf> {
    let value = value.as_ref();
    ensure!(!value.as_os_str().is_empty(), "empty repository path");
    ensure!(
        value
            .components()
            .all(|component| matches!(component, Component::Normal(_))),
        "repository path must be a normalized relative path: {}",
        value.display()
    );
    let path = fs::canonicalize(root.join(value))
        .with_context(|| format!("resolve repository path {}", value.display()))?;
    ensure!(
        path.starts_with(root),
        "repository path escapes the repository root"
    );
    Ok(path)
}

pub(crate) fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(64);
    for byte in digest {
        write!(&mut output, "{byte:02x}").expect("writing to a String cannot fail");
    }
    output
}

fn validate_sha256(value: &str, context: &str) -> Result<()> {
    ensure!(
        value.len() == 64
            && value
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)),
        "{context}: invalid sha256"
    );
    Ok(())
}

fn validate_reason(code: &str, reason: &str) -> Result<()> {
    ensure!(
        !code.is_empty()
            && code
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_'),
        "invalid exclusion reason code"
    );
    ensure!(!reason.trim().is_empty(), "empty exclusion reason");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn committed_corpus_is_closed_and_preflights_both_engines() {
        let validated = validate(&default_corpus_path()).unwrap();
        assert_eq!(validated.manifest.fixtures.len(), EXPECTED_IDS.len());
        assert_eq!(validated.manifest.corpus_id, CORPUS_ID);
    }

    #[test]
    fn oracle_records_fail_closed_on_missing_row_or_hash_mismatch() {
        let corpus = CorpusManifest {
            schema_version: 0,
            corpus_id: CORPUS_ID.into(),
            suite_id: SUITE_ID.into(),
            request: "static_base".into(),
            viewport: Viewport {
                width: VIEWPORT_WIDTH,
                height: VIEWPORT_HEIGHT,
            },
            background: "#ffffff".into(),
            oracle_bake: "fixtures/scoreboard/svg-rect-path-v0/oracle-bake.json".into(),
            fixtures: vec![CorpusFixture {
                id: "rect-solid".into(),
                source: "fixtures/scoreboard/svg-rect-path-v0/svg/rect-solid.svg".into(),
                source_sha256: "a".repeat(64),
                oracle: "fixtures/scoreboard/svg-rect-path-v0/chromium/rect-solid.png".into(),
            }],
            excluded_families: vec![],
        };
        let mut oracle = OracleBakeManifest {
            schema_version: 0,
            corpus_id: CORPUS_ID.into(),
            corpus_manifest_sha256: "b".repeat(64),
            kind: "chromium".into(),
            browser_version: "test".into(),
            bake_script_sha256: "c".repeat(64),
            capture: CaptureContract {
                width: VIEWPORT_WIDTH,
                height: VIEWPORT_HEIGHT,
                device_scale_factor: 1,
                omit_background: true,
                source_transport: "data-url-from-hashed-bytes".into(),
                javascript_enabled: false,
                network_enabled: false,
                source_mutation: false,
                style_injection: false,
                animation_control: false,
            },
            fixtures: vec![],
        };
        assert!(validate_oracle_records(&corpus, &oracle)
            .unwrap_err()
            .to_string()
            .contains("incomplete"));

        oracle.fixtures.push(OracleFixture {
            id: "rect-solid".into(),
            source_sha256: "d".repeat(64),
            oracle_sha256: "e".repeat(64),
        });
        assert!(validate_oracle_records(&corpus, &oracle)
            .unwrap_err()
            .to_string()
            .contains("source hash"));
    }

    #[test]
    fn strict_check_rejects_preflight_coverage_gaps() {
        let preflights = [FixturePreflight {
            fixture_id: "future-row".into(),
            legacy: EnginePreflight::Accepted,
            chassis: EnginePreflight::Unsupported {
                reason_code: "chassis_preflight_rejected",
                detail: "outside the bounded profile".into(),
            },
        }];

        let error = ensure_all_preflights_accepted(&preflights).unwrap_err();
        assert!(error
            .to_string()
            .contains("future-row: chassis preflight failed"));
    }
}
