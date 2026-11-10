#[allow(clippy::wildcard_imports)]
use super::*;

fn crystal_chimes() -> CardIndex {
    card_index("fbff7452-1da4-4cd0-8247-a625e4b4da2b")
}

#[test]
fn crystal_chimes_returns_enchantments_from_graveyard_to_hand() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                crystal_chimes(),
                plains(),
                plains(),
                plains(),
                sylvan_library(),
            ],
        )
        .hand(0, &[baylee_core::generated::index::GRIZZLY_BEARS])
        .hand(1, &[sylvan_library()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    hand_to_graveyard(
        &mut engine,
        p0,
        baylee_core::generated::index::GRIZZLY_BEARS,
    );
    hand_to_graveyard(&mut engine, PlayerId::new(1), sylvan_library());
    let lib = on_battlefield(&engine, p0, sylvan_library()).expect("seated");
    bury(&mut engine, &[lib]);
    assert!(in_graveyard(&engine, p0, sylvan_library()).is_some());

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, crystal_chimes(), 0);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, crystal_chimes()).is_none());
    assert!(in_graveyard(&engine, p0, sylvan_library()).is_none());
    assert!(in_hand(&engine, p0, sylvan_library()).is_some());
    assert!(in_graveyard(&engine, p0, baylee_core::generated::index::GRIZZLY_BEARS).is_some());
    assert!(in_graveyard(&engine, PlayerId::new(1), sylvan_library()).is_some());
}
