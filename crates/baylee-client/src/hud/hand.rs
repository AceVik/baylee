//! The hand bar along the bottom, its scrolling, and the hover preview
//! that rises out of it.

#[allow(clippy::wildcard_imports)] // the HUD's own vocabulary
use super::*;

/// The hand bar: a clipping container with the scrolling strip inside,
/// the commander zone pinned to its right end. Always on top of the
/// own-board overlay.
#[allow(clippy::too_many_arguments)]
#[allow(clippy::too_many_lines)] // strip + commander zone are one flat build
pub(super) fn spawn_hand_bar(
    commands: &mut Commands,
    lang: Lang,
    board: &baylee_client_core::BoardModel,
    view: &PlayerView,
    statics: &GameStatic,
    hovered: Option<ObjectId>,
    selected: &[ObjectId],
    selectable: &[ObjectId],
    armed: Option<&crate::Armed>,
    layout: HandLayout,
    scroll: f32,
    textures: &mut CardTextures,
    assets: &AssetServer,
    fonts: &UiFonts,
    faces: &FaceCtx<'_>,
    sheen: &crate::sheen::Sheen,
    mut cards: Option<&mut UiCards<'_>>,
) -> Entity {
    let bar = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                bottom: px(0),
                left: px(0),
                right: px(0),
                height: px(HAND_BAR_H),
                padding: UiRect::axes(px(HAND_BAR_PAD), px(HAND_BAR_PAD)),
                overflow: Overflow::clip(),
                ..default()
            },
            // The bar was 88% black across the whole bottom of the window,
            // and a hand of cards laid on a black strip is a hand of cards in
            // a *panel* — the one thing on this screen that is not supposed
            // to read as an interface. So it was made transparent, and the
            // cards ended up lying directly on the sky. Two things were lost
            // with the strip and only one of them was meant to go: the panel,
            // yes; but also the dark ground every card's glow was being read
            // against, which is why the owner reports the hand glow as
            // missing (`halo` has the measurement).
            //
            // This is the ground back without the panel. A vertical gradient
            // from nothing at the top edge to a soft blue-black at the
            // bottom: it has no top edge to read as a frame, it darkens where
            // the cards actually are, and it stops well short of the strip's
            // 88% — a veil over the table rather than a lid on it, and
            // `VEIL_ALPHA` has the measurement. The hue is `palette::PANEL`'s, one
            // step cooler — the felt and the sky are both saturated and a
            // neutral grey over either of them reads as dirt.
            //
            // No `BoxShadow` here, ever: a shadow is drawn from a node's
            // rectangle and not from its paint, so even a transparent bar
            // with an elevation shadow lays a hard dark band the width of the
            // window across the table. That is the trap this node has already
            // fallen into once, and a gradient is what a soft edge costs
            // instead.
            BackgroundColor(Color::NONE),
            BackgroundGradient::from(LinearGradient::to_bottom(vec![
                ColorStop::percent(VEIL.with_alpha(0.0), 0.0),
                ColorStop::percent(VEIL.with_alpha(VEIL_ALPHA * 0.45), 38.0),
                ColorStop::percent(VEIL.with_alpha(VEIL_ALPHA), 100.0),
            ])),
            ZIndex(2),
            Pickable::IGNORE,
        ))
        .id();

    let strip = commands
        .spawn((
            HandStrip,
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                top: px(HAND_HEADROOM),
                height: px(HAND_CARD_H),
                // Spawn already at the current scroll offset — starting at
                // zero and correcting next frame is the hand's flicker.
                margin: UiRect::left(px(HAND_STRIP_INSET + layout.lead - scroll)),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();

    for (i, card) in board.hand.iter().enumerate() {
        let is_selected = selected.contains(&card.id);
        let is_hovered = hovered == Some(card.id);
        // A card this choice would accept. Distinct from `playable`: that
        // one is the engine offering to put the card on the stack, this one
        // is a question already asked pointing at the hand.
        let is_offered = selectable.contains(&card.id);
        // What this client is offering to do with the card. A card in hand is
        // never activatable — that is a battlefield word — so the only bit
        // this can carry is the armed one, and `Deed::Run` puts nothing here
        // because the lands it would tap are on the table, not in the hand.
        let offer = crate::cardmat::Offer::on(armed, &[card.id], false);
        // No border: the card is rounded like a real one; hover/selection
        // read as a soft accent glow instead of a frame.
        let shadow = if is_selected {
            halo(palette::ACCENT, 1.0)
        } else if is_hovered || is_offered || card.playable || card.reachable {
            // Four different claims, and the two that matter are the two the
            // player reads without being told: gold is the engine saying yes,
            // indigo is this client offering to tap lands first. Weight, not
            // hue, separates a claim from an answer — a hover is the same
            // accent as a selection, one step quieter.
            let (tint, weight) = if is_hovered {
                (palette::ACCENT, 0.85)
            } else if is_offered {
                // The same gold as an offer from the engine, because that
                // is exactly what it is -- weaker only so a card already
                // picked still stands out from the ones that could be.
                (palette::ACCENT, 0.70)
            } else if card.playable {
                (palette::ACTIVE, 1.0)
            } else {
                // Not the afterthought it was. "Enough mana to cast it" is
                // what a player means by a card glowing, and for anything
                // that is not already paid for that is *this* light, not the
                // gold one — so it is the same halo, cooled rather than
                // dimmed away.
                (palette::REACHABLE, 0.88)
            };
            halo(tint, weight)
        } else {
            soft_shadow()
        };
        let built = view
            .hand
            .iter()
            .find(|h| h.id == card.id)
            .and_then(|h| faces.hand(h, textures, Some(card.art)));
        let image = textures.get(card.art, statics, assets);
        let visual = spawn_card_art(
            commands,
            lang,
            image,
            built.as_ref(),
            HAND_CARD_W,
            HAND_CARD_H,
            crate::face::Detail::Full,
            fonts,
            // A card in hand is not on a battlefield, so no keyword glow: the
            // border tells a player what is protected *there*, and a hand
            // that glowed would be saying something that is not yet true.
            // The armed ring is not a keyword and is drawn — it is a claim
            // about the card *in the hand*, and the hand is where the player
            // is looking when they arm a spell.
            //
            // The crest is the same exception for the same reason. It is not
            // a battlefield truth that has to wait: a commander is a
            // commander in every zone, and this is the zone CR 903.9b leaves
            // it in. `glow_of` cannot supply it here because a `HandObject`
            // is not a `PublicObject`, so the one bit it can say is added at
            // the call site.
            CardLook::art(
                card.art,
                finish_of(statics, Some(card.art)),
                crate::cardmat::glow_of(None, offer)
                    | if card.commander {
                        crate::cardmat::glow::COMMANDER
                    } else {
                        0
                    },
            )
            .with_sweep(sheen.of(card.id, crate::sheen::Surface::Hand)),
            cards.as_deref_mut(),
        );
        // Positioned by the layout rule; the strip's margin carries the
        // scroll offset (applied per frame, not rebuilt).
        let left = i as f32 * layout.step;
        let entity = commands
            .spawn((
                HandCardVisual { object: card.id },
                Node {
                    position_type: PositionType::Absolute,
                    left: px(left),
                    // An armed card stands out of the row, the way the table
                    // lifts an armed permanent. The bar keeps exactly this
                    // much headroom inside its own clip (`HAND_BAR_H` is the
                    // card plus twice ten, and the strip starts at ten), so
                    // the raise never cuts the card's top edge off.
                    top: px(if offer.armed { -ARMED_RAISE } else { 0.0 }),
                    width: px(HAND_CARD_W),
                    height: px(HAND_CARD_H),
                    border_radius: card_radius(HAND_CARD_W),
                    overflow: Overflow::clip(),
                    ..default()
                },
                shadow,
            ))
            .id();
        commands.entity(entity).add_child(visual);
        commands.entity(strip).add_child(entity);
    }
    commands.entity(bar).add_child(strip);

    // The command zone is *not* drawn here any more. It is a zone on the
    // table like the graveyard and the exile pile, and it was the only one
    // that had been copied into the hand bar as a flat 2D card — so a seat's
    // commander existed twice, in two sizes, in two renderers, and the 3D
    // slot beside the mat sat empty beneath the copy. `table::spawn_piles`
    // and the pile placements draw it now, where a public zone belongs
    // (CR 903.6), and the cast tax rides on the card itself.
    bar
}

/// Where the hand should be scrolled to, given where it is now.
///
/// The hovered card is brought fully into view — but **only when the hover
/// came from the keyboard**, and that condition is the whole of this
/// function's reason to exist.
///
/// The keyboard cursor can name a card scrolled off the end of the bar, and a
/// cursor on something nobody can see is not a cursor. A pointer never can:
/// the card is under it, so it is on the screen by construction. Scrolling it
/// "fully into view" therefore does nothing a player asked for and slides the
/// whole hand sideways beneath their own hand — and it does not settle, because
/// the strip moves, a different card arrives under the stationary pointer,
/// that card is hovered, and the strip moves again. The hand jumping and the
/// hand flickering were one branch, running for a hover it was never meant to
/// serve.
///
/// The result is unclamped: the caller owns the bounds, which depend on a
/// window this knows nothing about.
pub(super) fn hand_scroll_to(
    scroll: f32,
    hovered: Option<usize>,
    from_pointer: bool,
    layout: HandLayout,
    available: f32,
) -> f32 {
    let Some(index) = hovered.filter(|_| !from_pointer) else {
        return scroll;
    };
    let start = index as f32 * layout.step;
    let end = start + HAND_CARD_W;
    if start < scroll {
        start
    } else if end > scroll + available {
        end - available
    } else {
        scroll
    }
}

/// Applies the hand scroll offset and keeps the hovered card visible.
///
/// Runs per frame instead of being part of the rebuild: wheel ticks and
/// cursor moves must not respawn the whole strip.
pub fn apply_hand_scroll(
    mut duel: ResMut<Duel>,
    windows: Query<&Window>,
    mut strips: Query<&mut Node, With<HandStrip>>,
) {
    let (Some(board), Ok(window)) = (duel.board.as_ref(), windows.single()) else {
        return;
    };
    // The same width the rebuild lays the row out in. The two used to
    // differ, and that is what moved the row sideways on every rebuild.
    let available = hand_available(window.width());
    let layout = hand_layout(board.hand.len(), HAND_CARD_W, available);
    let max_scroll = (layout.content_width - available).max(0.0);

    let hovered = board.hand.iter().position(|c| Some(c.id) == duel.hovered);
    duel.hand_scroll = hand_scroll_to(
        duel.hand_scroll,
        hovered,
        duel.hovered_at.is_some(),
        layout,
        available,
    )
    .clamp(0.0, max_scroll);

    for mut node in &mut strips {
        let wanted = UiRect::left(px(HAND_STRIP_INSET + layout.lead - duel.hand_scroll));
        if node.margin != wanted {
            node.margin = wanted;
        }
    }
}

/// The middle of hand card `index`, in window pixels.
///
/// Every term the bar itself applies, in the order the bar applies them: its
/// own padding, the strip's inset, the layout's centring `lead`, the scroll
/// offset, and the card's place in the row. It exists because the preview
/// used to compute a shorter version of this sum — inset and step, no
/// padding and no `lead` — so the bubble opened `HAND_BAR_PAD + lead` to the
/// left of the card it belonged to. `lead` is half the bar's spare room, so
/// the emptier the hand, the further away the preview stood: five hundred
/// pixels on a wide window, which reads as a panel with no connection to
/// anything.
#[must_use]
pub(super) fn hand_card_x(layout: HandLayout, scroll: f32, index: usize) -> f32 {
    HAND_BAR_PAD + HAND_STRIP_INSET + layout.lead - scroll
        + index as f32 * layout.step
        + HAND_CARD_W / 2.0
}

/// Where the preview panel stands.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(super) enum PreviewAt {
    /// A card in the hand bar. The bubble sits above the bar with its tail on
    /// the card, which is where a hand card's preview has always opened and
    /// is the one place in the client where the card has a position the HUD
    /// itself knows.
    Hand(f32),
    /// A card on the felt, at the screen rectangle its four corners project
    /// to. The panel opens at that rectangle's edge — the card's edge, and
    /// not the point on its rim where the pointer crossed in, which is
    /// inside the card and put the panel over the thing it describes.
    Card(Rect),
    /// Anything the pointer found that has no rectangle of its own — a row in
    /// the zone browser, an entry in the stack panel. The panel stands beside
    /// the pointer instead, the way an ordinary tooltip does.
    Pointer(Vec2),
    /// A hover with no pointer behind it: the keyboard cursor names a card
    /// without standing anywhere, so the panel falls back to the middle.
    Loose,
}

