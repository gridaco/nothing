# Web-first capability status

<!-- GENERATED FILE - do not edit. Regenerate:
UPDATE_CAPABILITY_STATUS=1 cargo test -p websem --test capability_status -->

A generated, freshness-gated view of the two corpora that track the
SVG engine of record's capability: what renders (the Chromium-baked
cells) and what departs by name (the refusal register). Every refusal
line is the compiler's actual departure, recompiled by the gate
(`crates/websem/tests/capability_status.rs`), so this file cannot
drift from behavior. The prose statement of record is
[crates/n0_cli/README.md](../../crates/n0_cli/README.md); the rung
history is the
[D-N register](../../docs/wg/consolidation/svg-engine-of-record.md);
the per-fixture rationale is
[unsupported/README.md](./unsupported/README.md).

Not a conformance claim: no score is computed or implied (FLIP is
unratified), and the corpus enumerates constructs, not the SVG
surface.

## Chromium-baked cells (277)

Each renders byte-exact against its committed Chromium oracle
(six curved cells and three gradient ramps carry a declared, bounded
tolerance — see [README.md](./README.md)). Every thumbnail below
*is* that committed oracle, which byte-exactness makes this
engine's own render too; hover for the cell's name, click through
to its fixture source. No new image is committed for this view.

<a href="./html-inline-svg-currentcolor-rect.html" title="html-inline-svg-currentcolor-rect (html-inline-svg)"><img src="./chromium/html-inline-svg-currentcolor-rect.png" width="56" alt="html-inline-svg-currentcolor-rect"></a>
<a href="./html-webpage-mockup.html" title="html-webpage-mockup (html-inline-svg)"><img src="./chromium/html-webpage-mockup.png" width="56" alt="html-webpage-mockup"></a>
<a href="./svg-anchor-container.svg" title="svg-anchor-container (standalone-svg)"><img src="./chromium/svg-anchor-container.png" width="56" alt="svg-anchor-container"></a>
<a href="./svg-circle-defaults-clip.svg" title="svg-circle-defaults-clip (standalone-svg)"><img src="./chromium/svg-circle-defaults-clip.png" width="56" alt="svg-circle-defaults-clip"></a>
<a href="./svg-circle-fill.svg" title="svg-circle-fill (standalone-svg)"><img src="./chromium/svg-circle-fill.png" width="56" alt="svg-circle-fill"></a>
<a href="./svg-circle-viewbox-scaled.svg" title="svg-circle-viewbox-scaled (standalone-svg)"><img src="./chromium/svg-circle-viewbox-scaled.png" width="56" alt="svg-circle-viewbox-scaled"></a>
<a href="./svg-circle-zero-r.svg" title="svg-circle-zero-r (standalone-svg)"><img src="./chromium/svg-circle-zero-r.png" width="56" alt="svg-circle-zero-r"></a>
<a href="./svg-context-paint-attr-fill-from-fill.svg" title="svg-context-paint-attr-fill-from-fill (standalone-svg)"><img src="./chromium/svg-context-paint-attr-fill-from-fill.png" width="56" alt="svg-context-paint-attr-fill-from-fill"></a>
<a href="./svg-context-paint-attr-fill-from-stroke.svg" title="svg-context-paint-attr-fill-from-stroke (standalone-svg)"><img src="./chromium/svg-context-paint-attr-fill-from-stroke.png" width="56" alt="svg-context-paint-attr-fill-from-stroke"></a>
<a href="./svg-context-paint-attr-stroke-from-fill.svg" title="svg-context-paint-attr-stroke-from-fill (standalone-svg)"><img src="./chromium/svg-context-paint-attr-stroke-from-fill.png" width="56" alt="svg-context-paint-attr-stroke-from-fill"></a>
<a href="./svg-context-paint-attr-stroke-from-stroke.svg" title="svg-context-paint-attr-stroke-from-stroke (standalone-svg)"><img src="./chromium/svg-context-paint-attr-stroke-from-stroke.png" width="56" alt="svg-context-paint-attr-stroke-from-stroke"></a>
<a href="./svg-context-paint-bbox-contributors.svg" title="svg-context-paint-bbox-contributors (standalone-svg)"><img src="./chromium/svg-context-paint-bbox-contributors.png" width="56" alt="svg-context-paint-bbox-contributors"></a>
<a href="./svg-context-paint-css-fill-from-fill.svg" title="svg-context-paint-css-fill-from-fill (standalone-svg)"><img src="./chromium/svg-context-paint-css-fill-from-fill.png" width="56" alt="svg-context-paint-css-fill-from-fill"></a>
<a href="./svg-context-paint-css-fill-from-stroke.svg" title="svg-context-paint-css-fill-from-stroke (standalone-svg)"><img src="./chromium/svg-context-paint-css-fill-from-stroke.png" width="56" alt="svg-context-paint-css-fill-from-stroke"></a>
<a href="./svg-context-paint-css-stroke-from-fill.svg" title="svg-context-paint-css-stroke-from-fill (standalone-svg)"><img src="./chromium/svg-context-paint-css-stroke-from-fill.png" width="56" alt="svg-context-paint-css-stroke-from-fill"></a>
<a href="./svg-context-paint-css-stroke-from-stroke.svg" title="svg-context-paint-css-stroke-from-stroke (standalone-svg)"><img src="./chromium/svg-context-paint-css-stroke-from-stroke.png" width="56" alt="svg-context-paint-css-stroke-from-stroke"></a>
<a href="./svg-context-paint-currentcolor-alpha.svg" title="svg-context-paint-currentcolor-alpha (standalone-svg)"><img src="./chromium/svg-context-paint-currentcolor-alpha.png" width="56" alt="svg-context-paint-currentcolor-alpha"></a>
<a href="./svg-context-paint-host-none.svg" title="svg-context-paint-host-none (standalone-svg)"><img src="./chromium/svg-context-paint-host-none.png" width="56" alt="svg-context-paint-host-none"></a>
<a href="./svg-context-paint-inheritance-css-wide.svg" title="svg-context-paint-inheritance-css-wide (standalone-svg)"><img src="./chromium/svg-context-paint-inheritance-css-wide.png" width="56" alt="svg-context-paint-inheritance-css-wide"></a>
<a href="./svg-context-paint-linear-obb-host-box.svg" title="svg-context-paint-linear-obb-host-box (standalone-svg)"><img src="./chromium/svg-context-paint-linear-obb-host-box.png" width="56" alt="svg-context-paint-linear-obb-host-box"></a>
<a href="./svg-context-paint-linear-userspace-host.svg" title="svg-context-paint-linear-userspace-host (standalone-svg)"><img src="./chromium/svg-context-paint-linear-userspace-host.png" width="56" alt="svg-context-paint-linear-userspace-host"></a>
<a href="./svg-context-paint-missing-url-fallback.svg" title="svg-context-paint-missing-url-fallback (standalone-svg)"><img src="./chromium/svg-context-paint-missing-url-fallback.png" width="56" alt="svg-context-paint-missing-url-fallback"></a>
<a href="./svg-context-paint-multi-instance-light-tree.svg" title="svg-context-paint-multi-instance-light-tree (standalone-svg)"><img src="./chromium/svg-context-paint-multi-instance-light-tree.png" width="56" alt="svg-context-paint-multi-instance-light-tree"></a>
<a href="./svg-context-paint-nested.svg" title="svg-context-paint-nested (standalone-svg)"><img src="./chromium/svg-context-paint-nested.png" width="56" alt="svg-context-paint-nested"></a>
<a href="./svg-context-paint-nested-url-owner-box.svg" title="svg-context-paint-nested-url-owner-box (standalone-svg)"><img src="./chromium/svg-context-paint-nested-url-owner-box.png" width="56" alt="svg-context-paint-nested-url-owner-box"></a>
<a href="./svg-context-paint-plain-no-context.svg" title="svg-context-paint-plain-no-context (standalone-svg)"><img src="./chromium/svg-context-paint-plain-no-context.png" width="56" alt="svg-context-paint-plain-no-context"></a>
<a href="./svg-context-paint-radial-obb-host-box.svg" title="svg-context-paint-radial-obb-host-box (standalone-svg)"><img src="./chromium/svg-context-paint-radial-obb-host-box.png" width="56" alt="svg-context-paint-radial-obb-host-box"></a>
<a href="./svg-context-paint-radial-userspace-host.svg" title="svg-context-paint-radial-userspace-host (standalone-svg)"><img src="./chromium/svg-context-paint-radial-userspace-host.png" width="56" alt="svg-context-paint-radial-userspace-host"></a>
<a href="./svg-context-paint-stopless-fallback-inert.svg" title="svg-context-paint-stopless-fallback-inert (standalone-svg)"><img src="./chromium/svg-context-paint-stopless-fallback-inert.png" width="56" alt="svg-context-paint-stopless-fallback-inert"></a>
<a href="./svg-css-transform-beats-attribute.svg" title="svg-css-transform-beats-attribute (standalone-svg)"><img src="./chromium/svg-css-transform-beats-attribute.png" width="56" alt="svg-css-transform-beats-attribute"></a>
<a href="./svg-css-transform-compound.svg" title="svg-css-transform-compound (standalone-svg)"><img src="./chromium/svg-css-transform-compound.png" width="56" alt="svg-css-transform-compound"></a>
<a href="./svg-css-transform-group.svg" title="svg-css-transform-group (standalone-svg)"><img src="./chromium/svg-css-transform-group.png" width="56" alt="svg-css-transform-group"></a>
<a href="./svg-css-transform-invalid-falls-back.svg" title="svg-css-transform-invalid-falls-back (standalone-svg)"><img src="./chromium/svg-css-transform-invalid-falls-back.png" width="56" alt="svg-css-transform-invalid-falls-back"></a>
<a href="./svg-css-transform-none-restores.svg" title="svg-css-transform-none-restores (standalone-svg)"><img src="./chromium/svg-css-transform-none-restores.png" width="56" alt="svg-css-transform-none-restores"></a>
<a href="./svg-css-transform-percent.svg" title="svg-css-transform-percent (standalone-svg)"><img src="./chromium/svg-css-transform-percent.png" width="56" alt="svg-css-transform-percent"></a>
<a href="./svg-css-transform-property.svg" title="svg-css-transform-property (standalone-svg)"><img src="./chromium/svg-css-transform-property.png" width="56" alt="svg-css-transform-property"></a>
<a href="./svg-css-transform-rotate-quadrant.svg" title="svg-css-transform-rotate-quadrant (standalone-svg)"><img src="./chromium/svg-css-transform-rotate-quadrant.png" width="56" alt="svg-css-transform-rotate-quadrant"></a>
<a href="./svg-css-transform-sheet-beats-attribute.svg" title="svg-css-transform-sheet-beats-attribute (standalone-svg)"><img src="./chromium/svg-css-transform-sheet-beats-attribute.png" width="56" alt="svg-css-transform-sheet-beats-attribute"></a>
<a href="./svg-css-transform-webkit.svg" title="svg-css-transform-webkit (standalone-svg)"><img src="./chromium/svg-css-transform-webkit.png" width="56" alt="svg-css-transform-webkit"></a>
<a href="./svg-currentcolor-rect.svg" title="svg-currentcolor-rect (standalone-svg)"><img src="./chromium/svg-currentcolor-rect.png" width="56" alt="svg-currentcolor-rect"></a>
<a href="./svg-display-none-group.svg" title="svg-display-none-group (standalone-svg)"><img src="./chromium/svg-display-none-group.png" width="56" alt="svg-display-none-group"></a>
<a href="./svg-display-none-root.svg" title="svg-display-none-root (standalone-svg)"><img src="./chromium/svg-display-none-root.png" width="56" alt="svg-display-none-root"></a>
<a href="./svg-display-none-shape.svg" title="svg-display-none-shape (standalone-svg)"><img src="./chromium/svg-display-none-shape.png" width="56" alt="svg-display-none-shape"></a>
<a href="./svg-element-opacity.svg" title="svg-element-opacity (standalone-svg)"><img src="./chromium/svg-element-opacity.png" width="56" alt="svg-element-opacity"></a>
<a href="./svg-ellipse-auto-rx.svg" title="svg-ellipse-auto-rx (standalone-svg)"><img src="./chromium/svg-ellipse-auto-rx.png" width="56" alt="svg-ellipse-auto-rx"></a>
<a href="./svg-ellipse-fill.svg" title="svg-ellipse-fill (standalone-svg)"><img src="./chromium/svg-ellipse-fill.png" width="56" alt="svg-ellipse-fill"></a>
<a href="./svg-ellipse-negative-rx-auto.svg" title="svg-ellipse-negative-rx-auto (standalone-svg)"><img src="./chromium/svg-ellipse-negative-rx-auto.png" width="56" alt="svg-ellipse-negative-rx-auto"></a>
<a href="./svg-fill-inherited-rect.svg" title="svg-fill-inherited-rect (standalone-svg)"><img src="./chromium/svg-fill-inherited-rect.png" width="56" alt="svg-fill-inherited-rect"></a>
<a href="./svg-fill-invalid-initial-rect.svg" title="svg-fill-invalid-initial-rect (standalone-svg)"><img src="./chromium/svg-fill-invalid-initial-rect.png" width="56" alt="svg-fill-invalid-initial-rect"></a>
<a href="./svg-fill-named-rect.svg" title="svg-fill-named-rect (standalone-svg)"><img src="./chromium/svg-fill-named-rect.png" width="56" alt="svg-fill-named-rect"></a>
<a href="./svg-fill-none-rect.svg" title="svg-fill-none-rect (standalone-svg)"><img src="./chromium/svg-fill-none-rect.png" width="56" alt="svg-fill-none-rect"></a>
<a href="./svg-fill-opacity-inherited.svg" title="svg-fill-opacity-inherited (standalone-svg)"><img src="./chromium/svg-fill-opacity-inherited.png" width="56" alt="svg-fill-opacity-inherited"></a>
<a href="./svg-fill-opacity-overlap.svg" title="svg-fill-opacity-overlap (standalone-svg)"><img src="./chromium/svg-fill-opacity-overlap.png" width="56" alt="svg-fill-opacity-overlap"></a>
<a href="./svg-fill-opacity-percentage.svg" title="svg-fill-opacity-percentage (standalone-svg)"><img src="./chromium/svg-fill-opacity-percentage.png" width="56" alt="svg-fill-opacity-percentage"></a>
<a href="./svg-fill-opacity-times-alpha.svg" title="svg-fill-opacity-times-alpha (standalone-svg)"><img src="./chromium/svg-fill-opacity-times-alpha.png" width="56" alt="svg-fill-opacity-times-alpha"></a>
<a href="./svg-gradient-css-transform.svg" title="svg-gradient-css-transform (standalone-svg)"><img src="./chromium/svg-gradient-css-transform.png" width="56" alt="svg-gradient-css-transform"></a>
<a href="./svg-gradient-currentcolor.svg" title="svg-gradient-currentcolor (standalone-svg)"><img src="./chromium/svg-gradient-currentcolor.png" width="56" alt="svg-gradient-currentcolor"></a>
<a href="./svg-gradient-degenerate-pad.svg" title="svg-gradient-degenerate-pad (standalone-svg)"><img src="./chromium/svg-gradient-degenerate-pad.png" width="56" alt="svg-gradient-degenerate-pad"></a>
<a href="./svg-gradient-degenerate-repeat.svg" title="svg-gradient-degenerate-repeat (standalone-svg)"><img src="./chromium/svg-gradient-degenerate-repeat.png" width="56" alt="svg-gradient-degenerate-repeat"></a>
<a href="./svg-gradient-fallback.svg" title="svg-gradient-fallback (standalone-svg)"><img src="./chromium/svg-gradient-fallback.png" width="56" alt="svg-gradient-fallback"></a>
<a href="./svg-gradient-fill-opacity.svg" title="svg-gradient-fill-opacity (standalone-svg)"><img src="./chromium/svg-gradient-fill-opacity.png" width="56" alt="svg-gradient-fill-opacity"></a>
<a href="./svg-gradient-hard-stop.svg" title="svg-gradient-hard-stop (standalone-svg)"><img src="./chromium/svg-gradient-hard-stop.png" width="56" alt="svg-gradient-hard-stop"></a>
<a href="./svg-gradient-href-cross-type.svg" title="svg-gradient-href-cross-type (standalone-svg)"><img src="./chromium/svg-gradient-href-cross-type.png" width="56" alt="svg-gradient-href-cross-type"></a>
<a href="./svg-gradient-interp-unpremul.svg" title="svg-gradient-interp-unpremul (standalone-svg)"><img src="./chromium/svg-gradient-interp-unpremul.png" width="56" alt="svg-gradient-interp-unpremul"></a>
<a href="./svg-gradient-linear.svg" title="svg-gradient-linear (standalone-svg)"><img src="./chromium/svg-gradient-linear.png" width="56" alt="svg-gradient-linear"></a>
<a href="./svg-gradient-linear-bbox-offset.svg" title="svg-gradient-linear-bbox-offset (standalone-svg)"><img src="./chromium/svg-gradient-linear-bbox-offset.png" width="56" alt="svg-gradient-linear-bbox-offset"></a>
<a href="./svg-gradient-linear-userspace.svg" title="svg-gradient-linear-userspace (standalone-svg)"><img src="./chromium/svg-gradient-linear-userspace.png" width="56" alt="svg-gradient-linear-userspace"></a>
<a href="./svg-gradient-not-in-defs.svg" title="svg-gradient-not-in-defs (standalone-svg)"><img src="./chromium/svg-gradient-not-in-defs.png" width="56" alt="svg-gradient-not-in-defs"></a>
<a href="./svg-gradient-path-bbox.svg" title="svg-gradient-path-bbox (standalone-svg)"><img src="./chromium/svg-gradient-path-bbox.png" width="56" alt="svg-gradient-path-bbox"></a>
<a href="./svg-gradient-radial.svg" title="svg-gradient-radial (standalone-svg)"><img src="./chromium/svg-gradient-radial.png" width="56" alt="svg-gradient-radial"></a>
<a href="./svg-gradient-radial-custom.svg" title="svg-gradient-radial-custom (standalone-svg)"><img src="./chromium/svg-gradient-radial-custom.png" width="56" alt="svg-gradient-radial-custom"></a>
<a href="./svg-gradient-radial-diagonal-percent.svg" title="svg-gradient-radial-diagonal-percent (standalone-svg)"><img src="./chromium/svg-gradient-radial-diagonal-percent.png" width="56" alt="svg-gradient-radial-diagonal-percent"></a>
<a href="./svg-gradient-radial-r0.svg" title="svg-gradient-radial-r0 (standalone-svg)"><img src="./chromium/svg-gradient-radial-r0.png" width="56" alt="svg-gradient-radial-r0"></a>
<a href="./svg-gradient-spread-reflect.svg" title="svg-gradient-spread-reflect (standalone-svg)"><img src="./chromium/svg-gradient-spread-reflect.png" width="56" alt="svg-gradient-spread-reflect"></a>
<a href="./svg-gradient-spread-repeat.svg" title="svg-gradient-spread-repeat (standalone-svg)"><img src="./chromium/svg-gradient-spread-repeat.png" width="56" alt="svg-gradient-spread-repeat"></a>
<a href="./svg-gradient-stop-nonmonotonic.svg" title="svg-gradient-stop-nonmonotonic (standalone-svg)"><img src="./chromium/svg-gradient-stop-nonmonotonic.png" width="56" alt="svg-gradient-stop-nonmonotonic"></a>
<a href="./svg-gradient-stroke.svg" title="svg-gradient-stroke (standalone-svg)"><img src="./chromium/svg-gradient-stroke.png" width="56" alt="svg-gradient-stroke"></a>
<a href="./svg-gradient-stroke-css.svg" title="svg-gradient-stroke-css (standalone-svg)"><img src="./chromium/svg-gradient-stroke-css.png" width="56" alt="svg-gradient-stroke-css"></a>
<a href="./svg-gradient-stylesheet-fill.svg" title="svg-gradient-stylesheet-fill (standalone-svg)"><img src="./chromium/svg-gradient-stylesheet-fill.png" width="56" alt="svg-gradient-stylesheet-fill"></a>
<a href="./svg-gradient-transform.svg" title="svg-gradient-transform (standalone-svg)"><img src="./chromium/svg-gradient-transform.png" width="56" alt="svg-gradient-transform"></a>
<a href="./svg-gradient-use-clone-order.svg" title="svg-gradient-use-clone-order (standalone-svg)"><img src="./chromium/svg-gradient-use-clone-order.png" width="56" alt="svg-gradient-use-clone-order"></a>
<a href="./svg-gradient-zero-bbox.svg" title="svg-gradient-zero-bbox (standalone-svg)"><img src="./chromium/svg-gradient-zero-bbox.png" width="56" alt="svg-gradient-zero-bbox"></a>
<a href="./svg-gradient-zero-stops-fallback.svg" title="svg-gradient-zero-stops-fallback (standalone-svg)"><img src="./chromium/svg-gradient-zero-stops-fallback.png" width="56" alt="svg-gradient-zero-stops-fallback"></a>
<a href="./svg-group-inherited-fill.svg" title="svg-group-inherited-fill (standalone-svg)"><img src="./chromium/svg-group-inherited-fill.png" width="56" alt="svg-group-inherited-fill"></a>
<a href="./svg-group-nested-transforms.svg" title="svg-group-nested-transforms (standalone-svg)"><img src="./chromium/svg-group-nested-transforms.png" width="56" alt="svg-group-nested-transforms"></a>
<a href="./svg-group-paint-order.svg" title="svg-group-paint-order (standalone-svg)"><img src="./chromium/svg-group-paint-order.png" width="56" alt="svg-group-paint-order"></a>
<a href="./svg-group-rotate-diagonal.svg" title="svg-group-rotate-diagonal (standalone-svg)"><img src="./chromium/svg-group-rotate-diagonal.png" width="56" alt="svg-group-rotate-diagonal"></a>
<a href="./svg-group-rotate-quarter.svg" title="svg-group-rotate-quarter (standalone-svg)"><img src="./chromium/svg-group-rotate-quarter.png" width="56" alt="svg-group-rotate-quarter"></a>
<a href="./svg-group-transform-translate.svg" title="svg-group-transform-translate (standalone-svg)"><img src="./chromium/svg-group-transform-translate.png" width="56" alt="svg-group-transform-translate"></a>
<a href="./svg-non-rendering-elements.svg" title="svg-non-rendering-elements (standalone-svg)"><img src="./chromium/svg-non-rendering-elements.png" width="56" alt="svg-non-rendering-elements"></a>
<a href="./svg-opacity-fill-stroke.svg" title="svg-opacity-fill-stroke (standalone-svg)"><img src="./chromium/svg-opacity-fill-stroke.png" width="56" alt="svg-opacity-fill-stroke"></a>
<a href="./svg-opacity-gradient-in-group.svg" title="svg-opacity-gradient-in-group (standalone-svg)"><img src="./chromium/svg-opacity-gradient-in-group.png" width="56" alt="svg-opacity-gradient-in-group"></a>
<a href="./svg-opacity-group-nonhalf.svg" title="svg-opacity-group-nonhalf (standalone-svg)"><img src="./chromium/svg-opacity-group-nonhalf.png" width="56" alt="svg-opacity-group-nonhalf"></a>
<a href="./svg-opacity-group-overlap.svg" title="svg-opacity-group-overlap (standalone-svg)"><img src="./chromium/svg-opacity-group-overlap.png" width="56" alt="svg-opacity-group-overlap"></a>
<a href="./svg-opacity-hidden-in-group.svg" title="svg-opacity-hidden-in-group (standalone-svg)"><img src="./chromium/svg-opacity-hidden-in-group.png" width="56" alt="svg-opacity-hidden-in-group"></a>
<a href="./svg-opacity-nested-groups.svg" title="svg-opacity-nested-groups (standalone-svg)"><img src="./chromium/svg-opacity-nested-groups.png" width="56" alt="svg-opacity-nested-groups"></a>
<a href="./svg-opacity-rotated-group.svg" title="svg-opacity-rotated-group (standalone-svg)"><img src="./chromium/svg-opacity-rotated-group.png" width="56" alt="svg-opacity-rotated-group"></a>
<a href="./svg-opacity-stroke-only-fold.svg" title="svg-opacity-stroke-only-fold (standalone-svg)"><img src="./chromium/svg-opacity-stroke-only-fold.png" width="56" alt="svg-opacity-stroke-only-fold"></a>
<a href="./svg-opacity-times-fill-opacity.svg" title="svg-opacity-times-fill-opacity (standalone-svg)"><img src="./chromium/svg-opacity-times-fill-opacity.png" width="56" alt="svg-opacity-times-fill-opacity"></a>
<a href="./svg-opacity-transform-below.svg" title="svg-opacity-transform-below (standalone-svg)"><img src="./chromium/svg-opacity-transform-below.png" width="56" alt="svg-opacity-transform-below"></a>
<a href="./svg-opacity-transform-on-element.svg" title="svg-opacity-transform-on-element (standalone-svg)"><img src="./chromium/svg-opacity-transform-on-element.png" width="56" alt="svg-opacity-transform-on-element"></a>
<a href="./svg-opacity-translucent-overlap.svg" title="svg-opacity-translucent-overlap (standalone-svg)"><img src="./chromium/svg-opacity-translucent-overlap.png" width="56" alt="svg-opacity-translucent-overlap"></a>
<a href="./svg-opacity-use-compound.svg" title="svg-opacity-use-compound (standalone-svg)"><img src="./chromium/svg-opacity-use-compound.png" width="56" alt="svg-opacity-use-compound"></a>
<a href="./svg-opacity-zero-sibling.svg" title="svg-opacity-zero-sibling (standalone-svg)"><img src="./chromium/svg-opacity-zero-sibling.png" width="56" alt="svg-opacity-zero-sibling"></a>
<a href="./svg-path-arc.svg" title="svg-path-arc (standalone-svg)"><img src="./chromium/svg-path-arc.png" width="56" alt="svg-path-arc"></a>
<a href="./svg-path-arc-degenerate.svg" title="svg-path-arc-degenerate (standalone-svg)"><img src="./chromium/svg-path-arc-degenerate.png" width="56" alt="svg-path-arc-degenerate"></a>
<a href="./svg-path-arc-flags.svg" title="svg-path-arc-flags (standalone-svg)"><img src="./chromium/svg-path-arc-flags.png" width="56" alt="svg-path-arc-flags"></a>
<a href="./svg-path-arc-rotated.svg" title="svg-path-arc-rotated (standalone-svg)"><img src="./chromium/svg-path-arc-rotated.png" width="56" alt="svg-path-arc-rotated"></a>
<a href="./svg-path-arc-stroked.svg" title="svg-path-arc-stroked (standalone-svg)"><img src="./chromium/svg-path-arc-stroked.png" width="56" alt="svg-path-arc-stroked"></a>
<a href="./svg-path-closed-move-only-contour.svg" title="svg-path-closed-move-only-contour (standalone-svg)"><img src="./chromium/svg-path-closed-move-only-contour.png" width="56" alt="svg-path-closed-move-only-contour"></a>
<a href="./svg-path-cubic-fill.svg" title="svg-path-cubic-fill (standalone-svg)"><img src="./chromium/svg-path-cubic-fill.png" width="56" alt="svg-path-cubic-fill"></a>
<a href="./svg-path-draws-nothing.svg" title="svg-path-draws-nothing (standalone-svg)"><img src="./chromium/svg-path-draws-nothing.png" width="56" alt="svg-path-draws-nothing"></a>
<a href="./svg-path-fill-rule-evenodd.svg" title="svg-path-fill-rule-evenodd (standalone-svg)"><img src="./chromium/svg-path-fill-rule-evenodd.png" width="56" alt="svg-path-fill-rule-evenodd"></a>
<a href="./svg-path-fill-rule-inherited.svg" title="svg-path-fill-rule-inherited (standalone-svg)"><img src="./chromium/svg-path-fill-rule-inherited.png" width="56" alt="svg-path-fill-rule-inherited"></a>
<a href="./svg-path-fill-rule-nonzero.svg" title="svg-path-fill-rule-nonzero (standalone-svg)"><img src="./chromium/svg-path-fill-rule-nonzero.png" width="56" alt="svg-path-fill-rule-nonzero"></a>
<a href="./svg-path-hv-shorthand.svg" title="svg-path-hv-shorthand (standalone-svg)"><img src="./chromium/svg-path-hv-shorthand.png" width="56" alt="svg-path-hv-shorthand"></a>
<a href="./svg-path-in-scaled-group.svg" title="svg-path-in-scaled-group (standalone-svg)"><img src="./chromium/svg-path-in-scaled-group.png" width="56" alt="svg-path-in-scaled-group"></a>
<a href="./svg-path-polygon-fill.svg" title="svg-path-polygon-fill (standalone-svg)"><img src="./chromium/svg-path-polygon-fill.png" width="56" alt="svg-path-polygon-fill"></a>
<a href="./svg-path-quadratic.svg" title="svg-path-quadratic (standalone-svg)"><img src="./chromium/svg-path-quadratic.png" width="56" alt="svg-path-quadratic"></a>
<a href="./svg-path-relative-commands.svg" title="svg-path-relative-commands (standalone-svg)"><img src="./chromium/svg-path-relative-commands.png" width="56" alt="svg-path-relative-commands"></a>
<a href="./svg-path-smooth-cubic.svg" title="svg-path-smooth-cubic (standalone-svg)"><img src="./chromium/svg-path-smooth-cubic.png" width="56" alt="svg-path-smooth-cubic"></a>
<a href="./svg-path-two-subpaths.svg" title="svg-path-two-subpaths (standalone-svg)"><img src="./chromium/svg-path-two-subpaths.png" width="56" alt="svg-path-two-subpaths"></a>
<a href="./svg-path-unclosed-fill.svg" title="svg-path-unclosed-fill (standalone-svg)"><img src="./chromium/svg-path-unclosed-fill.png" width="56" alt="svg-path-unclosed-fill"></a>
<a href="./svg-percent-circle-diagonal.svg" title="svg-percent-circle-diagonal (standalone-svg)"><img src="./chromium/svg-percent-circle-diagonal.png" width="56" alt="svg-percent-circle-diagonal"></a>
<a href="./svg-percent-ellipse.svg" title="svg-percent-ellipse (standalone-svg)"><img src="./chromium/svg-percent-ellipse.png" width="56" alt="svg-percent-ellipse"></a>
<a href="./svg-percent-line.svg" title="svg-percent-line (standalone-svg)"><img src="./chromium/svg-percent-line.png" width="56" alt="svg-percent-line"></a>
<a href="./svg-percent-rect-in-viewbox.svg" title="svg-percent-rect-in-viewbox (standalone-svg)"><img src="./chromium/svg-percent-rect-in-viewbox.png" width="56" alt="svg-percent-rect-in-viewbox"></a>
<a href="./svg-percent-rect-root-units.svg" title="svg-percent-rect-root-units (standalone-svg)"><img src="./chromium/svg-percent-rect-root-units.png" width="56" alt="svg-percent-rect-root-units"></a>
<a href="./svg-percent-stroke-width.svg" title="svg-percent-stroke-width (standalone-svg)"><img src="./chromium/svg-percent-stroke-width.png" width="56" alt="svg-percent-stroke-width"></a>
<a href="./svg-points-trailing-comma.svg" title="svg-points-trailing-comma (standalone-svg)"><img src="./chromium/svg-points-trailing-comma.png" width="56" alt="svg-points-trailing-comma"></a>
<a href="./svg-polygon-fill.svg" title="svg-polygon-fill (standalone-svg)"><img src="./chromium/svg-polygon-fill.png" width="56" alt="svg-polygon-fill"></a>
<a href="./svg-polygon-fill-rule-evenodd.svg" title="svg-polygon-fill-rule-evenodd (standalone-svg)"><img src="./chromium/svg-polygon-fill-rule-evenodd.png" width="56" alt="svg-polygon-fill-rule-evenodd"></a>
<a href="./svg-polygon-single-point-square-cap.svg" title="svg-polygon-single-point-square-cap (standalone-svg)"><img src="./chromium/svg-polygon-single-point-square-cap.png" width="56" alt="svg-polygon-single-point-square-cap"></a>
<a href="./svg-polygon-stroke-closed.svg" title="svg-polygon-stroke-closed (standalone-svg)"><img src="./chromium/svg-polygon-stroke-closed.png" width="56" alt="svg-polygon-stroke-closed"></a>
<a href="./svg-polyline-fill-implicit-close.svg" title="svg-polyline-fill-implicit-close (standalone-svg)"><img src="./chromium/svg-polyline-fill-implicit-close.png" width="56" alt="svg-polyline-fill-implicit-close"></a>
<a href="./svg-polyline-single-point-square-cap.svg" title="svg-polyline-single-point-square-cap (standalone-svg)"><img src="./chromium/svg-polyline-single-point-square-cap.png" width="56" alt="svg-polyline-single-point-square-cap"></a>
<a href="./svg-polyline-stroke-open.svg" title="svg-polyline-stroke-open (standalone-svg)"><img src="./chromium/svg-polyline-stroke-open.png" width="56" alt="svg-polyline-stroke-open"></a>
<a href="./svg-preserve-aspect-ratio-align-max-meet.svg" title="svg-preserve-aspect-ratio-align-max-meet (standalone-svg)"><img src="./chromium/svg-preserve-aspect-ratio-align-max-meet.png" width="56" alt="svg-preserve-aspect-ratio-align-max-meet"></a>
<a href="./svg-preserve-aspect-ratio-explicit.svg" title="svg-preserve-aspect-ratio-explicit (standalone-svg)"><img src="./chromium/svg-preserve-aspect-ratio-explicit.png" width="56" alt="svg-preserve-aspect-ratio-explicit"></a>
<a href="./svg-preserve-aspect-ratio-none-stretch.svg" title="svg-preserve-aspect-ratio-none-stretch (standalone-svg)"><img src="./chromium/svg-preserve-aspect-ratio-none-stretch.png" width="56" alt="svg-preserve-aspect-ratio-none-stretch"></a>
<a href="./svg-preserve-aspect-ratio-slice-clip.svg" title="svg-preserve-aspect-ratio-slice-clip (standalone-svg)"><img src="./chromium/svg-preserve-aspect-ratio-slice-clip.png" width="56" alt="svg-preserve-aspect-ratio-slice-clip"></a>
<a href="./svg-rect-rounded.svg" title="svg-rect-rounded (standalone-svg)"><img src="./chromium/svg-rect-rounded.png" width="56" alt="svg-rect-rounded"></a>
<a href="./svg-rect-rounded-clamp.svg" title="svg-rect-rounded-clamp (standalone-svg)"><img src="./chromium/svg-rect-rounded-clamp.png" width="56" alt="svg-rect-rounded-clamp"></a>
<a href="./svg-rect-rounded-elliptical.svg" title="svg-rect-rounded-elliptical (standalone-svg)"><img src="./chromium/svg-rect-rounded-elliptical.png" width="56" alt="svg-rect-rounded-elliptical"></a>
<a href="./svg-rect-rounded-mirror-auto.svg" title="svg-rect-rounded-mirror-auto (standalone-svg)"><img src="./chromium/svg-rect-rounded-mirror-auto.png" width="56" alt="svg-rect-rounded-mirror-auto"></a>
<a href="./svg-rect-rounded-negative-rx-auto.svg" title="svg-rect-rounded-negative-rx-auto (standalone-svg)"><img src="./chromium/svg-rect-rounded-negative-rx-auto.png" width="56" alt="svg-rect-rounded-negative-rx-auto"></a>
<a href="./svg-rect-rounded-stroked.svg" title="svg-rect-rounded-stroked (standalone-svg)"><img src="./chromium/svg-rect-rounded-stroked.png" width="56" alt="svg-rect-rounded-stroked"></a>
<a href="./svg-shape-transform-matrix.svg" title="svg-shape-transform-matrix (standalone-svg)"><img src="./chromium/svg-shape-transform-matrix.png" width="56" alt="svg-shape-transform-matrix"></a>
<a href="./svg-sizing-auto-rect.svg" title="svg-sizing-auto-rect (standalone-svg)"><img src="./chromium/svg-sizing-auto-rect.png" width="56" alt="svg-sizing-auto-rect"></a>
<a href="./svg-stroke-cap-butt.svg" title="svg-stroke-cap-butt (standalone-svg)"><img src="./chromium/svg-stroke-cap-butt.png" width="56" alt="svg-stroke-cap-butt"></a>
<a href="./svg-stroke-cap-circle-round.svg" title="svg-stroke-cap-circle-round (standalone-svg)"><img src="./chromium/svg-stroke-cap-circle-round.png" width="56" alt="svg-stroke-cap-circle-round"></a>
<a href="./svg-stroke-cap-circle-square.svg" title="svg-stroke-cap-circle-square (standalone-svg)"><img src="./chromium/svg-stroke-cap-circle-square.png" width="56" alt="svg-stroke-cap-circle-square"></a>
<a href="./svg-stroke-cap-closed-butt.svg" title="svg-stroke-cap-closed-butt (standalone-svg)"><img src="./chromium/svg-stroke-cap-closed-butt.png" width="56" alt="svg-stroke-cap-closed-butt"></a>
<a href="./svg-stroke-cap-closed-round.svg" title="svg-stroke-cap-closed-round (standalone-svg)"><img src="./chromium/svg-stroke-cap-closed-round.png" width="56" alt="svg-stroke-cap-closed-round"></a>
<a href="./svg-stroke-cap-closed-square.svg" title="svg-stroke-cap-closed-square (standalone-svg)"><img src="./chromium/svg-stroke-cap-closed-square.png" width="56" alt="svg-stroke-cap-closed-square"></a>
<a href="./svg-stroke-cap-css-butt.svg" title="svg-stroke-cap-css-butt (standalone-svg)"><img src="./chromium/svg-stroke-cap-css-butt.png" width="56" alt="svg-stroke-cap-css-butt"></a>
<a href="./svg-stroke-cap-css-over-attr.svg" title="svg-stroke-cap-css-over-attr (standalone-svg)"><img src="./chromium/svg-stroke-cap-css-over-attr.png" width="56" alt="svg-stroke-cap-css-over-attr"></a>
<a href="./svg-stroke-cap-css-round.svg" title="svg-stroke-cap-css-round (standalone-svg)"><img src="./chromium/svg-stroke-cap-css-round.png" width="56" alt="svg-stroke-cap-css-round"></a>
<a href="./svg-stroke-cap-css-square.svg" title="svg-stroke-cap-css-square (standalone-svg)"><img src="./chromium/svg-stroke-cap-css-square.png" width="56" alt="svg-stroke-cap-css-square"></a>
<a href="./svg-stroke-cap-ellipse-round.svg" title="svg-stroke-cap-ellipse-round (standalone-svg)"><img src="./chromium/svg-stroke-cap-ellipse-round.png" width="56" alt="svg-stroke-cap-ellipse-round"></a>
<a href="./svg-stroke-cap-ellipse-square.svg" title="svg-stroke-cap-ellipse-square (standalone-svg)"><img src="./chromium/svg-stroke-cap-ellipse-square.png" width="56" alt="svg-stroke-cap-ellipse-square"></a>
<a href="./svg-stroke-cap-round.svg" title="svg-stroke-cap-round (standalone-svg)"><img src="./chromium/svg-stroke-cap-round.png" width="56" alt="svg-stroke-cap-round"></a>
<a href="./svg-stroke-cap-square.svg" title="svg-stroke-cap-square (standalone-svg)"><img src="./chromium/svg-stroke-cap-square.png" width="56" alt="svg-stroke-cap-square"></a>
<a href="./svg-stroke-circle.svg" title="svg-stroke-circle (standalone-svg)"><img src="./chromium/svg-stroke-circle.png" width="56" alt="svg-stroke-circle"></a>
<a href="./svg-stroke-dasharray.svg" title="svg-stroke-dasharray (standalone-svg)"><img src="./chromium/svg-stroke-dasharray.png" width="56" alt="svg-stroke-dasharray"></a>
<a href="./svg-stroke-dasharray-all-zero.svg" title="svg-stroke-dasharray-all-zero (standalone-svg)"><img src="./chromium/svg-stroke-dasharray-all-zero.png" width="56" alt="svg-stroke-dasharray-all-zero"></a>
<a href="./svg-stroke-dasharray-backend-saturation.svg" title="svg-stroke-dasharray-backend-saturation (standalone-svg)"><img src="./chromium/svg-stroke-dasharray-backend-saturation.png" width="56" alt="svg-stroke-dasharray-backend-saturation"></a>
<a href="./svg-stroke-dasharray-calc.svg" title="svg-stroke-dasharray-calc (standalone-svg)"><img src="./chromium/svg-stroke-dasharray-calc.png" width="56" alt="svg-stroke-dasharray-calc"></a>
<a href="./svg-stroke-dasharray-closed-ellipse-round.svg" title="svg-stroke-dasharray-closed-ellipse-round (standalone-svg)"><img src="./chromium/svg-stroke-dasharray-closed-ellipse-round.png" width="56" alt="svg-stroke-dasharray-closed-ellipse-round"></a>
<a href="./svg-stroke-dasharray-closed-path-square.svg" title="svg-stroke-dasharray-closed-path-square (standalone-svg)"><img src="./chromium/svg-stroke-dasharray-closed-path-square.png" width="56" alt="svg-stroke-dasharray-closed-path-square"></a>
<a href="./svg-stroke-dasharray-comma.svg" title="svg-stroke-dasharray-comma (standalone-svg)"><img src="./chromium/svg-stroke-dasharray-comma.png" width="56" alt="svg-stroke-dasharray-comma"></a>
<a href="./svg-stroke-dasharray-css.svg" title="svg-stroke-dasharray-css (standalone-svg)"><img src="./chromium/svg-stroke-dasharray-css.png" width="56" alt="svg-stroke-dasharray-css"></a>
<a href="./svg-stroke-dasharray-css-invalid-falls-back.svg" title="svg-stroke-dasharray-css-invalid-falls-back (standalone-svg)"><img src="./chromium/svg-stroke-dasharray-css-invalid-falls-back.png" width="56" alt="svg-stroke-dasharray-css-invalid-falls-back"></a>
<a href="./svg-stroke-dasharray-css-math.svg" title="svg-stroke-dasharray-css-math (standalone-svg)"><img src="./chromium/svg-stroke-dasharray-css-math.png" width="56" alt="svg-stroke-dasharray-css-math"></a>
<a href="./svg-stroke-dasharray-css-over-attr.svg" title="svg-stroke-dasharray-css-over-attr (standalone-svg)"><img src="./chromium/svg-stroke-dasharray-css-over-attr.png" width="56" alt="svg-stroke-dasharray-css-over-attr"></a>
<a href="./svg-stroke-dasharray-em-font-size.svg" title="svg-stroke-dasharray-em-font-size (standalone-svg)"><img src="./chromium/svg-stroke-dasharray-em-font-size.png" width="56" alt="svg-stroke-dasharray-em-font-size"></a>
<a href="./svg-stroke-dasharray-exponent.svg" title="svg-stroke-dasharray-exponent (standalone-svg)"><img src="./chromium/svg-stroke-dasharray-exponent.png" width="56" alt="svg-stroke-dasharray-exponent"></a>
<a href="./svg-stroke-dasharray-geometries.svg" title="svg-stroke-dasharray-geometries (standalone-svg)"><img src="./chromium/svg-stroke-dasharray-geometries.png" width="56" alt="svg-stroke-dasharray-geometries"></a>
<a href="./svg-stroke-dasharray-inherited.svg" title="svg-stroke-dasharray-inherited (standalone-svg)"><img src="./chromium/svg-stroke-dasharray-inherited.png" width="56" alt="svg-stroke-dasharray-inherited"></a>
<a href="./svg-stroke-dasharray-mixed-contours-round.svg" title="svg-stroke-dasharray-mixed-contours-round (standalone-svg)"><img src="./chromium/svg-stroke-dasharray-mixed-contours-round.png" width="56" alt="svg-stroke-dasharray-mixed-contours-round"></a>
<a href="./svg-stroke-dasharray-negative.svg" title="svg-stroke-dasharray-negative (standalone-svg)"><img src="./chromium/svg-stroke-dasharray-negative.png" width="56" alt="svg-stroke-dasharray-negative"></a>
<a href="./svg-stroke-dasharray-none.svg" title="svg-stroke-dasharray-none (standalone-svg)"><img src="./chromium/svg-stroke-dasharray-none.png" width="56" alt="svg-stroke-dasharray-none"></a>
<a href="./svg-stroke-dasharray-odd.svg" title="svg-stroke-dasharray-odd (standalone-svg)"><img src="./chromium/svg-stroke-dasharray-odd.png" width="56" alt="svg-stroke-dasharray-odd"></a>
<a href="./svg-stroke-dasharray-percent.svg" title="svg-stroke-dasharray-percent (standalone-svg)"><img src="./chromium/svg-stroke-dasharray-percent.png" width="56" alt="svg-stroke-dasharray-percent"></a>
<a href="./svg-stroke-dasharray-scaled-group.svg" title="svg-stroke-dasharray-scaled-group (standalone-svg)"><img src="./chromium/svg-stroke-dasharray-scaled-group.png" width="56" alt="svg-stroke-dasharray-scaled-group"></a>
<a href="./svg-stroke-dasharray-subpath-restart.svg" title="svg-stroke-dasharray-subpath-restart (standalone-svg)"><img src="./chromium/svg-stroke-dasharray-subpath-restart.png" width="56" alt="svg-stroke-dasharray-subpath-restart"></a>
<a href="./svg-stroke-dasharray-use-inherited.svg" title="svg-stroke-dasharray-use-inherited (standalone-svg)"><img src="./chromium/svg-stroke-dasharray-use-inherited.png" width="56" alt="svg-stroke-dasharray-use-inherited"></a>
<a href="./svg-stroke-dasharray-viewbox-percent.svg" title="svg-stroke-dasharray-viewbox-percent (standalone-svg)"><img src="./chromium/svg-stroke-dasharray-viewbox-percent.png" width="56" alt="svg-stroke-dasharray-viewbox-percent"></a>
<a href="./svg-stroke-dasharray-zero-round.svg" title="svg-stroke-dasharray-zero-round (standalone-svg)"><img src="./chromium/svg-stroke-dasharray-zero-round.png" width="56" alt="svg-stroke-dasharray-zero-round"></a>
<a href="./svg-stroke-default-width.svg" title="svg-stroke-default-width (standalone-svg)"><img src="./chromium/svg-stroke-default-width.png" width="56" alt="svg-stroke-default-width"></a>
<a href="./svg-stroke-ellipse.svg" title="svg-stroke-ellipse (standalone-svg)"><img src="./chromium/svg-stroke-ellipse.png" width="56" alt="svg-stroke-ellipse"></a>
<a href="./svg-stroke-inherited.svg" title="svg-stroke-inherited (standalone-svg)"><img src="./chromium/svg-stroke-inherited.png" width="56" alt="svg-stroke-inherited"></a>
<a href="./svg-stroke-invalid-width.svg" title="svg-stroke-invalid-width (standalone-svg)"><img src="./chromium/svg-stroke-invalid-width.png" width="56" alt="svg-stroke-invalid-width"></a>
<a href="./svg-stroke-join-arcs.svg" title="svg-stroke-join-arcs (standalone-svg)"><img src="./chromium/svg-stroke-join-arcs.png" width="56" alt="svg-stroke-join-arcs"></a>
<a href="./svg-stroke-join-bevel.svg" title="svg-stroke-join-bevel (standalone-svg)"><img src="./chromium/svg-stroke-join-bevel.png" width="56" alt="svg-stroke-join-bevel"></a>
<a href="./svg-stroke-join-css-arcs.svg" title="svg-stroke-join-css-arcs (standalone-svg)"><img src="./chromium/svg-stroke-join-css-arcs.png" width="56" alt="svg-stroke-join-css-arcs"></a>
<a href="./svg-stroke-join-css-bevel.svg" title="svg-stroke-join-css-bevel (standalone-svg)"><img src="./chromium/svg-stroke-join-css-bevel.png" width="56" alt="svg-stroke-join-css-bevel"></a>
<a href="./svg-stroke-join-css-miter.svg" title="svg-stroke-join-css-miter (standalone-svg)"><img src="./chromium/svg-stroke-join-css-miter.png" width="56" alt="svg-stroke-join-css-miter"></a>
<a href="./svg-stroke-join-css-miter-clip.svg" title="svg-stroke-join-css-miter-clip (standalone-svg)"><img src="./chromium/svg-stroke-join-css-miter-clip.png" width="56" alt="svg-stroke-join-css-miter-clip"></a>
<a href="./svg-stroke-join-css-over-attr.svg" title="svg-stroke-join-css-over-attr (standalone-svg)"><img src="./chromium/svg-stroke-join-css-over-attr.png" width="56" alt="svg-stroke-join-css-over-attr"></a>
<a href="./svg-stroke-join-css-round.svg" title="svg-stroke-join-css-round (standalone-svg)"><img src="./chromium/svg-stroke-join-css-round.png" width="56" alt="svg-stroke-join-css-round"></a>
<a href="./svg-stroke-join-miter.svg" title="svg-stroke-join-miter (standalone-svg)"><img src="./chromium/svg-stroke-join-miter.png" width="56" alt="svg-stroke-join-miter"></a>
<a href="./svg-stroke-join-miter-clip.svg" title="svg-stroke-join-miter-clip (standalone-svg)"><img src="./chromium/svg-stroke-join-miter-clip.png" width="56" alt="svg-stroke-join-miter-clip"></a>
<a href="./svg-stroke-join-round.svg" title="svg-stroke-join-round (standalone-svg)"><img src="./chromium/svg-stroke-join-round.png" width="56" alt="svg-stroke-join-round"></a>
<a href="./svg-stroke-length-units.svg" title="svg-stroke-length-units (standalone-svg)"><img src="./chromium/svg-stroke-length-units.png" width="56" alt="svg-stroke-length-units"></a>
<a href="./svg-stroke-line.svg" title="svg-stroke-line (standalone-svg)"><img src="./chromium/svg-stroke-line.png" width="56" alt="svg-stroke-line"></a>
<a href="./svg-stroke-line-fill-never-paints.svg" title="svg-stroke-line-fill-never-paints (standalone-svg)"><img src="./chromium/svg-stroke-line-fill-never-paints.png" width="56" alt="svg-stroke-line-fill-never-paints"></a>
<a href="./svg-stroke-miter-limit.svg" title="svg-stroke-miter-limit (standalone-svg)"><img src="./chromium/svg-stroke-miter-limit.png" width="56" alt="svg-stroke-miter-limit"></a>
<a href="./svg-stroke-miter-limit-css.svg" title="svg-stroke-miter-limit-css (standalone-svg)"><img src="./chromium/svg-stroke-miter-limit-css.png" width="56" alt="svg-stroke-miter-limit-css"></a>
<a href="./svg-stroke-miter-limit-css-below-one.svg" title="svg-stroke-miter-limit-css-below-one (standalone-svg)"><img src="./chromium/svg-stroke-miter-limit-css-below-one.png" width="56" alt="svg-stroke-miter-limit-css-below-one"></a>
<a href="./svg-stroke-nonuniform-scale.svg" title="svg-stroke-nonuniform-scale (standalone-svg)"><img src="./chromium/svg-stroke-nonuniform-scale.png" width="56" alt="svg-stroke-nonuniform-scale"></a>
<a href="./svg-stroke-opacity-join.svg" title="svg-stroke-opacity-join (standalone-svg)"><img src="./chromium/svg-stroke-opacity-join.png" width="56" alt="svg-stroke-opacity-join"></a>
<a href="./svg-stroke-opacity-over-fill.svg" title="svg-stroke-opacity-over-fill (standalone-svg)"><img src="./chromium/svg-stroke-opacity-over-fill.png" width="56" alt="svg-stroke-opacity-over-fill"></a>
<a href="./svg-stroke-over-fill.svg" title="svg-stroke-over-fill (standalone-svg)"><img src="./chromium/svg-stroke-over-fill.png" width="56" alt="svg-stroke-over-fill"></a>
<a href="./svg-stroke-paint-css.svg" title="svg-stroke-paint-css (standalone-svg)"><img src="./chromium/svg-stroke-paint-css.png" width="56" alt="svg-stroke-paint-css"></a>
<a href="./svg-stroke-paint-css-invalid-falls-back.svg" title="svg-stroke-paint-css-invalid-falls-back (standalone-svg)"><img src="./chromium/svg-stroke-paint-css-invalid-falls-back.png" width="56" alt="svg-stroke-paint-css-invalid-falls-back"></a>
<a href="./svg-stroke-paint-css-over-attr.svg" title="svg-stroke-paint-css-over-attr (standalone-svg)"><img src="./chromium/svg-stroke-paint-css-over-attr.png" width="56" alt="svg-stroke-paint-css-over-attr"></a>
<a href="./svg-stroke-paint-currentcolor.svg" title="svg-stroke-paint-currentcolor (standalone-svg)"><img src="./chromium/svg-stroke-paint-currentcolor.png" width="56" alt="svg-stroke-paint-currentcolor"></a>
<a href="./svg-stroke-paint-hex.svg" title="svg-stroke-paint-hex (standalone-svg)"><img src="./chromium/svg-stroke-paint-hex.png" width="56" alt="svg-stroke-paint-hex"></a>
<a href="./svg-stroke-paint-invalid-none.svg" title="svg-stroke-paint-invalid-none (standalone-svg)"><img src="./chromium/svg-stroke-paint-invalid-none.png" width="56" alt="svg-stroke-paint-invalid-none"></a>
<a href="./svg-stroke-paint-named.svg" title="svg-stroke-paint-named (standalone-svg)"><img src="./chromium/svg-stroke-paint-named.png" width="56" alt="svg-stroke-paint-named"></a>
<a href="./svg-stroke-paint-none.svg" title="svg-stroke-paint-none (standalone-svg)"><img src="./chromium/svg-stroke-paint-none.png" width="56" alt="svg-stroke-paint-none"></a>
<a href="./svg-stroke-paint-url-fallback.svg" title="svg-stroke-paint-url-fallback (standalone-svg)"><img src="./chromium/svg-stroke-paint-url-fallback.png" width="56" alt="svg-stroke-paint-url-fallback"></a>
<a href="./svg-stroke-paint-url-fallback-none.svg" title="svg-stroke-paint-url-fallback-none (standalone-svg)"><img src="./chromium/svg-stroke-paint-url-fallback-none.png" width="56" alt="svg-stroke-paint-url-fallback-none"></a>
<a href="./svg-stroke-paint-url-missing.svg" title="svg-stroke-paint-url-missing (standalone-svg)"><img src="./chromium/svg-stroke-paint-url-missing.png" width="56" alt="svg-stroke-paint-url-missing"></a>
<a href="./svg-stroke-paint-url-stopless-fallback-inert.svg" title="svg-stroke-paint-url-stopless-fallback-inert (standalone-svg)"><img src="./chromium/svg-stroke-paint-url-stopless-fallback-inert.png" width="56" alt="svg-stroke-paint-url-stopless-fallback-inert"></a>
<a href="./svg-stroke-path-closed.svg" title="svg-stroke-path-closed (standalone-svg)"><img src="./chromium/svg-stroke-path-closed.png" width="56" alt="svg-stroke-path-closed"></a>
<a href="./svg-stroke-path-open.svg" title="svg-stroke-path-open (standalone-svg)"><img src="./chromium/svg-stroke-path-open.png" width="56" alt="svg-stroke-path-open"></a>
<a href="./svg-stroke-rect-centred.svg" title="svg-stroke-rect-centred (standalone-svg)"><img src="./chromium/svg-stroke-rect-centred.png" width="56" alt="svg-stroke-rect-centred"></a>
<a href="./svg-stroke-scaled-group.svg" title="svg-stroke-scaled-group (standalone-svg)"><img src="./chromium/svg-stroke-scaled-group.png" width="56" alt="svg-stroke-scaled-group"></a>
<a href="./svg-stroke-width-calc.svg" title="svg-stroke-width-calc (standalone-svg)"><img src="./chromium/svg-stroke-width-calc.png" width="56" alt="svg-stroke-width-calc"></a>
<a href="./svg-stroke-width-css.svg" title="svg-stroke-width-css (standalone-svg)"><img src="./chromium/svg-stroke-width-css.png" width="56" alt="svg-stroke-width-css"></a>
<a href="./svg-stroke-width-css-calc.svg" title="svg-stroke-width-css-calc (standalone-svg)"><img src="./chromium/svg-stroke-width-css-calc.png" width="56" alt="svg-stroke-width-css-calc"></a>
<a href="./svg-stroke-width-css-invalid-falls-back.svg" title="svg-stroke-width-css-invalid-falls-back (standalone-svg)"><img src="./chromium/svg-stroke-width-css-invalid-falls-back.png" width="56" alt="svg-stroke-width-css-invalid-falls-back"></a>
<a href="./svg-stroke-width-css-min.svg" title="svg-stroke-width-css-min (standalone-svg)"><img src="./chromium/svg-stroke-width-css-min.png" width="56" alt="svg-stroke-width-css-min"></a>
<a href="./svg-stroke-width-css-over-attr.svg" title="svg-stroke-width-css-over-attr (standalone-svg)"><img src="./chromium/svg-stroke-width-css-over-attr.png" width="56" alt="svg-stroke-width-css-over-attr"></a>
<a href="./svg-stroke-width-css-percent.svg" title="svg-stroke-width-css-percent (standalone-svg)"><img src="./chromium/svg-stroke-width-css-percent.png" width="56" alt="svg-stroke-width-css-percent"></a>
<a href="./svg-stroke-width-css-unitless.svg" title="svg-stroke-width-css-unitless (standalone-svg)"><img src="./chromium/svg-stroke-width-css-unitless.png" width="56" alt="svg-stroke-width-css-unitless"></a>
<a href="./svg-stroke-width-em-font-size.svg" title="svg-stroke-width-em-font-size (standalone-svg)"><img src="./chromium/svg-stroke-width-em-font-size.png" width="56" alt="svg-stroke-width-em-font-size"></a>
<a href="./svg-stroke-width-px.svg" title="svg-stroke-width-px (standalone-svg)"><img src="./chromium/svg-stroke-width-px.png" width="56" alt="svg-stroke-width-px"></a>
<a href="./svg-stroke-width-rem.svg" title="svg-stroke-width-rem (standalone-svg)"><img src="./chromium/svg-stroke-width-rem.png" width="56" alt="svg-stroke-width-rem"></a>
<a href="./svg-stroke-zero-extent-rect.svg" title="svg-stroke-zero-extent-rect (standalone-svg)"><img src="./chromium/svg-stroke-zero-extent-rect.png" width="56" alt="svg-stroke-zero-extent-rect"></a>
<a href="./svg-stroke-zero-length-dot.svg" title="svg-stroke-zero-length-dot (standalone-svg)"><img src="./chromium/svg-stroke-zero-length-dot.png" width="56" alt="svg-stroke-zero-length-dot"></a>
<a href="./svg-stroke-zero-width.svg" title="svg-stroke-zero-width (standalone-svg)"><img src="./chromium/svg-stroke-zero-width.png" width="56" alt="svg-stroke-zero-width"></a>
<a href="./svg-style-attribute-fill-rect.svg" title="svg-style-attribute-fill-rect (standalone-svg)"><img src="./chromium/svg-style-attribute-fill-rect.png" width="56" alt="svg-style-attribute-fill-rect"></a>
<a href="./svg-style-element-fill-rect.svg" title="svg-style-element-fill-rect (standalone-svg)"><img src="./chromium/svg-style-element-fill-rect.png" width="56" alt="svg-style-element-fill-rect"></a>
<a href="./svg-transform-malformed-drops.svg" title="svg-transform-malformed-drops (standalone-svg)"><img src="./chromium/svg-transform-malformed-drops.png" width="56" alt="svg-transform-malformed-drops"></a>
<a href="./svg-transform-no-separator.svg" title="svg-transform-no-separator (standalone-svg)"><img src="./chromium/svg-transform-no-separator.png" width="56" alt="svg-transform-no-separator"></a>
<a href="./svg-transform-runtogether.svg" title="svg-transform-runtogether (standalone-svg)"><img src="./chromium/svg-transform-runtogether.png" width="56" alt="svg-transform-runtogether"></a>
<a href="./svg-translucent-fill-rgba.svg" title="svg-translucent-fill-rgba (standalone-svg)"><img src="./chromium/svg-translucent-fill-rgba.png" width="56" alt="svg-translucent-fill-rgba"></a>
<a href="./svg-use.svg" title="svg-use (standalone-svg)"><img src="./chromium/svg-use.png" width="56" alt="svg-use"></a>
<a href="./svg-use-ancestor-circle.svg" title="svg-use-ancestor-circle (standalone-svg)"><img src="./chromium/svg-use-ancestor-circle.png" width="56" alt="svg-use-ancestor-circle"></a>
<a href="./svg-use-chain.svg" title="svg-use-chain (standalone-svg)"><img src="./chromium/svg-use-chain.png" width="56" alt="svg-use-chain"></a>
<a href="./svg-use-context-differs.svg" title="svg-use-context-differs (standalone-svg)"><img src="./chromium/svg-use-context-differs.png" width="56" alt="svg-use-context-differs"></a>
<a href="./svg-use-currentcolor.svg" title="svg-use-currentcolor (standalone-svg)"><img src="./chromium/svg-use-currentcolor.png" width="56" alt="svg-use-currentcolor"></a>
<a href="./svg-use-cycle-nothing.svg" title="svg-use-cycle-nothing (standalone-svg)"><img src="./chromium/svg-use-cycle-nothing.png" width="56" alt="svg-use-cycle-nothing"></a>
<a href="./svg-use-defs-rect.svg" title="svg-use-defs-rect (standalone-svg)"><img src="./chromium/svg-use-defs-rect.png" width="56" alt="svg-use-defs-rect"></a>
<a href="./svg-use-display-none-target.svg" title="svg-use-display-none-target (standalone-svg)"><img src="./chromium/svg-use-display-none-target.png" width="56" alt="svg-use-display-none-target"></a>
<a href="./svg-use-duplicate-id-first.svg" title="svg-use-duplicate-id-first (standalone-svg)"><img src="./chromium/svg-use-duplicate-id-first.png" width="56" alt="svg-use-duplicate-id-first"></a>
<a href="./svg-use-forward-ref.svg" title="svg-use-forward-ref (standalone-svg)"><img src="./chromium/svg-use-forward-ref.png" width="56" alt="svg-use-forward-ref"></a>
<a href="./svg-use-group.svg" title="svg-use-group (standalone-svg)"><img src="./chromium/svg-use-group.png" width="56" alt="svg-use-group"></a>
<a href="./svg-use-href-beats-xlink.svg" title="svg-use-href-beats-xlink (standalone-svg)"><img src="./chromium/svg-use-href-beats-xlink.png" width="56" alt="svg-use-href-beats-xlink"></a>
<a href="./svg-use-inherit-fill.svg" title="svg-use-inherit-fill (standalone-svg)"><img src="./chromium/svg-use-inherit-fill.png" width="56" alt="svg-use-inherit-fill"></a>
<a href="./svg-use-missing-nothing.svg" title="svg-use-missing-nothing (standalone-svg)"><img src="./chromium/svg-use-missing-nothing.png" width="56" alt="svg-use-missing-nothing"></a>
<a href="./svg-use-own-fill-wins.svg" title="svg-use-own-fill-wins (standalone-svg)"><img src="./chromium/svg-use-own-fill-wins.png" width="56" alt="svg-use-own-fill-wins"></a>
<a href="./svg-use-rendered-twice.svg" title="svg-use-rendered-twice (standalone-svg)"><img src="./chromium/svg-use-rendered-twice.png" width="56" alt="svg-use-rendered-twice"></a>
<a href="./svg-use-transform-xy.svg" title="svg-use-transform-xy (standalone-svg)"><img src="./chromium/svg-use-transform-xy.png" width="56" alt="svg-use-transform-xy"></a>
<a href="./svg-use-wh-inert.svg" title="svg-use-wh-inert (standalone-svg)"><img src="./chromium/svg-use-wh-inert.png" width="56" alt="svg-use-wh-inert"></a>
<a href="./svg-use-xlink-href.svg" title="svg-use-xlink-href (standalone-svg)"><img src="./chromium/svg-use-xlink-href.png" width="56" alt="svg-use-xlink-href"></a>
<a href="./svg-use-xy.svg" title="svg-use-xy (standalone-svg)"><img src="./chromium/svg-use-xy.png" width="56" alt="svg-use-xy"></a>
<a href="./svg-viewbox-only-sizing-rect.svg" title="svg-viewbox-only-sizing-rect (standalone-svg)"><img src="./chromium/svg-viewbox-only-sizing-rect.png" width="56" alt="svg-viewbox-only-sizing-rect"></a>
<a href="./svg-viewbox-unequal-default.svg" title="svg-viewbox-unequal-default (standalone-svg)"><img src="./chromium/svg-viewbox-unequal-default.png" width="56" alt="svg-viewbox-unequal-default"></a>
<a href="./svg-viewbox-uniform-offset-rect.svg" title="svg-viewbox-uniform-offset-rect (standalone-svg)"><img src="./chromium/svg-viewbox-uniform-offset-rect.png" width="56" alt="svg-viewbox-uniform-offset-rect"></a>
<a href="./svg-visibility-collapse-shape.svg" title="svg-visibility-collapse-shape (standalone-svg)"><img src="./chromium/svg-visibility-collapse-shape.png" width="56" alt="svg-visibility-collapse-shape"></a>
<a href="./svg-visibility-hidden-shape.svg" title="svg-visibility-hidden-shape (standalone-svg)"><img src="./chromium/svg-visibility-hidden-shape.png" width="56" alt="svg-visibility-hidden-shape"></a>
<a href="./svg-visibility-rule-beats-attribute.svg" title="svg-visibility-rule-beats-attribute (standalone-svg)"><img src="./chromium/svg-visibility-rule-beats-attribute.png" width="56" alt="svg-visibility-rule-beats-attribute"></a>
<a href="./svg-visibility-unhide.svg" title="svg-visibility-unhide (standalone-svg)"><img src="./chromium/svg-visibility-unhide.png" width="56" alt="svg-visibility-unhide"></a>

