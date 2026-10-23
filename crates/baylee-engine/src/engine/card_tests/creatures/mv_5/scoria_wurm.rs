#[allow(clippy::wildcard_imports)]
use super::*;

fn scoria_wurm() -> CardIndex {
    card_index("ee45c80c-699c-4e37-97e6-1859c6601c99")
}

#[test]
fn scoria_wurm_flips_coin_on_upkeep() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[scoria_wurm()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p0);

    assert!(
        on_battlefield(&engine, p0, scoria_wurm()).is_some()
            || in_hand(&engine, p0, scoria_wurm()).is_some()
    );
}
