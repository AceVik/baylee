//! Serra Bestiary — `{W}{W}` Enchantment — Aura (Homelands)
//!
//! Enchant creature
//! At the beginning of your upkeep, sacrifice this Aura unless you pay `{W}{W}`.
//! Enchanted creature can't attack or block, and its activated abilities with `{T}` in their costs
//! can't be activated.
//!
//! The last clause is not on the card (`Modifier::CantActivate` would also lock abilities without
//! `{T}` in the cost, which is more than printed), so the card is `Partial` and no test here says
//! anything about the enchanted creature's activated abilities.

#[allow(clippy::wildcard_imports)]
use super::*;

fn serra_bestiary() -> CardIndex {
    card_index("320d8f2b-005d-4769-80d2-f46d96afffe7")
}

fn prodigal_sorcerer() -> CardIndex {
    card_index("5e961d15-5972-4e4b-9385-1cd7cd7c6bbe")
}

/// P0 puts Serra Bestiary on P1's Prodigal Sorcerer in the first main phase; P1 also has a Gray
/// Ogre, and P0 a Grizzly Bears to attack with. Returns the engine, the Sorcerer and the Ogre.
fn bestiary_on_the_sorcerer() -> (Engine<RegistryLookup>, ObjectId, ObjectId) {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .hand(0, &[serra_bestiary()])
        .battlefield(0, &[plains(), plains(), grizzly_bears()])
        .battlefield(1, &[prodigal_sorcerer(), gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bestiary_card = in_hand(&engine, p0, serra_bestiary()).unwrap();
    let tim = on_battlefield(&engine, p1, prodigal_sorcerer()).unwrap();
    let ogre = on_battlefield(&engine, p1, gray_ogre()).unwrap();

    tap_all_mana(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::CastSpell {
                card: bestiary_card,
            },
        )
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![tim],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    (engine, tim, ogre)
}

#[test]
fn serra_bestiary_stops_the_enchanted_creature_attacking_and_only_that_one() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let (mut engine, tim, ogre) = bestiary_on_the_sorcerer();

    // P1's turn: the enchanted Prodigal Sorcerer cannot attack, the Gray Ogre can.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    assert!(
        engine
            .apply(
                p1,
                PlayerAction::DeclareAttackers {
                    attackers: vec![(tim, Defender::Player(p0))],
                },
            )
            .is_err(),
        "enchanted Prodigal Sorcerer cannot attack"
    );
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(ogre, Defender::Player(p0))],
            },
        )
        .expect("the unenchanted Gray Ogre may attack");
}

#[test]
fn serra_bestiary_stops_the_enchanted_creature_blocking_and_only_that_one() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let (mut engine, tim, ogre) = bestiary_on_the_sorcerer();
    let bears = on_battlefield(&engine, p0, grizzly_bears()).unwrap();

    // P0 attacks with the Bears: the Sorcerer cannot block, the Ogre can.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(bears, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    assert!(
        engine
            .apply(
                p1,
                PlayerAction::DeclareBlockers {
                    blockers: vec![(tim, bears)],
                },
            )
            .is_err(),
        "enchanted Prodigal Sorcerer cannot block"
    );
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(ogre, bears)],
            },
        )
        .expect("the unenchanted Gray Ogre may block");
}

#[test]
fn serra_bestiary_is_sacrificed_at_its_controllers_upkeep_unless_ww_is_paid() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    // Declined: the Aura goes. Paid: it stays. The opponent's upkeep in between asks nothing
    // ("your upkeep"): `pass_until` would stop with a panic on a question it does not know.
    for pay in [false, true] {
        let (mut engine, _tim, _ogre) = bestiary_on_the_sorcerer();

        pass_until(&mut engine, |e| {
            e.state().turn.active == p1 && e.state().turn.phase == Phase::FirstMain
        });
        assert!(
            on_battlefield(&engine, p0, serra_bestiary()).is_some(),
            "the opponent's upkeep did not cost the Aura anything"
        );

        pass_until(&mut engine, |e| {
            e.state().turn.active == p0 && matches!(e.pending(), Pending::YesNo { .. })
        });
        engine.apply(p0, PlayerAction::YesNo(pay)).unwrap();
        if pay {
            // The window for making {W}{W}: both Plains.
            tap_all_mana(&mut engine, p0);
            engine.apply(p0, PlayerAction::PassPriority).unwrap();
        }
        pass_until(&mut engine, |e| {
            e.state().turn.active == p0 && e.state().turn.phase == Phase::FirstMain
        });
        assert_eq!(
            on_battlefield(&engine, p0, serra_bestiary()).is_some(),
            pay,
            "paid {{W}}{{W}}: {pay}"
        );
    }
}
