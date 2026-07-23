# csscascade

`csscascade` is the engine's internal Stylo bridge. It parses a namespace-aware
document, adapts that document to Stylo's DOM traits, and resolves browser-grade
computed CSS values. It does not perform layout or painting.

The crate has one implementation:

| Module | Ownership |
| --- | --- |
| `dom` | Frozen, arena-allocated `DemoDom` parsed by html5ever |
| `adapter` | Stylo and selectors trait implementations over that DOM |
| `cascade` | Stylesheet collection, Stylo setup, and computed-style traversal |

`DemoDom` and the `Html*` adapter names predate their use for namespace-aware
HTML and SVG content. They are current implementation names, not a claim that
the production surface is HTML-only.

## Pipeline

```text
source bytes
    -> DemoDom
    -> adapter::bootstrap_dom
    -> CascadeDriver
    -> computed values attached to element data
    -> semantic consumer
```

The semantic consumer owns normalization into its downstream contract.
`csscascade` never owns a renderer IR, layout tree, graphics backend, resource
loader, or source I/O policy.

## Diagnostic

The retained example exercises the live path and prints resolved longhands:

```sh
cargo run -p csscascade --example resolve_and_print
cargo run -p csscascade --example resolve_and_print -- fixtures/test-html/L0/hello.html
```

## Known constraints

- `adapter::bootstrap_dom` installs the document in a process-global slot and
  intentionally leaks replaced documents. Handles resolve through the current
  slot, so callers must serialize sessions. This is the next lifecycle seam to
  replace; it is not a host contract.
- Cascade environment values such as viewport, device-pixel ratio, color
  scheme, and pointer capabilities are not yet a complete explicit host input.
- External stylesheets and other resources are not loaded here. A host must
  declare and resolve resources outside the pure cascade operation.
- SVG paint properties are available through the workspace's official Stylo
  revision, but source-to-cascade ingress and semantic consumption remain
  producer work.

The consolidation constraints are defined by the
[Web-First Amendment](../../docs/wg/consolidation/web-first.md). The current
module and lifetime details are recorded in [ARCHITECTURE.md](./ARCHITECTURE.md).

## License

MIT or Apache-2.0
