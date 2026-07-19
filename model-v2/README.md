# model-v2 — node geometry / layout / transform model redesign

Workbench for the fundamental redesign of the Grida node model: how a node's
geometry, position, size, rotation/transform, and layout participation are
represented — in the **Rust engine** (`crates/grida`) and the **format spec**
(`format/grida.fbs`). The proving stack has since grown source-format, engine,
and host harnesses needed to test that model end to end, including Grida XML
ingestion and explicit-time SVG animation. Those remain contained proofs;
production TS/editor, WASM, importer, renderer, and runtime migration are still
out of scope here.

> **Archive note (2026-07-19).** This directory is the **frozen workbench
> archive** of the v2 model program. Its proving stack was promoted into
> the workspace at the landing of
> [gridaco/nothing#5](https://github.com/gridaco/nothing/pull/5):
> `a/lab` → [`crates/n0-model`](../crates/n0-model), `engine/` →
> [`crates/n0`](../crates/n0), `a/spike-canvas` →
> [`crates/n0_dev`](../crates/n0_dev). What remains here — the phase
> papers, the experiment dirs with their frozen outputs and verdicts, the
> demo pages, and the format drafts — is the decision record, kept
> verbatim, with file and directory names canonicalized at landing (the
> workbench dir `a/` is now [`anchor/`](./anchor/); the experiment dirs
> dropped their `eN-` ledger prefixes; the candidate models dropped
> their letter slots — the ledger ids remain as the register keys in
> prose and in [`anchor/README.md`](./anchor/README.md)'s table).
> **Relative paths inside the frozen papers refer to the pre-promotion
> layout** (`a/lab`, `engine/`, `a/spike-canvas`); follow the map
> above. Tracking issue:
> [gridaco/nothing#9](https://github.com/gridaco/nothing/issues/9)
> (formerly gridaco/grida#957, transferred at the engine split).
> Implementation status and module boundaries live with the engine:
> [`crates/n0/ANIMATION.md`](../crates/n0/ANIMATION.md).

## Run

```sh
# the model crate (formerly a/lab) — full conformance suite
cargo test -p n0-model

# the dev shell (formerly a/spike-canvas) — native skia window on the model
cargo run --release -p n0_dev

# the demo pages (proof, model walkthrough, edge cases, DEC-0 fork, free editing)
python3 -m http.server 4173 --directory model-v2/anchor/.preview
```

## Why this exists

The current system answers the same question three different ways:

- leaf nodes: baked `AffineTransform` + `size`
- containers: `position` enum + `rotation: f32` scalar + `layout_dimensions`
- format spec (`LayerTrait`): `layout` + `post_layout_transform` (unimplemented, self-flagged provisional)

reconciled at runtime by a lossy, branchy resolver. This was never reconciled
because the underlying questions were never decided. This directory decides
them — problems first, then candidates, then spec.

## Phase discipline

| phase                   | artifact                                                       | status                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| ----------------------- | -------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1. Problems & harnesses | `problems.md`, `harnesses.md`, `study.md`                      | stable draft                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| 2. Candidate models     | `paradigm.md`, `axes.md`, `models/*`, `finale.md`, `triage.md` | **DECIDED — `anchor`** (+5 triage amendments)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| 3. Spec                 | normative doc + `grida.fbs` draft                              | **experiments RUN, model PROVEN** — E1–E10 complete with verdicts; **DEC-0 decided: VISUAL-ONLY rotation (the CSS framing), CSS-pure sizing** ([`anchor/dec0-visual-only.md`](./anchor/dec0-visual-only.md)); flips built (E-A14, cross-zero resize); conformance lab; native interactive spike (now [`crates/n0_dev`](../crates/n0_dev)); open calls parked in [`anchor/DECISIONS.md`](./anchor/DECISIONS.md). Remaining: fold deltas into a normative rewrite of `models/anchor.md` + WG graduation                                                                  |
| 4. Runtime              | `crates/grida` implementation                                  | **proving engine BUILT and PROMOTED** — [`crates/n0`](../crates/n0) (formerly `engine/`, `anchor-engine`): the `resolve → drawlist → paint` pipeline + query/journal/replay/damage sockets, spike re-hosted onto it, gate green (shots byte-identical, replay deterministic, budgets baselined). The bounded explicit-time SVG animation checkpoint adds pure sampling, Base/Sample frame integration, exact-time rendering, a caller-owned playback clock, and a controlled native host; see [`crates/n0/ANIMATION.md`](../crates/n0/ANIMATION.md). Production migration remains separate |

Ground rules:

- **Problems before solutions.** When a new unclear part arises, it becomes a
  catalog entry — not an inline patch to a proposal.
- **Every claim is evidence-linked** to current code, the format spec, or a
  studied peer system.
- **No candidate survives without a harness run.** `harnesses.md` is the test
  suite for designs.
- An earlier in-chat probe sketched one candidate (scalars-canonical + per-axis
  anchors + layout-visible rotation + a transform-quarantine node). It is
  deliberately **not recorded here as a decision** — it re-enters in phase 2 as
  one candidate among others, subject to the harnesses.

## Files

- [`problems.md`](./problems.md) — the problem catalog (P1–P11): each unclear
  part stated precisely, with its tension, option space, and evidence.
- [`harnesses.md`](./harnesses.md) — the constraints (H1–H10) any candidate
  must pass, each with a concrete pass/fail probe, plus the tension map
  between harnesses.
- [`study.md`](./study.md) — comparative study of peer systems (CSS, SVG,
  Flutter, SwiftUI, Figma, tldraw): facts we reason with, not designs we copy.
- [`paradigm.md`](./paradigm.md) — phase-2 **candidate** paradigm
  ("one box, one way"): nouns, laws, how each problem lands, trades declared,
  falsification criteria. Not ratified.
- [`finale.md`](./finale.md) — the phase-2 finale, **decided: `anchor`**;
  preserved with the pre-decision concession bill and deciding question.
- [`survey.md`](./survey.md) — the 32-question instrument itself, saved
  clean (no answers, no verdict): administration rules, all questions with
  options, and scoring guidance. Reusable for future re-runs or other
  respondents.
- [`triage.md`](./triage.md) — the 2026-07-07 run of the survey: answers,
  scoring key, verdict, the five amendments, and the one open (tilted)
  fork.
- [`editor.md`](./editor.md) — the editor-experience operation catalog:
  every gesture as **gesture → writes → effect → ripple** with stable
  `OP-*` ids, the six operation laws (incl. the three sanctioned
  state→intent bake moments), and per-op FORK marks.
- [`conformance.md`](./conformance.md) — the model-agnostic test corpus:
  metamorphic laws, per-area invariants + edge registries, the executable
  merge matrix, and the compatibility checklist (CSS/Figma/SVG/current
  engine) with Y / N-deviation / spectrum verdicts. FORK rows are the
  finale's probes.
- [`axes.md`](./axes.md) — the decision-space factoring: **Axis 1 = semantic
  model** (`anchor` vs `bake` — decide first), **Axis 2 = representation &
  mutation protocol** (struct vs sheet, key granularity — tunable after,
  bounded by the atom rule). Re-scopes `sheet`; source of harnesses H11/H12.
- [`anchor/`](./anchor/) — **the winner's workbench**: the experiment ledger E1–E10
  (each with verdicts and lab tests), the decision register
  ([`anchor/DECISIONS.md`](./anchor/DECISIONS.md)), the DEC-0 normative rules, the
  ship-readiness census ([`anchor/LIMITS.md`](./anchor/LIMITS.md)), the peer-compat
  matrix, the phase-4 engine layer programs with day-1 contracts
  ([`anchor/ENGINE.md`](./anchor/ENGINE.md)), the Rust conformance lab (now
  [`crates/n0-model`](../crates/n0-model)), and the native interactive
  spike (now [`crates/n0_dev`](../crates/n0_dev)).
- [`models/`](./models/) — concrete candidate models, one file each,
  harness-scored, best-faith. Files carry their working identifiers
  (renamed from letter slots at landing);
  the names are the working identifiers:
  - [`models/anchor.md`](./models/anchor.md) — **`anchor`** (the anchored box model):
    intent-canonical scalars, per-axis bindings, lens quarantine. Proposed
    best fit.
  - [`models/sheet.md`](./models/sheet.md) — **`sheet`** (the property sheet model):
    CSS-faithful flat registry, rulebook conflicts, post-layout transforms.
  - [`models/bake.md`](./models/bake.md) — **`bake`** (the materialized matrix
    model): Figma-faithful matrix + state canonicalism, edit-time layout.
  - [`models/wire.md`](./models/wire.md) — **`wire`** (the wired geometry model):
    relational archetype — referent-general bindings, dataflow DAG, the WG
    Level-4 destination. Priced and deferred; `anchor` grows into it
    additively.

## Relationship to existing docs

- [`docs/wg/feat-layout/index.md`](../docs/wg/feat-layout/index.md) — the
  anchor+flex+grid positioning vision (draft, gridaco/grida#437). It covers positioning
  intent only and is **silent on rotation/transform and their layout
  coupling** — that gap is a large part of this catalog. When phase 3 produces
  a spec, it graduates into `docs/wg/` under WG doctrine (code-agnostic) and
  this directory's evidence links stay behind as the working record.
- [`format/grida.fbs`](../format/grida.fbs) — the current archive draft; its
  header rules (unset-vs-default, tables-over-structs, additive evolution) are
  binding harnesses on whatever phase 3 encodes (see H9).
