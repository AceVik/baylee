//! `cards/sorceries/mv_2/song_of_blood.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::MOUNTAIN;

fn song_of_blood() -> CardIndex {
    card_index("ab4b6524-3667-4db3-b5cb-1b02ee944906")
}

/// `Song of Blood` ({1}{R}, Sorcery, `Coverage::Partial`) mills four cards on
/// resolution.
#[test]
fn song_of_blood_mills_four_cards() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, MOUNTAIN)
        .battlefield(0, &[MOUNTAIN, MOUNTAIN])
        .hand(0, &[song_of_blood()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let gy_before = engine.state().zones.list(ZoneLocation::Graveyard(p0)).len();
    let library_before = engine.state().zones.list(ZoneLocation::Library(p0)).len();
    let opponent_library = engine.state().zones.list(ZoneLocation::Library(p1)).len();
    cast_from_hand(&mut engine, p0, song_of_blood());
    pass_until(&mut engine, stack_is_empty);

    // 4 milled + Song of Blood itself
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p0)).len(),
        gy_before + 5
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Library(p0)).len(),
        library_before - 4,
        "four cards left the caster's library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Library(p1)).len(),
        opponent_library,
        "\"Mill four cards\" is the caster's own library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p1)).len(),
        0
    );
}
