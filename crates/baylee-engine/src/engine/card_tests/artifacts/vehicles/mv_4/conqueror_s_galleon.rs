//! `cards/artifacts/vehicles/mv_4/conqueror_s_galleon.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Conqueror's Galleon` // `Conqueror's Foothold` (`Coverage::Partial`):
/// "When this Vehicle attacks, exile it at end of combat, then return it to the battlefield
/// transformed under your control. Crew 4 (Tap any number of creatures you control with total
/// power 4 or more: This Vehicle becomes an artifact creature until end of turn.)
/// // `{{T}}`: Add `{{C}}`. `{{2}}`, `{{T}}`: Draw a card, then discard a card.
/// `{{4}}`, `{{T}}`: Draw a card. `{{6}}`, `{{T}}`: Return target card from your graveyard to your hand."
///
/// Under `Coverage::Partial`, Crew 4 and the attack-transform trigger are omitted, leaving
/// `Conqueror's Galleon` as a `{4}` artifact vehicle with printed power 2 and toughness 10.
/// The test casts `Conqueror's Galleon` from hand, confirms its 2/10 body and non-creature artifact
/// status, confirms that `LegalActions::abilities` offers no crew ability with mana floating,
/// and confirms that it cannot be declared as an attacker in `Pending::ChooseAttackers`.
#[test]
fn conqueror_s_galleon_casts_as_uncrewed_vehicle_and_cannot_attack() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(306, plains())
        .battlefield(
            0,
            &[plains(), plains(), plains(), plains(), plains(), plains()],
        )
        .hand(0, &[conqueror_s_galleon()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, conqueror_s_galleon());
    pass_until(&mut engine, stack_is_empty);

    let galleon = on_battlefield(&engine, p0, conqueror_s_galleon())
        .expect("Conqueror's Galleon on battlefield");
    assert_eq!(pt(&engine, galleon), (2, 10), "printed 2/10 body");

    let t = types(&engine, galleon);
    assert!(t.contains(TypeSet::ARTIFACT), "Galleon is an artifact");
    assert!(
        !t.contains(TypeSet::CREATURE),
        "uncrewed vehicle is not a creature"
    );
    assert!(!t.contains(TypeSet::LAND), "Galleon is not a land");

    // Float mana from remaining Plains and verify that no crew ability is offered.
    tap_all_mana(&mut engine, p0);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        !legal.abilities.iter().any(|(src, _)| *src == galleon),
        "under `Coverage::Partial` no crew ability is offered"
    );

    // Advance to combat and verify that Galleon cannot attack.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("pass_until stopped on ChooseAttackers");
    };
    assert!(
        !attackers.contains(&galleon),
        "uncrewed vehicle cannot be declared as an attacker"
    );
}
