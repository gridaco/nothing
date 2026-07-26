# csscascade

`csscascade` is the engine's internal Stylo bridge. It parses a namespace-aware
document, adapts that document to Stylo's DOM traits, and resolves browser-grade
computed CSS values. It does not perform layout or painting.

The crate has one implementation:

| Module    | Ownership                                                        |
| --------- | ---------------------------------------------------------------- |
| `dom`     | Frozen, arena-allocated `DemoDom` parsed by html5ever            |
| `adapter` | Stylo and selectors trait implementations over that DOM          |
| `cascade` | Stylesheet collection, Stylo setup, and computed-style traversal |

`DemoDom` and the `Html*` adapter names predate their use for namespace-aware
HTML and SVG content. They are current implementation names, not a claim that
the production surface is HTML-only.

## Pipeline

```text
source bytes
    -> DemoDom
    -> DocumentSession
    -> CascadeDriver::new(&mut session).style_document()
    -> computed values attached to element data
    -> session-bound read handles
    -> semantic consumer
```

The semantic consumer owns normalization into its downstream contract.
`csscascade` never owns a renderer IR, layout tree, graphics backend, resource
loader, or source I/O policy.

`DocumentSession` owns one frozen DOM and its Stylo lock/data. `HtmlDocument`,
`HtmlNode`, and `HtmlElement` are pointer-sized `Copy` handles whose Rust
lifetime is borrowed from that session. Their node identity includes the
session, so arena-local identifiers from separate live documents cannot
cross-resolve. Styling exclusively borrows the session and consumes the
`CascadeDriver`; read handles can exist before or after that pass, never
alongside mutation-capable cascade state.

## Diagnostic

The retained example exercises the live path and prints resolved longhands:

```sh
cargo run -p csscascade --example resolve_and_print
cargo run -p csscascade --example resolve_and_print -- fixtures/test-html/L0/hello.html
```

## Known constraints

- Cascade environment values such as viewport, device-pixel ratio, color
  scheme, and pointer capabilities are not yet a complete explicit host input.
- External stylesheets and other resources are not loaded here. A host must
  declare and resolve resources outside the pure cascade operation.
- SVG paint properties are available through the workspace's official Stylo
  revision, but source-to-cascade ingress and semantic consumption remain
  producer work.
- The session is deliberately not declared `Send` or `Sync`: Stylo's computed
  element data uses interior mutation during the exclusive cascade pass.

The consolidation constraints are defined by the
[Web-First Amendment](../../docs/wg/consolidation/web-first.md). The current
module and lifetime details are recorded in [ARCHITECTURE.md](./ARCHITECTURE.md).

## License

MIT or Apache-2.0
