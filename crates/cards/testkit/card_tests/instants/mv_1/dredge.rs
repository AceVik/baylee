//! Dredge — {B} — Instant
//! Oracle: Sacrifice a creature or land.
//! Oracle: Draw a card.

use super::*;

#[test]
fn dredge_sacrifices_a_creature_or_land_and_draws_a_card() {
    let dredge = card_index("efc9498a-b740-44de-aef3-01a1fa4ee49a");
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), grizzly_bears()])
        .hand(0, &[dredge])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    let swamp_id = on_battlefield(&engine, p0, swamp()).unwrap();
    let bears_id = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    cast_from_hand(&mut engine, p0, dredge);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("expected sacrifice choice for Dredge");
    };
    assert!(options.contains(&swamp_id));
    assert!(options.contains(&bears_id));

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![bears_id],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, grizzly_bears()).is_some());
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before
    );
}
