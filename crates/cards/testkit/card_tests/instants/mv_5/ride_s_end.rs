//! `cards/instants/mv_5/ride_s_end.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "89e209f3-c474-453f-9659-ccebfc88bc0f"
fn ride_s_end() -> CardIndex {
    card_index("89e209f3-c474-453f-9659-ccebfc88bc0f")
}

/// Ride's End: "{4}{W} Instant — This spell costs {3} less to cast if it
/// targets a tapped permanent. Exile target creature or Vehicle."
///
/// CR 601.2c, CR 601.2f: only creatures and Vehicles are legal targets, so
/// tapped lands on the battlefield cannot satisfy the `{3}` reduction. With an
/// untapped creature, tapped lands, and only two Plains, Ride's End is not
/// castable at `{1}{W}`.
#[test]
fn ride_s_end_is_not_castable_for_1w_when_only_lands_are_tapped() {
    let mut engine = table(
        980,
        &[ride_s_end()],
        &[plains(), plains(), mountain()],
        &[],
        &[gray_ogre(), mountain()],
    );
    let spell = in_hand(&engine, me(), ride_s_end()).expect("Ride's End in hand");
    let opp_land = on_battlefield(&engine, them(), mountain()).expect("their Mountain");
    let my_mountain = on_battlefield(&engine, me(), mountain()).expect("my Mountain");

    // Tap lands on both sides (leaving no floating mana from them) while
    // leaving the opponent's creature untapped and only two Plains available.
    engine.state.set_tapped(my_mountain, true);
    engine.state.set_tapped(opp_land, true);
    tap_only(&mut engine, plains());

    let Pending::Priority { player, legal } = engine.pending() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert_eq!(*player, me());
    assert!(
        !legal.castable.contains(&spell),
        "tapped lands are not legal targets (CR 601.2c, 601.2f), so {{W}}{{W}} cannot cast Ride's End: {legal:?}"
    );
    assert_eq!(
        legal.spell_reduction(spell, CastModeKind::Normal),
        0,
        "no guaranteed reduction when no legal target is tapped"
    );
}

/// Targeting an untapped creature pays the full `{4}{W}` even while lands are
/// tapped, and exiles that creature.
#[test]
fn ride_s_end_targeting_untapped_creature_pays_full_4w_and_exiles_it() {
    let mut engine = table(
        981,
        &[ride_s_end()],
        &[plains(), plains(), plains(), plains(), plains(), mountain()],
        &[],
        &[gray_ogre(), mountain()],
    );
    let ogre = on_battlefield(&engine, them(), gray_ogre()).expect("their Ogre");
    let my_mountain = on_battlefield(&engine, me(), mountain()).expect("my Mountain");
    let opp_land = on_battlefield(&engine, them(), mountain()).expect("their Mountain");
    engine.state.set_tapped(my_mountain, true);
    engine.state.set_tapped(opp_land, true);

    tap_only(&mut engine, plains());
    assert_eq!(
        engine.state().players[usize::from(me().get())]
            .mana_pool
            .total(),
        5,
        "five Plains float five white mana"
    );

    cast_with_floating(&mut engine, me(), ride_s_end());
    let menu = aim_at(&mut engine, me(), ogre);
    assert_eq!(menu, vec![ogre], "only the creature is a legal target");
    assert_eq!(
        engine.state().players[usize::from(me().get())]
            .mana_pool
            .total(),
        0,
        "targeting an untapped creature must consume all five mana ({{4}}{{W}})"
    );
    settle(&mut engine);

    assert!(
        exiled(&engine, them(), gray_ogre()),
        "the targeted creature is exiled"
    );
    assert!(on_battlefield(&engine, them(), gray_ogre()).is_none());
}

/// Targeting an opponent's tapped creature applies the `{3}` reduction, making
/// Ride's End castable for `{1}{W}` with two Plains and exiling the creature.
#[test]
fn ride_s_end_targeting_tapped_creature_costs_1w_and_exiles_it() {
    let mut engine = table(
        982,
        &[ride_s_end()],
        &[plains(), plains()],
        &[],
        &[gray_ogre()],
    );
    let spell = in_hand(&engine, me(), ride_s_end()).expect("Ride's End in hand");
    let ogre = on_battlefield(&engine, them(), gray_ogre()).expect("their Ogre");
    engine.state.set_tapped(ogre, true);

    tap_only(&mut engine, plains());
    let Pending::Priority { player, legal } = engine.pending() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert_eq!(*player, me());
    assert!(
        legal.castable.contains(&spell),
        "with a tapped creature to target, {{W}}{{W}} pays the reduced {{1}}{{W}}"
    );

    cast_with_floating(&mut engine, me(), ride_s_end());
    let menu = aim_at(&mut engine, me(), ogre);
    assert_eq!(menu, vec![ogre]);
    assert_eq!(
        engine.state().players[usize::from(me().get())]
            .mana_pool
            .total(),
        0,
        "{{1}}{{W}} consumed from the two Plains"
    );
    settle(&mut engine);

    assert!(
        exiled(&engine, them(), gray_ogre()),
        "the tapped creature is exiled"
    );
    assert!(on_battlefield(&engine, them(), gray_ogre()).is_none());
}