/// Which card the preview shows and where it opens. Art comes from the hand,
/// the battlefield lanes, the piles, the stack panel, or the command zone.
pub(super) fn preview_anchor(
    board: &baylee_client_core::BoardModel,
    view: &PlayerView,
    hovered: Option<ObjectId>,
    layout: HandLayout,
    scroll: f32,
    spot: Option<crate::HoverSpot>,
) -> Option<(Option<ImageKey>, PreviewAt)> {
    let h = hovered?;
    // Where the hover happened, for everything that is not in the hand bar.
    // Checked once here rather than at each arm below, because "where the
    // panel goes" is the same question whatever the card turned out to be.
    let at = match spot {
        Some(crate::HoverSpot::Card(rect)) => PreviewAt::Card(rect),
        Some(crate::HoverSpot::Point(p)) => PreviewAt::Pointer(p),
        None => PreviewAt::Loose,
    };
    if let Some(i) = board.hand.iter().position(|c| c.id == h) {
        return Some((
            Some(board.hand[i].art),
            PreviewAt::Hand(hand_card_x(layout, scroll, i)),
        ));
    }
    for pod in &board.pods {
        for lane in &pod.lanes {
            for group in &lane.groups {
                if group.representative == h {
                    // A token has no art; it still gets a preview, built from
                    // its projected characteristics alone.
                    return Some((group.art, at));
                }
            }
        }
    }
    // A pile's top card lies face up on the table beside its mat and lifts
    // under the pointer like any other card, so it has to preview like one —
    // without this arm it lifts and shows nothing, which reads as a bug in
    // the preview rather than as a card that has none.
    for pod in &board.pods {
        for pile in &pod.piles {
            if pile.top == Some(h) {
                return Some((pile.art, at));
            }
        }
    }
    // The stack. Its panel draws cards an inch across, which is enough to
    // recognise a spell and not enough to read one — and the stack is
    // precisely where "what is about to happen" has to be read in a hurry.
    // A target is checked before its parent because a target of one entry can
    // be the spell of another, and the one under the pointer is the smaller
    // picture.
    for item in &board.stack {
        for target in &item.targets {
            if target.object() == Some(h) {
                return Some((target.art, at));
            }
        }
    }
    if let Some(item) = board.stack.iter().find(|item| item.id == h) {
        return Some((item.art, at));
    }
    if let Some(cmd) = view
        .command
        .get(view.seat.get() as usize)
        .and_then(|cmds| cmds.iter().find(|c| c.id == h))
    {
        return Some((
            cmd.card
                .map(|c| ImageKey::new(c.print, c.face, ArtSize::Normal)),
            at,
        ));
    }
    // Everything the zone browser lists and no arm above reaches: a graveyard
    // card that is not the top one, anything in an exile pile, and the cards
    // the engine is *showing* this seat, which live in no zone it can
    // otherwise see. The tray draws them 74 px across — enough to tell a
    // creature from a land and nowhere near enough to read one — so a search
    // through a hundred-card library was a wall of thumbnails a player had to
    // recognise by picture alone.
    //
    // Last, and deliberately: every arm above answers a card that is *also*
    // drawn somewhere, and this one answers by zone membership alone, so
    // putting it earlier would take the hand's tail and the pile's own place
    // away from cards that have them.
    if let Some(object) = view.object(h) {
        return Some((
            object
                .card
                .map(|c| ImageKey::new(c.print, c.face, ArtSize::Normal)),
            at,
        ));
    }
    None
}

