//! `cards/enchantments/mv_4/infernal_darkness.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn infernal_darkness() -> CardIndex {
    card_index("dd821fca-79a0-48a9-b1cf-5a496bc04f65")
}

fn first_upkeep_paid(mine: &[CardIndex], theirs: &[CardIndex]) -> Engine<RegistryLookup> {
    lk::game_with(SEED, mine, theirs, &[], &[], |e| {
        // This fixture accepts both the black mana and life portions of the first upkeep.
        lk::float(e, P0, &[(ManaColor::Black, 1)]);
        lk::yes_no(e, true);
    })
}

fn wait_for_mana_upkeep(engine: &mut Engine<RegistryLookup>, turn: u32) {
    pass_until(engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayMana { .. },
                ..
            }
        )
    });
    assert_eq!(
        (engine.state().turn.number, engine.state().turn.active),
        (turn, P0),
        "cumulative upkeep asks in its controller's own upkeep, not in the opponent's"
    );
}

fn tap_restored_land_colors(engine: &mut Engine<RegistryLookup>) {
    tap_all_mana_but(engine, P0, None);
    for (color, expected) in [
        (ManaColor::Green, 1),
        (ManaColor::White, 1),
        (ManaColor::Blue, 1),
        (ManaColor::Black, 0),
    ] {
        assert_eq!(lk::pool_of(engine, P0, color), expected);
    }
}

/// Infernal Darkness (dd821fca-79a0-48a9-b1cf-5a496bc04f65) — {2}{B}{B} Enchantment:
/// - "Cumulative upkeep—Pay {B} and 1 life." (Partial: asked as {B} per age counter, then 1 life
///   per age counter, as two payments in a row.)
/// - "If a land is tapped for mana, it produces {B} instead of any other type."
#[test]
fn infernal_darkness_turns_all_land_mana_black_until_cumulative_upkeep_is_declined() {
    let mut engine = first_upkeep_paid(
        &[infernal_darkness(), forest(), plains(), island()],
        &[index::MOUNTAIN],
    );
    let darkness = lk::obj(&engine, P0, infernal_darkness());
    assert_eq!(
        engine
            .state()
            .object(darkness)
            .unwrap()
            .counters
            .get(baylee_cards_dsl::counters::AGE),
        1
    );
    assert_eq!(
        lk::life(&engine, P0),
        19,
        "the first upkeep cost {{B}} and 1 life (one age counter)"
    );

    // Tapping Forest, Plains, and Island while Infernal Darkness is on the battlefield produces {B}{B}{B}.
    tap_all_mana_but(&mut engine, P0, None);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        3,
        "Forest, Plains, and Island all produce {{B}} instead of their normal colors"
    );
    assert_eq!(engine.state().players[0].mana_pool.total(), 3);

    // Opponent's Mountain also produces {B} on their turn.
    reach_their_main_phase(&mut engine, P1);
    tap_all_mana_but(&mut engine, P1, None);
    assert_eq!(
        engine.state().players[1]
            .mana_pool
            .available(ManaColor::Black),
        1,
        "opponent's Mountain also produces {{B}}"
    );

    // Second upkeep: 2 age counters, pay {B}{B} (produced by tapping two non-Swamp lands!).
    wait_for_mana_upkeep(&mut engine, 3);
    assert_eq!(
        engine
            .state()
            .object(darkness)
            .unwrap()
            .counters
            .get(baylee_cards_dsl::counters::AGE),
        2
    );
    lk::yes_no(&mut engine, true);
    tap_all_mana_but(&mut engine, P0, None);
    engine.apply(P0, PlayerAction::PassPriority).unwrap();
    // After the mana, the life: 1 life for each of the 2 age counters.
    assert!(
        matches!(
            engine.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayLife { amount: 2 },
                ..
            }
        ),
        "after the {{B}}{{B}} the upkeep asks for 2 life, got {:?}",
        engine.pending()
    );
    lk::yes_no(&mut engine, true);
    lk::settle(&mut engine);
    assert!(on_battlefield(&engine, P0, infernal_darkness()).is_some());
    assert_eq!(
        lk::life(&engine, P0),
        17,
        "the second upkeep cost 2 life on top of the first one's 1"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        1,
        "two of the three {{B}} paid the second upkeep's {{B}}{{B}}"
    );

    // Third upkeep: decline -> Infernal Darkness is sacrificed, and lands produce their normal colors again.
    wait_for_mana_upkeep(&mut engine, 5);
    lk::yes_no(&mut engine, false);
    lk::settle(&mut engine);
    assert!(in_graveyard(&engine, P0, infernal_darkness()).is_some());
    assert_eq!(
        lk::life(&engine, P0),
        17,
        "declining the payment costs no life"
    );

    tap_restored_land_colors(&mut engine);
}

#[test]
fn infernal_darkness_is_sacrificed_when_the_life_is_refused_after_the_mana() {
    let engine = lk::game_with(SEED, &[infernal_darkness(), forest()], &[], &[], &[], |e| {
        if matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayMana { .. },
                ..
            }
        ) {
            lk::float(e, P0, &[(ManaColor::Black, 1)]);
            lk::yes_no(e, true);
        } else {
            // The life: refused.
            lk::yes_no(e, false);
        }
    });
    assert!(
        in_graveyard(&engine, P0, infernal_darkness()).is_some(),
        "the mana alone does not keep Infernal Darkness: the life is part of the cost"
    );
    assert_eq!(lk::life(&engine, P0), 20, "the refused life was not paid");
}

#[test]
fn infernal_darkness_changes_land_mana_only_not_an_artifacts() {
    let mut engine = first_upkeep_paid(&[infernal_darkness(), forest(), sol_ring()], &[]);
    tap_all_mana_but(&mut engine, P0, None);
    assert_eq!(
        lk::pool_of(&engine, P0, ManaColor::Black),
        1,
        "the Forest is a land: it produces {{B}}"
    );
    assert_eq!(
        lk::pool_of(&engine, P0, ManaColor::Colorless),
        2,
        "Sol Ring is not a land: its {{C}}{{C}} is not turned black"
    );
}
