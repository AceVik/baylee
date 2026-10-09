//! One line of a list (§7): a pool row, a deck row, a deck section's
//! heading. Every cell is one line, cut and never wrapped, inside a row of a
//! fixed height that clips — so the row's height is the pitch whatever its
//! words are, and two rows can never overlap (§10 #6).

#[allow(clippy::wildcard_imports)] // the builder's own vocabulary
use super::*;
use crate::lobby::{hover_of_card, hover_of_entry, print_mark};
use baylee_client_core::deckbuilder::PoolCard;

/// A pool row's place in the shown results, for the cursor's highlight.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct PoolRowAt(pub(crate) usize);

/// A deck row's place in the drawn list, for the cursor's highlight.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct DeckRowAt(pub(crate) usize);

/// The count badge on a pool row: how many of this card the deck holds,
/// kept by `virtual_rows::in_deck` without redrawing the row.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct InDeck(pub(crate) usize);

/// The ground a row rests at.
pub(crate) const ROW_GROUND: Color = Color::srgba(1.0, 1.0, 1.0, 0.03);
/// The ground a row is lit to under the pointer.
pub(crate) const ROW_HOT: Color = Color::srgba(1.0, 1.0, 1.0, 0.08);

/// A pool row's second line: type line · P/T · first keyword — never rules
/// text (§7: the Huntmaster reads `Creature — Human Werewolf // Creature —
/// Werewolf · 2/2 // 4/4 · Transform`).
#[must_use]
pub(crate) fn subline(card: &PoolCard) -> String {
    let mut parts = vec![card.type_line.clone()];
    if let Some(stats) = card.stats.as_ref().filter(|s| !s.is_empty()) {
        parts.push(stats.clone());
    }
    if let Some(keyword) = first_keyword(card) {
        parts.push(keyword);
    }
    parts.join(" \u{b7} ")
}

/// The card's first keyword, read off its front face's keyword lines in the
/// card's own words (the gateway's, else the compiled English Oracle): the
/// row stands for the face it is filed under, so a Huntmaster does not read
/// as its back face's Trample.
fn first_keyword(card: &PoolCard) -> Option<String> {
    let oracle: String = if card.oracle_text.is_empty() {
        baylee_cards::generated_oracle::ORACLE
            .get(card.index as usize)
            .and_then(|faces| faces.first())
            .map_or_else(String::new, |face| (*face).to_owned())
    } else {
        card.oracle_text
            .split("\n//")
            .next()
            .unwrap_or_default()
            .to_owned()
    };
    baylee_client_core::textface::keyword_chips(oracle.lines())
        .into_iter()
        .next()
}

/// The row frame every list line stands in: absolute at its top, its own
/// height, clipping.
fn frame(kit: Kit, top: f32, height: f32) -> Node {
    Node {
        position_type: PositionType::Absolute,
        top: px_fixed(top),
        left: px_fixed(0.0),
        width: Val::Percent(100.0),
        height: px_fixed(height),
        align_items: AlignItems::Center,
        column_gap: kit.m.px(10.0),
        padding: UiRect::axes(kit.m.px(6.0), px_fixed(0.0)),
        border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_CONTROL)),
        overflow: Overflow::clip(),
        ..default()
    }
}

/// A card's print at a height, at its own aspect.
fn print(commands: &mut Commands, hover: &crate::lobby::HoverCard, height: f32) -> Entity {
    let thumb = crate::lobby::thumbnails::spawn(commands, hover);
    commands.entity(thumb).insert(Node {
        width: px_fixed((height * 5.0 / 7.0).round()),
        height: px_fixed(height.round()),
        flex_shrink: 0.0,
        border_radius: BorderRadius::all(px_fixed(4.0)),
        ..default()
    });
    thumb
}

