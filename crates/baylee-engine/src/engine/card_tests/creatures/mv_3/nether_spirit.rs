//! `cards/creatures/mv_3/nether_spirit.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "194e2563-c0bf-4151-bbb0-0d626130b055"
fn nether_spirit() -> CardIndex {
    card_index("194e2563-c0bf-4151-bbb0-0d626130b055")
}

/// `Nether Spirit` (`Coverage::Implemented`):
/// "At the beginning of your upkeep, if this card is the only creature card in your graveyard, you may return this card to the battlefield."
///
/// Verifies that `Nether Spirit` does not trigger when another creature card is in
/// your graveyard, and triggers to return to the battlefield when it is the only
/// creature card in your graveyard (even alongside noncreature cards).
#[test]
fn nether_spirit_returns_on_upkeep_only_when_sole_creature_in_graveyard() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(9409, swamp())
        .battlefield(0, &[nether_spirit(), grizzly_bears(), swamp()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    // Move Nether Spirit, Grizzly Bears, and Swamp into P0's graveyard.
    let spirit = on_battlefield(&engine, p0, nether_spirit()).expect("Nether Spirit");
    let bear = on_battlefield(&engine, p0, grizzly_bears()).expect("Grizzly Bears");
    let land = on_battlefield(&engine, p0, swamp()).expect("Swamp");
    for obj in [spirit, bear, land] {
        engine
            .dev_state_mut(p0)
            .unwrap()
            .move_object(
                obj,
                ZoneLocation::Graveyard(p0),
                ZonePosition::Top,
                Cause::Effect,
            )
            .unwrap();
    }
    engine.refresh_offer();

    // Advance to P1's main phase and then through P0's next upkeep with Grizzly Bears in graveyard:
    // Nether Spirit does not trigger because Grizzly Bears is also a creature card in P0's graveyard.
    assert!(walk_to_own_main(&mut engine, p1));
    assert!(walk_to_own_main(&mut engine, p0));
    assert!(
        on_battlefield(&engine, p0, nether_spirit()).is_none(),
        "Nether Spirit stays in graveyard when another creature card is present"
    );

    // Exile Grizzly Bears from P0's graveyard (leaving Nether Spirit + Swamp in graveyard).
    let bear_in_gy = in_graveyard(&engine, p0, grizzly_bears()).expect("bear in graveyard");
    engine
        .dev_state_mut(p0)
        .unwrap()
        .move_object(
            bear_in_gy,
            ZoneLocation::Exile(p0),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    engine.refresh_offer();

    // Advance to P0's next upkeep: Nether Spirit triggers and returns to the battlefield!
    assert!(walk_to_own_main(&mut engine, p1));
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::MayDo,
                ..
            }
        )
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, nether_spirit()).is_some(),
        "Nether Spirit returned from graveyard to the battlefield"
    );
}
