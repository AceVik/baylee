#[allow(clippy::wildcard_imports)]
use super::*;

fn devout_harpist() -> CardIndex {
    card_index("f57901c2-80c6-49d6-8de3-c96aaeb72bbf")
}

#[test]
fn devout_harpist_destroys_target_aura_attached_to_a_creature() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let aura = baylee_core::generated::index::GRANITE_GRIP;
    let mut engine = Duel::new(SEED, plains())
        .hand(0, &[devout_harpist(), aura])
        .battlefield(
            0,
            &[
                grizzly_bears(),
                plains(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, devout_harpist());
    pass_until(&mut engine, stack_is_empty);

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    let bear = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    cast_from_hand(&mut engine, p0, aura);
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bear],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let harp = on_battlefield(&engine, p0, devout_harpist()).unwrap();
    let hs = on_battlefield(&engine, p0, aura).unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: harp,
                ability_index: 0,
            },
        )
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![hs],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, aura).is_none());
    assert!(in_graveyard(&engine, p0, aura).is_some());
}
