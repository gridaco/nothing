# Consolidation scoreboard

`grida_dev scoreboard` is the consolidation program's comparison instrument.
It measures the legacy engine and the chassis against the same declared oracle
and against each other. The legacy engine is context, never the oracle.

The instrument is present but scoring is not yet authorized. Registry decision
FLIP remains open in
[gridaco/nothing#49](https://github.com/gridaco/nothing/issues/49), and the
[decision proposal](../../docs/wg/consolidation/flip-rule.md) is explicitly
unratified. Until the owner records GO and that proposal carries a ratification
record, `scoreboard run` refuses before corpus validation or either renderer is
called. The committed Chromium images are oracle inputs, not scores.

## Corpus contract

Scoreboard v0 uses the closed
[`svg-rect-path-v0` corpus](../../fixtures/scoreboard/svg-rect-path-v0/corpus.json).
Its manifest fixes the ordered denominator, source digests, 128×128 viewport,
oracle paths, and an explicit excluded-family patrol ledger. Each included row
sends the identical checked-in SVG bytes to the legacy SVG entry point, the
chassis's bounded authored-Base entry point, and Chromium. No model bridge or
oracle-only source rewrite is permitted.

The corpus is intentionally limited to the direct static rectangle/path
intersection already accepted by both engine entry points. Unsupported source
families remain visible in the exclusion ledger. `scoreboard check` requires
both entry points to accept every row in this fixed v0 corpus. During an
authorized report run, the same preflight is retained as per-engine evidence:
an entry-point rejection becomes `UNSUPPORTED` without calling that renderer,
while a failure after an accepted preflight remains an error. A later admitted
row therefore stays in the denominator with an explicit disposition rather
than being silently removed or classified by matching diagnostic text.

## Commands

Run commands from the repository root.

```sh
# Validate the closed corpus, source/oracle hashes, bake provenance, and both
# parser entry points. This performs no rasterization or image comparison.
cargo run -p grida_dev -- scoreboard check

# Deliberately sealed until FLIP is ratified on gridaco/nothing#49.
cargo run -p grida_dev -- scoreboard run

# Derive a fresh review candidate from an authorized, complete report.
# This never overwrites the committed baseline.
cargo run -p grida_dev -- scoreboard bless
```

The shared path options are `--corpus`, `--report`, and `--baseline` where
applicable. `bless` also accepts `--candidate`. Defaults are:

| Artifact | Default |
|---|---|
| Corpus | `fixtures/scoreboard/svg-rect-path-v0/corpus.json` |
| Report | `target/scoreboard/report-v0.json` |
| Baseline | `fixtures/scoreboard/svg-rect-path-v0/baseline-v0.json` |
| Baseline candidate | `target/scoreboard/baseline-v0.candidate.json` |
| Hard run budget | 120 seconds |

The baseline is intentionally absent before the first authorized score. The
ratified active configuration must declare that absence explicitly; after the
first baseline lands, it pins the required baseline digest. A missing or
mistyped path therefore cannot silently disable regression checks. The
120-second budget is part of the proposed rule identity and has no command-line
override. The timer starts before corpus, oracle, parser, renderer, comparator,
and prior-baseline validation. Those stages run behind a terminal-command
watchdog that returns at the deadline even if an in-process stage is blocked.
The worker has no report path; only a complete in-budget result returns to the
caller that may publish the create-new report.

## Chromium bake

The checked-in Chromium PNGs are produced by
[`scoreboard_bake_chromium.ts`](./scripts/scoreboard_bake_chromium.ts). The
script verifies each source digest, transports that exact hashed buffer to a
JavaScript-disabled, network-disabled browser context without DOM mutation,
style injection, or animation control, captures it twice, and requires
byte-identical PNG output. Repository inputs and output parents must contain no
symlink components. It records the Chromium version, corpus digest,
script digest, capture policy, and per-row oracle digests in
[`oracle-bake.json`](../../fixtures/scoreboard/svg-rect-path-v0/oracle-bake.json).

The bake command is create-new-only. Its default outputs must not already
exist; a re-bake never replaces the committed oracle in place. Prepare a fresh
corpus candidate that preserves the ordered row/source identities but declares
fresh repository-relative `oracle_bake` and per-row `oracle` paths, then pass
all three matching paths:

```sh
pnpm --filter @grida/reftest exec tsx \
  ../../crates/grida_dev/scripts/scoreboard_bake_chromium.ts \
  --corpus target/scoreboard/oracle-candidate/corpus.json \
  --out target/scoreboard/oracle-candidate/chromium \
  --bake-manifest target/scoreboard/oracle-candidate/oracle-bake.json
```

## Report and baseline laws

An authorized report records exact corpus, oracle-bake, scoring-method, rule,
and run identities. Every included row has one tagged cell for each engine and
all three comparisons: legacy-vs-oracle, chassis-vs-oracle, and
legacy-vs-chassis. Comparison cells carry integer differing/scoring-pixel
counts or a reasoned unavailable state. Coverage obeys
`included = scored + unsupported + error` for each engine.

The baseline is a deterministic projection of a valid report. It excludes
host, timestamp, output paths, prior-baseline pointer, and other run-local
data. Baseline candidates are direct create-new files under
`target/scoreboard/`; symlinked directory components are refused, and a
candidate cannot be written directly to the committed baseline path. Blessing
fails closed
for an unratified rule, identity mismatch, incomplete coverage, missing
comparison triple, invalid provenance, or an over-budget run. A scored row
becoming unsupported/error, or an engine-to-oracle differing-pixel count
increasing, is a regression; aggregates cannot hide it.

The threshold and eligibility laws belong to the owner-gated WG decision, not
to the report or bless command. See the consolidation
[method](../../docs/wg/consolidation/method.md) for the full gate lifecycle.

## CI posture

The consolidation workflow runs the synthetic scoreboard contract tests and
`scoreboard check` when engine scope changes. It does not invoke
`scoreboard run` while FLIP is unratified. Once the owner records GO, enabling
the real run and committing the first baseline are a separate consolidation
step with their own gate evidence.
