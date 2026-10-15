//! `cards/creatures/artifacts/mv_5/tangle_hulk.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn tangle_hulk() -> CardIndex {
    card_index("f6d0360c-ebc6-48b5-80ad-d52dadcdd5f5")
}

/// Tangle Hulk — "{2}{G}: Regenerate this creature."
#[test]
fn tangle_hulk_regenerates_for_two_and_green() {
    let p0 = PlayerId::new(0);
    for shielded in [true, false] {
        let mut engine = Duel::new(SEED, forest())
            .battlefield(0, &[tangle_hulk(), forest(), forest(), forest()])
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);
        let hulk = on_battlefield(&engine, p0, tangle_hulk()).expect("seated");
        assert_eq!(pt(&engine, hulk), (5, 3));
        if shielded {
            tap_all_mana(&mut engine, p0);
            raise_a_shield(&mut engine, p0, hulk, 0);
        }
        kill(&mut engine, hulk);
        let alive = engine.state().object(hulk).map(|o| o.zone) == Some(Zone::Battlefield);
        assert_eq!(alive, shielded);
    }
}
