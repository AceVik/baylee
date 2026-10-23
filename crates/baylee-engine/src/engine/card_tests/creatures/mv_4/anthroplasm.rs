#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

fn anthroplasm() -> CardIndex {
    card_index("f7536d94-4062-4ccc-a086-1c47954b24c7")
}

#[test]
fn anthroplasm_enters_with_two_counters_and_resets_to_x_counters() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .hand(0, &[anthroplasm()])
        .battlefield(0, &[island(), island(), island(), island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, anthroplasm());
    pass_until(&mut engine, stack_is_empty);

    let anth = on_battlefield(&engine, p0, anthroplasm()).unwrap();
    assert_eq!(pt(&engine, anth), (2, 2));

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: anth,
                ability_index: 0,
            },
        )
        .unwrap();
    engine.apply(p0, PlayerAction::ChooseNumber(4)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, anth), (4, 4));
}

#[test]
fn anthroplasm_can_replace_its_counters_with_zero() {
    let p0 = PlayerId::new(0);
    let mut engine = lk::game(SEED, &[anthroplasm()], &[], &[], &[]);
    let anth = lk::obj(&engine, p0, anthroplasm());
    activate(&mut engine, p0, anthroplasm(), 0);
    engine.apply(p0, PlayerAction::ChooseNumber(0)).unwrap();
    assert!(is_tapped(&engine, anth));
    lk::settle(&mut engine);
    assert!(in_graveyard(&engine, p0, anthroplasm()).is_some());
}
