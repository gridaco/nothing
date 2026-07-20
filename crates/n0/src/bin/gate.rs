//! ENG-0.2 / S-5 · the rig. `cargo run --release --bin gate` runs the checks
//! that keep every optimization honest, before the engine grows:
//!
//! 1. **shots** — on the baseline-owning CI host, the re-hosted spike's
//!    `--shot` output is byte-identical to the committed goldens; other hosts
//!    prove two-run determinism (the spike owns golden pixels).
//! 2. **replays** — each corpus `.replay` plays twice to a bit-identical
//!    document and result sequence (determinism, ENG-5.2).
//! 3. **diff** — the oracle law (ENG-0.2): every render optimization proves
//!    `optimized == reference` (pixel/drawlist). Empty until the first win;
//!    pins the reference oracle's determinism meanwhile.
//! 4. **bench** — resolve + drawlist timings stay within the checked-in
//!    budgets (`rig/baselines.json`), fail past `max(1.5x, +50us)`.
//!
//! `--bless-shots` + `--bless-bench` re-record the host-owned baseline set.
//! Paint TIMING is deliberately not gated (GPU-noisy); paint CORRECTNESS is
//! the shot gate.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use n0::cache::{composited_to_bytes, SceneCache};
use n0::{drawlist, frame, paint, replay};
use n0_model::math::Affine;
use n0_model::model::*;
use n0_model::resolve::{resolve, ResolveOptions, RotationInFlow};

const STATES: [&str; 4] = ["default", "crosszero", "rot45", "ungroup"];
const CORPUS: [&str; 3] = ["crosszero", "rot45", "ungroup"];

fn manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn baselines_path() -> PathBuf {
    manifest().join("rig/baselines.json")
}

fn read_baselines() -> Option<serde_json::Value> {
    std::fs::read_to_string(baselines_path())
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
}

fn baseline_host_id(baselines: &serde_json::Value) -> Option<&str> {
    baselines
        .get("host_id")
        .or_else(|| baselines.get("machine"))
        .and_then(|host| host.as_str())
}

fn current_host_id() -> String {
    std::env::var("N0_GATE_HOST_ID")
        .unwrap_or_else(|_| format!("{}-{}", std::env::consts::ARCH, std::env::consts::OS))
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let bless_shots = args.iter().any(|a| a == "--bless-shots");
    let bless_bench = args.iter().any(|a| a == "--bless-bench");
    if bless_shots != bless_bench {
        eprintln!(
            "shot and timing baselines share one host owner; pass \
             --bless-shots and --bless-bench together"
        );
        std::process::exit(2);
    }
    let baselines = read_baselines();
    let host_id = current_host_id();
    let require_host_baselines = std::env::var_os("N0_GATE_REQUIRE_HOST_BASELINES").is_some();
    if bless_shots {
        let provenance = [
            "N0_GATE_HOST_ID",
            "N0_GATE_SOURCE_SHA",
            "N0_GATE_CI_RUN_ID",
            "N0_GATE_RUST_TOOLCHAIN",
            "N0_GATE_SKIA_SAFE",
        ];
        let missing: Vec<&str> = provenance
            .into_iter()
            .filter(|name| std::env::var(name).map_or(true, |value| value.is_empty()))
            .collect();
        if !require_host_baselines || !missing.is_empty() {
            let missing = if missing.is_empty() {
                "none".to_string()
            } else {
                missing.join(", ")
            };
            eprintln!(
                "baseline candidates require N0_GATE_REQUIRE_HOST_BASELINES and complete CI \
                 provenance; missing: {}",
                missing
            );
            std::process::exit(2);
        }
    }

    println!("== n0 gate ==");
    println!("host: {host_id}");
    let mut ok = true;
    let (shots_ok, shot_candidate) = gate_shots(
        bless_shots,
        baselines.as_ref().and_then(baseline_host_id),
        &host_id,
        require_host_baselines,
    );
    ok &= shots_ok;
    ok &= gate_replays();
    ok &= gate_diff();
    let (bench_ok, bench_candidate) = gate_bench(
        bless_bench,
        baselines.as_ref(),
        &host_id,
        require_host_baselines,
    );
    ok &= bench_ok;

    if ok && bless_shots {
        let Some(bench_candidate) = bench_candidate.as_deref() else {
            eprintln!("baseline candidate is incomplete");
            std::process::exit(1);
        };
        match install_baseline_set(&shot_candidate, bench_candidate) {
            Ok(()) => println!("\n[baseline] complete host-owned set installed"),
            Err(error) => {
                eprintln!("failed to install baseline candidate: {error}");
                std::process::exit(1);
            }
        }
    }

    if ok {
        println!("\nGATE: PASS");
    } else {
        eprintln!("\nGATE: FAIL");
        std::process::exit(1);
    }
}

