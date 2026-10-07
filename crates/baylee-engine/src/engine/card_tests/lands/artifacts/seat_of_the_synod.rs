//! `cards/lands/artifacts/seat_of_the_synod.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Seat of the Synod prints no mana line in any basic-land-type sense: it is
/// an artifact land whose whole text is `{T}: Add {U}`, so the CR 305.6
/// shortcut (`LegalActions::mana_abilities`) cannot name it and its ability
/// arrives as an ordinary `(source, 0)` entry instead. Both facts are the
/// point — the type line is read back off the permanent, and the mana is
/// taken through `ActivateAbility` rather than through the shortcut, which
/// is the half `tap_all_mana` reads a second list for (#159).
#[test]
fn seat_of_the_synod_enters_as_an_artifact_land_and_taps_for_blue() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(41, forest())
        .hand(0, &[seat_of_the_synod()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, seat_of_the_synod());
    pass_until(&mut engine, stack_is_empty);
    let kinds = types(&engine, land);
    assert!(
        kinds.contains(TypeSet::LAND) && kinds.contains(TypeSet::ARTIFACT),
        "an artifact land is both types at once: {kinds:?}"
    );
    assert!(!is_tapped(&engine, land), "played, and it enters untapped");

    // The two lists are read before anything is tapped: nothing here carries
    // a basic land type, so the shortcut is empty and the printed ability is
    // the one that has to be offered.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.mana_abilities.contains(&land),
        "no basic land type, so CR 305.6 names nothing: {:?}",
        legal.mana_abilities
    );
    assert!(
        legal.abilities.contains(&(land, 0)),
        "but the {{T}}: Add {{U}} it prints has an index: {:?}",
        legal.abilities
    );

    let taken = tap_all_mana(&mut engine, p0);
    assert_eq!(taken, 1, "the Seat is the only mana route on the table");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Blue),
        1,
        "the one colour the card prints, and no other"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "one tap, one mana"
    );
    assert!(is_tapped(&engine, land), "{{T}} was the whole of the price");
}
