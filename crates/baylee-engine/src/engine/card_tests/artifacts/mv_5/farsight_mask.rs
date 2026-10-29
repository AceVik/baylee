//! Farsight Mask — `{5}` Artifact (MRD).
//! Oracle: "Whenever a source an opponent controls deals damage to you, if this artifact is untapped, you may draw a card."

use super::*;

fn farsight_mask() -> CardIndex {
    card_index("c82091b2-f2f1-4a85-80d6-91ab218f0a94")
}

#[test]
fn farsight_mask_draws_a_card_when_opponent_source_deals_damage_to_you_while_untapped() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mask = farsight_mask();
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mask])
        .battlefield(1, &[mountain()])
        .hand(1, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    let mtn = on_battlefield(&engine, p1, mountain()).unwrap();
    // Pass priority from p0 to p1 so p1 can tap Mountain and cast Lightning Bolt.
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    tap_mana_where(&mut engine, p1, |id| id == mtn);
    cast_with_floating(&mut engine, p1, lightning_bolt());
    engine
        .apply(
            p1,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p0],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[0].life, 17);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1
    );
}