/// One pool row: print, name, the type line, pips, the count badge, `+`
/// and `⋯`.
#[allow(clippy::too_many_lines)] // one row, read left to right
pub(crate) fn pool_row(
    commands: &mut Commands,
    kit: Kit,
    state: &LobbyState,
    slot: usize,
    at: usize,
    pitch: f32,
) -> Option<Entity> {
    let deck = state.lobby.builder();
    let lang = state.lobby.lang();
    let card = deck.card(slot)?;
    let m = kit.m;
    let gap = m.scaled(4.0);
    let height = pitch - gap;
    let hover = hover_of_card(card);
    let row = commands
        .spawn((
            Role::Row,
            frame(kit, at as f32 * pitch, height),
            BackgroundColor(ROW_GROUND),
            crate::ambience::Feel::tinting_to(ROW_GROUND, ROW_HOT),
            Press::Build(BuildPress::Inspect(slot)),
            hover.clone(),
            PoolRowAt(at),
        ))
        .id();
    if at == 0 {
        commands.entity(row).insert(crate::tour::TourAnchor(
            baylee_client_core::tour::Anchor::BuildFirstRow,
        ));
    }
    let thumb = print(
        commands,
        &hover,
        (height - m.scaled(4.0)).min(m.scaled(67.0)),
    );
    commands.entity(thumb).insert((
        Press::Build(BuildPress::PickPrint(slot)),
        Pickable::default(),
    ));

    let info = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                flex_grow: 1.0,
                flex_basis: px_fixed(0.0),
                min_width: px_fixed(0.0),
                row_gap: m.px(2.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let name = cell(commands, kit, &card.name, m.text, tokens::INK, true);
    let mut top = vec![name];
    if let Some((said, ink)) = coverage_mark(card.coverage) {
        let flag = commands
            .spawn((
                Text::new(said.text(lang)),
                tf_bold(kit.fonts, m.small * 0.9),
                TextColor(ink),
                TextLayout::no_wrap(),
                Node {
                    flex_shrink: 0.0,
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        top.push(flag);
    }
    let first = line(commands, kit, &top);
    let second = cell(commands, kit, &subline(card), m.small, tokens::MUTED, false);
    commands.entity(info).add_children(&[first, second]);

    let mut kids = vec![thumb, info];
    if let Some(cost) =
        crate::manaui::spawn_cost_or_text(commands, kit.fonts, &card.mana_cost, m.small * 1.25)
    {
        commands.entity(cost).insert(Node {
            flex_shrink: 0.0,
            align_items: AlignItems::Center,
            ..default()
        });
        kids.push(cost);
    }
    kids.push(badge(commands, kit, slot, deck));
    kids.push(controls::button(
        commands,
        kit,
        "+",
        Weight::Secondary,
        Live::Yes,
        None,
        Press::Build(BuildPress::AddFromPool(slot, false)),
    ));
    if card.commander && state.commander_pick.is_some() {
        let partner = state.commander_pick == Some(true);
        kids.push(controls::button(
            commands,
            kit,
            if partner {
                Phrase::BuildPartner.text(lang)
            } else {
                Phrase::BuildChoose.text(lang)
            },
            Weight::Primary,
            Live::Yes,
            None,
            if partner {
                Press::Build(BuildPress::AddPartner(slot))
            } else {
                Press::Build(BuildPress::SetCommander(slot))
            },
        ));
    }
    kids.push(icon_button(
        commands,
        kit,
        mark::MORE,
        state.build.menu == Some(BuildMenu::Pool(slot)),
        (
            Press::Build(BuildPress::PoolMenu(slot)),
            focus::MenuOpener(BuildMenu::Pool(slot)),
        ),
    ));
    commands.entity(row).add_children(&kids);
    Some(row)
}

/// The count badge: how many the deck holds, both lists together; hidden
/// at none.
fn badge(commands: &mut Commands, kit: Kit, slot: usize, deck: &DeckBuilder) -> Entity {
    let held = deck.count_of(slot, Zone::Main) + deck.count_of(slot, Zone::Side);
    let m = kit.m;
    let disc = commands
        .spawn((
            Node {
                min_width: px_fixed(m.scaled(22.0).max(m.small * 1.6)),
                min_height: px_fixed(m.scaled(22.0).max(m.small * 1.6)),
                padding: UiRect::axes(m.px(6.0), px_fixed(0.0)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                flex_shrink: 0.0,
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PILL)),
                display: if held == 0 {
                    Display::None
                } else {
                    Display::Flex
                },
                ..default()
            },
            BackgroundColor(tokens::GOLD.with_alpha(0.22)),
            Pickable::IGNORE,
            InDeck(slot),
        ))
        .id();
    let n = commands
        .spawn((
            Text::new(held.to_string()),
            tf_bold(kit.fonts, m.small),
            TextColor(tokens::GOLD),
            TextLayout::no_wrap(),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(disc).add_child(n);
    disc
}

/// One deck row: print, name (and its printing), pips, `− n +`, `⋯`.
#[allow(clippy::too_many_arguments)] // one row: where, which, and what it reads
pub(crate) fn deck_row(
    commands: &mut Commands,
    kit: Kit,
    state: &LobbyState,
    zone: Zone,
    at: usize,
    order: usize,
    top: f32,
    pitch: f32,
) -> Option<Entity> {
    let deck = state.lobby.builder();
    let entry = deck.entries(zone).get(at)?;
    let card = deck.card(entry.slot)?;
    let m = kit.m;
    let height = pitch - m.scaled(4.0);
    let hover = hover_of_entry(card, &entry.print);
    let row = commands
        .spawn((
            Role::Row,
            frame(kit, top, height),
            BackgroundColor(ROW_GROUND),
            crate::ambience::Feel::tinting_to(ROW_GROUND, ROW_HOT),
            Press::Build(BuildPress::Inspect(entry.slot)),
            hover.clone(),
            DeckRowAt(order),
        ))
        .id();
    let thumb = print(
        commands,
        &hover,
        (height - m.scaled(6.0)).max(m.scaled(20.0)),
    );
    commands.entity(thumb).insert((
        Press::Build(BuildPress::PickRowPrint(at)),
        Pickable::default(),
    ));
    let ink = if card.coverage.trustworthy() {
        tokens::INK
    } else {
        tokens::MUTED
    };
    let name = cell(commands, kit, &card.name, m.text, ink, false);
    commands.entity(name).insert(Node {
        min_width: px_fixed(0.0),
        flex_shrink: 1.0,
        flex_grow: 1.0,
        flex_basis: px_fixed(0.0),
        overflow: Overflow::clip_x(),
        ..default()
    });
    let mut kids = vec![thumb, name];
    // A row that names a printing shows it, or two lines of one card would
    // read as a fault in the list.
    let chosen = print_mark(&entry.print);
    if !chosen.is_empty() {
        let mark = cell(commands, kit, &chosen, m.small * 0.9, tokens::ACCENT, false);
        commands.entity(mark).insert(Node {
            max_width: m.px(110.0),
            min_width: px_fixed(0.0),
            flex_shrink: 1.0,
            overflow: Overflow::clip_x(),
            ..default()
        });
        kids.push(mark);
    }
    if let Some(cost) =
        crate::manaui::spawn_cost_or_text(commands, kit.fonts, &card.mana_cost, m.small * 1.15)
    {
        commands.entity(cost).insert(Node {
            flex_shrink: 0.0,
            align_items: AlignItems::Center,
            ..default()
        });
        kids.push(cost);
    }
    kids.push(controls::stepper(
        commands,
        kit,
        &entry.count.to_string(),
        Press::Build(BuildPress::RemoveRow(at)),
        Press::Build(BuildPress::AddRow(at)),
    ));
    kids.push(icon_button(
        commands,
        kit,
        mark::MORE,
        state.build.menu == Some(BuildMenu::Deck(at)),
        (
            Press::Build(BuildPress::DeckMenu(at)),
            focus::MenuOpener(BuildMenu::Deck(at)),
        ),
    ));
    commands.entity(row).add_children(&kids);
    Some(row)
}

/// A deck section's heading: its caret, its name and count; a press folds
/// it or opens it again.
#[allow(clippy::too_many_arguments)] // one heading: where, which, and what it says
pub(crate) fn section_head(
    commands: &mut Commands,
    kit: Kit,
    lang: Lang,
    key: SectionKey,
    cards: u32,
    folded: bool,
    top: f32,
    height: f32,
) -> Entity {
    let m = kit.m;
    let row = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: px_fixed(top),
                left: px_fixed(0.0),
                width: Val::Percent(100.0),
                height: px_fixed(height),
                align_items: AlignItems::Center,
                column_gap: m.px(6.0),
                padding: UiRect::axes(m.px(4.0), px_fixed(0.0)),
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(Color::NONE),
            Press::Build(BuildPress::ToggleSection(key)),
        ))
        .id();
    let caret = glyph(
        commands,
        kit,
        if folded { mark::FOLDED } else { mark::OPEN },
        m.small * 0.9,
        tokens::MUTED,
    );
    let name = caption(
        commands,
        kit,
        &format!("{} \u{b7} {cards}", key.heading(lang)),
    );
    commands.entity(row).add_children(&[caret, name]);
    row
}
