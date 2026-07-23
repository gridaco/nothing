//! ENG-2.2 · damage as data. [`diff_frame`] compares the actual resolved and
//! drawlist products with exact equality, so geometry, paint-only values,
//! opacity scopes, text/path artifacts, and painter ordering all flow into one
//! covering result. [`diff`] remains the geometry-only compatibility primitive.
//! Damage is asserted and shown, not yet consumed for partial repaint (OS-2a).

use n0_model::math::{Affine, RectF};
use n0_model::model::NodeId;
use n0_model::path::ResolvedPathArtifact;
use n0_model::resolve::Resolved;
use n0_model::text_layout::TextLayout;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use crate::drawlist::{DrawList, Item, ItemKind};
use crate::frame::FrameProduct;
use crate::paint::PaintEnvironmentKey;

/// What changed between two frame products: the touched nodes and the
/// world-space rect that bounds their before+after ink (covers
/// appear/disappear).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Damage {
    pub changed: Vec<NodeId>,
    pub union_world: Option<RectF>,
}

impl Damage {
    pub fn is_empty(&self) -> bool {
        self.changed.is_empty()
    }
}

/// One owner's exact producer-side state at the complete-frame damage seam.
///
/// The material value remains producer-owned; the policy only requires exact
/// equality. Coverage is separate because it is both material comparison data
/// and the before/after world-space result envelope.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DamageOwner<M> {
    material: M,
    coverage: Option<RectF>,
}

impl<M> DamageOwner<M> {
    pub(crate) fn new(material: M, coverage: Option<RectF>) -> Self {
        Self { material, coverage }
    }
}

/// Complete immutable frame data needed by the engine's one damage policy.
///
/// Item ownership is supplied independently from [`Item::node`]. This keeps
/// the policy exact over the existing private drawlist while allowing an
/// engine-internal producer to attribute the same items with its own stable
/// ordered key. The drawlist remains responsible for its private text-font
/// environment; no text contract is exposed here.
#[derive(Debug)]
pub(crate) struct FrameDamageInput<'a, K, M> {
    owners: BTreeMap<K, DamageOwner<M>>,
    items: Vec<DamageItem<'a, K>>,
    drawlist: &'a DrawList,
    environment: PaintEnvironmentKey,
}