/// The gap between the pointer and the panel it opened.
///
/// Wide enough that the panel never lands under the cursor arrow itself: the
/// preview describes the card the pointer is on, and one that covers the
/// pointer is describing something the player can no longer see.
const PREVIEW_GAP: f32 = 18.0;

/// How close the preview may come to an edge of the window.
const PREVIEW_INSET: f32 = 8.0;

/// The corners the panel's own corner may take and still be **wholly on the
/// screen**, with [`PREVIEW_INSET`] left around it.
///
/// This is the hard bound, and it is the one thing every arm below obeys
/// without exception: a preview is a thing the player is *reading*, and half
/// of one hanging off an edge is worth nothing at all. `high` is floored at
/// `low` so a panel bigger than the window still starts at the inset rather
/// than at a bound below its own origin — the crossed-bounds clamp, which
/// puts the panel's *top* off the screen instead of its bottom.
fn viewport(panel: Vec2, window: Vec2) -> (Vec2, Vec2) {
    let low = Vec2::splat(PREVIEW_INSET);
    (low, (window - panel - Vec2::splat(PREVIEW_INSET)).max(low))
}

/// The band the panel *prefers*: the viewport, kept off the window's own top
/// edge and out from under the hand bar.
///
/// A preference rather than a rule, and the difference is the whole point.
/// The bar is 174 logical pixels of hand, and a preview at a large
/// `preview_scale` on a modest window does not fit above it. The old code
/// clamped into the band anyway with its bounds crossed, which is a clamp
/// whose answer is whichever bound the library happens to apply last — a
/// placement nobody chose.
///
/// So: the band while the panel fits in it, and when it does not, **as high
/// as the panel can sit**. That is the least of the card the hand can cover,
/// and it is a decision rather than an accident.
fn band(panel: Vec2, window: Vec2) -> (Vec2, Vec2) {
    let (low, high) = viewport(panel, window);
    // Never past `high.y`: a panel taller than the window has nowhere to be
    // but at the inset, and the top edge must not be pushed off the screen
    // to keep a rule about the bottom one.
    let top = EDGE.clamp(low.y, high.y);
    let bottom = window.y - HAND_BAR_H - PREVIEW_INSET - panel.y;
    let floor = if bottom >= top { bottom } else { top };
    (Vec2::new(low.x, top), Vec2::new(high.x, floor))
}