// ── 1. shots ────────────────────────────────────────────────────────────

fn gate_shots(
    bless: bool,
    baseline_host: Option<&str>,
    host_id: &str,
    require_host_baselines: bool,
) -> (bool, Vec<(PathBuf, Vec<u8>)>) {
    println!("\n[shots] spike --shot vs goldens");
    let spike = manifest().join("../../target/release/n0_dev");
    let goldens = manifest().join("../n0_dev/shots");
    if !spike.exists() {
        eprintln!(
            "  MISSING spike binary: {}\n  build it first: cargo build --release -p n0_dev",
            spike.display()
        );
        return (false, Vec::new());
    }
    let owns_baselines = baseline_host == Some(host_id);
    if !bless && !owns_baselines {
        println!(
            "  host baseline owned by {}; checking same-host determinism only",
            baseline_host.unwrap_or("MISSING")
        );
    }
    let mut all = true;
    let mut candidate = Vec::new();
    for state in STATES {
        let tmp = std::env::temp_dir().join(format!("n0-gate-{state}.png"));
        let status = Command::new(&spike)
            .arg("--shot")
            .arg(&tmp)
            .arg(state)
            .status();
        let golden = goldens.join(format!("{state}.png"));
        if bless {
            match (status, std::fs::read(&tmp)) {
                (Ok(status), Ok(bytes)) if status.success() => {
                    candidate.push((golden, bytes));
                    println!("  {state:10} captured");
                }
                _ => {
                    eprintln!("  {state:10} FAILED to produce a baseline");
                    all = false;
                }
            }
            continue;
        }
        let same = if owns_baselines {
            matches!(status, Ok(s) if s.success()) && files_equal(&tmp, &golden)
        } else {
            let repeat = std::env::temp_dir().join(format!("n0-gate-{state}-repeat.png"));
            let repeat_status = Command::new(&spike)
                .arg("--shot")
                .arg(&repeat)
                .arg(state)
                .status();
            matches!(status, Ok(s) if s.success())
                && matches!(repeat_status, Ok(s) if s.success())
                && files_equal(&tmp, &repeat)
        };
        let verdict = if owns_baselines {
            if same {
                "IDENTICAL"
            } else {
                "DIFF"
            }
        } else if same {
            "DETERMINISTIC"
        } else {
            "NONDETERMINISTIC"
        };
        println!("  {state:10} {verdict}");
        all &= same;
    }
    if !bless && baseline_host.is_none() {
        eprintln!("  committed shot baseline has no host owner");
        all = false;
    } else if !bless && !owns_baselines && require_host_baselines {
        eprintln!("  REQUIRED host-owned shot baselines are absent");
        all = false;
    }
    (all, candidate)
}

fn install_baseline_set(shots: &[(PathBuf, Vec<u8>)], bench: &str) -> std::io::Result<()> {
    if shots.len() != STATES.len() {
        return Err(std::io::Error::other("shot candidate is incomplete"));
    }

    let stage = manifest().join(format!(
        "../../target/n0-gate-baseline-candidate-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&stage)?;
    let mut staged_shots = Vec::with_capacity(shots.len());
    for (destination, bytes) in shots {
        let filename = destination
            .file_name()
            .ok_or_else(|| std::io::Error::other("shot baseline has no filename"))?;
        let staged = stage.join(filename);
        std::fs::write(&staged, bytes)?;
        staged_shots.push((staged, destination));
    }
    let staged_bench = stage.join("baselines.json");
    std::fs::write(&staged_bench, bench)?;

    for (staged, destination) in staged_shots {
        std::fs::copy(staged, destination)?;
    }
    std::fs::copy(staged_bench, baselines_path())?;
    let _ = std::fs::remove_dir_all(stage);
    Ok(())
}

fn files_equal(a: &Path, b: &Path) -> bool {
    match (std::fs::read(a), std::fs::read(b)) {
        (Ok(x), Ok(y)) => x == y,
        _ => false,
    }
}

// ── 2. replays ──────────────────────────────────────────────────────────

fn gate_replays() -> bool {
    println!("\n[replays] play twice, bit-identical");
    let dir = manifest().join("../../fixtures/n0-replay");
    let mut all = true;
    for name in CORPUS {
        let path = dir.join(format!("{name}.replay"));
        let text = match std::fs::read_to_string(&path) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("  {name:10} unreadable: {e}");
                all = false;
                continue;
            }
        };
        let rep = match replay::parse_string(&text) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("  {name:10} parse error: {e}");
                all = false;
                continue;
            }
        };
        let (d1, res1) = replay::play(&rep).expect("corpus oracle must be supported");
        let (d2, res2) = replay::play(&rep).expect("corpus oracle must be supported");
        let deterministic = n0_model::textir::print(&d1) == n0_model::textir::print(&d2)
            && replay::resolved_bits_eq(&resolve(&d1, &rep.opts), &resolve(&d2, &rep.opts))
            && res1 == res2;
        println!(
            "  {name:10} {} ({} op{})",
            if deterministic {
                "DETERMINISTIC"
            } else {
                "DIVERGED"
            },
            rep.ops.len(),
            if rep.ops.len() == 1 { "" } else { "s" }
        );
        all &= deterministic;
    }
    all
}

