# csscascade architecture

This file describes the implementation that exists. Consolidation doctrine and
decisions live under [`docs/wg/consolidation`](../../docs/wg/consolidation/).

## Boundary

`csscascade` owns one operation:

```text
namespace-aware document + cascade environment
    -> resolved computed values on document elements
```

Its inputs and outputs remain source-semantic. It does not define the
source-neutral resolved-frame contract, layout policy, drawlist, painter, image
decoder, font database, filesystem access, network access, or an ambient clock.

## Live modules

### `dom`

`DemoDom` is the sole document storage. html5ever builds a flat arena whose
nodes have stable `NodeId` indices. Element records retain namespaces,
attributes, parsed inline declarations, selector metadata, and slots for
Stylo's computed element data. The arena is frozen after parsing.

Each parsed DOM owns the exact `SharedRwLock` used for its inline declarations.
Stylesheets, document handles, and the cascade driver clone or borrow that same
lock; data from separate documents never shares a lock accidentally.

### `adapter`

The `HtmlNode`, `HtmlElement`, and `HtmlDocument` handles implement the traits
Stylo and `selectors` require. Despite their historical names, those handles
also represent elements in SVG and other namespaces.

`DocumentSession` owns one boxed session record containing the DOM and one
stable record per node. A handle is one lifetime-bound shared reference to such
a node record. This keeps the concrete handle exactly one machine word, as
Stylo's typeless sharing cache requires, while the record carries both its
owning session and arena-local `NodeId`.

Handle equality and hashing use stable record identity. Stylo's opaque element
and node identities use stable arena-node addresses. Thus equal `NodeId` values
from two live documents remain distinct at every adapter identity boundary.

The stable records contain the adapter's only internal raw owner pointer. It is
sound because the owner is allocated first in a private `Box`, records are then
allocated in a stable boxed slice, neither allocation is moved or replaced,
and every public handle is lifetime-bound to `&DocumentSession`. No public API
can observe a record after its session drops.

### `cascade`

`CascadeDriver` owns Stylo's `Stylist`, stylesheet-lock clone, snapshots, and
animation set. Construction collects embedded `<style>` blocks and installs the
current compact user-agent stylesheet. It exclusively borrows one
`DocumentSession`, and `style_document` consumes the driver. The thread-local
traversal context exists only inside that one pass.

The driver does not copy computed values into a second styled-tree structure.
Consumers inspect the resolved element data and normalize the fields they own.
The exclusive, consuming lifecycle prevents a readable `ElementDataRef` from
overlapping Stylo's unsafe mutable element-data access.

## Call sequence

```text
DemoDom::parse_from_bytes
    -> DocumentSession::new(dom)
    -> CascadeDriver::new(&mut session).style_document()
    -> session.document()
    -> consumer traversal and normalization
```

Multiple sessions may remain live and be read in any order. Their local node
identifiers, style locks, computed values, and adapter handles remain separate.
`DocumentSession` has no manual `Send` or `Sync` implementation.

## Environment

The current device construction still embeds part of the static rendering
environment. A correct host boundary must eventually provide, as explicit
data, at least:

- viewport dimensions;
- device-pixel ratio;
- color-scheme and media preferences;
- pointer and hover capabilities;
- font and other resource environment revisions.

Moving these values into explicit data is separate from changing cascade
semantics. No consumer should infer them from ambient process state.

## Dependency direction

```text
html5ever + Stylo
        |
        v
   csscascade
        |
        v
source-semantic consumers
```

`csscascade` must remain independent of the legacy node model, `.grida`
serialization, layout engines, `rframe`, `n0`, and graphics backends. HTML and
inline SVG share this document and cascade; standalone SVG still needs its
conforming XML grammar entry before joining the same semantic machinery.

## Verification

Unit and integration tests cover independent live sessions, adapter identity,
DOM adaptation, typed Stylo SVG paint properties, and dependency provenance.
`resolve_and_print` is the only example target because it exercises the live
implementation rather than a parallel prototype.