/// How large the preview's picture may be drawn in this window.
///
/// Placement can put a panel anywhere; it cannot make one smaller than it is,
/// and a preview taller than the screen is cut off wherever it is put. So the
/// size is bounded here first — against the window less its padding, which is
/// the one bound that cannot be traded away — and the aspect is kept, because
/// a squashed card is a card a player reads the wrong number off.
///
/// Not against the hand bar as well: standing clear of the hand is a
/// *preference* the placement expresses, and paying for it in picture size on
/// a short window would make every preview smaller to protect a strip the
/// panel is allowed to overlap anyway.
///
/// `want` is what `preview_scale` asked for, and is usually granted whole:
/// this bites on a small window, a large scale, or both.
pub(super) fn preview_art_size(want: Vec2, pad: f32, window: Vec2) -> Vec2 {
    // The panel is the picture plus `pad` on every side, so the picture's own
    // room is the window less the inset and that padding.
    let room = window - Vec2::splat(2.0 * PREVIEW_INSET + 2.0 * pad);
    if room.x <= 0.0 || room.y <= 0.0 || want.x <= 0.0 || want.y <= 0.0 {
        return want;
    }
    want * (room.x / want.x).min(room.y / want.y).min(1.0)
}

/// Where the preview panel's top left corner goes, in logical pixels.
///
/// Pure arithmetic on purpose — it is the whole of the placement, and the
/// alternative is reading it off a photograph.
pub(super) fn preview_place(at: PreviewAt, panel: Vec2, window: Vec2) -> Vec2 {
    let (low, high) = viewport(panel, window);
    let banded = |v: Vec2| v.clamp(low, high);
    match at {
        PreviewAt::Hand(x) => banded(Vec2::new(
            x - panel.x / 2.0,
            window.y - HAND_BAR_H - 10.0 - panel.y,
        )),
        PreviewAt::Loose => banded(Vec2::new(
            (window.x - panel.x) / 2.0,
            window.y - HAND_BAR_H - 10.0 - panel.y,
        )),
        PreviewAt::Card(rect) => beside(rect, panel, low, high, window),
        // The pointer is a rectangle of no size: the same arithmetic, with
        // the gap measured from the one point there is.
        PreviewAt::Pointer(p) => beside(Rect::from_corners(p, p), panel, low, high, window),
    }
}

