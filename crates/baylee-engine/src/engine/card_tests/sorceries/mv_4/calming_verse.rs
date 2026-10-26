//! `cards/sorceries/mv_4/calming_verse.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

fn calming_verse() -> CardIndex {
    card_index("c70ae479-0bd1-481c-885f-16bfa096cf58")
}

#[test]
fn calming_verse_spares_your_enchantments_when_all_your_lands_are_tapped() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[calming_verse()])
        .battlefield(0, &[forest(), forest(), forest(), forest(), index::CRUSADE])
        .battlefield(1, &[index::BAD_MOON])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Tap all four Forests to cast Calming Verse -> no untapped land controlled on resolution.
    cast_from_hand(&mut engine, p0, calming_verse());
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p1, index::BAD_MOON).is_some());
    assert!(on_battlefield(&engine, p0, index::CRUSADE).is_some());
}

#[test]
fn calming_verse_destroys_your_enchantments_when_you_control_an_untapped_land() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[calming_verse(), forest()])
        .battlefield(0, &[forest(), forest(), forest(), forest(), index::CRUSADE])
        .battlefield(1, &[index::BAD_MOON])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Tap the 4 Forests first, then play an untapped 5th Forest before casting Calming Verse.
    tap_all_mana(&mut engine, p0);
    play_land(&mut engine, p0, forest());
    cast_with_floating(&mut engine, p0, calming_verse());
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p1, index::BAD_MOON).is_some());
    assert!(in_graveyard(&engine, p0, index::CRUSADE).is_some());
}
