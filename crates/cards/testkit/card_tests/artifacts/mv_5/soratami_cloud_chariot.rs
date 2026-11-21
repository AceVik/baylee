//! `cards/artifacts/mv_5/soratami_cloud_chariot.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn soratami_cloud_chariot() -> CardIndex {
    card_index("1a46373a-b48a-4118-92e0-d856ce409690")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn serra_angel() -> CardIndex {
    card_index("4b7ac066-e5c7-43e6-9e7e-2739b24a905d")
}

/// Soratami Cloud Chariot (1a46373a-b48a-4118-92e0-d856ce409690) — {5} Artifact:
/// "{2}: Target creature you control gains flying until end of turn."
/// "{2}: Prevent all combat damage that would be dealt to and dealt by target
///  creature you control this turn."
///
/// Ability 0 grants flying only to a creature you control until end of turn.
#[test]
fn soratami_cloud_chariot_grants_flying_to_own_creature_until_end_of_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                soratami_cloud_chariot(),
                grizzly_bears(),
                forest(),
                forest(),
            ],
        )
        .battlefield(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let my_bears = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let their_bears = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    assert!(!keywords(&engine, my_bears).contains(KeywordSet::FLYING));

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, soratami_cloud_chariot(), 0);
    let menu = aim_at(&mut engine, p0, my_bears);
    assert!(menu.contains(&my_bears) && !menu.contains(&their_bears));
    pass_until(&mut engine, stack_is_empty);
    assert!(keywords(&engine, my_bears).contains(KeywordSet::FLYING));

    // Next turn: flying wore off at end of turn.
    pass_until(&mut engine, |e| e.state().turn.active == p1);
    assert!(!keywords(&engine, my_bears).contains(KeywordSet::FLYING));
}

/// Ability 1 prevents all COMBAT damage dealt to and dealt by target creature
/// you control (both attacker and blocker survive with 0 damage), while
/// noncombat Lightning Bolt STILL deals full 3 damage!
#[test]
fn soratami_cloud_chariot_prevents_combat_damage_both_ways_but_not_bolt() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                soratami_cloud_chariot(),
                serra_angel(),
                forest(),
                forest(),
                mountain(),
            ],
        )
        .hand(0, &[lightning_bolt()])
        .battlefield(1, &[serra_angel()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let my_angel = on_battlefield(&engine, p0, serra_angel()).unwrap();
    let their_angel = on_battlefield(&engine, p1, serra_angel()).unwrap();

    // Activate ability 1 ({2}) targeting own Serra Angel.
    let my_forests = forests(&engine, p0, 2);
    tap_mana_where(&mut engine, p0, |id| my_forests.contains(&id));
    activate(&mut engine, p0, soratami_cloud_chariot(), 1);
    let menu = aim_at(&mut engine, p0, my_angel);
    assert!(menu.contains(&my_angel) && !menu.contains(&their_angel));
    pass_until(&mut engine, stack_is_empty);

    // Attack with 4/4 Serra Angel; opponent blocks with 4/4 Serra Angel -> 0 combat damage both ways!
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(my_angel, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(their_angel, my_angel)],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::SecondMain)
    });

    assert_eq!(
        engine.state().object(my_angel).unwrap().zone,
        Zone::Battlefield
    );
    assert_eq!(
        engine.state().object(my_angel).unwrap().damage,
        0,
        "0 combat damage to it"
    );
    assert_eq!(
        engine.state().object(their_angel).unwrap().zone,
        Zone::Battlefield
    );
    assert_eq!(
        engine.state().object(their_angel).unwrap().damage,
        0,
        "0 combat damage by it"
    );

    // Noncombat Bolt still deals full 3 damage to my_angel!
    let my_mtn = on_battlefield(&engine, p0, mountain()).unwrap();
    tap_mana_where(&mut engine, p0, |id| id == my_mtn);
    cast_with_floating(&mut engine, p0, lightning_bolt());
    aim_at(&mut engine, p0, my_angel);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(my_angel).unwrap().damage,
        3,
        "noncombat Lightning Bolt is NOT prevented"
    );
}