/// Where the little card underneath a copy stands, given where its preview
/// ended up.
///
/// Beside the preview and never on it — the preview is the picture being
/// explained, and half of it behind a thumbnail explains nothing. Its foot
/// sits on the preview's, which is what makes the two read as one thing.
///
/// A pure function for the same reason [`preview_place`] is one: this is the
/// half that can be quietly wrong, because a preview is *already* placed near
/// whichever window edge had the room, so "beside it" is very often outside
/// the window.
pub(super) fn underneath_place(preview: Rect, thumb: Vec2, window: Vec2) -> Vec2 {
    let gap = PREVIEW_GAP / 3.0;
    let x = if preview.max.x + gap + thumb.x + gap <= window.x {
        preview.max.x + gap
    } else {
        preview.min.x - gap - thumb.x
    };
    // Inside the window without exception, which is the clamp the branch
    // above cannot make on its own: a preview wide enough to fill the window
    // leaves no room on either flank.
    Vec2::new(
        x.clamp(gap, (window.x - thumb.x - gap).max(gap)),
        (preview.max.y - thumb.y).clamp(gap, (window.y - thumb.y - gap).max(gap)),
    )
}

/// The panel beside `card`, clear of it, and on the screen.
///
/// Split out because a card and a bare pointer want exactly the same
/// placement and differ only in how wide the thing being described is — and
/// that difference is the whole of the fault this fixes. A permanent on the
/// felt is about a hundred pixels across, so a panel opened `PREVIEW_GAP`
/// from the *pointer* opened some eighty pixels inside the card and covered
/// it; opened from the card's own right edge it stands clear of it.
fn beside(card: Rect, panel: Vec2, low: Vec2, high: Vec2, window: Vec2) -> Vec2 {
    let (from, to) = (card.min.x, card.max.x);
    let (top, bottom) = (card.min.y, card.max.y);
    let middle = card.center().y;
    // Kept clear of the window's own edge and of the hand bar while it fits
    // there, and inside the window without exception. It used to be kept
    // clear of the tab strip and the phase rail as well; both are on the
    // table now, so the preview may open a hundred pixels higher than it
    // could.
    let (bl, bh) = band(panel, window);
    // Centred on the span, which is what makes it read as belonging to it.
    let y = (middle - panel.y / 2.0).clamp(bl.y, bh.y);

    // On whichever side it fits — a panel that always opened to the right
    // would run off the screen for everything in the right-hand third of the
    // table, and clamping it back would put it straight over the card again.
    let right = to + PREVIEW_GAP;
    let left = from - PREVIEW_GAP - panel.x;
    if right + panel.x <= high.x {
        return Vec2::new(right, y);
    }
    if left >= low.x {
        return Vec2::new(left, y);
    }
    // Neither side has the room across. Before settling for covering the
    // card, try clearing it the other way: a card near the top or the bottom
    // of a tall window has room above or below it even when it has none
    // beside it, and a panel that clears the thing it describes is the whole
    // point of this function.
    let x = (f32::midpoint(from, to) - panel.x / 2.0).clamp(low.x, high.x);
    let above = top - PREVIEW_GAP - panel.y;
    let below = bottom + PREVIEW_GAP;
    if above >= bl.y {
        return Vec2::new(x, above);
    }
    if below <= bh.y {
        return Vec2::new(x, below);
    }
    // Nothing clears it. The panel goes on the side with the most room and
    // covers as little of the card as there is to cover — still, and above
    // everything else, wholly on the screen.
    let side = if window.x - to >= from { right } else { left };
    Vec2::new(side.clamp(low.x, high.x), y)
}

