//! `cards/creatures/mv_1/sacred_guide.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::SAVANNAH_LIONS;

fn sacred_guide() -> CardIndex {
    card_index("de9e8b71-897d-4a9d-942b-af10f31e0425")
}

/// `Sacred Guide` ({W}, 1/1 Creature — Human Cleric, `Coverage::Partial`):
/// `{1}{W}, Sacrifice this creature: Reveal cards from the top of your library
/// until you reveal a white card and put that card into your hand.`
#[test]
fn sacred_guide_sacrifices_to_search_library_for_a_white_card_into_hand() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, SAVANNAH_LIONS)
        .battlefield(0, &[plains(), plains(), sacred_guide()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, sacred_guide(), 0);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, sacred_guide()).is_none());
    assert!(in_graveyard(&engine, p0, sacred_guide()).is_some());
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1
    );
}
