# Migration provenance (gridaco/grida → gridaco/nothing)

The engine arrived 2026-07-17 via `git filter-repo` from gridaco/grida
(main @ 2c73d553a, preserved remotely as branch
`snapshot/pre-engine-split-at-202607`), carrying 1,867 commits of history
(2021 → 2026), with the wasm-artifact history carved to tips-only and
commit-message `#NNN` refs rewritten to `gridaco/grida#NNN`.

`grida-migration-commit-map.txt` is the complete **original → final** SHA
bridge, cumulative across all filter passes: one line per pre-migration
commit — surviving commits map to their rewritten SHA, pruned commits
(those touching none of the extracted paths) map to the all-zeros SHA.
Use it to translate any pre-migration SHA (from old issues, PRs, or blame
links) into this repo's history.
