//! `cards/creatures/mv_4/defender_en_vec.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_cards_dsl::counters;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

/// Defender en-Vec ({3}{W} 2/4 Creature — Human Cleric):
/// "Fading 4
/// Remove a fade counter from this creature: Prevent the next 2 damage that would be dealt to any target this turn."
#[test]
fn defender_en_vec_enters_with_four_fade_counters_and_removes_one_to_prevent_two_damage() {
    let subject = card_index("bb8a55a5-596e-427c-b745-4de8b0dfb095");
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                plains(),
                mountain(),
                grizzly_bears(),
            ],
        )
        .hand(0, &[subject, lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let plains_ids: Vec<ObjectId> = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == plains()))
        })
        .collect();
    tap_mana_where(&mut engine, P0, |id| plains_ids.contains(&id));
    cast_with_floating(&mut engine, P0, subject);
    pass_until(&mut engine, stack_is_empty);

    let defender = on_battlefield(&engine, P0, subject).unwrap();
    let bears = on_battlefield(&engine, P0, grizzly_bears()).unwrap();
    assert_eq!(
        engine
            .state()
            .object(defender)
            .unwrap()
            .counters
            .get(counters::FADE),
        4
    );

    // Spend 1 fade counter to shield Grizzly Bears from 2 of Lightning Bolt's 3 damage.
    activate(&mut engine, P0, subject, 1);
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![bears],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine
            .state()
            .object(defender)
            .unwrap()
            .counters
            .get(counters::FADE),
        3
    );

    cast_from_hand(&mut engine, P0, lightning_bolt());
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![bears],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().object(bears).unwrap().damage, 1);

    // Upkeep removes another fade counter (3 -> 2).
    reach_their_main_phase(&mut engine, P1);
    reach_their_main_phase(&mut engine, P0);
    assert_eq!(
        engine
            .state()
            .object(defender)
            .unwrap()
            .counters
            .get(counters::FADE),
        2
    );
}
