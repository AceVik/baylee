//! `cards/artifacts/mv_5/storm_cauldron.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn storm_cauldron() -> CardIndex {
    card_index("5a51b168-02d1-4eb4-8ccc-614ab6f6cffa")
}

/// Storm Cauldron ({5} Artifact):
/// - "Each player may play an additional land during each of their turns." (not played)
/// - "Whenever a land is tapped for mana, return it to its owner's hand."
#[test]
fn storm_cauldron_bounces_lands_tapped_for_mana_and_grants_no_extra_land_play() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[storm_cauldron()])
        .hand(0, &[forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // One land per turn: the additional land play is not played (it would be
    // the controller's alone, while the printed card gives it to each player).
    let first = in_hand(&engine, p0, forest()).unwrap();
    engine
        .apply(p0, PlayerAction::PlayLand { card: first })
        .unwrap();
    let second = in_hand(&engine, p0, forest()).unwrap();
    assert!(
        engine
            .apply(p0, PlayerAction::PlayLand { card: second })
            .is_err(),
        "no additional land play"
    );
    let land = on_battlefield(&engine, p0, forest()).unwrap();

    // Tapping a Forest for mana triggers Storm Cauldron to return that Forest to its owner's hand.
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: land,
                ability_index: 0,
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().object(land).unwrap().zone, Zone::Hand);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1
    );
}

/// "Whenever a land is tapped for mana, return it to its owner's hand." names
/// any player's land, and only a land: the opponent's Forest goes back to the
/// opponent's hand, and their Sol Ring (not a land) stays tapped on the table.
#[test]
fn storm_cauldron_bounces_the_opponents_land_but_not_an_artifact_mana_source() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[storm_cauldron()])
        .battlefield(1, &[forest(), sol_ring()])
        .start();
    keep_mulligans(&mut engine);
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && e.state().turn.phase == Phase::FirstMain && stack_is_empty(e)
    });
    let ring = on_battlefield(&engine, p1, sol_ring()).unwrap();
    let land = on_battlefield(&engine, p1, forest()).unwrap();

    // Not a land: tapped for mana, it stays. A printed mana ability is an
    // ordinary entry of `legal.abilities`; `ActivateManaAbility` is the
    // basic-land shortcut and does not list a Sol Ring.
    activate(&mut engine, p1, sol_ring(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().object(ring).unwrap().zone, Zone::Battlefield);
    assert!(is_tapped(&engine, ring));

    // The opponent's land: tapped for mana, it goes to its owner's hand.
    engine
        .apply(p1, PlayerAction::ActivateManaAbility { source: land })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().object(land).unwrap().zone, Zone::Hand);
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Hand(p1))
            .contains(&land)
    );
    assert!(
        !engine
            .state()
            .zones
            .list(ZoneLocation::Hand(p0))
            .contains(&land)
    );
}
