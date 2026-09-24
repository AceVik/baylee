//! The hand zone along the bottom, its scrolling, and the hover preview
//! that rises out of it.

#[allow(clippy::wildcard_imports)] // the HUD's own vocabulary
use super::*;

/// The hand zone: a clipping container with the scrolling strip inside and
/// the veil beneath it, reaching [`LEDGE_H`] further up than the cards do so
/// that the ledge has something to stand on.
///
/// The ledge itself is **not** built here and is not a child of this node —
/// see `ledge::spawn_ledge` for why a sibling.
#[allow(clippy::too_many_arguments)]
#[allow(clippy::too_many_lines)] // strip, veil and cards are one flat build
pub(super) fn spawn_hand_zone(
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
    order: crate::hand_order::HandOrder,
    groups: &[crate::hand_order::HandGroup],
    textures: &mut CardTextures,
    assets: &AssetServer,
    fonts: &UiFonts,
    faces: &FaceCtx<'_>,
    sheen: &crate::sheen::Sheen,
    touch: &crate::touch::Touched,
    mut cards: Option<&mut UiCards<'_>>,
    cloth: Option<Handle<crate::frontal::FrontalMaterial>>,
) -> Entity {
    let zone = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                bottom: px(0),
                left: px(0),
                right: px(0),
                height: px(HAND_ZONE_H),
                padding: UiRect::axes(px(HAND_BAR_PAD), px(HAND_BAR_PAD)),
                overflow: Overflow::clip(),
                ..default()
            },
            // The strip under the cards was 88% black across the whole bottom
            // of the window, and a hand of cards laid on a black strip is a
            // hand of cards in a *panel* — the one thing on this screen that
            // is not supposed to read as an interface. So it was made
            // transparent, and the cards ended up lying directly on the sky.
            // Two things were lost with the strip and only one of them was
            // meant to go: the panel, yes; but also the dark ground every
            // card's glow was being read against, which is why the owner
            // reported the hand glow as missing (`halo` has the measurement).
            //
            // The ground is back as the veil child below — a gradient from
            // nothing to a soft blue-black, a veil over the table rather than
            // a lid on it. The zone itself paints nothing at all.
            //
            // No `BoxShadow` here, ever: a shadow is drawn from a node's
            // rectangle and not from its paint, so even a transparent zone
            // with an elevation shadow lays a hard dark band the width of the
            // window across the table. That is the trap this node has already
            // fallen into once. The ledge is allowed one because the ledge is
            // opaque and its shadow falls on the cards, which is the whole
            // point of it.
            BackgroundColor(Color::NONE),
            ZIndex(Z_HAND),
            // Hoverable, and still blocking nothing. The zone lies over
            // the table and a click has always gone straight through it —
            // which is what `should_block_lower: false` keeps — but a wheel
            // has to *land* somewhere to be the hand's, and the gaps between
            // the cards are most of it. `Pickable::IGNORE` is both bits
            // off, which made the whole zone invisible to the pointer and
            // left "is this scroll the hand's" to a rectangle measured from
            // the bottom of the window.
            Pickable {
                should_block_lower: false,
                is_hoverable: true,
            },
            super::HandScroll,
        ))
        .id();

    // The skirt, and the reason it is a child rather than the zone's own
    // paint: a `BackgroundColor` is a colour where this is a surface, and the
    // zone is also the node a wheel has to land on, which a material node
    // would stop being.
    //
    // It covers the **whole** zone — `top: 0`, not `top: LEDGE_H`. That is
    // the owner's "make the background complete, like it's a container"
    // (14.09.2026), and it inverts the reason the ground used to start at the
    // shelf's lower edge: a ground behind an opaque shelf is a ground nobody
    // can see, and the shelf is no longer opaque. It is also what makes the
    // shelf's own transparency affordable, because the shelf is then
    // composited over this cloth instead of over the sky —
    // `palette::LEDGE_SOFT` carries that arithmetic.
    //
    // Spawned before the strip and added as a child first: children paint in
    // order, and a ground added after the cards is a ground drawn over them.
    //
    // What it is painted with is `crate::frontal`, and it arrives here as a
    // handle rather than as a material because this whole zone is respawned
    // on every pointer move. Without a render world there is no handle and a
    // flat ground of the same dye is drawn instead: a ground is a ground, and
    // every headless test that builds this tree needs one.
    let skirt = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: px(0),
                left: px(0),
                right: px(0),
                bottom: px(0),
                // The same two corners the shelf rounds and the cloth cuts, so
                // the headless ground is the shape the drawn one is. The
                // shader cuts them itself — whether a `BorderRadius` reaches a
                // `MaterialNode` at all is a question — and the two agree
                // because they read the same constant.
                border_radius: BorderRadius::top(px(crate::frontal::CORNER)),
                ..default()
            },
            BackgroundColor(Color::NONE),
            Pickable::IGNORE,
        ))
        .id();
    match cloth {
        Some(handle) => {
            commands
                .entity(skirt)
                .insert((MaterialNode(handle), crate::frontal::Hanging));
        }
        None => {
            commands
                .entity(skirt)
                .insert(BackgroundColor(palette::DOCK_GROUND.with_alpha(GROUND)));
        }
    }
    commands.entity(zone).add_child(skirt);

    let strip = commands
        .spawn((
            HandStrip,
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                // Under the ledge, then the air a raised card needs. The
                // zone's own padding does not move an absolutely-positioned
                // child: taffy measures one against the padding *box*, which
                // is the border box less the border, and there is no border.
                top: px(LEDGE_H + HAND_HEADROOM),
                height: px(HAND_CARD_H),
                // Spawn already at the current scroll offset — starting at
                // zero and correcting next frame is the hand's flicker.
                margin: UiRect::left(px(HAND_STRIP_INSET + layout.lead - scroll)),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();

    for (i, group) in groups.iter().enumerate() {
        let end = groups.get(i + 1).map_or(board.hand.len(), |g| g.start);
        let label = format!(
            "{} · {}",
            order.group_label(group.key, lang),
            end - group.start
        );
        let heading = commands
            .spawn((
                Text::new(label),
                TextLayout::no_wrap(),
                tf_bold(fonts, 10.0),
                TextColor(palette::CANDLE),
                Node {
                    position_type: PositionType::Absolute,
                    left: px(layout.start(group.start)),
                    width: px(layout.start(end - 1) + HAND_CARD_W - layout.start(group.start)),
                    top: px(-HAND_HEADROOM + 2.0),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(strip).add_child(heading);
    }

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
        //
        // `Proposing::Owed` cannot reach here for the same reason and is not
        // passed: a payment window's plan taps lands, and a land being tapped
        // for it is on the battlefield by definition. The hand is given the
        // armed half or nothing, which is the whole of what it can draw.
        let offer = crate::cardmat::Offer::on(
            armed.map_or(crate::Proposing::Nothing, crate::Proposing::Armed),
            &[card.id],
            false,
        );
        // No border: the card is rounded like a real one; hover/selection
        // read as a soft accent glow instead of a frame.
        let shadow = if is_selected {
            halo(palette::CANDLE, 1.0)
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
        let left = layout.start(i);
        let entity = commands
            .spawn((
                HandCardVisual { object: card.id },
                ZIndex(if is_hovered {
                    2
                } else {
                    i32::from(offer.armed || is_selected)
                }),
                crate::hud::HandRowCard,
                Node {
                    position_type: PositionType::Absolute,
                    left: px(left),
                    // An armed card stands out of the row, the way the table
                    // lifts an armed permanent. [`HAND_HEADROOM`] is the air
                    // it rises into and the ledge above it is what stops it,
                    // so the raise never reaches the zone's clip at all.
                    //
                    // Spawned where the card *is* rather than where it
                    // belongs, because this tree is rebuilt on every hover
                    // change: a node born at its resting pose would snap a
                    // travelling card into place whenever the pointer crossed
                    // the row, which is the jump `touch` exists to remove.
                    // [`crate::touch::settle`] writes it from here on, and a
                    // card nothing has touched is answered with exactly the
                    // pose this line used to hold on its own.
                    top: px(touch.lift_of(card.id, if offer.armed { -ARMED_RAISE } else { 0.0 })),
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
        // The pane the press is darkened with, over the art and under
        // nothing. `Pickable::IGNORE` because a node in front of the card is
        // a node the pointer would report instead of it — the mistake a
        // button's own label made once, and this one covers the whole face.
        let shade = commands
            .spawn((
                crate::touch::Shade { object: card.id },
                Node {
                    position_type: PositionType::Absolute,
                    left: px(0),
                    top: px(0),
                    width: px(HAND_CARD_W),
                    height: px(HAND_CARD_H),
                    border_radius: card_radius(HAND_CARD_W),
                    ..default()
                },
                BackgroundColor(Color::NONE),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(entity).add_child(shade);
        if is_selected {
            let mark = commands
                .spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        top: px(4),
                        right: px(4),
                        padding: UiRect::axes(px(6), px(2)),
                        border_radius: BorderRadius::all(px(4)),
                        ..default()
                    },
                    BackgroundColor(palette::CANDLE),
                    Pickable::IGNORE,
                ))
                .with_child((
                    Text::new(glyph::CHECK.to_string()),
                    icon_tf(fonts, 12.0),
                    TextColor(palette::DOCK_GROUND),
                    Pickable::IGNORE,
                ))
                .id();
            commands.entity(entity).add_child(mark);
        }
        commands.entity(strip).add_child(entity);
    }
    commands.entity(zone).add_child(strip);
    if layout.scrollable {
        for (direction, label) in [(-1, "‹"), (1, "›")] {
            let arrow = super::ledge::answer(
                commands,
                fonts,
                label,
                super::ledge::Weight::Secondary,
                None,
            );
            commands.entity(arrow).insert((
                MenuButton {
                    action: MenuAction::ScrollHand(direction),
                },
                HandPage(direction),
                ZIndex(10),
                Node {
                    position_type: PositionType::Absolute,
                    left: if direction < 0 { px(2) } else { Val::Auto },
                    right: if direction > 0 { px(2) } else { Val::Auto },
                    top: px(LEDGE_H + HAND_HEADROOM + HAND_CARD_H * 0.4),
                    width: px(26),
                    height: px(38),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    border: UiRect::all(px(1)),
                    border_radius: BorderRadius::all(px(4)),
                    ..default()
                },
            ));
            commands.entity(zone).add_child(arrow);
        }
        let track = commands
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: px(HAND_STRIP_INSET),
                    right: px(HAND_STRIP_INSET),
                    bottom: px(2),
                    height: px(3),
                    ..default()
                },
                BackgroundColor(palette::DOCK_EDGE.with_alpha(0.35)),
                Pickable::IGNORE,
            ))
            .id();
        let thumb = commands
            .spawn((
                HandScrollThumb,
                Node {
                    position_type: PositionType::Absolute,
                    height: percent(100),
                    ..default()
                },
                BackgroundColor(palette::CANDLE.with_alpha(0.8)),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(track).add_child(thumb);
        commands.entity(zone).add_child(track);
    }

    // The command zone is *not* drawn here any more. It is a zone on the
    // table like the graveyard and the exile pile, and it was the only one
    // that had been copied into the hand zone as a flat 2D card — so a seat's
    // commander existed twice, in two sizes, in two renderers, and the 3D
    // slot beside the mat sat empty beneath the copy. `table::spawn_piles`
    // and the pile placements draw it now, where a public zone belongs
    // (CR 903.6), and the cast tax rides on the card itself.
    zone
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
    let start = layout.start(index);
    let end = start + HAND_CARD_W;
    if start < scroll {
        start
    } else if end > scroll + available {
        end - available
    } else {
        scroll
    }
}

/// Position indicator for an overflowing hand.
#[derive(Component)]
pub struct HandScrollThumb;

/// A page arrow whose visibility follows the current scroll bounds.
#[derive(Component)]
pub struct HandPage(i8);

/// Apply scrolling without rebuilding cards; keyboard hover follows group gaps.
pub fn apply_hand_scroll(
    mut duel: ResMut<Duel>,
    windows: Query<&Window>,
    mut strips: Query<&mut Node, (With<HandStrip>, Without<HandScrollThumb>)>,
    mut thumbs: Query<&mut Node, (With<HandScrollThumb>, Without<HandStrip>)>,
    mut pages: Query<(&HandPage, &mut Visibility)>,
) {
    let (Some(board), Ok(window)) = (duel.board.as_ref(), windows.single()) else {
        return;
    };
    // The same width the rebuild lays the row out in. The two used to
    // differ, and that is what moved the row sideways on every rebuild.
    let available = hand_available(window.width());
    let layout = grouped_hand_layout(board.hand.len(), available, &duel.hand_groups);
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

    for (page, mut visibility) in &mut pages {
        *visibility = if (page.0 < 0 && duel.hand_scroll <= 0.0)
            || (page.0 > 0 && duel.hand_scroll >= max_scroll)
        {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
    }
    for mut node in &mut thumbs {
        let width = (available * available / layout.content_width.max(1.0))
            .clamp(20.0, available.max(20.0));
        node.width = px(width);
        node.left = px((available - width).max(0.0) * duel.hand_scroll / max_scroll.max(1.0));
    }
    for mut node in &mut strips {
        let wanted = UiRect::left(px(HAND_STRIP_INSET + layout.lead - duel.hand_scroll));
        if node.margin != wanted {
            node.margin = wanted;
        }
    }
}

/// The middle of hand card `index`, in window pixels.
///
/// Every term the zone itself applies, in the order it applies them: the
/// strip's inset, the layout's centring `lead`, the scroll offset, and the
/// card's place in the row. It exists because the preview used to compute a
/// shorter version of this sum — inset and step, no `lead` — so the bubble
/// opened `lead` to the left of the card it belonged to. `lead` is half the
/// zone's spare room, so the emptier the hand, the further away the preview
/// stood: five hundred pixels on a wide window, which reads as a panel with
/// no connection to anything.
///
/// **[`HAND_BAR_PAD`] is not a term here**, and it was, for as long as this
/// function has existed. The strip is an absolutely-positioned child and
/// taffy lays one of those out against the padding *box* — the border box
/// less the border, of which there is none — so the zone's padding moves the
/// strip not at all. The ten pixels were real in the sum and not on the
/// screen, which is why they read as a bubble slightly to the right of its
/// card rather than as anything obviously broken.
///
/// Measured, because the test that was here restated this same sum and so
/// agreed with it however wrong it was: at 1728 logical pixels with four
/// cards in hand, `/state` reports card centres at 714, 814, 914 and 1014,
/// and this function answered 724.
#[must_use]
pub(super) fn hand_card_x(layout: HandLayout, scroll: f32, index: usize) -> f32 {
    HAND_STRIP_INSET + layout.lead - scroll + layout.start(index) + HAND_CARD_W / 2.0
}

/// Where the preview panel stands.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(super) enum PreviewAt {
    /// A card in the hand zone. The bubble sits above the bar with its tail on
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
    // Where the hover happened, for everything that is not in the hand zone.
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
    if let Some(group) = board.group(h) {
        // A token has no art; it still gets a preview, built from its
        // projected characteristics alone.
        return Some((group.art, at));
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
/// edge and out from under the hand zone.
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
    let bottom = window.y - HAND_ZONE_H - PREVIEW_INSET - panel.y;
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
/// Not against the hand zone as well: standing clear of the hand is a
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
pub(super) fn preview_place(
    at: PreviewAt,
    panel: Vec2,
    window: Vec2,
    keep_out: Option<Rect>,
) -> Vec2 {
    let (low, high) = viewport(panel, window);
    let banded = |v: Vec2| v.clamp(low, high);
    let place = match at {
        PreviewAt::Hand(x) => banded(Vec2::new(
            x - panel.x / 2.0,
            window.y - HAND_ZONE_H - 10.0 - panel.y,
        )),
        PreviewAt::Loose => banded(Vec2::new(
            (window.x - panel.x) / 2.0,
            window.y - HAND_ZONE_H - 10.0 - panel.y,
        )),
        PreviewAt::Card(rect) => beside(rect, panel, low, high, window),
        // The pointer is a rectangle of no size: the same arithmetic, with
        // the gap measured from the one point there is.
        PreviewAt::Pointer(p) => beside(Rect::from_corners(p, p), panel, low, high, window),
    };
    keep_out.map_or(place, |out| clear_of(place, panel, out, low, high))
}

/// Do two rectangles share any area at all?
///
/// Written out rather than taken from `Rect::intersect`, which answers with a
/// rectangle and leaves the caller to decide what an empty one looks like —
/// and two panels touching along an edge are not overlapping, which a test on
/// a width of zero gets right only by accident.
fn overlaps(a: Rect, b: Rect) -> bool {
    a.min.x < b.max.x && b.min.x < a.max.x && a.min.y < b.max.y && b.min.y < a.max.y
}

/// The same placement, moved **sideways** until it is clear of `keep_out`.
///
/// The drawer is the one panel a preview can be asked to share a height with:
/// it grows upward out of the ledge, centred, and a hand card's preview is
/// placed centred on that card and just above the same ledge. So a colour
/// chooser and the preview of the card asking for the colour arrive at the
/// same y by construction, and the preview covers the thing it is there to
/// help the player answer.
///
/// **Sideways and never up**, which is the part worth stating because "move
/// it above the drawer" is the obvious fix and is wrong twice: the drawer's
/// height is whatever its content asked for, so there is no bound on how far
/// up the preview would go, and above the ledge is where the table is — the
/// preview would clear the chooser by covering the board it was opened to
/// explain. Sideways it stays at the height the hand put it at, which is the
/// height a player is already looking.
///
/// The near side wins when both clear, so the panel moves as little as it
/// can; when neither does, the flank with more room, and the panel keeps
/// whatever the clamp leaves it. A preview wider than the room beside an open
/// drawer has nowhere to be, and covering some of the drawer from the side
/// with more air is the least bad of the placements that remain — it is not a
/// case a window this client supports can reach, and it is a decision rather
/// than whichever bound a clamp applied last.
fn clear_of(place: Vec2, panel: Vec2, keep_out: Rect, low: Vec2, high: Vec2) -> Vec2 {
    if !overlaps(Rect::from_corners(place, place + panel), keep_out) {
        return place;
    }
    let right = keep_out.max.x + PREVIEW_GAP;
    let left = keep_out.min.x - PREVIEW_GAP - panel.x;
    // `high.x` is already the largest *top-left* x a panel of this width may
    // take, so a side fits when its own corner lands inside the band — the
    // panel's width is spent once, in `viewport`, and must not be again here.
    let x = match (left >= low.x, right <= high.x) {
        (true, true) => {
            if (left - place.x).abs() <= (right - place.x).abs() {
                left
            } else {
                right
            }
        }
        (true, false) => left,
        (false, true) => right,
        (false, false) => {
            if keep_out.min.x - low.x >= high.x - keep_out.max.x {
                low.x
            } else {
                high.x
            }
        }
    };
    Vec2::new(x.clamp(low.x, high.x), place.y)
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
    // Kept clear of the window's own edge and of the hand zone while it fits
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

/// The ground drawn where there is no render world to draw the cloth on.
///
/// A headless app has no `Assets<FrontalMaterial>` and every overlay test
/// builds this tree, so the zone needs a ground that is arithmetic rather
/// than a shader. It is the cloth's own dye at the cloth's own density where
/// it leaves the shelf — **read** from [`crate::frontal::SKIRT`] and not
/// restated, because the one thing a fallback must not do is put the ground a
/// test measures somewhere else.
///
/// Not a gradient any more. The gradient this replaces ran from nothing at
/// the top to 0.58 at the bottom, which is what left the lower two thirds of
/// the zone reading as sky; the owner's instruction of 14.09.2026 is a
/// container, and a container has one ground.
const GROUND: f32 = crate::frontal::SKIRT;

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
/// The ledge: the zone's top edge, and the one place on this screen a player
/// works at.
///
/// Six pixels of air, a 28-pixel button, six more — the button being a
/// keycap's twenty plus its own padding. Not the lobby's 44: the lobby is the
/// one responsive screen and the table is a desktop picture with the keyboard
/// first, and a 44-px button is a 56-px ledge. Every pixel of ledge is a
/// pixel of table, or 0.72 pixels of hand card; that is the whole of the
/// exchange, and it is why this number is small.
pub const LEDGE_H: f32 = 40.0;

/// The zone along the bottom: the ledge, the room a raised card needs, the
/// cards, and the room under them.
///
/// It replaces `HAND_BAR_H`, which named this strip while it was only a
/// backdrop for a row of cards. The hand and the question the engine is
/// asking are one zone now, and its top edge is the ledge.
pub const HAND_ZONE_H: f32 = LEDGE_H + HAND_HEADROOM + HAND_CARD_H + HAND_FOOTROOM;

/// The zone is what the camera is framed against, so its height is a budget
/// and not a result.
///
/// `Canvas::hud.bottom` is this number, and `CameraRig::home` frames the table
/// in what is left — so a zone that grows takes the table with it, silently
/// and on every screen. Group headings add twelve pixels to the former
/// 191-pixel budget; the wider duel's deeper lanes compensate on the table.
const _: () = assert!(HAND_ZONE_H <= 203.0);

/// What stands out of a raised card has to fit under the ledge.
///
/// This is [`HAND_HEADROOM`]'s old promise, moved to where the room actually
/// is now. The halo is no longer cut by the clip — it runs on under the ledge
/// — but "under the ledge" is only true while the ledge is deep enough to
/// cover it, and both numbers can move on their own.
const _: () = assert!(ARMED_RAISE + HALO_REACH <= LEDGE_H + HAND_HEADROOM);

/// How much room the zone keeps between a card and the ledge, and why it is
/// no longer twenty-five.
///
/// Twenty-five was `ARMED_RAISE + HALO_REACH`, and it was measured: the zone
/// clips its children, a `BoxShadow` is a child's paint like any other, and a
/// glow cut off flat along a horizontal line stops reading as a light and
/// starts reading as a box drawn round the card. The photograph behind it is
/// a red-channel step of 8 across the clip line against 72 at the card's own
/// edge.
///
/// The arrangement is what changed, not the arithmetic. The zone now reaches
/// `LEDGE_H` further up than the cards do, and the ledge is an **opaque**
/// sibling standing on that strip: a halo that reaches past the card's top
/// has forty pixels to spend before the clip is anywhere near it, and what
/// ends it is the ledge *covering* it. A card runs under a shelf instead of
/// being cut by a line — and the shelf casts its shadow down onto it.
///
/// So this is only the air between the card's raised top edge and the shelf
/// it must not touch: [`ARMED_RAISE`] plus four.
pub const HAND_HEADROOM: f32 = ARMED_RAISE + 16.0;

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
pub const ARMED_RAISE: f32 = 8.0;

/// The press is spent out of room that already exists, in both directions.
///
/// A pressed card sinks by [`Touch::SINK`] and an armed one rises by
/// [`ARMED_RAISE`]; a card that is both does the two at once and reaches
/// neither edge. Stated here rather than in `client-core`, because the two
/// halves of it live in two crates and this is the one that knows the clip —
/// and stated at compile time, because the failure it guards is a card's edge
/// sliced off by an invisible line, which no test would think to look at.
const _: () = assert!(baylee_client_core::touch::Touch::SINK <= HAND_FOOTROOM);
const _: () = assert!(baylee_client_core::touch::Touch::SINK < ARMED_RAISE);
