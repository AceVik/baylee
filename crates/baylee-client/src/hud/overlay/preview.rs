//! The hover preview: where it stands, what reaches past it, its faces.

#[allow(clippy::wildcard_imports)] // the overlay's shared vocabulary
use super::*;

/// The small cards beside a permanent's preview that are attached to it
/// (#305), with their caption.
#[derive(Component)]
pub(in crate::hud) struct PreviewAttached;

/// The full preview bubble, kept clear by persistent card annotations.
#[derive(Component)]
pub(crate) struct PreviewBounds;

/// The preview bubble's padding, in logical pixels: the gap its shadow needs
/// to read as a shadow rather than as a rim.
pub(super) const PREVIEW_PAD: f32 = 6.0;

/// How far past the bubble's padding the count badge reaches, in logical
/// pixels, for a card `img_w` wide: what the bubble's clip has to be let out
/// by so the badge and its shadow are not cut off, and how much taller than
/// the bubble the panel is placed as (#261).
///
/// The top's, which is the furthest it reaches past any edge: the preview
/// stands its badge over the card's top-right corner
/// ([`crate::badgemat::PREVIEW`]), and its shadow passes the card's right
/// edge by far less. The margin is the same on all four sides, so it lets
/// that out too, and nothing else, since the bubble is sized by what it
/// holds.
pub(in crate::hud) fn badge_reach(img_w: f32) -> f32 {
    let over = -baylee_client_core::cardplate::badge_quad_rect(crate::badgemat::PREVIEW)[1] * img_w;
    (over - PREVIEW_PAD).max(0.0)
}

/// How far past the bubble's padding the plate reaches off the card's right
/// edge, in logical pixels, for a card `img_w` wide: the preview stands it
/// beside the printed box, over the black border
/// ([`crate::platemat::preview_quad`]), as a row with room to spare does, so
/// a little of it and its shadow hang past the card.
pub(in crate::hud) fn plate_reach(img_w: f32) -> f32 {
    let quad = crate::platemat::preview_quad(baylee_client_core::cardplate::KIND_FIGHT);
    ((quad[2] - 1.0) * img_w - PREVIEW_PAD).max(0.0)
}

