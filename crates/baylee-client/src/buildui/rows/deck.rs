//! Deck rows and their quantity actions.

#[allow(clippy::wildcard_imports)] // the builder module's shared vocabulary
use super::*;

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
            Press::Build(BuildPress::InspectRow(at)),
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
    let blocked = deck.blocked(card.index).is_some();
    let ink = if blocked {
        tokens::DANGER
    } else if card.coverage.trustworthy() {
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
        // Keep the mana renderer's scaled gap between complete symbols.
        kids.push(cost);
    }
    kids.push(quantity::stepper(
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
