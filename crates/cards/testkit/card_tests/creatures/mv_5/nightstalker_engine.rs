//! `cards/creatures/mv_5/nightstalker_engine.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn nightstalker_engine() -> CardIndex {
    card_index("128b6751-4b47-4d7d-895e-c5d85776f6c4")
}

/// Power equals the number of creature cards in your graveyard only, ignoring
/// noncreature cards in your graveyard (the resolved Lightning Bolts) and
/// creature cards in an opponent's graveyard.
#[test]
fn nightstalker_engine_counts_only_creature_cards_in_your_graveyard() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(
            0,
            &[
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                mountain(),
                mountain(),
                mountain(),
                grizzly_bears(),
                gray_ogre(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .hand(
            0,
            &[
                nightstalker_engine(),
                lightning_bolt(),
                lightning_bolt(),
                lightning_bolt(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    let swamps = all_on_battlefield(&engine, p0, swamp());
    tap_mana_where(&mut engine, p0, |id| swamps.contains(&id));
    let spell = in_hand(&engine, p0, nightstalker_engine()).unwrap();
    engine
        .apply(p0, PlayerAction::CastSpell { card: spell })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    let stalker = on_battlefield(&engine, p0, nightstalker_engine()).expect("resolved");
    assert_eq!(pt(&engine, stalker), (0, 3), "empty graveyard -> 0/3");

    let mtns = all_on_battlefield(&engine, p0, mountain());
    let opp_elf = on_battlefield(&engine, p1, llanowar_elves()).unwrap();
    let bears = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let ogre = on_battlefield(&engine, p0, gray_ogre()).unwrap();

    // Bolt #1 kills opponent's Elf (in p1's graveyard) + puts Bolt #1 in p0's graveyard: still 0/3.
    tap_mana_where(&mut engine, p0, |id| id == mtns[0]);
    let b1 = in_hand(&engine, p0, lightning_bolt()).unwrap();
    engine
        .apply(p0, PlayerAction::CastSpell { card: b1 })
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![opp_elf],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, stalker),
        (0, 3),
        "opponent's dead creature and own noncreature Lightning Bolt do not count"
    );

    // Bolt #2 and Bolt #3 kill p0's Bears and Ogre: grows to 1/3 then 2/3 (ignoring the 3 Bolts).
    for (mtn, victim, expected) in [(mtns[1], bears, 1), (mtns[2], ogre, 2)] {
        tap_mana_where(&mut engine, p0, |id| id == mtn);
        let bolt = in_hand(&engine, p0, lightning_bolt()).unwrap();
        engine
            .apply(p0, PlayerAction::CastSpell { card: bolt })
            .unwrap();
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![victim],
                    players: vec![],
                },
            )
            .unwrap();
        pass_until(&mut engine, stack_is_empty);
        assert_eq!(pt(&engine, stalker), (expected, 3));
    }

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    let _ = attack_and_collect_blocks(&mut engine, stalker, p1);
    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert_eq!(engine.state().players[1].life, 18);
}
