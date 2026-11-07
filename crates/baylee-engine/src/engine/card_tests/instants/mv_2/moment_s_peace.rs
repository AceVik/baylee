//! `cards/instants/mv_2/moment_s_peace.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn moment_s_peace() -> CardIndex {
    card_index("b6d22228-a45e-4296-9ae2-1649e04b1c53")
}

#[track_caller]
fn blocked_attack(engine: &mut Engine<RegistryLookup>, attacker: ObjectId, blocker: ObjectId) {
    let offer = lk::attack(engine, &[attacker]);
    assert!(lk::may_block(&offer, blocker, attacker));
    lk::block(engine, &[(blocker, attacker)]);
    lk::through_combat(engine);
}

#[track_caller]
fn start_flashback(engine: &mut Engine<RegistryLookup>, player: PlayerId, peace: ObjectId) {
    engine
        .apply(player, PlayerAction::CastSpell { card: peace })
        .unwrap();
    if matches!(engine.pending(), Pending::ChooseCastMode { .. }) {
        let flashback = choose_cast_kind(engine, CastModeKind::Flashback);
        engine
            .apply(player, PlayerAction::ChooseMode(flashback))
            .unwrap();
    }
}

/// One ordinary {1}{G} spell prevents combat damage on both sides of a block,
/// not just damage to a player. A Lightning Bolt still deals its noncombat
/// damage, and next turn's unblocked attack is no longer prevented.
#[test]
fn moment_s_peace_prevents_only_this_turns_combat_damage() {
    let mut engine = lk::game(
        SEED,
        &[index::HILL_GIANT],
        &[index::GRAY_OGRE],
        &[moment_s_peace(), lightning_bolt()],
        &[],
    );
    let giant = lk::obj(&engine, P0, index::HILL_GIANT);
    let ogre = lk::obj(&engine, P1, index::GRAY_OGRE);

    lk::float(&mut engine, P0, &[(ManaColor::Green, 2)]);
    cast_with_floating(&mut engine, P0, moment_s_peace());
    assert_eq!(lk::pool_total(&engine, P0), 0);
    lk::settle(&mut engine);
    assert!(in_graveyard(&engine, P0, moment_s_peace()).is_some());

    blocked_attack(&mut engine, giant, ogre);
    assert_eq!(lk::damage_on(&engine, giant), 0, "attacker takes none");
    assert_eq!(lk::damage_on(&engine, ogre), 0, "blocker takes none");
    assert_eq!(lk::life(&engine, P1), 20, "no trample or player damage");

    lk::float(&mut engine, P0, &[(ManaColor::Red, 1)]);
    lk::cast_at(&mut engine, P0, lightning_bolt(), &[], &[P1]);
    assert_eq!(
        lk::life(&engine, P1),
        17,
        "noncombat damage is not prevented"
    );

    reach_their_main_phase(&mut engine, P1);
    reach_their_main_phase(&mut engine, P0);
    let _ = lk::attack(&mut engine, &[giant]);
    lk::block(&mut engine, &[]);
    lk::through_combat(&mut engine);
    assert_eq!(lk::life(&engine, P1), 14, "next turn's combat deals three");
}

/// Two mana are insufficient for {2}{G}. With the third mana, flashback pays
/// the entire pool, prevents the same blocked combat and exiles the spell.
#[test]
fn moment_s_peace_flashback_costs_three_prevents_combat_and_exiles() {
    let mut engine = lk::game(
        SEED,
        &[index::GRIZZLY_BEARS],
        &[index::GRAY_OGRE],
        &[moment_s_peace()],
        &[],
    );
    let bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);
    let ogre = lk::obj(&engine, P1, index::GRAY_OGRE);
    let peace = hand_to_graveyard(&mut engine, P0, moment_s_peace());

    lk::float(&mut engine, P0, &[(ManaColor::Green, 2)]);
    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(!legal.castable.contains(&peace));
    assert!(!legal.payable.contains(&peace));
    assert!(
        engine
            .apply(P0, PlayerAction::CastSpell { card: peace })
            .is_err()
    );
    assert_eq!(lk::zone_of(&engine, peace), Zone::Graveyard);

    lk::float(&mut engine, P0, &[(ManaColor::Green, 1)]);
    start_flashback(&mut engine, P0, peace);
    assert_eq!(lk::pool_total(&engine, P0), 0);
    assert_eq!(lk::zone_of(&engine, peace), Zone::Stack);
    lk::settle(&mut engine);
    assert_eq!(
        engine.state().shields.len(),
        1,
        "flashback must resolve the Fog effect, not merely exile the card"
    );

    blocked_attack(&mut engine, bears, ogre);
    assert_eq!(lk::damage_on(&engine, bears), 0);
    assert_eq!(lk::damage_on(&engine, ogre), 0);
    assert_eq!(lk::life(&engine, P1), 20);
    assert_eq!(lk::zone_of(&engine, peace), Zone::Exile);
    assert!(in_graveyard(&engine, P0, moment_s_peace()).is_none());
}
