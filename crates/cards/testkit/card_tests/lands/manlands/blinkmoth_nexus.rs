//! `cards/lands/manlands/blinkmoth_nexus.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Blinkmoth Nexus is `Coverage::Implemented`.  It has three abilities:
/// index 0 is `{T}: Add {C}` (printed, in `legal.abilities`); index 1 is
/// `{1}: This land becomes a 1/1 Blinkmoth artifact creature with flying until
/// end of turn`; index 2 is `{1}, {T}: Target Blinkmoth creature gets +1/+1
/// until end of turn`.
///
/// After animation (ability 1) the Nexus is a creature **and** still a land.
/// It has summoning sickness — played this turn — so it cannot pay the `{T}`
/// in ability 2 itself this same turn.  These two facts are what the test
/// strikes: the types are both present, and the pump ability is not offered
/// because the Nexus that has summoning sickness cannot tap for it.
///
/// A second copy seated before the game began (via `starting_battlefield`)
/// does not have summoning sickness, so it is pointed at itself after
/// animation to confirm that the pump offer appears for the seated one.
#[test]
fn blinkmoth_nexus_animates_into_a_creature_that_stays_a_land_and_has_summoning_sickness() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(71, forest())
        .battlefield(0, &[blinkmoth_nexus(), forest()])
        .hand(0, &[blinkmoth_nexus()])
        .start();
    keep_mulligans(&mut engine);
    // The seated Nexus is from the starting board; the played one is from hand.
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let seated = on_battlefield(&engine, p0, blinkmoth_nexus()).expect("the seated Nexus");
    let land_id = play_land(&mut engine, p0, blinkmoth_nexus());
    assert_ne!(seated, land_id, "two distinct Nexus objects");

    // Before animation neither is a creature.
    for id in [seated, land_id] {
        assert!(
            !engine
                .state()
                .object(id)
                .expect("still on the battlefield")
                .characteristics()
                .types
                .contains(TypeSet::CREATURE),
            "a land is not a creature before the {{1}} is paid"
        );
    }

    // Tap the Forest for {1}, animate the played Nexus (ability index 1).
    tap_all_mana(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: land_id,
                ability_index: 1,
            },
        )
        .expect("{1} is payable from the Forest");
    pass_until(&mut engine, |e| {
        e.state()
            .object(land_id)
            .is_some_and(|o| o.characteristics().types.contains(TypeSet::CREATURE))
    });

    // After animation: the played Nexus is a creature AND still a land.
    let obj = engine
        .state()
        .object(land_id)
        .expect("the animated Nexus is still an object");
    let types = obj.characteristics().types;
    assert!(types.contains(TypeSet::CREATURE), "it became a creature");
    assert!(types.contains(TypeSet::LAND), "it's still a land");
    assert_eq!(pt(&engine, land_id), (1, 1), "1/1 as printed");
    assert!(
        obj.characteristics().keywords.contains(KeywordSet::FLYING),
        "with flying"
    );

    // Summoning sickness: the played Nexus cannot pay its own {T}.
    // Ability 2 targets a Blinkmoth creature, and the only Blinkmoth is the
    // animated played one — which is summoning sick and cannot tap for the cost.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal
            .abilities
            .iter()
            .any(|(s, i)| *s == land_id && *i == 2),
        "the played Nexus is summoning sick so it cannot pay {{T}} for ability 2: {:?}",
        legal.abilities
    );
}

/// Blinkmoth Nexus: "{1}: This land becomes a 1/1 Blinkmoth artifact
/// creature with flying until end of turn." then "{1}, {T}: Target Blinkmoth
/// creature gets +1/+1 until end of turn." The animated land is its own
/// Blinkmoth target; as a plain land the pump has no target at all.
#[test]
fn blinkmoth_nexus_pumps_itself_once_it_is_a_blinkmoth_creature() {
    let p0 = PlayerId::new(0);
    let nexus = card_index("40d45c02-6416-4e19-8fe3-0ddadf5ba627");
    let mut engine = Duel::new(2104, forest())
        .battlefield(0, &[nexus, forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let n = on_battlefield(&engine, p0, nexus).expect("nexus");
    let forests: Vec<_> = lands_of(&engine, p0)
        .into_iter()
        .filter(|id| *id != n)
        .collect();

    let offer = priority_offer(&engine);
    assert!(
        !offer.abilities.contains(&(n, 2)),
        "no Blinkmoth creature, no pump"
    );

    tap_mana_where(&mut engine, p0, |id| id == forests[0]);
    activate(&mut engine, p0, nexus, 1);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, n), (1, 1));

    tap_mana_where(&mut engine, p0, |id| id == forests[1]);
    activate(&mut engine, p0, nexus, 2);
    unf_aim_and_pay(&mut engine, p0, Some(n), None);
    assert_eq!(pt(&engine, n), (2, 2));
    assert!(is_tapped(&engine, n));
}
