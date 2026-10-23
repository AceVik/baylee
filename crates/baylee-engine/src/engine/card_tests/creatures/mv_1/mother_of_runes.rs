#[allow(clippy::wildcard_imports)]
use super::*;

fn mother_of_runes() -> CardIndex {
    card_index("60433b48-d27f-413c-905c-43839b1943f1")
}

#[test]
fn mother_of_runes_grants_protection_from_chosen_color() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), mother_of_runes()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mom = on_battlefield(&engine, p0, mother_of_runes()).unwrap();

    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: mom,
                ability_index: 0,
            },
        )
        .unwrap();
    if matches!(engine.pending(), Pending::ChooseTargets { .. }) {
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![mom],
                    players: vec![],
                },
            )
            .unwrap();
    }
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseColor { .. }) || stack_is_empty(e)
    });
    if matches!(engine.pending(), Pending::ChooseColor { .. }) {
        engine
            .apply(p0, PlayerAction::ChooseColor(ManaColor::Red))
            .unwrap();
    }
    pass_until(&mut engine, stack_is_empty);

    assert!(is_tapped(&engine, mom));
}
