//! Pool rows and their count badge.

#[allow(clippy::wildcard_imports)] // the builder module's shared vocabulary
use super::*;

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
    // Blocked on this gateway: the deck that holds it will not sit down
    // here (`docs/protocol.md` §"Blocked cards"); the sheet says why.
    if deck.blocked(card.index).is_some() {
        let flag = commands
            .spawn((
                Text::new(Phrase::BuildBlocked.text(lang)),
                tf_bold(kit.fonts, m.small * 0.9),
                TextColor(tokens::DANGER),
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
        // Keep the mana renderer's scaled gap between complete symbols.
        kids.push(cost);
    }
    kids.push(badge(commands, kit, slot, deck));
    kids.push(icon_button(
        commands,
        kit,
        '\u{f067}',
        false,
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
