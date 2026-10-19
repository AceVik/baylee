//! `cards/creatures/mv_3/orc_general.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::choice::ChoicePrompt;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn orc_general() -> CardIndex {
    card_index("1f696446-30ba-42ab-b4fc-ee9c956b0a62")
}

/// Goblin Flotilla: a Goblin to sacrifice.
fn goblin_flotilla() -> CardIndex {
    card_index("014ca38f-d4ac-4417-bfa8-785a7d6fce43")
}

/// Orc General (1f696446-30ba-42ab-b4fc-ee9c956b0a62) — "{T}, Sacrifice
/// another Orc or Goblin: Other Orc creatures get +1/+1 until end of turn."
/// Two Generals (2/2 Orcs) and a Goblin of mine, a General of theirs and a
/// Gray Ogre of mine: sacrificing the Goblin pumps the other two Generals
/// (mine and theirs), not the activator, not the Ogre.
#[test]
fn orc_general_sacrifices_a_goblin_to_pump_the_other_orcs() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[orc_general(), orc_general(), goblin_flotilla(), gray_ogre()],
        )
        .battlefield(1, &[orc_general()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    let generals = all_on_battlefield(&engine, P0, orc_general());
    assert_eq!(generals.len(), 2);
    let goblin = on_battlefield(&engine, P0, goblin_flotilla()).expect("the Goblin is out");
    let ogre = on_battlefield(&engine, P0, gray_ogre()).expect("the Ogre is out");
    let their_general = on_battlefield(&engine, P1, orc_general()).expect("their General");

    activate(&mut engine, P0, orc_general(), 0);
    let Pending::ChooseCards {
        player,
        options,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "the cost asks what to sacrifice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, P0);
    assert_eq!(prompt, ChoicePrompt::CostSacrifice);
    assert!(options.contains(&goblin), "a Goblin may be sacrificed");
    assert!(
        !options.contains(&ogre),
        "an Ogre is neither Orc nor Goblin"
    );
    assert!(
        !options.contains(&their_general),
        "only my own permanents can be sacrificed"
    );
    assert_eq!(
        options.iter().filter(|id| generals.contains(id)).count(),
        1,
        "the other General is an Orc to sacrifice; the activator itself is not 'another'"
    );
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![goblin],
            },
        )
        .expect("sacrifice the Goblin");
    pass_until(&mut engine, stack_is_empty);

    let activator = *generals
        .iter()
        .find(|id| is_tapped(&engine, **id))
        .expect("one General paid {T}");
    let other = *generals
        .iter()
        .find(|id| **id != activator)
        .expect("the other General");
    assert!(
        in_graveyard(&engine, P0, goblin_flotilla()).is_some(),
        "the Goblin was sacrificed"
    );
    assert_eq!(pt(&engine, activator), (2, 2), "'other' Orcs only");
    assert_eq!(pt(&engine, other), (3, 3), "my other General");
    assert_eq!(pt(&engine, their_general), (3, 3), "any other Orc creature");
    assert_eq!(pt(&engine, ogre), (2, 2), "an Ogre is not an Orc");

    reach_their_main_phase(&mut engine, P1);
    assert_eq!(pt(&engine, other), (2, 2), "until end of turn only");
    assert_eq!(pt(&engine, their_general), (2, 2));
}

/// The cost can be another Orc.
#[test]
fn orc_general_can_sacrifice_another_orc() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[orc_general(), orc_general()])
        .battlefield(1, &[orc_general()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    let generals = all_on_battlefield(&engine, P0, orc_general());
    let their_general = on_battlefield(&engine, P1, orc_general()).expect("their General");
    activate(&mut engine, P0, orc_general(), 0);
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!(
            "the cost asks what to sacrifice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(options.len(), 1, "just the other General");
    let victim = options[0];
    assert!(generals.contains(&victim));
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![victim],
            },
        )
        .expect("sacrifice the other General");
    pass_until(&mut engine, stack_is_empty);
    assert!(
        engine
            .state()
            .object(victim)
            .is_some_and(|o| o.zone != Zone::Battlefield)
    );
    assert_eq!(pt(&engine, their_general), (3, 3));
}

/// Nothing to sacrifice, no activation.
#[test]
fn orc_general_needs_another_orc_or_goblin() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[orc_general(), gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    let general = on_battlefield(&engine, P0, orc_general()).expect("the General");
    assert!(
        engine
            .apply(
                P0,
                PlayerAction::ActivateAbility {
                    source: general,
                    ability_index: 0
                }
            )
            .is_err(),
        "an Ogre is not an Orc or a Goblin and it cannot sacrifice itself"
    );
    assert!(!is_tapped(&engine, general));
}
