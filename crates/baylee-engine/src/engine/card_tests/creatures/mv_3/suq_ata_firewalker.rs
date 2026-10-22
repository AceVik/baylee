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