/// Lays one of the preview's shells over its card: a node of the frame's,
/// one side's like the badge and the plate, so it turns with the front and
/// is hidden with it at the quarter turn, over
/// [`crate::shellui::PREVIEW_QUAD`], where `shell_ui.wgsl` lays each shell
/// out.
pub(super) fn spawn_shell(
    commands: &mut Commands,
    surfaces: &mut Surfaces<'_>,
    frame: Entity,
    look: crate::shellmat::ShellLook,
    img_w: f32,
    img_h: f32,
) {
    let Some(material) = surfaces.shell(look) else {
        return;
    };
    let [x0, y0, x1, y1] = crate::shellui::PREVIEW_QUAD;
    let down = img_h / baylee_client_core::cardrail::CARD_TALL;
    let node = commands
        .spawn((
            crate::shellui::PreviewShell,
            MaterialNode(material),
            crate::flip::Side::Front,
            Visibility::Inherited,
            Node {
                position_type: PositionType::Absolute,
                left: px(x0 * img_w),
                top: px(y0 * down),
                width: px((x1 - x0) * img_w),
                height: px((y1 - y0) * down),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(frame).add_child(node);
}

/// How far past the bubble's padding the shells a card wears reach off it,
/// in logical pixels, for a card `img_w` wide: `[side, top]`, the first off
/// its sides and its foot, the second over its top edge. Nothing for a card
/// that wears none past its edge, as a wave does not. Their node reaches the
/// same way round every look ([`crate::shellui::PREVIEW_QUAD`]), and over the
/// top only a wall reaches further than a dome's foot does off the sides.
pub(in crate::hud) fn shell_reach(img_w: f32, shells: crate::shellmat::Shells) -> [f32; 2] {
    use crate::shellui::{PREVIEW_SIDE, PREVIEW_TOP};
    if !shells.reach_off_the_card() {
        return [0.0; 2];
    }
    let top = if shells.wall {
        PREVIEW_TOP
    } else {
        PREVIEW_SIDE
    };
    [PREVIEW_SIDE, top].map(|past| (past * img_w - PREVIEW_PAD).max(0.0))
}

/// Where the preview panel's top-left corner goes with everything that
/// hangs off its card: the badge `hang[0]` over its top edge, the plate
/// `hang[1]` off its right one, and the shells `shell` off its sides, its
/// foot and its top ([`shell_reach`]). Placed as a panel that much bigger
/// all round, so each lands on the screen wherever the panel does.
pub(in crate::hud) fn place_around(
    at: super::hand::PreviewAt,
    panel: Vec2,
    window: Vec2,
    keep_out: Option<Rect>,
    hang: [f32; 2],
    shell: [f32; 2],
) -> Vec2 {
    let ([badge, plate], [side, top]) = (hang, shell);
    let grown = panel + Vec2::new(side + plate.max(side), side);
    place_with_badge(at, grown, window, keep_out, badge.max(top)) + Vec2::new(side, 0.0)
}

/// What the preview takes of the screen beside its panel at `place`: the
/// plate `plate` off its card's right edge and the shells `shell` off its
/// sides, as [`place_around`] is given them, so what stands beside it (the
/// card underneath a copy, the cards attached to it, #305) stands clear of
/// both. The attachments' first column covered the plate's toughness until
/// 25.09. Its height is the panel's, whose foot what stands beside it
/// shares.
pub(in crate::hud) fn preview_taken(place: Vec2, panel: Vec2, plate: f32, shell: [f32; 2]) -> Rect {
    let side = shell[0];
    Rect::new(
        place.x - side,
        place.y,
        place.x + panel.x + plate.max(side),
        place.y + panel.y,
    )
}

/// Where the preview panel's top-left corner goes when a badge reaches
/// `reach` over its top edge: placed as a panel that much taller, so the
/// badge lands on the screen wherever the panel does.
pub(in crate::hud) fn place_with_badge(
    at: super::hand::PreviewAt,
    panel: Vec2,
    window: Vec2,
    keep_out: Option<Rect>,
    reach: f32,
) -> Vec2 {
    let shift = Vec2::new(0.0, reach);
    preview_place(at, panel + shift, window, keep_out) + shift
}

/// The preview's objects' material caches: its strip's, its count badge's,
/// its plate's and its shells'. Each is there only once its plugin is.
pub(super) type PreviewObjects<'w> = (
    Option<ResMut<'w, crate::marksmat::UiMarksMaterials>>,
    Option<ResMut<'w, crate::badgemat::UiBadgeMaterials>>,
    Option<ResMut<'w, crate::platemat::UiPlateMaterials>>,
    Option<ResMut<'w, crate::shellui::UiShellMaterials>>,
);

/// What the preview shows once it has been turned over.
///
/// `Some` is the printing's second face; `None` means the printed back —
/// which is the answer for every ordinary card, and the reason shift now
/// turns anything at all. It used to turn only a double-faced card, so the
/// gesture did nothing on nine cards out of ten and read as broken rather
/// than as inapplicable.
///
/// `has_back` is asked separately because having a *key* says nothing about
/// whether the printing has a second picture: every printing has one key, and
/// a card the seat may not see has none while still being turnable — over to
/// the back, which is exactly what everyone else at the table is looking at.
pub(in crate::hud) const fn far_face(key: Option<ImageKey>, has_back: bool) -> Option<ImageKey> {
    match key {
        Some(key) if has_back => Some(ImageKey {
            face: match key.face {
                baylee_client_core::images::Face::Front => baylee_client_core::images::Face::Back,
                baylee_client_core::images::Face::Back => baylee_client_core::images::Face::Front,
            },
            ..key
        }),
        _ => None,
    }
}

/// Where one face of the preview sits inside the frame that turns it.
///
/// Both faces stand in the same place, one on top of the other, and that is
/// the whole of it — but it has to be said, because a UI node laid out in its
/// parent's flow is an *item in a row*. Two of them in a frame one card wide
/// and taffy shrinks each to half of it: every preview in the client drew its
/// card squeezed into the left half of the panel, with the hidden face's
/// empty slot beside it. `Visibility::Hidden` does not give a node's place
/// back — only `Display::None` does, and a face that left the layout would
/// resize the frame halfway through the turn.
pub(in crate::hud) fn face_node(width: f32, height: f32) -> Node {
    Node {
        position_type: PositionType::Absolute,
        left: px(0),
        top: px(0),
        width: px(width),
        height: px(height),
        overflow: Overflow::clip(),
        ..default()
    }
}

/// Keep the whole preview, including its hints, above the playable hand ledge.
pub(in crate::hud) fn preview_with_footer_size(want: Vec2, window: Vec2, slip: f32) -> Vec2 {
    preview_art_size(
        want,
        PREVIEW_PAD,
        window - Vec2::new(0.0, HAND_ZONE_H + slip + super::preview_keys::HEIGHT),
    )
}

/// Whether the hovered object has a second picture to turn over to.
///
/// The view says which face is up, not whether there is another one, so the
/// answer comes from the registry the client already links for ability labels
/// and mana sources. A token or a face-down permanent has no card and
/// therefore no back.
///
/// It used to count the card's *compiled* faces, which is a third question
/// and answered neither (#115): an Adventure prints two names on one piece of
/// card, so this returned `true` for nine of them and the overlay then asked
/// Scryfall's `back` shelf for a picture that answers 404. The registry's
/// `sides` table is read off the printing instead, and the same table serves
/// the deck builder — one answer, computed once, rather than this predicate
/// and `PoolCard`'s disagreeing in two crates.
///
/// This doc had been sitting six items further up, above a `pick_hint` that
/// had its own, since whichever splice put it there — the shape
/// `doc-comment-splice-beheads-the-next-item` is named for. It came back when
/// `pick_hint` went to the drawer and left it standing over nothing.
pub(super) fn has_back_image(view: &PlayerView, hovered: Option<ObjectId>) -> bool {
    hovered
        .and_then(|id| {
            view.hand
                .iter()
                .find(|card| card.id == id)
                .map(|card| &card.card)
                .or_else(|| view.object(id).and_then(|object| object.card.as_ref()))
        })
        .is_some_and(|card| baylee_cards::sides::has_back_image(card.index))
}
