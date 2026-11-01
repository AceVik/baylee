//! Duplicant — `{6}` 2/4 Artifact Creature — Shapeshifter (MRD).
//! Oracle: "Imprint — When this creature enters, you may exile target nontoken creature. As long as a card exiled with this creature is a creature card, this creature has the power, toughness, and creature types of the last creature card exiled with it. It's still a Shapeshifter."

use super::*;

fn duplicant() -> CardIndex {
    card_index("ea86abfa-6cab-4ef0-8463-34136fc25b59")
}

#[test]
fn duplicant_exiles_target_nontoken_creature_and_copies_its_pt_and_subtypes_while_staying_shapeshifter()
 {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let dup = duplicant();
    let hunter = fangren_hunter();
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[forest(), forest(), forest(), forest(), forest(), forest()],
        )
        .hand(0, &[dup])
        .battlefield(1, &[hunter])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let hunter_id = on_battlefield(&engine, p1, hunter).unwrap();
    cast_from_hand(&mut engine, p0, dup);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    aim_at(&mut engine, p0, hunter_id);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p1, hunter).is_none());
    let dup_id = on_battlefield(&engine, p0, dup).unwrap();
    assert_eq!(pt(&engine, dup_id), (4, 4));
    let subs = engine
        .state()
        .object(dup_id)
        .unwrap()
        .characteristics()
        .subtypes;
    assert!(subs.contains(baylee_core::generated::subtypes::creature::BEAST));
    assert!(subs.contains(baylee_core::generated::subtypes::creature::SHAPESHIFTER));
}