impl<'a, K: Copy + Ord, M> FrameDamageInput<'a, K, M> {
    pub(crate) fn new(
        owners: BTreeMap<K, DamageOwner<M>>,
        item_owners: impl IntoIterator<Item = K>,
        drawlist: &'a DrawList,
        environment: PaintEnvironmentKey,
    ) -> Self {
        let item_owners = item_owners.into_iter().collect::<Vec<_>>();
        assert_eq!(
            item_owners.len(),
            drawlist.items.len(),
            "complete-frame damage needs exactly one owner per draw item"
        );
        assert!(
            item_owners.iter().all(|owner| owners.contains_key(owner)),
            "every draw-item owner needs material state and world coverage"
        );
        let items = item_owners
            .into_iter()
            .zip(&drawlist.items)
            .map(|(owner, item)| DamageItem { owner, item })
            .collect();
        Self {
            owners,
            items,
            drawlist,
            environment,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct DamageItem<'a, K> {
    owner: K,
    item: &'a Item,
}

/// Engine-private result from the complete-frame policy before attribution is
/// projected into the public n0 [`Damage`] type.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct FrameDamage<K> {
    pub(crate) changed: Vec<K>,
    pub(crate) union_world: Option<RectF>,
}

impl<K> Default for FrameDamage<K> {
    fn default() -> Self {
        Self {
            changed: Vec::new(),
            union_world: None,
        }
    }
}

/// Diff two resolved tiers: the nodes whose resolved geometry changed and the
/// world rect that bounds their before+after ink. Comparison is exact f32
/// equality per column (justified by ENG-0.3 determinism — "unchanged" is
/// identity, not an epsilon guess; -0.0 vs 0.0 is not a meaningful visual
/// change, so `==` is the right relation here). O(n) in slot count; day 1 it
/// is asserted and shown (HUD), not yet consumed for partial repaint (OS-2a).
pub fn diff(prev: &Resolved, next: &Resolved) -> Damage {
    let n = prev.slot_count().max(next.slot_count());
    let mut changed = BTreeSet::new();
    for id in 0..n as NodeId {
        if slot_changed(prev, next, id) {
            changed.insert(id);
        }
    }
    finish_damage(prev, next, changed)
}

/// Diff complete frame products. Resolved columns cover geometry, text, and
/// paths; per-node drawlist groups cover paints, opacity scopes, clips,
/// strokes, and painter order. The product-owned paint environment additionally
/// covers resource readiness and replacing bytes behind one logical id.
pub fn diff_frame(prev: &FrameProduct, next: &FrameProduct) -> Damage {
    let prev = frame_damage_input(prev);
    let next = frame_damage_input(next);
    let damage = diff_inputs(&prev, &next);
    Damage {
        changed: damage.changed,
        union_world: damage.union_world,
    }
}

/// Compare two complete-frame inputs with the engine's one exact damage
/// policy. This stays crate-private until a resolved render contract earns a
/// concrete public shape from multiple producers.
pub(crate) fn diff_inputs<K: Copy + Ord, M: PartialEq>(
    prev: &FrameDamageInput<'_, K, M>,
    next: &FrameDamageInput<'_, K, M>,
) -> FrameDamage<K> {
    let mut changed = BTreeSet::new();
    for owner in prev.owners.keys().chain(next.owners.keys()) {
        if prev.owners.get(owner) != next.owners.get(owner) {
            changed.insert(*owner);
        }
    }

    let prev_groups = group_items(prev);
    let next_groups = group_items(next);
    let mut appearance_changed = BTreeSet::new();
    for owner in prev_groups.keys().chain(next_groups.keys()) {
        if !same_group(prev_groups.get(owner), next_groups.get(owner)) {
            appearance_changed.insert(*owner);
        }
    }
    changed.extend(appearance_changed.iter().copied());

    // Remove nodes whose own item group already changed, then compare the
    // remaining `(node, ordinal-within-node)` permutation. Inserting an
    // opacity scope for a parent therefore marks the parent without falsely
    // marking every shifted descendant, while a genuine painter-order change
    // still marks every item whose relative position moved.
    let prev_positions = item_positions(prev, &appearance_changed);
    let next_positions = item_positions(next, &appearance_changed);
    for token in prev_positions.keys().chain(next_positions.keys()) {
        if prev_positions.get(token) != next_positions.get(token) {
            changed.insert(token.0);
        }
    }

    // The exact font registry is list-owned rather than repeated on each text
    // item. A registry change therefore damages the text nodes that consume
    // it even when their backend-independent item data is identical.
    if !prev.drawlist.same_text_fonts(next.drawlist) {
        for item in prev.items.iter().chain(&next.items) {
            if matches!(
                &item.item.kind,
                ItemKind::TextFill { .. } | ItemKind::TextStroke { .. }
            ) {
                changed.insert(item.owner);
            }
        }
    }

    if prev.environment != next.environment {
        changed.extend(prev.items.iter().map(|item| item.owner));
        changed.extend(next.items.iter().map(|item| item.owner));
    }

    finish_frame_damage(prev, next, changed)
}

fn group_items<'a, K: Copy + Ord, M>(
    input: &'a FrameDamageInput<'_, K, M>,
) -> BTreeMap<K, Vec<&'a Item>> {
    let mut groups = BTreeMap::<K, Vec<&Item>>::new();
    for item in &input.items {
        groups.entry(item.owner).or_default().push(item.item);
    }
    groups
}

fn same_group(prev: Option<&Vec<&Item>>, next: Option<&Vec<&Item>>) -> bool {
    match (prev, next) {
        (None, None) => true,
        (Some(prev), Some(next)) => {
            prev.len() == next.len()
                && prev
                    .iter()
                    .zip(next)
                    .all(|(prev, next)| same_item(prev, next))
        }
        _ => false,
    }
}

