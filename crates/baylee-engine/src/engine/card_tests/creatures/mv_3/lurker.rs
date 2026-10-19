//! `cards/creatures/mv_3/lurker.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::turn::{Phase, Step};

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn lurker() -> CardIndex {
    card_index("4aedbbaf-b6ae-4bf0-84eb-31b0b28cfdfa")
}

fn scarwood_hag() -> CardIndex {
    card_index("2b483041-4dea-468a-9857-50dae2e8a57d")
}

/// Casts a Lightning Bolt from P0's hand off one floating {R} and returns the
/// objects its target question offers.
#[track_caller]
fn bolt_menu(engine: &mut Engine<RegistryLookup>) -> Vec<ObjectId> {
    engine.state.players[0].mana_pool.add(ManaColor::Red, 1);
    engine.refresh_offer();
    cast_with_floating(engine, P0, lightning_bolt());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("the Bolt asks for its target, got {:?}", engine.pending())
    };
    options
}

/// Finishes the Bolt by sending it at the other player's face.
#[track_caller]
fn bolt_the_face(engine: &mut Engine<RegistryLookup>) {
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![P1],
            },
        )
        .expect("the player is a legal target");
    pass_until(engine, stack_is_empty);
}

/// Lurker (4aedbbaf-b6ae-4bf0-84eb-31b0b28cfdfa) — "This creature can't be
/// the target of spells unless it attacked or blocked this turn." Idle, a
/// Lightning Bolt cannot be aimed at it while another creature can be.
#[test]
fn lurker_cannot_be_targeted_by_a_spell_while_idle() {
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[lightning_bolt()])
        .battlefield(1, &[lurker(), gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    let lurker_id = on_battlefield(&engine, P1, lurker()).expect("the Lurker is out");
    let ogre = on_battlefield(&engine, P1, gray_ogre()).expect("the Ogre is out");
    let menu = bolt_menu(&mut engine);
    assert!(menu.contains(&ogre), "an ordinary creature is on the menu");
    assert!(
        !menu.contains(&lurker_id),
        "the Lurker has neither attacked nor blocked"
    );
}

/// Once it has attacked, it can be targeted for the rest of that turn.
#[test]
fn lurker_can_be_targeted_after_it_attacked() {
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[lightning_bolt(), lightning_bolt()])
        .battlefield(0, &[lurker()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    let lurker_id = on_battlefield(&engine, P0, lurker()).expect("the Lurker is out");

    let before = bolt_menu(&mut engine);
    assert!(
        !before.contains(&lurker_id),
        "even its own controller's spell cannot target it before it attacks"
    );
    bolt_the_face(&mut engine);
    assert_eq!(engine.state().players[1].life, 17);

    pass_until(&mut engine, |e| {
        e.state().turn.step == Step::DeclareAttackers
    });
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(lurker_id, Defender::Player(P1))],
            },
        )
        .expect("the Lurker attacks");
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    let after = bolt_menu(&mut engine);
    assert!(
        after.contains(&lurker_id),
        "it attacked this turn, so spells may target it"
    );
}

/// Blocking counts too.
#[test]
fn lurker_can_be_targeted_after_it_blocked() {
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[lightning_bolt()])
        .battlefield(0, &[gray_ogre()])
        .battlefield(1, &[lurker()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    let ogre = on_battlefield(&engine, P0, gray_ogre()).expect("the Ogre is out");
    let lurker_id = on_battlefield(&engine, P1, lurker()).expect("the Lurker is out");
    pass_until(&mut engine, |e| {
        e.state().turn.step == Step::DeclareAttackers
    });
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(ogre, Defender::Player(P1))],
            },
        )
        .expect("the Ogre attacks");
    pass_until(&mut engine, |e| {
        e.state().turn.step == Step::DeclareBlockers
    });
    engine
        .apply(
            P1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(lurker_id, ogre)],
            },
        )
        .expect("the Lurker blocks");
    let menu = bolt_menu(&mut engine);
    assert!(
        menu.contains(&lurker_id),
        "it blocked this turn, so spells may target it"
    );
}

/// The restriction is about spells: an ability can target it while idle.
#[test]
fn lurker_can_be_targeted_by_an_ability_while_idle() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[scarwood_hag()])
        .battlefield(1, &[lurker()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    let lurker_id = on_battlefield(&engine, P1, lurker()).expect("the Lurker is out");
    activate(&mut engine, P0, scarwood_hag(), 1);
    let menu = aim_at(&mut engine, P0, lurker_id);
    assert!(menu.contains(&lurker_id), "an ability may target it");
    pass_until(&mut engine, stack_is_empty);
}
