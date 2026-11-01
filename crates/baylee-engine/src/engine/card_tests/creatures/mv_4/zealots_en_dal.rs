#[allow(clippy::wildcard_imports)]
use super::*;

fn zealots_en_dal() -> CardIndex {
    card_index("3fa67c92-6328-4d0a-95e3-656d447ab20e")
}

#[test]
fn zealots_en_dal_gains_one_life_on_upkeep_when_all_nonlands_are_white() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, plains())
        .hand(0, &[zealots_en_dal()])
        .battlefield(0, &[plains(), plains(), plains(), plains()])
        .battlefield(1, &[gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, zealots_en_dal());
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, zealots_en_dal()).is_some());

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert_eq!(engine.state().players[0].life, 21);
}

#[test]
fn zealots_en_dal_does_not_gain_life_with_a_colorless_nonland_you_control() {
    let mut e = Duel::new(SEED, plains())
        .battlefield(0, &[zealots_en_dal(), sol_ring()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, PlayerId::new(0));
    assert_eq!(e.state().players[0].life, 20);
}
