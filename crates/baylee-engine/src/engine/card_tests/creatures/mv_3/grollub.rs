#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::partials_w1b_kit as kit;
fn grollub() -> CardIndex {
    card_index("22c8c952-2713-418e-8259-aea09b4e740b")
}
#[test]
fn grollub_gives_every_opponent_the_actual_damage_amount_even_when_lethal() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::table(SEED, forest(), 3)
        .battlefield(0, &[grollub(), mountain()])
        .hand(0, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let beast = on_battlefield(&e, p0, grollub()).unwrap();
    cast_from_hand(&mut e, p0, lightning_bolt());
    kit::settle(&mut e, p0, |_, p| kit::choose_targets(p, &[beast], &[]));
    assert!(in_graveyard(&e, p0, grollub()).is_some());
    assert_eq!(e.state().players[0].life, 20);
    assert_eq!(e.state().players[1].life, 23);
    assert_eq!(e.state().players[2].life, 23);
}
