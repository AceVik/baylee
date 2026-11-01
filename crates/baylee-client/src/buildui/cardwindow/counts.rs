//! Main-deck and sideboard quantities in the card window.

#[allow(clippy::wildcard_imports)] // the builder module's shared vocabulary
use super::*;

/// IN THIS DECK: the main and sideboard steppers, written at once.
pub(super) fn counts(commands: &mut Commands, env: &Env, slot: usize) -> Entity {
    let kit = env.kit;
    let m = kit.m;
    let lang = env.lang();
    let deck = env.deck();
    let head = cell(
        commands,
        kit,
        Phrase::CardHeadInDeck.text(lang),
        m.small,
        tokens::MUTED,
        true,
    );
    let mut parts = vec![head];
    for (zone, label, id) in [
        (Zone::Main, Phrase::CardMain, "main"),
        (Zone::Side, Phrase::CardSide, "side"),
    ] {
        let label = cell(commands, kit, label.text(lang), m.small, tokens::INK, false);
        let stepper = crate::buildui::rows::quantity::stepper(
            commands,
            kit,
            &deck.count_of(slot, zone).to_string(),
            (
                Press::Build(BuildPress::WindowStep(zone, false)),
                Stop::item(CARD, id, 0),
            ),
            (
                Press::Build(BuildPress::WindowStep(zone, true)),
                Stop::item(CARD, id, 1),
            ),
        );
        // A label and its stepper are one piece: the row wraps between
        // the pairs, never between "Sideboard" and its − n + (4K pass,
        // 09.10.2026).
        let pair = commands
            .spawn((
                Node {
                    flex_wrap: FlexWrap::NoWrap,
                    align_items: AlignItems::Center,
                    column_gap: m.px(6.0),
                    flex_shrink: 0.0,
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(pair).add_children(&[label, stepper]);
        parts.push(pair);
    }
    let row = wrap(commands, kit, &parts);
    commands.entity(row).insert((
        Node {
            width: Val::Percent(100.0),
            flex_wrap: FlexWrap::Wrap,
            align_items: AlignItems::Center,
            column_gap: m.px(12.0),
            row_gap: m.px(6.0),
            border: UiRect::top(px_fixed(1.0)),
            padding: UiRect::top(m.px(8.0)),
            ..default()
        },
        BorderColor::all(tokens::BORDER),
    ));
    row
}