fn same_item(prev: &Item, next: &Item) -> bool {
    // Ownership is carried by DamageItem. Comparing Item::node here would
    // silently re-couple the policy to n0 arena slots.
    prev.world == next.world && prev.kind == next.kind
}

fn item_positions<K: Copy + Ord, M>(
    input: &FrameDamageInput<'_, K, M>,
    excluded: &BTreeSet<K>,
) -> BTreeMap<(K, usize), usize> {
    let mut ordinals = BTreeMap::<K, usize>::new();
    let mut positions = BTreeMap::new();
    let mut position = 0;
    for item in &input.items {
        if excluded.contains(&item.owner) {
            continue;
        }
        let ordinal = ordinals.entry(item.owner).or_default();
        positions.insert((item.owner, *ordinal), position);
        *ordinal += 1;
        position += 1;
    }
    positions
}

fn finish_frame_damage<K: Copy + Ord, M>(
    prev: &FrameDamageInput<'_, K, M>,
    next: &FrameDamageInput<'_, K, M>,
    changed: BTreeSet<K>,
) -> FrameDamage<K> {
    let mut union_world = None;
    for owner in &changed {
        if let Some(r) = prev.owners.get(owner).and_then(|owner| owner.coverage) {
            union_world = Some(union_rect(union_world, r));
        }
        if let Some(r) = next.owners.get(owner).and_then(|owner| owner.coverage) {
            union_world = Some(union_rect(union_world, r));
        }
    }
    FrameDamage {
        changed: changed.into_iter().collect(),
        union_world,
    }
}

fn finish_damage(prev: &Resolved, next: &Resolved, changed: BTreeSet<NodeId>) -> Damage {
    let mut union_world = None;
    for &id in &changed {
        // Cover the node's ink in BOTH states (appear/disappear/move).
        if let Some(r) = prev.aabb_opt(id) {
            union_world = Some(union_rect(union_world, r));
        }
        if let Some(r) = next.aabb_opt(id) {
            union_world = Some(union_rect(union_world, r));
        }
    }
    Damage {
        changed: changed.into_iter().collect(),
        union_world,
    }
}

#[derive(Debug, Clone, Copy)]
struct ResolvedDamageFact<'a> {
    box_in_parent: Option<RectF>,
    local: Option<Affine>,
    world: Option<Affine>,
    text_layout: Option<&'a Arc<TextLayout>>,
    path: Option<&'a Arc<ResolvedPathArtifact>>,
}

impl ResolvedDamageFact<'_> {
    fn is_absent(&self) -> bool {
        self.box_in_parent.is_none()
            && self.local.is_none()
            && self.world.is_none()
            && self.text_layout.is_none()
            && self.path.is_none()
    }
}

impl PartialEq for ResolvedDamageFact<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.box_in_parent == other.box_in_parent
            && self.local == other.local
            && self.world == other.world
            && self.text_layout == other.text_layout
            && match (self.path, other.path) {
                (None, None) => true,
                (Some(prev), Some(next)) => prev.same_visual_geometry(next),
                _ => false,
            }
    }
}

fn resolved_damage_fact(resolved: &Resolved, id: NodeId) -> ResolvedDamageFact<'_> {
    ResolvedDamageFact {
        box_in_parent: resolved.box_opt(id),
        local: resolved.local_opt(id),
        world: resolved.world_opt(id),
        text_layout: resolved.text_layout_opt(id),
        path: resolved.resolved_path_opt(id),
    }
}

fn frame_damage_input(
    frame: &FrameProduct,
) -> FrameDamageInput<'_, NodeId, ResolvedDamageFact<'_>> {
    let resolved = frame.resolved();
    let mut owners = BTreeMap::new();
    for id in 0..resolved.slot_count() as NodeId {
        let material = resolved_damage_fact(resolved, id);
        let coverage = resolved.aabb_opt(id);
        // Preserve the old slot-walk relation: an out-of-range slot and a
        // tombstoned all-None slot compare equal.
        if !material.is_absent() || coverage.is_some() {
            owners.insert(id, DamageOwner::new(material, coverage));
        }
    }
    FrameDamageInput::new(
        owners,
        frame.drawlist().items.iter().map(|item| item.node),
        frame.drawlist(),
        frame.environment(),
    )
}

