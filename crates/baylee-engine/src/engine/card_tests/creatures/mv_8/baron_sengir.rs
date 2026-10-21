//! Baron Sengir — `{5}{B}{B}{B}` 5/5 Legendary Creature — Vampire Noble (Homelands)
//!
//! Flying
//! Whenever a creature dealt damage by Baron Sengir this turn dies, put a
//! +2/+2 counter on Baron Sengir.
//! `{T}`: Regenerate another target Vampire.

#[allow(clippy::wildcard_imports)]
use super::*;

fn baron_sengir() -> CardIndex {
    card_index("fc728cd7-2b5b-45f3-896a-a45648bcb61d")
}

fn sengir_vampire() -> CardIndex {
    card_index("749141aa-f6c4-4ad8-b146-406e68ae9b0b")
}

#[test]
fn baron_sengir_gains_plus_two_plus_two_counter_on_kill_and_regenerates_another_vampire() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .hand(0, &[lightning_bolt(), lightning_bolt()])
        .battlefield(0, &[baron_sengir(), sengir_vampire(), mountain(), mountain()])
        .battlefield(1, &[sengir_vampire()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let baron = on_battlefield(&engine, p0, baron_sengir()).unwrap();
    let ally_vampire = on_battlefield(&engine, p0, sengir_vampire()).unwrap();
    let opp_vampire = on_battlefield(&engine, p1, sengir_vampire()).unwrap();
    assert_eq!(pt(&engine, baron), (5, 5));
    assert!(keywords(&engine, baron).contains(KeywordSet::FLYING));

    // Attack with Baron Sengir; p1 blocks with 4/4 Sengir Vampire.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(baron, Defender::Player(p1))],
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
                blockers: vec![(opp_vampire, baron)],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });

    // Opponent's 4/4 Sengir Vampire died after being dealt 5 damage by Baron Sengir -> +2/+2 counter!
    assert_eq!(
        engine.state().object(opp_vampire).unwrap().zone,
        Zone::Graveyard
    );
    assert_eq!(
        engine
            .state()
            .object(baron)
            .unwrap()
            .counters
            .get(CounterKind::Plus {
                power: 2,
                toughness: 2,
            }),
        1
    );
    assert_eq!(pt(&engine, baron), (7, 7));

    // On p0's next turn, Baron Sengir untaps and uses {T}: Regenerate another target Vampire.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && e.state().turn.phase == Phase::FirstMain
    });
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0 && e.state().turn.phase == Phase::FirstMain
    });
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: baron,
                ability_index: 1,
            },
        )
        .unwrap();
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseTargets");
    };
    assert!(
        !options.contains(&baron),
        "Baron Sengir can only target *another* Vampire, not itself"
    );
    assert!(options.contains(&ally_vampire));
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![ally_vampire],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    // Cast two Lightning Bolts (6 damage) at 4/4 ally_vampire: it regenerates!
    tap_all_mana(&mut engine, p0);
    for _ in 0..2 {
        let bolt = in_hand(&engine, p0, lightning_bolt()).unwrap();
        engine
            .apply(p0, PlayerAction::CastSpell { card: bolt })
            .unwrap();
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![ally_vampire],
                    players: vec![],
                },
            )
            .unwrap();
        pass_until(&mut engine, stack_is_empty);
    }
    assert_eq!(
        engine.state().object(ally_vampire).unwrap().zone,
        Zone::Battlefield,
        "another target Vampire regenerated from lethal damage via Baron Sengir"
    );
    assert!(is_tapped(&engine, ally_vampire));
}
