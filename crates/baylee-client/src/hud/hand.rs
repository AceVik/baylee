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
    layout: HandLayout,
    scroll: f32,
    textures: &mut CardTextures,
    assets: &AssetServer,
    fonts: &UiFonts,
    faces: &FaceCtx<'_>,
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
            BackgroundColor(palette::PANEL),
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
                margin: UiRect::left(px(10.0 - scroll)),
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
            CardLook::art(card.art, finish_of(statics, Some(card.art)), 0),
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
                    top: px(0),
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

    // ---- commander zone (right end): the command zone with the cast
    // counter above the card. Commander format only — hidden otherwise.
    let commanders = view
        .command
        .get(view.seat.get() as usize)
        .map_or(&[][..], Vec::as_slice);
    if !commanders.is_empty() {
        let casts = view
            .seat(view.seat)
            .map_or(&[][..], |s| s.commander_casts.as_slice());
        let zone = commands
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    right: px(10),
                    bottom: px(10),
                    flex_direction: FlexDirection::Row,
                    column_gap: px(6),
                    padding: UiRect::all(px(6)),
                    border_radius: btn_radius(),
                    ..default()
                },
                BackgroundColor(palette::PANEL_LIT),
                soft_shadow(),
                Pickable::IGNORE,
            ))
            .id();
        for (i, cmd) in commanders.iter().enumerate() {
            let times_cast = casts.get(i).copied().unwrap_or(0);
            let key = cmd
                .card
                .map(|c| ImageKey::new(c.print, c.face, ArtSize::Small));
            let built = faces.object(cmd, textures, key);
            let image = match key {
                Some(key) => textures.get(key, statics, assets),
                None => textures.card_back(),
            };
            let visual = spawn_card_art(
                commands,
                lang,
                image,
                built.as_ref(),
                OVERLAY_CARD_W * 0.75,
                OVERLAY_CARD_H * 0.75,
                crate::face::Detail::Compact,
                fonts,
                match key {
                    Some(key) => {
                        CardLook::art(key, finish_of(statics, Some(key)), glow_bits(cmd.keywords))
                    }
                    None => CardLook::back(FinishTreatment::Plain, 0),
                },
                cards.as_deref_mut(),
            );
            let card = commands
                .spawn((
                    HandCardVisual { object: cmd.id },
                    Node {
                        width: px(OVERLAY_CARD_W * 0.75),
                        height: px(OVERLAY_CARD_H * 0.75),
                        border_radius: card_radius(OVERLAY_CARD_W * 0.75),
                        overflow: Overflow::clip(),
                        ..default()
                    },
                    soft_shadow(),
                    children![(
                        // Cast counter, floating above the card's top.
                        Text::new(format!("×{times_cast}")),
                        tf(fonts, 12.0),
                        TextColor(palette::ACCENT),
                        Node {
                            position_type: PositionType::Absolute,
                            top: px(-6),
                            left: percent(50),
                            margin: UiRect::left(px(-10)),
                            padding: UiRect::axes(px(4), px(1)),
                            border_radius: btn_radius(),
                            ..default()
                        },
                        BackgroundColor(palette::PANEL),
                    ),],
                ))
                .id();
            commands.entity(card).add_child(visual);
            commands.entity(zone).add_child(card);
        }
        commands.entity(bar).add_child(zone);
    }
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
        let wanted = UiRect::left(px(10.0 - duel.hand_scroll));
        if node.margin != wanted {
            node.margin = wanted;
        }
    }
}

/// Which card the preview shows and where its anchor (the bubble's tail
/// target) sits horizontally: hand cards anchor at their strip position;
/// everything else anchors at the screen's centre (`None`). Art comes
/// from the hand, the battlefield lanes, or the command zone.
pub(super) fn preview_anchor(
    board: &baylee_client_core::BoardModel,
    view: &PlayerView,
    hovered: Option<ObjectId>,
    layout: HandLayout,
    scroll: f32,
) -> Option<(Option<ImageKey>, Option<f32>)> {
    let h = hovered?;
    if let Some(i) = board.hand.iter().position(|c| c.id == h) {
        let x = 10.0 + i as f32 * layout.step - scroll + HAND_CARD_W / 2.0;
        return Some((Some(board.hand[i].art), Some(x)));
    }
    for pod in &board.pods {
        for lane in &pod.lanes {
            for group in &lane.groups {
                if group.representative == h {
                    // A token has no art; it still gets a preview, built from
                    // its projected characteristics alone.
                    return Some((group.art, None));
                }
            }
        }
    }
    if let Some(cmd) = view
        .command
        .get(view.seat.get() as usize)
        .and_then(|cmds| cmds.iter().find(|c| c.id == h))
    {
        return Some((
            cmd.card
                .map(|c| ImageKey::new(c.print, c.face, ArtSize::Normal)),
            None,
        ));
    }
    None
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
/// Height of the tab bar at the top (the overlay starts below it).
pub const TAB_H: f32 = 48.0;
/// The hand bar's height, including its padding.
pub const HAND_BAR_H: f32 = HAND_CARD_H + 20.0;
