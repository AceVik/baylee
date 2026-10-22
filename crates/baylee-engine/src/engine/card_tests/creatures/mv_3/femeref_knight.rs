//! `cards/creatures/mv_3/femeref_knight.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::FEMEREF_KNIGHT;

const P0: PlayerId = PlayerId::new(0);

/// Femeref Knight (4365521e-996d-4eca-bbc9-b0988fa973ae) — {2}{W} 2/2:
/// "Flanking. {W}: This creature gains vigilance until end of turn."
#[test]
fn femeref_knight_has_flanking_and_gains_vigilance_until_end_of_turn() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[FEMEREF_KNIGHT, plains()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let knight = on_battlefield(&engine, P0, FEMEREF_KNIGHT).unwrap();
    let kw = engine
        .state()
        .object(knight)
        .unwrap()
        .characteristics()
        .keywords;
    assert!(kw.contains(KeywordSet::FLANKING));
    assert!(!kw.contains(KeywordSet::VIGILANCE));

    tap_all_mana(&mut engine, P0);
    activate(&mut engine, P0, FEMEREF_KNIGHT, 1);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        engine
            .state()
            .object(knight)
            .unwrap()
            .characteristics()
            .keywords
            .contains(KeywordSet::VIGILANCE)
    );
}
