//! Chimney Imp — `{4}{B}` 1/2 Creature — Imp (MRD).
//! Oracle: "Flying. When this creature dies, target opponent puts a card from their hand on top of their library."

use super::*;

fn chimney_imp() -> CardIndex {
    card_index("3901bf30-b7c1-4977-a7b1-fcdafcc266cd")
}

#[test]
fn chimney_imp_makes_target_opponent_put_a_card_from_hand_on_top_of_library_when_it_dies() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let imp = chimney_imp();
    let elf = llanowar_elves();
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[imp, mountain()])
        .hand(0, &[lightning_bolt()])
        .hand(1, &[elf])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let imp_id = on_battlefield(&engine, p0, imp).unwrap();
    assert!(keywords(&engine, imp_id).contains(KeywordSet::FLYING));
    let p1_card = in_hand(&engine, p1, elf).unwrap();
    let p1_lib_before = library_size(&engine, p1);

    tap_all_mana_but(&mut engine, p0, None);
    cast_with_floating(&mut engine, p0, lightning_bolt());
    aim_at(&mut engine, p0, imp_id);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![p1_card],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_hand(&engine, p1, elf).is_none());
    assert_eq!(library_size(&engine, p1), p1_lib_before + 1);
}