fn slot_changed(prev: &Resolved, next: &Resolved, id: NodeId) -> bool {
    resolved_damage_fact(prev, id) != resolved_damage_fact(next, id)
        || prev.aabb_opt(id) != next.aabb_opt(id)
}

fn union_rect(acc: Option<RectF>, r: RectF) -> RectF {
    match acc {
        None => r,
        Some(a) => {
            let x0 = a.x.min(r.x);
            let y0 = a.y.min(r.y);
            let x1 = (a.x + a.w).max(r.x + r.w);
            let y1 = (a.y + a.h).max(r.y + r.h);
            RectF {
                x: x0,
                y: y0,
                w: x1 - x0,
                h: y1 - y0,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame;
    use crate::paint::PaintCtx;
    use n0_model::model::{
        AxisBinding, Color, CornerSmoothing, DocBuilder, Header, Paints, Payload,
        RectangularCornerRadius, ShapeDesc, SizeIntent,
    };
    use n0_model::properties::{
        PropertyKey, PropertyTarget, PropertyValue, PropertyValues, ValueView,
    };
    use n0_model::resolve::ResolveOptions;
    use skia_safe::FontMgr;

    const INTER: &[u8] =
        include_bytes!("../../../fixtures/fonts/Inter/Inter-VariableFont_opsz,wght.ttf");

    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    struct OwnerKey(u64);

    const RECT: OwnerKey = OwnerKey(10);
    const TEXT: OwnerKey = OwnerKey(20);

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct MaterialFact {
        kind: &'static str,
        revision: u8,
    }

    fn coverage() -> RectF {
        RectF {
            x: 10.0,
            y: 10.0,
            w: 28.0,
            h: 24.0,
        }
    }

    fn rect_list(node: NodeId, color: Color) -> DrawList {
        let mut list = DrawList::default();
        list.items.push(Item {
            node,
            world: Affine::translate(10.0, 10.0),
            kind: ItemKind::RectFill {
                w: 28.0,
                h: 24.0,
                corner_radius: RectangularCornerRadius::default(),
                corner_smoothing: CornerSmoothing::default(),
                paints: Paints::solid(color),
            },
        });
        list
    }

    fn literal_input<'a>(
        list: &'a DrawList,
        owner: OwnerKey,
        revision: u8,
        environment: PaintEnvironmentKey,
    ) -> FrameDamageInput<'a, OwnerKey, MaterialFact> {
        FrameDamageInput::new(
            BTreeMap::from([(
                owner,
                DamageOwner::new(
                    MaterialFact {
                        kind: "rectangle",
                        revision,
                    },
                    Some(coverage()),
                ),
            )]),
            [owner],
            list,
            environment,
        )
    }

    #[test]
    fn complete_policy_accepts_non_document_owners_and_ignores_item_node_slots() {
        let context = PaintCtx::new(None);
        let environment = context.environment_key();
        let before_list = rect_list(7, Color(0xFFDC_2626));
        let same_list_with_another_slot = rect_list(99, Color(0xFFDC_2626));
        let changed_list = rect_list(99, Color(0xFF25_63EB));
        let before = literal_input(&before_list, RECT, 1, environment);
        let same = literal_input(&same_list_with_another_slot, RECT, 1, environment);
        let changed = literal_input(&changed_list, RECT, 2, environment);

        assert_eq!(
            diff_inputs(&before, &same),
            FrameDamage::default(),
            "the supplied opaque owner, not the private drawlist slot, owns attribution"
        );
        assert_eq!(
            diff_inputs(&before, &changed),
            FrameDamage {
                changed: vec![RECT],
                union_world: Some(coverage()),
            },
            "one exact material transition returns that owner and its exact coverage"
        );
    }

    #[test]
    fn complete_policy_treats_owner_key_replacement_as_disappear_and_appear() {
        let context = PaintCtx::new(None);
        let environment = context.environment_key();
        let list = rect_list(7, Color(0xFFDC_2626));
        let replacement = OwnerKey(11);
        let before = literal_input(&list, RECT, 1, environment);
        let after = literal_input(&list, replacement, 1, environment);

        assert_eq!(
            diff_inputs(&before, &after),
            FrameDamage {
                changed: vec![RECT, replacement],
                union_world: Some(coverage()),
            }
        );
    }

    fn positioned_header(x: f32, y: f32, w: f32, h: f32) -> Header {
        let mut header = Header::new(SizeIntent::Fixed(w), SizeIntent::Fixed(h));
        header.x = AxisBinding::start(x);
        header.y = AxisBinding::start(y);
        header
    }

    fn projected_input<'a>(
        product: &'a FrameProduct,
        projection: &BTreeMap<NodeId, OwnerKey>,
    ) -> FrameDamageInput<'a, OwnerKey, ResolvedDamageFact<'a>> {
        let owners = projection
            .iter()
            .map(|(&node, &owner)| {
                (
                    owner,
                    DamageOwner::new(
                        resolved_damage_fact(product.resolved(), node),
                        product.resolved().aabb_opt(node),
                    ),
                )
            })
            .collect();
        let item_owners = product.drawlist().items.iter().map(|item| {
            *projection
                .get(&item.node)
                .expect("every mixed draw item has an opaque owner")
        });
        FrameDamageInput::new(
            owners,
            item_owners,
            product.drawlist(),
            product.environment(),
        )
    }

    #[test]
    fn projected_complete_policy_preserves_real_private_text_and_public_damage() {
        let mut builder = DocBuilder::new();
        let rect = builder.add(
            0,
            positioned_header(10.0, 10.0, 28.0, 24.0),
            Payload::Shape {
                desc: ShapeDesc::Rect,
            },
        );
        let text = builder.add(
            0,
            positioned_header(50.0, 10.0, 30.0, 24.0),
            Payload::Text {
                content: "A".into(),
                font_size: 18.0,
            },
        );
        builder.node_mut(rect).fills = Paints::solid(Color(0xFFDC_2626));
        builder.node_mut(text).fills = Paints::solid(Color::BLACK);
        let document = builder.build();
        let options = ResolveOptions {
            viewport: (180.0, 100.0),
            ..Default::default()
        };
        let typeface = FontMgr::new()
            .new_from_data(INTER, None)
            .expect("bundled Inter typeface");
        let context = PaintCtx::new(Some(typeface));
        let before =
            frame::resolve_and_build(&document, &options, &context).expect("valid base frame");

        let values = PropertyValues::new(
            &document,
            [(
                PropertyTarget::new(
                    document.key_of(rect).expect("live rectangle"),
                    PropertyKey::Fills,
                ),
                PropertyValue::Paints(Paints::solid(Color(0xFF25_63EB))),
            )],
        )
        .expect("valid exact fill override");
        let view = ValueView::new(&document, &values).expect("validated effective view");
        let after = frame::resolve_and_build_view(&view, &options, &context)
            .expect("valid effective frame");

        let shaped = before
            .drawlist()
            .items
            .iter()
            .find_map(|item| match &item.kind {
                ItemKind::TextFill { layout, .. } if item.node == text => Some(layout),
                _ => None,
            })
            .expect("real private shaped text is interleaved");
        assert!(!shaped.glyph_runs.is_empty());
        assert!(before.drawlist().same_text_fonts(after.drawlist()));

        let projection = BTreeMap::from([(rect, RECT), (text, TEXT)]);
        let projected_before = projected_input(&before, &projection);
        let projected_after = projected_input(&after, &projection);
        let projected = diff_inputs(&projected_before, &projected_after);
        assert_eq!(
            projected,
            FrameDamage {
                changed: vec![RECT],
                union_world: Some(coverage()),
            },
            "unchanged private text remains ordered but is not damaged"
        );

        let public = diff_frame(&before, &after);
        let public_projected = public
            .changed
            .iter()
            .map(|node| projection[node])
            .collect::<Vec<_>>();
        assert_eq!(projected.changed, public_projected);
        assert_eq!(projected.union_world, public.union_world);
    }
}
