//! `cards/creatures/mv_3/suq_ata_firewalker.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn suq_ata_firewalker() -> CardIndex {
    card_index("c9c0bb7e-8dfc-481c-80f7-b6652b63df39")
}

/// Suq'Ata Firewalker (`{1}{U}{U}`, `Coverage::Implemented`): 0/1 Human Wizard
/// that can't be the target of red spells or abilities from red sources and
/// taps to deal 1 damage to any target.
#[test]
fn suq_ata_firewalker_pings_and_cant_be_targeted_by_red_spells() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[suq_ata_firewalker(), grizzly_bears()])
        .battlefield(1, &[mountain()])
        .hand(1, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let firewalker = on_battlefield(&engine, p0, suq_ata_firewalker()).unwrap();
    let bear = on_battlefield(&engine, p0, grizzly_bears()).unwrap();

    // Tap Suq'Ata Firewalker to deal 1 damage to p1.
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: firewalker,
                ability_index: 1,
            },
        )
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[1].life, 19);

    // On p1's turn, Lightning Bolt cannot target Suq'Ata Firewalker.
    reach_their_main_phase(&mut engine, p1);
    cast_from_hand(&mut engine, p1, lightning_bolt());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        unreachable!();
    };
    assert!(
        !options.contains(&firewalker),
        "red spell cannot target Suq'Ata Firewalker"
    );
    assert!(options.contains(&bear));
}

fn mogg_fanatic() -> CardIndex {
    card_index("7efd5d62-6da2-428a-98e3-5b842f668656")
}

fn prodigal_sorcerer() -> CardIndex {
    card_index("5e961d15-5972-4e4b-9385-1cd7cd7c6bbe")
}

/// Suq'Ata Firewalker — "or abilities from red sources": the red Mogg
/// Fanatic's ability cannot name it, while the same kind of ability from the
/// blue Prodigal Sorcerer can.
#[test]
fn suq_ata_firewalker_cant_be_targeted_by_a_red_sources_ability_only() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[suq_ata_firewalker(), grizzly_bears()])
        .battlefield(1, &[mogg_fanatic(), prodigal_sorcerer()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let firewalker = on_battlefield(&engine, p0, suq_ata_firewalker()).unwrap();
    let bear = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let fanatic = on_battlefield(&engine, p1, mogg_fanatic()).unwrap();
    let sorcerer = on_battlefield(&engine, p1, prodigal_sorcerer()).unwrap();
    reach_their_main_phase(&mut engine, p1);

    engine
        .apply(
            p1,
            PlayerAction::ActivateAbility {
                source: fanatic,
                ability_index: 0,
            },
        )
        .unwrap();
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("a target question, got {:?}", engine.pending())
    };
    assert!(
        !options.contains(&firewalker),
        "an ability of a red source cannot target it"
    );
    assert!(options.contains(&bear), "but it can target the Bears");
    engine
        .apply(
            p1,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p0],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    engine
        .apply(
            p1,
            PlayerAction::ActivateAbility {
                source: sorcerer,
                ability_index: 0,
            },
        )
        .unwrap();
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("a target question, got {:?}", engine.pending())
    };
    assert!(
        options.contains(&firewalker),
        "an ability of a blue source may target it"
    );
}
