//! `cards/enchantments/auras/mv_2/cagemail.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn cagemail_grants_plus_two_plus_two_and_prevents_attacking() {
    let cagemail = card_index("fe1f40fe-011b-4686-b865-fed00c56d30a");
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), grizzly_bears()])
        .hand(0, &[cagemail])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bear = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    cast_from_hand(&mut engine, p0, cagemail);
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

    assert_eq!(pt(&engine, bear), (4, 4));
    assert!(keywords(&engine, bear).contains(KeywordSet::CANT_ATTACK));
}