// ── 3. differential (the oracle law, ENG-0.2) ─────────────────────────────

/// Every render optimization ships a row here proving `optimized(input) ==
/// reference(input)` — a fast-but-wrong cache aborts the build before anyone
/// reads a speedup. Today it holds ZERO optimization rows (each lands with its
/// win); it instead pins the L2 reference oracle itself — checked frame raster
/// must be deterministic (ENG-0.3), so every future pixel row can trust it.
fn gate_diff() -> bool {
    println!("\n[diff] oracle-law differential (ENG-0.2)");
    let opts = ResolveOptions {
        viewport: (2000.0, 1400.0),
        rotation_in_flow: RotationInFlow::VisualOnly,
    };
    let ctx = paint::PaintCtx::new(None);
    let view = Affine::scale(0.6, 0.6); // fit-ish so the frame has real draws
    let (w, h) = (1360, 900);
    let mut all = true;
    for name in CORPUS {
        let path = manifest().join(format!("../../fixtures/n0-replay/{name}.replay"));
        let rep = match std::fs::read_to_string(&path)
            .ok()
            .and_then(|t| replay::parse_string(&t).ok())
        {
            Some(r) => r,
            None => {
                eprintln!("  {name:10} corpus unreadable");
                all = false;
                continue;
            }
        };
        let (doc, _) = replay::play(&rep).expect("corpus oracle must be supported");
        let product = frame::resolve_and_build(&doc, &opts, &ctx)
            .expect("gate corpus frame must pass paint preflight");
        let a = product
            .raster_to_bytes(&view, w, h, &ctx)
            .expect("gate context is unchanged");
        let b = product
            .raster_to_bytes(&view, w, h, &ctx)
            .expect("gate context is unchanged");
        let same = a == b;
        println!(
            "  {name:10} reference {}",
            if same {
                "DETERMINISTIC"
            } else {
                "NONDETERMINISTIC"
            }
        );
        all &= same;

        // Win 1 · scene raster cache (L2): an integer-pan blit is byte-identical
        // to a fresh render. Prime the cache at `view`, pan by a whole pixel,
        // and compare the composite against a fresh render at the panned view.
        let panned = Affine {
            e: view.e + 40.0,
            f: view.f + 24.0,
            ..view
        };
        let mut cache = SceneCache::new(w, h);
        let _ = composited_to_bytes(&mut cache, &doc, &opts, &view, &ctx, false, w, h)
            .expect("gate cache-cold frame must pass paint preflight");
        let blit = composited_to_bytes(&mut cache, &doc, &opts, &panned, &ctx, false, w, h)
            .expect("gate cached frame must pass paint preflight");
        let fresh = {
            let product = frame::resolve_and_build(&doc, &opts, &ctx)
                .expect("gate fresh frame must pass paint preflight");
            product
                .raster_to_bytes(&panned, w, h, &ctx)
                .expect("gate context is unchanged")
        };
        let cache_ok = blit == fresh;
        println!(
            "  {name:10} scene-cache  {}",
            if cache_ok {
                "MATCHES fresh (integer pan)"
            } else {
                "DIFFERS"
            }
        );
        all &= cache_ok;
    }
    all
}

// ── 4. bench ────────────────────────────────────────────────────────────

