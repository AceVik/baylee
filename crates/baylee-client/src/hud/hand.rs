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
                padding: UiRect::axes(px(10), px(10)),
                overflow: Overflow::clip(),
                ..default()
            },
            // Nothing. The bar was 88% black across the whole bottom of the
            // window, and a hand of cards laid on a black strip is a hand of
            // cards in a *panel* — the one thing on this screen that is not
            // supposed to read as an interface. The cards keep their own
            // corner cut (`card_ui.wgsl` takes the scan's white corners out
            // in alpha) and their own shadow, so the strip has nothing left
            // to do but let the felt through.
            //
            // The shadow went with it, and had to: a `BoxShadow` is drawn
            // from the node's rectangle and not from its paint, so a
            // transparent bar with an elevation shadow under it still lays a
            // dark band the width of the window over the table. The function
            // that cast it is gone too — the phase rail was its other caller,
            // and the rail is on the table now.
            BackgroundColor(Color::NONE),
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
                top: px(10),
                height: px(HAND_CARD_H),
                // Spawn already at the current scroll offset — starting at
                // zero and correcting next frame is the hand's flicker.
                margin: UiRect::left(px(10.0 + layout.lead - scroll)),
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
            BoxShadow::new(
                palette::ACCENT,
                Val::Px(0.0),
                Val::Px(0.0),
                Val::Px(2.0),
                Val::Px(10.0),
            )
        } else if is_hovered || is_offered || card.playable || card.reachable {
            // Three different claims, three different glows. Gold is the
            // engine saying yes; indigo is this client offering to tap lands
            // first, which is a weaker thing and reads as one.
            let (tint, spread) = if is_hovered {
                (palette::ACCENT, 8.0)
            } else if is_offered {
                // The same gold as an offer from the engine, because that
                // is exactly what it is -- weaker only so a card already
                // picked still stands out from the ones that could be.
                (palette::ACCENT, 5.0)
            } else if card.playable {
                (palette::ACTIVE, 6.0)
            } else {
                (palette::REACHABLE, 5.0)
            };
            BoxShadow::new(
                tint,
                Val::Px(0.0),
                Val::Px(0.0),
                Val::Px(0.0),
                Val::Px(spread),
            )
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
    let available = (window.width() - 20.0).max(0.0);
    let layout = hand_layout(board.hand.len(), HAND_CARD_W, available);
    let max_scroll = (layout.content_width - available).max(0.0);

    // Keep the hovered card fully in view.
    if let Some(index) = board.hand.iter().position(|c| Some(c.id) == duel.hovered) {
        let start = index as f32 * layout.step;
        let end = start + HAND_CARD_W;
        if start < duel.hand_scroll {
            duel.hand_scroll = start;
        } else if end > duel.hand_scroll + available {
            duel.hand_scroll = end - available;
        }
    }
    duel.hand_scroll = duel.hand_scroll.clamp(0.0, max_scroll);

    for mut node in &mut strips {
        let wanted = UiRect::left(px(10.0 + layout.lead - duel.hand_scroll));
        if node.margin != wanted {
            node.margin = wanted;
        }
    }
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
        let x = 10.0 + i as f32 * layout.step - scroll + HAND_CARD_W / 2.0;
        return Some((Some(board.hand[i].art), PreviewAt::Hand(x)));
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

/// Where the preview panel's top left corner goes, in logical pixels.
///
/// Pure arithmetic on purpose — it is the whole of the placement, and the
/// alternative is reading it off a photograph.
pub(super) fn preview_place(at: PreviewAt, panel: Vec2, window: Vec2) -> Vec2 {
    // The band the panel may stand in: the tab strip and the phase rail
    // above, the hand bar below, and the window on both sides -- the rail
    // used to take the right-hand edge and no longer does. Clamped so that a
    // panel too tall for the band still starts at the top of it rather than
    // below its bottom, which is what a naive clamp with crossed bounds does.
    let low = Vec2::splat(PREVIEW_INSET);
    let high = (window - panel - Vec2::splat(PREVIEW_INSET)).max(low);
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
        PreviewAt::Card(rect) => beside(
            rect.min.x,
            rect.max.x,
            rect.center().y,
            panel,
            low,
            high,
            window,
        ),
        // The pointer is a rectangle of no width: the same arithmetic, with
        // the gap measured from the one point there is.
        PreviewAt::Pointer(p) => beside(p.x, p.x, p.y, panel, low, high, window),
    }
}

/// The panel beside a span, vertically centred on `middle`.
///
/// Split out because a card and a bare pointer want exactly the same
/// placement and differ only in how wide the thing being described is — and
/// that difference is the whole of the fault this fixes. A permanent on the
/// felt is about a hundred pixels across, so a panel opened `PREVIEW_GAP`
/// from the *pointer* opened some eighty pixels inside the card and covered
/// it; opened from the card's own right edge it stands clear of it.
#[allow(clippy::too_many_arguments)] // a span, a panel and the band it fits in
fn beside(
    from: f32,
    to: f32,
    middle: f32,
    panel: Vec2,
    low: Vec2,
    high: Vec2,
    window: Vec2,
) -> Vec2 {
    // On whichever side it fits — a panel that always opened to the right
    // would run off the screen for everything in the right-hand third of the
    // table, and clamping it back would put it straight over the card again.
    let right = to + PREVIEW_GAP;
    let left = from - PREVIEW_GAP - panel.x;
    let x = if right + panel.x <= high.x {
        right
    } else if left >= low.x {
        left
    } else if window.x - to >= from {
        // Neither side has the room. The clamp below is going to slide the
        // panel back over the card whatever happens, so it goes on the side
        // with more space and covers as little of it as there is to cover.
        right
    } else {
        left
    };
    // Kept clear of the window's own edge and of the hand bar. It used to be
    // kept clear of the tab strip and the phase rail as well; both are on the
    // table now, so the preview may open a hundred pixels higher than it
    // could.
    let y = middle - panel.y / 2.0;
    Vec2::new(x, y).clamp(
        Vec2::new(low.x, EDGE),
        Vec2::new(
            high.x,
            (window.y - HAND_BAR_H - PREVIEW_INSET - panel.y).max(low.y),
        ),
    )
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

/// Card width in the own-board overlay.
pub const OVERLAY_CARD_W: f32 = 86.0;
/// Card height in the own-board overlay (63:88).
pub const OVERLAY_CARD_H: f32 = OVERLAY_CARD_W * 88.0 / 63.0;
/// The hand bar's height, including its padding.
pub const HAND_BAR_H: f32 = HAND_CARD_H + 20.0;

/// How far an armed card stands out of the row.
///
/// Bounded by the bar's own padding: the strip sits ten pixels down inside a
/// clipping container, so anything up to ten is headroom that already exists
/// and anything past it would take the top off the card instead of raising
/// it.
const ARMED_RAISE: f32 = 8.0;