## The refusal register (56)

What the slice refuses, by name, in the compiler's own words —
**both refuse** is a document-level contract; **declared** renders
the rest and names the hole. A rung that admits a construct moves
its row into the cells above.

| Fixture | Admission | The compiler's departure |
| --- | --- | --- |
| `svg-clip-path` | declared | skipped svg/clipPath[1]: unsupported element <clipPath>; skipped svg/rect[2]: unsupported rendering attribute clip-path on <rect> (not yet consumed) |
| `svg-context-paint-fallback-extension` | declared | skipped svg/rect[2]: unsupported fill value "a context paint carries Stylo's non-standard fallback extension; Chromium drops this declaration" |
| `svg-css-individual-rotate` | declared | skipped svg/rect[2]: unsupported computed style: style attribute on <rect> declares rotate, which this cascade does not represent |
| `svg-css-transform-3d` | declared | skipped svg/rect[2]: unsupported computed style: transform on <rect> uses translate3d(), which is outside the 2D affine function set this slice consumes |
| `svg-css-transform-box` | declared | skipped svg/rect[2]: unsupported computed style: style attribute on <rect> declares transform-box, which this cascade does not represent |
| `svg-css-transform-origin` | declared | skipped svg/rect[2]: unsupported computed style: style attribute on <rect> declares transform-origin, which this cascade does not represent |
| `svg-display-contents` | declared | skipped svg/g[1]: unsupported computed style: display: contents is not yet consumed |
| `svg-element-opacity-gradient` | declared | skipped svg/rect[2]: unsupported computed style: opacity 0.5 over a gradient paint is not yet consumed (the paint carries one quantized alpha, and Chromium composites the element opacity after that quantization — expressing both needs a second paint-alpha factor) |
| `svg-filter` | declared | skipped svg/filter[1]: unsupported element <filter>; skipped svg/rect[2]: unsupported rendering attribute filter on <rect> (not yet consumed) |
| `svg-foreign-object` | declared | skipped svg/foreignObject[1]: unsupported element <foreignObject> |
| `svg-gradient-focal` | declared | skipped svg/rect[1]: unsupported fill value "url(#g): the radial gradient has a focal point or focal radius, which the shared radial paint leaf cannot state (concentric radials only)" |
| `svg-gradient-linearrgb` | declared | skipped svg/rect[1]: unsupported fill value "url(#g): color-interpolation: linearRGB interpolates stops in linear-light sRGB, which this slice does not execute (sRGB interpolation only)" |
| `svg-gradient-stop-css` | declared | declaration ignored at svg/style[1]: a stylesheet declares stop-color, which this cascade does not represent; elements it matches render without it |
| `svg-gradient-stop-style-attr` | declared | skipped svg/rect[1]: unsupported fill value "url(#g): a gradient <stop> declares stop-color in a style attribute, which this cascade does not represent" |
| `svg-gradient-unit-basis` | declared | skipped svg/rect[1]: unsupported fill value "url(#g): gradient geometry x2=\"4em\" uses a unit whose basis this slice does not consume (numbers, px, and percentages only)" |
| `svg-image` | declared | skipped svg/image[1]: unsupported element <image> |
| `svg-mask` | declared | skipped svg/mask[1]: unsupported element <mask>; skipped svg/rect[2]: unsupported rendering attribute mask on <rect> (not yet consumed) |
| `svg-nested-svg` | declared | skipped svg/svg[1]: unsupported element <svg> |
| `svg-path-css-d-property` | declared | declaration ignored at svg/style[1]: a stylesheet declares d, which this cascade does not represent; elements it matches render without it |
| `svg-path-malformed-d` | declared | skipped svg/path[1]: path data on <path> is invalid at byte 29 (near "qqq") |
| `svg-path-marker-end` | declared | skipped svg/path[1]: unsupported rendering attribute marker-end on <path> (not yet consumed) |
| `svg-path-no-leading-moveto` | declared | skipped svg/path[1]: path data on <path> is invalid at byte 0 (near "L10 10 L54 54 Z") |
| `svg-path-pathlength` | declared | skipped svg/path[1]: unsupported rendering attribute pathLength on <path> (not yet consumed) |
| `svg-path-trailing-dot-number` | declared | skipped svg/path[1]: path data on <path> is invalid at byte 1 (near "10. 10 L54 10 L54 54 Z") |
| `svg-pattern-paint-server` | declared | skipped svg/pattern[1]: unsupported element <pattern>; skipped svg/rect[2]: unsupported fill value "url(#p): url(#p) resolves to a <pattern> paint server, which the resolved frame cannot express" |
| `svg-points-odd-coordinate` | declared | skipped svg/polygon[1]: points on <polygon> is invalid at byte 17 (near "") |
| `svg-preserve-aspect-ratio-case-folded` | **both refuse** | preserveAspectRatio "xmidymid meet" is invalid |
| `svg-preserve-aspect-ratio-defer` | **both refuse** | preserveAspectRatio "defer xMidYMid meet" is invalid |
| `svg-preserve-aspect-ratio-invalid-align` | **both refuse** | preserveAspectRatio "xMidYMiddle meet" is invalid |
| `svg-root-opacity` | **both refuse** | unsupported computed style: opacity 0.5 on the root <svg> is not yet consumed (it composites the whole canvas, which needs a translucent surface entry) |
| `svg-smil-animate-transform` | declared | skipped svg/g[1]: its authored state is overridden at document load by the unsupported animation at svg/g[1]/animateTransform[1]: animation element <animateTransform> is outside the rect-x proving slice |
| `svg-smil-retarget-href` | **both refuse** | SVG animation at svg/rect[2]/set[1] is unsupported: animation element <set> is outside the rect-x proving slice; it carries href, so its target cannot be attributed to one element without id resolution; it is active at document load, so the authored state it overrides cannot render as the Base view |
| `svg-smil-set-load-active` | declared | skipped svg/rect[2]: its authored state is overridden at document load by the unsupported animation at svg/rect[2]/set[1]: animation element <set> is outside the rect-x proving slice |
| `svg-stroke-dasharray-cycle-overflow` | declared | skipped svg/path[1]: unsupported stroke value "a stroke-dasharray cycle has a finite authored grammar but its resolved total is not representable by this frame contract" |
| `svg-stroke-dasharray-escape` | declared | skipped svg/path[1]: unsupported stroke value "a stroke-dasharray on <path> carries a CSS escape this patrol cannot read" |
| `svg-stroke-dasharray-font-basis` | declared | skipped svg/path[1]: unsupported stroke value "a stroke-dasharray in em on <path> under an authored font-size carrying vw needs a basis this cascade does not have" |
| `svg-stroke-dasharray-sheet-unit` | declared | declaration ignored at svg/style[1]: a stylesheet declares a stroke-dasharray in ex, which needs a basis this cascade does not have; elements it matches render the wrong dash cycle |
| `svg-stroke-dasharray-var` | declared | skipped svg/path[1]: unsupported stroke value "a stroke-dasharray on <path> resolves through var(), an indirection this patrol cannot follow" |
| `svg-stroke-dashoffset` | declared | skipped svg/path[1]: unsupported computed style: style attribute on <path> declares stroke-dashoffset, which this cascade does not represent |
| `svg-stroke-paint-order` | declared | skipped svg/rect[1]: unsupported rendering attribute paint-order on <rect> (not yet consumed) |
| `svg-stroke-sheet-unit-width` | declared | declaration ignored at svg/style[1]: a stylesheet declares a stroke-width in ex, which needs a basis this cascade does not have; elements it matches render at the wrong width |
| `svg-stroke-vector-effect` | declared | skipped svg/g[1]/rect[1]: unsupported rendering attribute vector-effect on <rect> (not yet consumed) |
| `svg-stroke-width-calc-mixed` | declared | skipped svg/rect[1]: unsupported stroke value "a calc() stroke-width mixing lengths and percentages is not consumed" |
| `svg-stroke-width-font-basis` | declared | skipped svg/rect[1]: unsupported stroke value "a stroke-width in em on <rect> under an authored font-size carrying vw needs a basis this cascade does not have" |
| `svg-stroke-width-var` | declared | declaration ignored at svg/style[1]: a stylesheet declares a stroke-width through var(), an indirection this patrol cannot follow; elements it matches may render at the wrong width |
| `svg-switch` | declared | skipped svg/switch[1]: unsupported element <switch> |
| `svg-text-tspan` | declared | skipped svg/text[1]: unsupported element <tspan> |
| `svg-text-undeclared-font` | declared | skipped svg/text[1]: unsupported computed style: text resolution refused: font family "Undeclared" is not in the declared environment |
| `svg-use-authored-children` | declared | skipped svg/use[1]: unsupported <use>: it has authored element children, which Chromium replaces with the shadow content |
| `svg-use-external` | declared | skipped svg/use[1]: unsupported <use>: its reference is not a same-document fragment, and external resources are not resolved |
| `svg-use-stylesheet` | declared | skipped svg/use[1]: unsupported <use>: the document carries author CSS, and shadow-scoped selector matching is not yet consumed (selectors must match inside the cloned subtree alone — measured) |
| `svg-use-symbol` | declared | skipped svg/symbol[1]: unsupported element <symbol>; skipped svg/use[1]/symbol[1]: unsupported element <symbol> |
| `svg-viewbox-invalid-token` | **both refuse** | viewBox "0 0 invalid 64 64" is invalid |
| `svg-viewbox-repeated-comma` | **both refuse** | viewBox "0 0,,64 64" is invalid |
| `svg-viewbox-trailing-comma` | **both refuse** | viewBox "0 0 64 64," is invalid |
| `svg-width-percentage` | **both refuse** | unsupported SVG viewport sizing: percentage width="50%" on the root <svg> is not yet consumed |
