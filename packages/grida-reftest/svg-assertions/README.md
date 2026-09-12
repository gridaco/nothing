# SVG assertions

A developer harness for **one described claim, one discrete verdict**. It runs
the actual n0 CLI in strict and best-effort modes, captures pinned Chromium,
and preserves separately named image comparisons and diagnostics. No image
similarity score, pass-rate percentage, or SVG completeness estimate exists in
this tool. The reviewed proposal is
[gridaco/nothing#140](https://github.com/gridaco/nothing/issues/140).

This directory uses the package's installed dependencies only. It is not an
exported SDK or part of the legacy `reftest` command. It imports no legacy
scoring, renderer selection, or automatic reference-selection code.

## Run the pilot

From the repository root, use the normal [engine setup](../../../docs/contributing/setup.md)
and the Node/pnpm versions pinned by the repository, then:

```sh
pnpm install --frozen-lockfile
pnpm -C packages/grida-reftest exec playwright install chromium
just -f fixtures/web-first/justfile assertions-test
mkdir -p target
just -f fixtures/web-first/justfile assertions-gate "$PWD/target/svg-assertion-pilot"
```

The output directory must **not already exist**; its parent must exist. Each
new run needs a new name. Nothing is blessed, overwritten, or deleted. Open
`index.html` in that output directory for the images and explanations;
`report.json` includes commands, diagnostics, both repeats, input/tool hashes,
and every available named comparison. Output artifacts are local or CI review
artifacts, not a new committed corpus.

The pilot requires no downloaded resvg corpus, system font, external resource,
or resvg executable. It points to two ordinary registered Web-first cells and
one existing refusal witness. Their original fixture gates still apply.

The two positive cells use the unchanged
`tests/shapes/rect/simple-case.svg` from
[resvg-test-suite](https://github.com/linebender/resvg-test-suite), revision
`d8e064337faf01bc5a9579187a56dbdbe3eacc72`, and a separately identified derivative
changing only the subject's `green` fill to `blue`. The
[MIT notice](../../../fixtures/web-first/LICENSE.resvg) applies to both.
Both render at a 500×500 initial viewport; their `viewBox` remains untouched.
The fresh Chromium color pair must differ. Each complete output must match its
own committed Chromium reference, not the other color's image. The refusal
witness checks the exact current CLI declarations for geometry units; a pass
there is labelled **expected refusal**, never rendering support.

## Ordinary-opacity regression suite

[opacity-source.json](./opacity-source.json) applies the same instrument to
six described rendering claims and two expected refusals from
[gridaco/nothing#136](https://github.com/gridaco/nothing/issues/136).
The blur, repeating-pattern and radial-gradient scenes each pair group
opacity `.999` with `.998`: a one-pixel discrepancy still fails the complete
scene, and each control must change Chromium's pixels. Sheared group and
root-path cases check named refusals separately, not fallback-image accuracy.

```sh
just -f fixtures/web-first/justfile assertions-opacity-gate "$PWD/target/svg-assertion-opacity"
```

This uses a fresh output directory under the same setup and immutable-output
rules as the pilot. CI runs both suites and retains both reports. No resvg
reference, new tolerance, or claim of complete opacity support is introduced.

## A case is not its filename

```text
Input identity + declared environment + described claim
                         |
                  Renderer observations
                         |
             Independent named comparisons
                         |
             Reviewed executable assertion
                         |
                 PASS / FAIL / UNRESOLVED
```

[pilot.json](./pilot.json) is a complete manifest example. All fields are
mandatory, including explicit `null` where an optional reference or assertion
does not exist. Unknown fields, duplicate IDs, missing controls, invalid hashes,
and empty suites are errors. Paths are relative to the manifest, not the shell.
Source bytes and any baked/stored PNG are SHA-256 pinned. The manifest and tool
identities are copied into the observation record and verified against changes
during the run. Sources are copied into a fresh run directory before rendering;
the authored input is never rewritten to match another renderer's export.

- `description` states the specific claim; it may be `null` only without an
  assertion. Tags or descriptions are not painted into the SVG.
- `review` states whether someone reviewed the source and its environment, why,
  and any explicit blockers. This is a recorded review decision, **not an
  automatic SVG feature detector**. Presence of an element is insufficient.
- `required` controls CI obligation. It is not a support label.
- `assertion: null` produces an observation only. It cannot satisfy a required
  gate even if all pictures happen to match.
- `render-exact` selects a pinned Chromium reference and a different same-size
  Chromium control case with a stated reason. A control must be reviewed,
  repeatable and active. A filename-only placeholder cannot satisfy this check.
- `refusal` declares strict/best-effort exit codes and the exact expected
  diagnostic text. The success footer alone is removed from comparison; raw
  stdout/stderr remain recorded. A missing or additional diagnostic fails.
- `stored` is an optional historical second-opinion PNG. It is never an
  alternate route to passing the declared Chromium assertion.

Manifests and SVG inputs are bounded to 1 MiB each; SVG must be UTF-8.
Canvases are bounded to 1..2048 pixels per axis, and suites to 1..128 cases
per manifest. The initial profile is **static self-contained SVG without text,
external resources, or unresolved export sizing**. The review must establish
that profile; the harness does not add a parser, matcher, font resolver, or I/O
policy to make an ineligible case fit. Declare the limitation in `blockers`.
Chromium's failed resource requests are also retained as capture failures.

## Read results correctly

| Result                  | Meaning                                                                                                                                        |
| ----------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------- |
| `PASS` / `render-exact` | Both CLI admissions render without degradation and match the declared reference; repeats and fresh Chromium agree; the control changes pixels. |
| `PASS` / `refusal`      | The exact named refusal/degradation contract was verified. No rendering-support claim.                                                         |
| `FAIL`                  | The declared assertion was violated, including unexpected output, diagnostics, refusal, or unstable CLI rendering.                             |
| `UNRESOLVED`            | The input/reference/environment cannot support a judgment, or the claimed control is not discriminating. Not a hidden zero or pass.            |
| `OBSERVATION`           | No executable assertion was declared. Raw comparisons are evidence only.                                                                       |

The normal command exits nonzero unless all required assertions pass and every
requested case is accounted for, with no run-integrity failure. A required case
that cannot run cannot silently disappear. No-required-case manifests do not
certify a gate. `--observe` explicitly runs a non-gating exploration; it still
fails on run-integrity errors, and labels the invocation as observe-only.

Exact means **dimensions and all decoded RGBA bytes**, including RGB under
zero alpha. Compressed PNG bytes are recorded separately. Differences show
locations, maximum channel deltas, alpha differences and invisible-RGB
attribution, not normalized similarity. Repeat captures from one renderer also
require stable PNG bytes. No masking, blanket AA exclusion, resizing, background
flattening, or tolerance is available in this profile. Existing narrower
fixture exceptions are not extended to these cases.

The decoder admits noninterlaced PNGs with 8-bit channels or indexed palettes
(1/2/4/8-bit indices, 8-bit palette channels). Palette transparency retains its
hidden RGB. Non-palette `tRNS`, 16-bit channels, other channel depths, interlace
and animated PNG chunks explicitly refuse: this decoder cannot preserve their
meaning within the pilot's lossless, bounded RGBA8 contract. A required image
that cannot be decoded cannot pass. Complete chunk framing, CRCs, one leading
header, strict zlib checksums/input consumption and exact filtered-scanline
length are checked before accepting pixels. Decompression has a hard output
cap, PNG files are bounded to 32 MiB before reading, and nonregular file inputs
(including FIFOs) refuse without waiting for a writer. Only first samples for the current case retain decoded
buffers; repeats retain hashes, and cross-case controls reload one checked pair
at a time. Corpus size does not multiply the live decoded-image working set.

The viewer retains omissions next to best-effort pictures. Two missing images
can look identical; that never satisfies an undegraded rendering assertion.
A mismatch against a chosen reference is not rescued by matching resvg or a
historical PNG. A standards disagreement remains a separately reviewed question;
this tool does not turn an implementation majority into an oracle.

## Optional second opinions and exploration

For local exploration, explicitly download a pinned upstream corpus (local-only;
see the provisioning work in
[gridaco/nothing#7](https://github.com/gridaco/nothing/issues/7)). Write a scratch
manifest using the same contract; identify the original sources and optional
stored PNGs by hash. Do not import upstream result tables or score reports.

Run the CLI directly to add a pinned current resvg executable:

```sh
pnpm -C packages/grida-reftest exec tsx svg-assertions/cli.ts \
  --manifest /absolute/path/to/scratch-manifest.json \
  --out /absolute/path/to/new-output-directory \
  --observe \
  --resvg /absolute/path/to/resvg \
  --resvg-sha256 EXACT_EXECUTABLE_SHA256 \
  --resvg-version '0.47.0'
```

The paths and hash above are placeholders, not files included in this repo.
Record the installed binary's actual hash and version; installing resvg is an
explicit local step. It runs with system fonts disabled, on the same copied
source, and twice. Ineligible/unreviewed sources are not sent to resvg. Its
absence, identity mismatch, warning or render failure remains visibly distinct
from the assertion's chosen reference. Initial viewport and export scaling are
not interchangeable: the manifest review must establish alignment before
interpreting this additional comparison.

The runner builds `n0_cli` once and then uses `cargo run -p n0_cli --bin n0`
for every actual render. Execution is sequential and bounded: twenty minutes for
the build, one minute per render/capture process, and bounded captured output.
Timeouts terminate the process group. Chromium runs through
[the sole capture module](../../../fixtures/web-first/chromium_capture.ts)
in a bounded worker; no capture posture is duplicated here.

The cold-build allowance is for compilation, not slow rendering: the pinned
Linux Skia GL/SVG/WebP feature combination has no matching prebuilt archive,
and hosted source compilation exceeded the initial ten-minute build bound.
The enclosing CI job allows its existing 45 minutes plus 20 for this added
build. It does not change build features to match a different binary archive.

Run on a stable checkout without concurrent Cargo builds/tests or edits to the
inputs and tools. Cargo lock contention can consume a render's timeout; changing
a recorded executable or input invalidates the run. Keep that failed report and
rerun in a new directory once the environment is stable, without weakening the
assertion or extending a timeout to conceal the cause.

## Promotion and CI

Exploration identifies work. To promote a claim, review its meaning and source
dependencies, keep reductions separate, prove an active control, and register
new committed cells through the existing `just add` / `bake` / `gate` / `status`
workflow. Prove the real code-path gate fails under a deliberate perturbation,
then restore and re-gate. Do not loosen an assertion after observing failure.
The source corpus and
[Web checklist](../../../docs/wg/consolidation/web-checklist.md) remain the
authoritative evidence and work queue; this pilot is not another support list.

Consolidation's seam job runs the synthetic contract tests (zero discovered
tests is a failure) and real CLI pilot
on a clean checkout, then retains its report for review. Changes to this tool
or its dependency pins activate that gate. Existing Rust pixel/refusal tests
continue to run independently. Legacy runner retirement, WPT adapters, wider
resource/font profiles, a general consensus rule, and FLIP are not implemented.
