//! Stormwatch Eagle — {3}{U} — Creature — Bird
//! Oracle: Flying
//! Oracle: Sacrifice a land: Return this creature to its owner's hand.

use super::*;

#[test]
fn stormwatch_eagle_sacrifices_a_land_to_return_itself_to_hand() {
    let stormwatch_eagle = card_index("640b67d8-75a4-44d4-a2f5-7323bad4e1a8");
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(1, island())
        .battlefield(0, &[island(), island(), island(), island()])
        .hand(0, &[stormwatch_eagle])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    cast_from_hand(&mut engine, p0, stormwatch_eagle);
    pass_until(&mut engine, stack_is_empty);
    let eagle_id =
        on_battlefield(&engine, p0, stormwatch_eagle).expect("Stormwatch Eagle on board");
    assert!(keywords(&engine, eagle_id).contains(KeywordSet::FLYING));

    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: eagle_id,
                ability_index: 0,
            },
        )
        .expect("activate Stormwatch Eagle");
    if let Pending::ChooseCards { options, .. } = engine.pending().clone() {
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![options[0]],
                },
            )
            .expect("choose Island to sacrifice");
    }
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, stormwatch_eagle).is_none());
    assert!(in_hand(&engine, p0, stormwatch_eagle).is_some());
    assert!(in_graveyard(&engine, p0, island()).is_some());
    assert_eq!(all_on_battlefield(&engine, p0, island()).len(), 3);
}
