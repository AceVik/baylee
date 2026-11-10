//! `cards/creatures/artifacts/mv_5/soliton.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn soliton() -> CardIndex {
    card_index("c04b4323-db6a-4ab1-99b7-eef3acad3c4f")
}

/// Soliton — "{U}: Untap this creature." After attacking it is tapped, and a
/// single blue mana untaps it.
#[test]
fn soliton_untaps_for_one_blue() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[soliton(), island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let soliton_id = on_battlefield(&engine, p0, soliton()).expect("seated");
    assert_eq!(pt(&engine, soliton_id), (3, 4));
    attack_alone(&mut engine, p0, soliton_id, p1);
    through_combat(&mut engine);
    assert!(is_tapped(&engine, soliton_id), "the attack tapped it");

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, soliton(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert!(!is_tapped(&engine, soliton_id), "untapped");
}
