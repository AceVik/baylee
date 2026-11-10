//! Dwarven Sea Clan — `{2}{R}` 1/1 Creature — Dwarf (Homelands)
//!
//! `{T}`: Choose target attacking or blocking creature whose controller
//! controls an Island. Dwarven Sea Clan deals 2 damage to that creature at end
//! of combat. Activate only before the end of combat step.

#[allow(clippy::wildcard_imports)]
use super::*;

fn dwarven_sea_clan() -> CardIndex {
    card_index("55ae9711-65ac-4166-8a51-1deeb10a8904")
}

#[test]
fn dwarven_sea_clan_deals_two_damage_at_end_of_combat_to_combat_creature_whose_controller_controls_an_island()
 {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[dwarven_sea_clan(), grizzly_bears()])
        .battlefield(1, &[grizzly_bears(), gray_ogre(), island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let clan = on_battlefield(&engine, p0, dwarven_sea_clan()).unwrap();
    let p0_bears = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let p1_bears = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    let p1_ogre = on_battlefield(&engine, p1, gray_ogre()).unwrap();
    assert_eq!(pt(&engine, clan), (1, 1));
    assert!(
        !ability_offered(&engine, clan, 0),
        "Dwarven Sea Clan cannot activate outside combat"
    );

    // Pass to p1's turn so p1 attacks with Grizzly Bears while controlling an Island.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(p1_bears, Defender::Player(p0))],
            },
        )
        .unwrap();
    engine.apply(p1, PlayerAction::PassPriority).unwrap();

    // During DeclareAttackers step, p0 activates Dwarven Sea Clan.
    assert!(ability_offered(&engine, clan, 0));
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: clan,
                ability_index: 0,
            },
        )
        .unwrap();
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseTargets");
    };
    assert!(
        options.contains(&p1_bears),
        "attacking creature whose controller controls an Island is a legal target"
    );
    assert!(
        !options.contains(&p1_ogre),
        "nonattacking/nonblocking creature is not a legal target"
    );
    assert!(
        !options.contains(&p0_bears),
        "p0's noncombat creature (and p0 controls no Island) is not a legal target"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![p1_bears],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    // The damage is for the end of combat, not for when the ability resolves.
    assert_eq!(
        engine.state().object(p1_bears).unwrap().zone,
        Zone::Battlefield,
        "the 2 damage waits for the end of combat"
    );
    assert_eq!(engine.state().object(p1_bears).unwrap().damage, 0);

    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert_eq!(
        engine.state().players[0].life,
        18,
        "the attacker was still alive to deal its combat damage"
    );
    assert_eq!(
        engine.state().object(p1_bears).unwrap().zone,
        Zone::Graveyard,
        "Dwarven Sea Clan deals 2 lethal damage to the 2/2 attacker at end of combat"
    );
    assert_eq!(
        engine.state().object(p1_ogre).unwrap().damage,
        0,
        "the other creature of the Island's controller is untouched"
    );
}

/// "Whose controller controls an Island": the same attack, but the attacker's
/// controller has no Island, so there is no legal target and the ability is
/// not offered.
#[test]
fn dwarven_sea_clan_has_no_target_when_the_combat_creatures_controller_controls_no_island() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[dwarven_sea_clan()])
        .battlefield(1, &[grizzly_bears(), mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let clan = on_battlefield(&engine, p0, dwarven_sea_clan()).unwrap();
    let p1_bears = on_battlefield(&engine, p1, grizzly_bears()).unwrap();

    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(p1_bears, Defender::Player(p0))],
            },
        )
        .unwrap();
    engine.apply(p1, PlayerAction::PassPriority).unwrap();

    assert!(
        !ability_offered(&engine, clan, 0),
        "an attacker whose controller controls no Island is not a legal target"
    );
}
