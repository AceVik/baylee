//! Deck rows and their quantity actions.

#[allow(clippy::wildcard_imports)] // the builder module's shared vocabulary
use super::*;

/// One deck row: print, name (and its printing), pips, `− n +`, `⋯`.
#[allow(clippy::too_many_arguments, clippy::too_many_lines)] // one row: where, which, and what it reads
pub(crate) fn deck_row(
    commands: &mut Commands,
    kit: Kit,
    state: &LobbyState,
    prefs: Option<&baylee_client_core::prefs::Preferences>,
    zone: Zone,
    at: usize,
    order: usize,
    top: f32,
    pitch: f32,
) -> Option<Entity> {
    let deck = state.lobby.builder();
    let entry = deck.entries(zone).get(at)?;
    let card = deck.card(entry.slot)?;
    let (is_deck_fav, is_global_fav) = super::pool::fav_flags(state, prefs, &card.english_name);
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
    let r_dot =
        super::pool::rarity_dot(commands, kit, deck.entry_rarity(entry), state.lobby.lang());
    let name = cell(commands, kit, &card.name, m.text, ink, false);
    commands.entity(name).insert(Node {
        min_width: px_fixed(0.0),
        flex_shrink: 1.0,
        flex_grow: 1.0,
        flex_basis: px_fixed(0.0),
        overflow: Overflow::clip_x(),
        ..default()
    });
    let mut kids = vec![thumb, r_dot, name];
    if is_deck_fav || is_global_fav {
        let star = commands
            .spawn((
                Text::new("\u{f005}"),
                icon_tf(kit.fonts, m.small * 0.88),
                TextColor(if is_deck_fav {
                    tokens::ACCENT
                } else {
                    tokens::GOLD
                }),
                TextLayout::no_wrap(),
                Node {
                    flex_shrink: 0.0,
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        kids.push(star);
    }
    // What the check has against this card, in a word; the sentence is the
    // overview's and the card's.
    if let Some(word) = deck.badge(entry.slot) {
        kids.push(crate::buildui::legality::badge(
            commands,
            kit,
            word.text(state.lobby.lang()),
        ));
    }
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
