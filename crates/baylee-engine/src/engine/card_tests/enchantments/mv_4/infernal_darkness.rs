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

/// Infernal Darkness (dd821fca-79a0-48a9-b1cf-5a496bc04f65) — {2}{B}{B} Enchantment:
/// - "Cumulative upkeep—Pay {B} and 1 life." (Partial: pays {B} per age counter)
/// - "If a land is tapped for mana, it produces {B} instead of any other type."
#[test]
fn infernal_darkness_turns_all_land_mana_black_until_cumulative_upkeep_is_declined() {
    let mut engine = lk::game_with(
        SEED,
        &[infernal_darkness(), forest(), plains(), island()],
        &[index::MOUNTAIN],
        &[],
        &[],
        |e| {
            lk::float(e, P0, &[(ManaColor::Black, 1)]);
            lk::yes_no(e, true);
        },
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
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayMana { .. },
                ..
            }
        )
    });
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
    lk::settle(&mut engine);
    assert!(on_battlefield(&engine, P0, infernal_darkness()).is_some());
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        1,
        "two of the three {{B}} paid the second upkeep's {{B}}{{B}}"
    );

    // Third upkeep: decline -> Infernal Darkness is sacrificed, and lands produce their normal colors again.
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayMana { .. },
                ..
            }
        )
    });
    lk::yes_no(&mut engine, false);
    lk::settle(&mut engine);
    assert!(in_graveyard(&engine, P0, infernal_darkness()).is_some());

    tap_all_mana_but(&mut engine, P0, None);
    for (color, expected) in [
        (ManaColor::Green, 1),
        (ManaColor::White, 1),
        (ManaColor::Blue, 1),
        (ManaColor::Black, 0),
    ] {
        assert_eq!(lk::pool_of(&engine, P0, color), expected);
    }
}