/// The face for whatever the preview is pointing at.
///
/// The hand is checked first because a hand card is a [`baylee_view::HandObject`]
/// and never appears in [`PlayerView::object`]; everything else — battlefield,
/// stack, graveyard, exile, command zone — is one lookup.
pub(super) fn preview_face(
    faces: &FaceCtx<'_>,
    view: &PlayerView,
    textures: &CardTextures,
    hovered: ObjectId,
    art: Option<ImageKey>,
) -> Option<CardFace> {
    if let Some(card) = view.hand.iter().find(|c| c.id == hovered) {
        return faces.hand(card, textures, art);
    }
    faces.object(view.object(hovered)?, textures, art)
}

/// The hand zone's own ground: `palette::PANEL`'s hue, one step cooler and
/// carrying no alpha of its own — the gradient's stops supply that.
const VEIL: Color = Color::srgb(0.04, 0.055, 0.085);

/// How dark the veil gets at the window's bottom edge.
///
/// Under two thirds deliberately: the felt, the sky and a seat's mat all
/// still read through it, which is the difference between a ground and a
/// panel and is what the owner asked for in the same breath as asking for it
/// at all — *slightly* transparent.
///
/// The number is bigger than the picture, and that is worth knowing before
/// reaching for it: **the gradient composites in linear space**, so an alpha
/// here buys much less darkening than sRGB arithmetic predicts. Measured
/// against the sky at the bottom edge, `0.44` took 136 to 108 rather than the
/// 81 the naive sum gives. This value takes it to about 94, a third down,
/// which is a ground a card's glow can be read against.
const VEIL_ALPHA: f32 = 0.58;

