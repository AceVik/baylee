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

/// A spell that must target and has no legal target cannot be cast
/// (CR 601.2c). The client's own proof (`targeting`) reads a filter and not
/// what stops a target beyond it, and the AI check it fell back on was
/// skipped once a cast mode was payable: Terror lit up beside a White Knight
/// (protection from black, CR 702.16b) and Righteousness ("target blocking
/// creature") in a main phase, and the click tapped the lands for a cast the
/// engine refused (NEW-04, 3.11.2026). The engine's word,
/// `LegalActions::castable_once_paid`, keeps them dark; with a creature Terror
/// may destroy, Terror lights again.
#[test]
fn a_spell_with_no_legal_target_is_not_in_reach() {
    for (theirs, terror) in [
        (&["White Knight"][..], false),
        (&["Grizzly Bears"][..], true),
    ] {
        let mut preset = red_preset(&["Terror", "Righteousness"], &["Swamp", "Swamp", "Plains"]);
        preset.seats[1].starting_battlefield = theirs.iter().copied().map(named).collect();
        let mut table = Table::open_with(&preset);
        table.walk_to_main();
        let mut duel = baylee_client::Duel::default();
        refresh(&mut duel, &table);
        for (spell, reached) in [("Terror", terror), ("Righteousness", false)] {
            assert_eq!(
                duel.reachable.contains(&id_in_hand(&table, spell)),
                reached,
                "{spell} with {theirs:?} across the table"
            );
        }
    }
}

/// Mishra's Workshop: "Spend this mana only to cast artifact spells." The
/// engine never lets it pay an ability (`mana_provenance::admits`, CR 106.6),
/// and the client planned it for Rod of Ruin's `{3}` because the filter named
/// an artifact: the run tapped the Workshop and the payment refused its mana
/// (NEW-05, 3.11.2026). With Mountains beside it, the Mountains pay.
#[test]
fn a_workshop_never_pays_for_an_artifacts_ability() {
    for mountains in [0, 3] {
        let mut battlefield = vec!["Rod of Ruin", "Mishra's Workshop"];
        battlefield.extend(std::iter::repeat_n("Mountain", mountains));
        let mut table = Table::open_with(&red_preset(&[], &battlefield));
        table.walk_to_main();
        let view = table.view();
        let named_id = |name: &str| {
            view.battlefield
                .iter()
                .find(|o| o.name == name)
                .unwrap_or_else(|| panic!("{name} is on the table"))
                .id
        };
        let (rod, workshop) = (named_id("Rod of Ruin"), named_id("Mishra's Workshop"));
        let &(_, index, _) = table
            .legal()
            .unpaid_abilities
            .iter()
            .find(|(source, _, _)| *source == rod)
            .expect("the premise: the engine offers the Rod's ability, short only of mana");
        let plan = baylee_client::abilities::mana_for(view, table.legal(), rod, index);
        if mountains == 0 {
            assert_eq!(plan, None, "the Workshop's mana pays no ability");
        } else {
            let plan = plan.expect("three Mountains pay {3}");
            assert!(
                plan.steps.iter().all(|step| step.source != workshop),
                "the Workshop is never tapped for the Rod: {plan:?}"
            );
        }
    }
}

/// Under Urborg, Tomb of Yawgmoth and Yavimaya, Cradle of Growth each of the
/// two is a Swamp Forest with no printed mana ability, and taps for `{B}` or
/// `{G}` through CR 305.6. The client read the shortcut only for a land of one
/// basic type, so neither was a source and the Bears beside them answered no
/// click (NEW-09, 3.11.2026).
#[test]
fn a_land_with_two_basic_types_pays_either_colour() {
    let mut table = Table::open_with(&red_preset(
        &["Grizzly Bears"],
        &["Urborg, Tomb of Yawgmoth", "Yavimaya, Cradle of Growth"],
    ));
    table.walk_to_main();
    let bears = id_in_hand(&table, "Grizzly Bears");
    let mut duel = baylee_client::Duel::default();
    refresh(&mut duel, &table);
    assert!(
        duel.reachable.contains(&bears),
        "two Swamp Forests pay {{1}}{{G}}"
    );
    assert_eq!(
        baylee_client::mana_for(&duel, bears).map(|plan| plan.taps()),
        Some(2)
    );
}
