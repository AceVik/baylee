//! `cards/instants/mv_9/decree_of_savagery.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

fn decree_of_savagery() -> CardIndex {
    card_index("96a6a8d4-6077-470f-984e-b576a24557a2")
}

/// Casting Decree of Savagery puts four +1/+1 counters on each creature you control
/// (leaving opponent's untouched), and cycling it draws a card and optionally puts
/// four +1/+1 counters on one target creature.
#[test]
fn decree_of_savagery_cast_and_cycle_put_four_counters() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut bf0 = vec![forest(); 15];
    bf0.extend([index::GRIZZLY_BEARS, index::LLANOWAR_ELVES]);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &bf0)
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .hand(0, &[decree_of_savagery(), decree_of_savagery()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let my_bears = on_battlefield(&engine, p0, index::GRIZZLY_BEARS).unwrap();
    let my_elves = on_battlefield(&engine, p0, index::LLANOWAR_ELVES).unwrap();
    let their_bears = on_battlefield(&engine, p1, index::GRIZZLY_BEARS).unwrap();
    let lands = all_on_battlefield(&engine, p0, forest());

    tap_mana_where(&mut engine, p0, |id| lands[..9].contains(&id));
    cast_with_floating(&mut engine, p0, decree_of_savagery());
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert_eq!(pt(&engine, my_bears), (6, 6));
    assert_eq!(pt(&engine, my_elves), (5, 5));
    assert_eq!(pt(&engine, their_bears), (2, 2));

    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    tap_mana_where(&mut engine, p0, |id| lands[9..15].contains(&id));
    activate(&mut engine, p0, decree_of_savagery(), 1);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![my_bears],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert_eq!(pt(&engine, my_bears), (10, 10));
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before
    );
}
