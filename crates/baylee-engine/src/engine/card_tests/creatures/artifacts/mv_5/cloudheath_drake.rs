//! `cards/creatures/artifacts/mv_5/cloudheath_drake.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn cloudheath_drake() -> CardIndex {
    card_index("95500513-5519-4131-bb89-42383289f2bd")
}

/// Cloudheath Drake — flying, and "{1}{W}: This creature gains vigilance
/// until end of turn."
#[test]
fn cloudheath_drake_flies_and_gains_vigilance_for_one_white() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[cloudheath_drake(), plains(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let drake = on_battlefield(&engine, p0, cloudheath_drake()).expect("seated");
    assert_eq!(pt(&engine, drake), (3, 3));
    assert!(keywords(&engine, drake).contains(KeywordSet::FLYING));
    assert!(!keywords(&engine, drake).contains(KeywordSet::VIGILANCE));

    tap_all_mana(&mut engine, p0);
    assert_eq!(engine.state().players[0].mana_pool.total(), 2);
    activate(&mut engine, p0, cloudheath_drake(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert!(keywords(&engine, drake).contains(KeywordSet::VIGILANCE));

    attack_alone(&mut engine, p0, drake, p1);
    through_combat(&mut engine);
    assert!(!is_tapped(&engine, drake), "vigilance kept it untapped");
    assert_eq!(life_of(&engine, 1), 17, "three flying damage");
}

/// Both halves of the cost: two Forests cannot pay {1}{W}.
#[test]
fn cloudheath_drake_vigilance_needs_white() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[cloudheath_drake(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let drake = on_battlefield(&engine, p0, cloudheath_drake()).expect("seated");
    assert!(!priority_offer(&engine).abilities.contains(&(drake, 0)));
}
