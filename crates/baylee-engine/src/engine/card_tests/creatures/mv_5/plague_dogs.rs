//! `cards/creatures/mv_5/plague_dogs.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn plague_dogs() -> CardIndex {
    card_index("a7cc69fd-0c67-44fc-be9d-d8f5b1bd54f5")
}

/// Plague Dogs (a7cc69fd-0c67-44fc-be9d-d8f5b1bd54f5) — {4}{B} 3/3 Creature —
/// Phyrexian Zombie Dog:
/// "When this creature dies, all creatures get -1/-1 until end of turn."
/// "{2}, Sacrifice this creature: Draw a card."
///
/// Activating the sacrifice ability requires {2} (cannot be activated with
/// only 1 mana), sacrifices Plague Dogs, draws a card, and triggers its dies
/// ability which shrinks all creatures (both yours and your opponent's) by
/// -1/-1 until end of turn, killing 1/1 creatures on both sides and shrinking
/// a 2/2 to 1/1 until end of turn.
#[test]
fn plague_dogs_sacrifice_draws_a_card_and_its_death_shrinks_all_creatures() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(
            0,
            &[plague_dogs(), savannah_lions(), grizzly_bears(), swamp()],
        )
        .battlefield(1, &[savannah_lions(), grizzly_bears()])
        .hand(0, &[swamp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let dogs = on_battlefield(&engine, p0, plague_dogs()).unwrap();
    let my_bears = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let their_bears = on_battlefield(&engine, p1, grizzly_bears()).unwrap();

    // With only 1 Swamp tapped, {2}, Sacrifice this creature cannot be activated.
    tap_all_mana(&mut engine, p0);
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: dogs,
                    ability_index: 1,
                },
            )
            .is_err(),
        "only 1 mana available, so {{2}}, Sacrifice this creature is rejected"
    );

    // Play a second Swamp and tap it for the second mana.
    play_land(&mut engine, p0, swamp());
    tap_all_mana(&mut engine, p0);
    let before_hand = hand_size(&engine, p0);

    activate(&mut engine, p0, plague_dogs(), 1);
    assert_eq!(
        engine.state().object(dogs).unwrap().zone,
        Zone::Graveyard,
        "Plague Dogs is sacrificed as part of the activation cost"
    );
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        hand_size(&engine, p0),
        before_hand + 1,
        "activating the sacrifice ability draws 1 card"
    );
    assert!(
        on_battlefield(&engine, p0, savannah_lions()).is_none(),
        "own 2/1 dies to the symmetric -1/-1 until end of turn"
    );
    assert!(
        on_battlefield(&engine, p1, savannah_lions()).is_none(),
        "opponent's 2/1 dies to the symmetric -1/-1 until end of turn"
    );
    assert_eq!(
        pt(&engine, my_bears),
        (1, 1),
        "own 2/2 is 1/1 until end of turn"
    );
    assert_eq!(
        pt(&engine, their_bears),
        (1, 1),
        "opponent's 2/2 is 1/1 until end of turn"
    );

    reach_their_main_phase(&mut engine, p1);
    assert_eq!(
        pt(&engine, my_bears),
        (2, 2),
        "-1/-1 wears off at end of turn"
    );
    assert_eq!(pt(&engine, their_bears), (2, 2));
}
