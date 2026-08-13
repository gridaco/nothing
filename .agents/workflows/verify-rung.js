// The checklist loop's pre-landing verification ritual, as a saved workflow.
//
// Invoke with args = { brief: "<the rung brief>" } — a self-contained account
// of the staged (uncommitted) rung: what changed and why, the probe verdicts
// with the probe script path and its exact rerun command, which checklist rows
// tick and on what evidence, and any claim the judges should try to refute.
// The judges run `git status`/`git diff` themselves; the brief is the rung's
// testimony, not a substitute for the tree.
//
// Two judges, the shape proven on the stroke-join and linecap rungs:
// a tick/law auditor and an independent reproducer. Both default to
// skepticism; a finding without evidence is not a finding.

export const meta = {
  name: 'verify-rung',
  description: 'Adversarially verify a staged checklist-loop rung before landing',
  whenToUse:
    'After a rung is fully staged in the working tree and green locally; pass args={brief}.',
  phases: [{ title: 'Verify' }],
}

const VERDICT = {
  type: 'object',
  required: ['verdict', 'findings'],
  properties: {
    verdict: { type: 'string', enum: ['pass', 'must_fix', 'should_fix'] },
    findings: {
      type: 'array',
      items: {
        type: 'object',
        required: ['severity', 'claim', 'evidence'],
        properties: {
          severity: { type: 'string', enum: ['must_fix', 'should_fix', 'note'] },
          claim: { type: 'string' },
          evidence: { type: 'string' },
        },
      },
    },
  },
}

// Accept args as {brief}, as a JSON-encoded string of that shape, or as the
// brief text itself — harnesses differ in how they deliver args.
let input = args
if (typeof input === 'string') {
  try {
    const parsed = JSON.parse(input)
    input = typeof parsed === 'string' ? { brief: parsed } : parsed
  } catch {
    input = { brief: input }
  }
}
const brief = input && input.brief ? String(input.brief) : null
if (!brief) throw new Error('verify-rung requires args = { brief: string } (or the brief text itself)')

const COMMON = `You are verifying a staged, uncommitted rung of the Web checklist loop in this repository (your working directory). Run \`git status --short\` and \`git diff\` to see the change; new files may be untracked under fixtures/web-first/.

THE RUNG BRIEF (the session's testimony — verify it, do not trust it):
${brief}

Standing law you enforce (read the sources, not this summary): the strict tick rule and its named precedents live in the header of docs/wg/consolidation/web-checklist.md; the corpus disciplines (never-overwrite bake, measured-vs-celled markers, no tolerance block without a measured bound) live in fixtures/web-first/README.md; no conformance score or ratio may appear anywhere (FLIP law); probes are scratch and must not be committed; statements of record are linked, never restated.

Return structured findings. Default to skepticism: a finding you cannot evidence is not a finding, but a real gap must be named must_fix.`

phase('Verify')
const results = await parallel([
  () =>
    agent(
      `${COMMON}

You are the TICK + LAW AUDITOR. (1) For every checklist row the diff ticks: verify its FULL listed grammar (fetch the section's cited spec index if needed) now has Chromium-gated committed coverage, judged under the header's rules and precedents — and that no OTHER row is owed a tick or an untick by this change. (2) Audit every prose claim in the diff (README rows, statements of record, code comments) against committed artifacts: probe-only facts must be marked measured-not-celled; a claim of cell coverage that no cell backs is a must_fix. (3) Verify the checklist diff contains exactly the claimed tick flips and nothing else, and that no score, ratio, or machine-specific path appears anywhere in the diff.`,
      { label: 'tick-law-audit', phase: 'Verify', schema: VERDICT },
    ),
  () =>
    agent(
      `${COMMON}

You are the REPRO VERIFIER. (1) Re-run the probe script named in the brief with its stated command and confirm every claimed pair verdict. (2) Run the engine gate (just gate in fixtures/web-first/, or cargo test -p websem --test reftest_oracle) and confirm it passes; confirm from the test source what bar the new cells actually passed (byte-exact unless a tolerance block declares a measured bound). (3) Verify manifest hygiene: fixtures on disk match their entries, primitives.json stayed sorted and additions-only, oracle-bake.json gained exactly the new records with correct hashes. (4) Run the STATUS freshness gate (just status) and confirm no further diff. (5) Spot-check the fixtures' bytes are minimal and their geometry actually discriminates the measured claims.`,
      { label: 'repro', phase: 'Verify', schema: VERDICT },
    ),
])

const [audit, repro] = results
return { audit, repro }
