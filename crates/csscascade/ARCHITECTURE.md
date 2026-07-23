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

### `adapter`

The `HtmlNode`, `HtmlElement`, and `HtmlDocument` handles implement the traits
Stylo and `selectors` require. Despite their historical names, those handles
also represent elements in SVG and other namespaces.

The adapter currently stores the active `DemoDom` behind a process-global raw
pointer. `bootstrap_dom` replaces that pointer and leaks the previous arena.
Because a handle contains only a `NodeId`, it resolves against whichever arena
is current. Separate consumers cannot safely overlap sessions, even if each
consumer owns a crate-local mutex.

This lifetime model is an implementation defect to remove, not an API to
preserve.

### `cascade`

`CascadeDriver` owns Stylo's `Stylist`, stylesheet lock, snapshots, animation
set, and thread-local traversal context. Construction collects embedded
`<style>` blocks and installs the current compact user-agent stylesheet.
`flush` commits stylesheet changes; `style_document` traverses the document and
attaches computed values to each element.

The driver does not copy computed values into a second styled-tree structure.
Consumers inspect the resolved element data and normalize the fields they own.

## Call sequence

```text
DemoDom::parse_from_bytes
    -> CascadeDriver::new(&dom)
    -> adapter::bootstrap_dom(dom)
    -> CascadeDriver::flush(document)
    -> CascadeDriver::style_document(document)
    -> consumer traversal and normalization
```

The driver must be created before `bootstrap_dom` moves the arena. The entire
sequence must currently run inside one process-wide critical section supplied
by the host.

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

Unit and integration tests cover DOM adaptation, typed Stylo SVG paint
properties, and dependency provenance. `resolve_and_print` is the only example
target because it exercises the live implementation rather than a parallel
prototype.
