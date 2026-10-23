#[allow(clippy::wildcard_imports)]
use super::*;

fn engineered_plague() -> CardIndex {
    card_index("dc8210b4-8e17-4156-994b-e95e215e3e34")
}

#[test]
fn engineered_plague_gives_creatures_of_chosen_type_minus_one_minus_one() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .hand(0, &[engineered_plague()])
        .battlefield(0, &[swamp(), swamp(), swamp(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves(), gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, engineered_plague());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseSubtype { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::ChooseSubtype(baylee_core::generated::subtypes::creature::ELF),
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p1, llanowar_elves()).is_none());
    assert!(in_graveyard(&engine, p1, llanowar_elves()).is_some());
    assert!(
        in_graveyard(&engine, p0, llanowar_elves()).is_some(),
        "both controllers are affected"
    );
    assert_eq!(
        pt(&engine, on_battlefield(&engine, p1, gray_ogre()).unwrap()),
        (2, 2)
    );
}
