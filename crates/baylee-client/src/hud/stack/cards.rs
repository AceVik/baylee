//! Stack card, target and sentence drawing.

#[allow(clippy::wildcard_imports)] // the stack panel shares one drawing vocabulary
use super::*;

/// The printed sentence on a full row, in its box, with a planeswalker's
/// badge before it. Answers the block to put in the row's body and the box,
/// which the row names for the wheel ([`StackTextRow`]).
///
/// A walker prints its cost at the head of the line — `+2: Look at the top
/// card…` — and that badge is the mark a player recognises a walker's ability
/// by. Set as letters it is three characters of prose at the front of a
/// sentence; set as the shape the card draws, it is an **initial**, and what
/// is left beside it is what the ability does. Both halves come from one cut
/// ([`baylee_client_core::abilitysheet::loyalty_cut`]), so the cost cannot be
/// both drawn and printed, nor dropped without being drawn. The badge stands
/// outside the box, so it does not scroll away with the first line.
///
/// Everything else on the stack keeps its whole line. A cost paid in mana is
/// already a row of marks inside the sentence and has no second shape to
/// stand as, and a trigger has no cost at all.
///
/// The sentence is never cut. It was, at [`STACK_SENTENCE_LINES`] lines by a
/// character budget, and a cut sentence is a sentence whose last clause — so
/// often the one naming what it targets — a player cannot read anywhere on
/// the panel. It stands whole in a box `lines` tall now, and scrolls.
pub(in crate::hud) fn spawn_stack_sentence(
    commands: &mut Commands,
    fonts: &UiFonts,
    key: StackKey,
    object: ObjectId,
    blocks: Vec<TextBlock>,
    lines: f32,
) -> (Entity, Entity) {
    use baylee_client_core::abilitysheet;

    let (initial, blocks) = abilitysheet::loyalty_cut(blocks);
    let sentence = commands
        .spawn((
            Text::default(),
            super::tf_serif(fonts, STACK_SENTENCE_PT, 400),
            // Stated, so the box is a whole number of these tall.
            bevy::text::LineHeight::Px(STACK_SENTENCE_LINE),
            TextColor(palette::INK),
            Arriving::ink(key, palette::INK.alpha()),
            Pickable::IGNORE,
        ))
        .id();
    for piece in spans_of(&blocks, None) {
        let ink = if piece.reminder {
            palette::MUTED
        } else {
            palette::INK
        };
        let mark = piece.mark.then(|| piece.text.clone());
        let span = commands
            .spawn((
                TextSpan::new(piece.text),
                if piece.mark {
                    crate::manaui::mana_tf(fonts, STACK_SENTENCE_PT * STACK_MARK)
                } else if piece.reminder {
                    super::tf_serif_italic(fonts, STACK_SENTENCE_PT, 400)
                } else {
                    super::tf_serif(fonts, STACK_SENTENCE_PT, 400)
                },
                TextColor(ink),
                Arriving::ink(key, ink.alpha()),
                Pickable::IGNORE,
            ))
            .id();
        if let Some(mark) = mark {
            crate::manaui::ink_span(commands, span, &mark, STACK_SENTENCE_PT * STACK_MARK);
        }
        commands.entity(sentence).add_child(span);
    }

    // The box: as tall as the sentence up to `lines` of it, and scrolled
    // past that. Its width is what the bar beside it leaves, so the prose
    // wraps inside it whether the bar shows or not.
    let text_box = commands
        .spawn((
            StackTextBox { object },
            // What the shell's checks read as a scroll container.
            crate::shellkit::role::Role::Scroll,
            Node {
                flex_direction: FlexDirection::Column,
                flex_grow: 1.0,
                flex_basis: px(0),
                min_width: px(0),
                max_height: px(lines * STACK_SENTENCE_LINE),
                overflow: Overflow::scroll_y(),
                ..default()
            },
            ScrollPosition::default(),
            Pickable::IGNORE,
        ))
        .add_child(sentence)
        .id();
    // The panel's own scrollbar, in the panel's own colours: one look for
    // "there is more of this" in the panel, whichever list it is.
    let (track, grip) = bar_colours();
    let (bar, thumb) =
        super::ledge::log::scrollbar_parts(commands, text_box, (track, grip), fill_bar);
    commands.entity(bar).insert((
        StackTextBar { text_box },
        // Until a layout has measured the box; `stack_text` stands it then,
        // or at once from what it knew of this entry before a rebuild.
        Visibility::Hidden,
        Arriving::fill(key, track.alpha()),
    ));
    commands
        .entity(thumb)
        .insert(Arriving::fill(key, grip.alpha()));
    let scroller = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                column_gap: px(STACK_TEXT_GAP),
                min_width: px(0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .add_children(&[text_box, bar])
        .id();

    let Some(loy) = initial else {
        return (scroller, text_box);
    };
    // Sharing the line with the badge, and able to shrink back off its own
    // natural width — prose that cannot shrink pushes the row wide instead of
    // wrapping inside it.
    commands.entity(scroller).insert(Node {
        flex_direction: FlexDirection::Row,
        column_gap: px(STACK_TEXT_GAP),
        flex_grow: 1.0,
        flex_basis: Val::Auto,
        min_width: px(0),
        ..default()
    });
    (
        spawn_walker_line(commands, fonts, key, loy, scroller),
        text_box,
    )
}

/// The badge and the sentence, as one line.
///
/// The badge is laid on the **panel**, not on parchment, so it is drawn the
/// other way up from the sheet's: a light body carrying a dark number. That
/// is the same claim the sheet's dark-on-parchment badge makes, made on the
/// ground this panel actually has — a `PARCHMENT_INK` lozenge here would be a
/// hole in the panel with nothing legible in it.
pub(in crate::hud) fn spawn_walker_line(
    commands: &mut Commands,
    fonts: &UiFonts,
    key: StackKey,
    loy: manapip::Loyalty,
    sentence: Entity,
) -> Entity {
    let line = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::FlexStart,
                column_gap: px(STACK_INITIAL_GAP),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    // Exactly one line tall, with the badge centred in it: an initial belongs
    // on the sentence's first line, and a badge aligned to the block would
    // either ride above that line's own middle or sink into the paragraph.
    let seat = commands
        .spawn((
            Node {
                height: px(STACK_SENTENCE_PT * super::SERIF_SCALE * STACK_LINE),
                align_items: AlignItems::Center,
                flex_shrink: 0.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let badge = crate::manaui::spawn_loyalty_badge(
        commands,
        fonts,
        loy,
        STACK_SENTENCE_PT * STACK_INITIAL,
        palette::INK,
        palette::DIALOG_LIT,
    );
    // There is no subtree opacity in `bevy_ui`, so each piece of the badge
    // arrives on the row's own progress or none of it does — and a solid mark
    // standing at full strength over a row that has not faded in yet is the
    // one-frame flash [`ease_the_stack_in`] exists to prevent. Both pieces are
    // **ink** now: the body is the badge's own glyph rather than two painted
    // boxes, so a `fill` here would animate a `BackgroundColor` nothing has.
    commands
        .entity(badge.body)
        .insert(Arriving::ink(key, palette::INK.alpha()));
    commands
        .entity(badge.numeral)
        .insert(Arriving::ink(key, palette::DIALOG_LIT.alpha()));
    commands.entity(seat).add_child(badge.root);
    commands.entity(line).add_children(&[seat, sentence]);
    line
}

/// The arrow and everything a stack entry points at, as one wrapping row.
#[allow(clippy::too_many_arguments)] // the same slot arguments again
pub(in crate::hud) fn spawn_stack_targets(
    commands: &mut Commands,
    lang: Lang,
    item: &baylee_client_core::board::StackItem,
    key: StackKey,
    full: bool,
    view: &PlayerView,
    statics: &GameStatic,
    textures: &mut CardTextures,
    assets: &AssetServer,
    fonts: &UiFonts,
    faces: &FaceCtx<'_>,
    mut cards: Option<&mut UiCards<'_>>,
) -> Entity {
    let row = commands
        .spawn((
            StackTargets,
            Node {
                flex_direction: FlexDirection::Row,
                column_gap: px(4),
                align_items: AlignItems::Center,
                flex_wrap: FlexWrap::Wrap,
                row_gap: px(4),
                margin: UiRect::top(px(2)),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();

    // The arrow belongs to the sentence, not to the queue: a queued row is
    // already the narrow one, and a glyph that is a third of its body width
    // buys nothing a thumbnail sitting under a name does not already say.
    if full {
        let arrow = commands
            .spawn((
                Text::new("→"),
                tf(fonts, 14.0),
                TextColor(palette::CANDLE),
                Arriving::ink(key, palette::CANDLE.alpha()),
                // A label, and the row's hover is the row's — as above.
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(row).add_child(arrow);
    }

    for target in &item.targets {
        let chip = spawn_stack_target(
            commands,
            lang,
            target,
            key,
            full,
            view,
            statics,
            textures,
            assets,
            fonts,
            faces,
            cards.as_deref_mut(),
        );
        commands.entity(row).add_child(chip);
    }
    row
}

/// The face a stack entry should show instead of its art, if any.
///
/// A spell on the stack has its own object and its own projected text; an
/// ability has neither, so it keeps its picture and its name.
pub(in crate::hud) fn stack_face(
    item: &baylee_client_core::board::StackItem,
    view: &PlayerView,
    faces: &FaceCtx<'_>,
    textures: &CardTextures,
) -> Option<CardFace> {
    let object = view.object(item.id)?;
    faces.object(object, textures, item.art)
}

/// A card in the stack panel: its art, its face, or a plain plate with the
/// card's back colour when the seat may not know what it is.
#[allow(clippy::too_many_arguments)] // the slot, the card, and the stores
pub(in crate::hud) fn spawn_stack_card(
    commands: &mut Commands,
    lang: Lang,
    key: StackKey,
    object: Option<ObjectId>,
    art: Option<ImageKey>,
    face: Option<CardFace>,
    width: f32,
    height: f32,
    statics: &GameStatic,
    textures: &mut CardTextures,
    assets: &AssetServer,
    fonts: &UiFonts,
    cards: Option<&mut UiCards<'_>>,
    widths: &crate::face::Widths<'_>,
) -> Entity {
    let slot = commands
        .spawn((
            Node {
                width: px(width),
                height: px(height),
                flex_shrink: 0.0,
                border_radius: card_radius(width),
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(palette::DIALOG_LIT),
            soft_shadow(),
            Arriving::fill(key, palette::DIALOG_LIT.alpha()),
        ))
        .id();
    // Who speaks for this picture. A stack card is drawn an inch across —
    // enough to recognise a spell, nowhere near enough to read one — and the
    // stack is where a player most needs to read. `HandCardVisual` is what
    // `pointer_hover` looks for, and it already speaks for every card the HUD
    // draws rather than the felt; a card whose object this seat may not know
    // (something cast face down) reports nothing and simply does not preview.
    //
    // An **entry** passes `None` and is not the exception it looks like: its
    // whole row carries the component instead, so that the name and the
    // printed sentence answer for the spell as well as the picture does. A
    // picture that stayed pickable inside a pickable row would also swallow
    // the row's own hover — a `Node` under the pointer is the hover, and its
    // parent is then not hovered at all — which is the mistake the hand zone's
    // labels have already made once. A **target** chip is its own object and
    // keeps its own handle.
    match object {
        Some(object) => {
            commands.entity(slot).insert(HandCardVisual { object });
        }
        None => {
            commands.entity(slot).insert(Pickable::IGNORE);
        }
    }
    if let Some(key) = art {
        let image = textures.get(key, statics, assets);
        let visual = spawn_card_art(
            commands,
            lang,
            image,
            face.as_ref(),
            width,
            height,
            crate::face::Detail::Compact,
            fonts,
            // Nothing on the stack is on a battlefield, so no glow: the
            // border says what the rules have made a *permanent*, and a
            // spell wearing one would be claiming something untrue.
            CardLook::art(key, finish_of(statics, Some(key))),
            cards,
            widths,
        );
        commands.entity(slot).add_child(visual);
    }
    // The picture's own fade, laid over it rather than mixed into it. The art
    // is a `MaterialNode` on a material shared with every card that looks the
    // same, so an alpha written there would fade the hand as well; a veil the
    // colour of the empty slot fades exactly this one, and costs one node.
    let veil = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                right: px(0),
                top: px(0),
                bottom: px(0),
                ..default()
            },
            BackgroundColor(palette::DIALOG_LIT.with_alpha(0.0)),
            Pickable::IGNORE,
            Arriving::veil(key),
        ))
        .id();
    commands.entity(slot).add_child(veil);
    slot
}

/// One target of a stack entry: its picture when it has one, a chip when it
/// does not — a player, a token this seat cannot name, a face-down permanent.
#[allow(clippy::too_many_arguments)] // as above
pub(in crate::hud) fn spawn_stack_target(
    commands: &mut Commands,
    lang: Lang,
    target: &baylee_client_core::board::StackTarget,
    key: StackKey,
    full: bool,
    view: &PlayerView,
    statics: &GameStatic,
    textures: &mut CardTextures,
    assets: &AssetServer,
    fonts: &UiFonts,
    faces: &FaceCtx<'_>,
    cards: Option<&mut UiCards<'_>>,
) -> Entity {
    let (width, height) = if full {
        (STACK_TARGET_W, STACK_TARGET_H)
    } else {
        (STACK_QUEUED_TARGET_W, STACK_QUEUED_TARGET_H)
    };
    if target.art.is_some() && target.is_current {
        return spawn_stack_card(
            commands,
            lang,
            key,
            target.object(),
            target.art,
            None,
            width,
            height,
            statics,
            textures,
            assets,
            fonts,
            cards,
            &faces.widths,
        );
    }
    // A player is a name and a heart, not a rectangle pretending to be a
    // card. Anything else with no picture (a token, something face down) is
    // its name in the same chip, so the row never has a hole in it.
    let (glyph, label) = match target.player() {
        Some(player) => (Some(glyph::HEART), statics.seat_name(player).to_string()),
        None => (
            None,
            crate::choices::target_label(
                lang,
                target.what,
                crate::choices::FaceNames {
                    view: Some(view),
                    texts: Some(faces.texts),
                },
                Some(statics),
            ),
        ),
    };
    let size = if full { 12.0 } else { 11.0 };
    let chip = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                column_gap: px(4),
                align_items: AlignItems::Center,
                padding: UiRect::axes(px(6.0), px(3.0)),
                border_radius: BorderRadius::all(px(9)),
                ..default()
            },
            BackgroundColor(palette::DIALOG_LIT),
            Pickable::IGNORE,
            Arriving::fill(key, palette::DIALOG_LIT.alpha()),
        ))
        .id();
    if let Some(glyph) = glyph {
        let icon = commands
            .spawn((
                Text::new(glyph.to_string()),
                icon_tf(fonts, size - 2.0),
                TextColor(palette::DANGER),
                Arriving::ink(key, palette::DANGER.alpha()),
            ))
            .id();
        commands.entity(chip).add_child(icon);
    }
    let text = commands
        .spawn((
            Text::new(label),
            tf(fonts, size),
            TextColor(palette::INK),
            Arriving::ink(key, palette::INK.alpha()),
        ))
        .id();
    commands.entity(chip).add_child(text);
    chip
}
