---
name: links
description: >
  Write any link or URL so it resolves where it is rendered, not where it
  lives in the repo. Use when authoring or editing a link — in docs/wg specs,
  crate READMEs and docstrings, the npm-published wasm package README, or
  skills. This is the engine-repo variant of the grida links doctrine.
---

# Links (engine repo)

A link is correct or broken **at the surface it is rendered on**, for **that
surface's audience** — not where the text lives in the repo. Before you write
any link: (1) where is this text rendered? (2) who reads it there? (3) when
they click, where does it resolve *from that host* — and is that the thing
you meant?

## Surfaces in this repo

| Surface                | Lives in                                    | Audience         |
| ---------------------- | ------------------------------------------- | ---------------- |
| GitHub (repo browse)   | everything — incl. `docs/wg/**` (also published by this repo's `www/` docs app, which resolves the same `.md`-suffixed relative links; GitHub remains the canonical host) | engine developers |
| npm (`npmjs.com`)      | `crates/grida-canvas-wasm` package README   | package users    |
| Raw / no host          | IDE, stack traces, compiled output          | whoever has it   |

## First-party hosts

| URL                                  | What                                                                                            |
| ------------------------------------ | ----------------------------------------------------------------------------------------------- |
| `https://github.com/gridaco/nothing` | **This repo.** File → `…/blob/main/<path>`; dir → `…/tree/main/<path>`; `main` only, never a SHA |
| `https://github.com/gridaco/grida`   | The product monorepo (editor, packages, service docs). Same file/dir forms                       |
| `https://grida.co`                   | The product · `https://grida.co/docs` = the product docs site (publishes **grida's** docs tree)  |

## Decision table

| Rendered in                       | Target                                   | Use                                                       |
| --------------------------------- | ---------------------------------------- | --------------------------------------------------------- |
| anywhere in this repo             | anything in this repo (crates, docs/wg, fixtures, format) | **relative repo path** — same host (GitHub) everywhere |
| anywhere in this repo             | a grida-repo file/dir                    | **absolute** `https://github.com/gridaco/grida/blob|tree/main/<path>` |
| anywhere in this repo             | a published grida docs page (staying clusters: platform, ai, …) | hosted URL `https://grida.co/docs/<path>` |
| npm package README                | anything in-repo                         | **absolute** `https://github.com/gridaco/nothing/blob/main/<path>` — relative dies on npmjs.com |
| any                               | external                                 | `https://…` as-is                                         |

## Hard rules

- **Never author `https://grida.co/docs/wg/<engine-cluster>/…` links.** The
  docs site does not publish this repo's wg tree (canvas, format, research,
  engine feat-*). Old published URLs survive via a redirect shim in grida —
  do not author *new* links through a redirect; use the same-repo relative
  path (in-repo surfaces) or this repo's absolute GitHub URL (external
  surfaces).
- **Cross-repo links are always absolute.** A relative path can never reach
  the other repo.
- **`main` only** — never pin a commit/branch SHA.
- **Cross-host `#fragment`:** link the file, not a guessed anchor, unless
  verified in the target host's slug scheme.
- **No local-only references — clean them before commit**: absolute machine
  paths (`/Users/...`), `/tmp`, untracked or gitignored files
  (`fixtures/local/**` may be *described* as local-only, never linked as if
  it resolves). If `git ls-files` doesn't show it and it isn't a public URL,
  it does not exist for the audience.