fn gate_bench(
    bless: bool,
    prior: Option<&serde_json::Value>,
    host_id: &str,
    require_host_baselines: bool,
) -> (bool, Option<String>) {
    println!("\n[bench] resolve + drawlist (min of 11, microseconds)");

    // "starter" = the corpus's normalized starter doc (no dependency on the
    // spike's scene builder); "flat10k" = a synthetic stress canvas.
    let starter = starter_doc();
    let starter_opts = ResolveOptions {
        viewport: (2000.0, 1400.0),
        rotation_in_flow: RotationInFlow::VisualOnly,
    };
    let flat = flat_canvas(10_000);

    let measured = [
        ("starter", bench_doc(&starter, &starter_opts)),
        ("flat10k", bench_doc(&flat, &starter_opts)),
    ];

    let prior_host = prior.and_then(baseline_host_id);
    let same_host = prior_host == Some(host_id);

    let mut all = true;
    for (name, (r_us, b_us)) in measured {
        print!("  {name:10} resolve {r_us:8.1}  build {b_us:8.1}");
        if bless {
            println!("   (recording)");
            continue;
        }
        let Some(prior) = prior else {
            println!("   MISSING baseline");
            all = false;
            continue;
        };
        if prior_host.is_none() {
            println!("   MISSING baseline owner");
            all = false;
            continue;
        }
        if !same_host {
            println!(
                "   (baseline owned by {} — comparison skipped)",
                prior_host.unwrap_or("MISSING")
            );
            all &= !require_host_baselines;
            continue;
        }
        let base = prior.get("entries").and_then(|e| e.get(name));
        let br = base
            .and_then(|b| b.get("resolve_us"))
            .and_then(|v| v.as_f64());
        let bb = base
            .and_then(|b| b.get("build_us"))
            .and_then(|v| v.as_f64());
        let r_ok = br.map(|base| within(r_us, base)).unwrap_or(false);
        let b_ok = bb.map(|base| within(b_us, base)).unwrap_or(false);
        println!(
            "   resolve {}  build {}",
            verdict(r_ok, r_us, br),
            verdict(b_ok, b_us, bb)
        );
        all &= r_ok && b_ok;
    }

    let candidate = if bless {
        let json = serde_json::json!({
            "host_id": host_id,
            "source_sha": std::env::var("N0_GATE_SOURCE_SHA").unwrap_or_else(|_| "unrecorded".into()),
            "ci_run_id": std::env::var("N0_GATE_CI_RUN_ID").unwrap_or_else(|_| "unrecorded".into()),
            "rust_toolchain": std::env::var("N0_GATE_RUST_TOOLCHAIN").unwrap_or_else(|_| "unrecorded".into()),
            "skia_safe": std::env::var("N0_GATE_SKIA_SAFE").unwrap_or_else(|_| "unrecorded".into()),
            "note": "CI-owned min-of-11 microseconds; regenerate shots and bench together after etiology",
            "entries": {
                "starter": { "resolve_us": measured[0].1.0, "build_us": measured[0].1.1 },
                "flat10k": { "resolve_us": measured[1].1.0, "build_us": measured[1].1.1 },
            }
        });
        Some(serde_json::to_string_pretty(&json).unwrap() + "\n")
    } else {
        None
    };
    (all, candidate)
}

/// Budget rule: pass under `max(1.5x baseline, baseline + 50us)` — the floor
/// stops timer noise from failing sub-microsecond-ish entries.
fn within(measured: f64, baseline: f64) -> bool {
    measured <= (baseline * 1.5).max(baseline + 50.0)
}

fn verdict(ok: bool, measured: f64, baseline: Option<f64>) -> String {
    match baseline {
        Some(b) => format!("{}({measured:.1}/{b:.1})", if ok { "OK " } else { "OVER " }),
        None => "MISSING".to_string(),
    }
}

fn bench_doc(doc: &Document, opts: &ResolveOptions) -> (f64, f64) {
    let mut r_min = f64::MAX;
    let mut b_min = f64::MAX;
    for _ in 0..11 {
        let t0 = Instant::now();
        let resolved = resolve(doc, opts);
        let t1 = Instant::now();
        // The replay benchmark deliberately measures the deterministic lab
        // oracle. Shaped rendering enters through `frame::resolve_and_build`.
        let _ = drawlist::build_glyphless_unchecked(doc, &resolved);
        let t2 = Instant::now();
        r_min = r_min.min((t1 - t0).as_secs_f64() * 1e6);
        b_min = b_min.min((t2 - t1).as_secs_f64() * 1e6);
    }
    (r_min, b_min)
}

fn starter_doc() -> Document {
    // The normalized starter IR lives in every corpus replay's header.
    let path = manifest().join("../../fixtures/n0-replay/crosszero.replay");
    let text = std::fs::read_to_string(&path).expect("corpus present for bench");
    replay::parse_string(&text).expect("parse corpus").doc
}

fn flat_canvas(n: usize) -> Document {
    let mut b = DocBuilder::new();
    for i in 0..n {
        let mut h = Header::new(SizeIntent::Fixed(40.0), SizeIntent::Fixed(28.0));
        h.x = AxisBinding::start((i % 100) as f32 * 19.0);
        h.y = AxisBinding::start((i / 100) as f32 * 13.0);
        h.rotation = (i % 7) as f32 * 5.0;
        b.add(
            0,
            h,
            Payload::Shape {
                desc: ShapeDesc::Rect,
            },
        );
    }
    b.build()
}
