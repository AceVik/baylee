//! What the engine's offer says that the planner must honour, read off tables
//! a `LocalHost` dealt: the price it will charge for a cast (CR 601.2f).
//!
//! The planner prices a cast before a land is tapped, which is exactly when
//! the engine has not been asked to cast it, so the only place it can learn
//! the engine's price is `LegalActions`. A hand-built offer would prove the
//! client reads a field; these prove the engine fills it.

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;

fn named(name: &str) -> DeckEntry {
    DeckEntry {
        rule_print: None,
        choices: baylee_core::card_metadata::DeckChoices::default(),
        card: baylee_cards::decks::by_name(name).unwrap_or_else(|| panic!("{name} is in the pool")),
        print: PrintRef::new(0),
    }
}

/// Seat 0 with `battlefield` on the table and `hand` in hand, over a deck of
/// Mountains.
fn red_preset(hand: &[&str], battlefield: &[&str]) -> GamePreset {
    let mut preset = white_preset(&[], 0);
    for seat in &mut preset.seats {
        seat.deck = (0..60).map(|_| named("Mountain")).collect();
    }
    preset.seats[0].starting_hand = Some(hand.iter().copied().map(named).collect());
    preset.seats[0].starting_battlefield = battlefield.iter().copied().map(named).collect();
    preset
}

/// Goblin Warchief: "Goblin spells you cast cost {1} less to cast." The
/// engine charges the Sky Raider `{1}{R}` under it; the client priced the
/// printed `{2}{R}`, so with two Mountains the Goblin answered no click and
/// with three its run tapped all three (NEW-03, 3.11.2026).
#[test]
fn a_goblin_under_the_warchief_is_planned_at_the_reduced_price() {
    for (mountains, taps) in [(2, 2), (3, 2)] {
        let mut battlefield = vec!["Goblin Warchief"];
        battlefield.extend(std::iter::repeat_n("Mountain", mountains));
        let mut table = Table::open_with(&red_preset(&["Goblin Sky Raider"], &battlefield));
        table.walk_to_main();
        let raider = id_in_hand(&table, "Goblin Sky Raider");
        assert_eq!(
            table
                .legal()
                .spell_reduction(raider, baylee_engine::choice::CastModeKind::Normal),
            1,
            "the premise: the engine publishes the Warchief's reduction"
        );

        let mut duel = baylee_client::Duel::default();
        refresh(&mut duel, &table);
        assert!(
            duel.reachable.contains(&raider),
            "{mountains} Mountains pay {{1}}{{R}}, so the Sky Raider is in reach"
        );
        assert_eq!(
            baylee_client::mana_for(&duel, raider).map(|plan| plan.taps()),
            Some(taps),
            "with {mountains} Mountains the run taps for the price the engine charges"
        );
    }
}