/// A card's glow: a wide soft halo with a tight bright ring inside it.
///
/// This was one `BoxShadow` with a blur of five or six and **no spread**,
/// which is to say a light that started falling off at the card's own edge —
/// the same shape, and very nearly the same size, as the drop shadow every
/// other card in the bar wears. It read as a glow for as long as the bar
/// painted an 88%-black strip behind it. With the strip gone the hand lies on
/// sky, and gold at 16% over pale blue is nothing at all; the owner reported
/// it as the glow having been *removed*, which is a fair reading of what is
/// on screen.
///
/// So: two shadows, because a light has a source. The ring says where it
/// comes from and the halo says how far it carries, and `spread_radius`
/// pushes both out past the card's edge before either begins to fall off,
/// which is the part that was missing.
///
/// `weight` scales the alpha of both, and is the only thing that separates
/// the three claims that share [`palette::ACCENT`]. Hue separates the two
/// that do not.
pub(super) fn halo(tint: Color, weight: f32) -> BoxShadow {
    BoxShadow(vec![
        ShadowStyle {
            color: tint.with_alpha(HALO_ALPHA * weight),
            x_offset: px(0.0),
            y_offset: px(0.0),
            spread_radius: px(HALO_SPREAD),
            blur_radius: px(HALO_BLUR),
        },
        ShadowStyle {
            color: tint.with_alpha(RING_ALPHA * weight),
            x_offset: px(0.0),
            y_offset: px(0.0),
            spread_radius: px(0.0),
            blur_radius: px(RING_BLUR),
        },
    ])
}

/// How far past the card's edge the halo is pushed before it starts to fall
/// off.
pub(super) const HALO_SPREAD: f32 = 2.0;
/// The halo's falloff, and the reason [`HALO_REACH`] is not this number.
pub(super) const HALO_BLUR: f32 = 8.0;
/// The halo's strongest alpha, at the card's edge.
const HALO_ALPHA: f32 = 0.62;
/// The ring's falloff: short, so the card keeps a hard edge to be lit from.
const RING_BLUR: f32 = 3.0;
/// The ring's alpha. Brighter than the halo and over far fewer pixels.
const RING_ALPHA: f32 = 0.88;

/// How far a halo actually carries, which is what the bar has to keep clear
/// of its own clip.
///
/// `blur_radius` is the gaussian's σ and `box_shadow.wgsl` integrates the
/// real thing, so the alpha at one σ past the edge is about 16% of the tint,
/// at one and a half about 7%, and at two about 2%.
///
/// Two, measured rather than argued. At one and a half the armed card — the
/// one raised by [`ARMED_RAISE`], so the one with the least room left — was
/// photographed with a step of 8 in the red channel across the bar's clip
/// line, against 72 at the card's own edge: an eleven-percent seam, faint but
/// there. Two σ puts that under one 8-bit level, and costs four pixels of
/// table.
pub(super) const HALO_REACH: f32 = HALO_SPREAD / 2.0 + 2.0 * HALO_BLUR;

/// Card width in the own-board overlay.
pub const OVERLAY_CARD_W: f32 = 86.0;
/// Card height in the own-board overlay (63:88).
pub const OVERLAY_CARD_H: f32 = OVERLAY_CARD_W * 88.0 / 63.0;
/// The hand bar's height, including its padding.
pub const HAND_BAR_H: f32 = HAND_CARD_H + HAND_HEADROOM + HAND_FOOTROOM;

/// How much room the bar keeps above a card, and why it is not ten.
///
/// The bar clips its children, and a `BoxShadow` is a child's paint like any
/// other — so a gap shorter than the tallest thing that can stand out of a
/// card cuts a glow off flat, which reads as a rectangle drawn round the card
/// rather than as a light coming off it. Two things stand out: the halo, and
/// an armed card raised by [`ARMED_RAISE`] with its halo still on. Both at
/// once is the bound.
pub const HAND_HEADROOM: f32 = ARMED_RAISE + HALO_REACH;

/// The room under a card, which nothing has to clear: below the bottom edge
/// is the window's own edge, and a halo cut off there is cut off by the
/// screen.
pub const HAND_FOOTROOM: f32 = 10.0;

/// How far an armed card stands out of the row.
///
/// This used to be bounded by the bar's padding — ten pixels of headroom that
/// happened to already exist. [`HAND_HEADROOM`] is derived from it now rather
/// than the other way round: a raise that has to fit inside a gap and a gap
/// sized to hold a raise are the same statement, written in the direction
/// that cannot silently go wrong.
pub(super) const ARMED_RAISE: f32 = 8.0;
