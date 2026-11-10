//! `cards/sorceries/mv_2/morgue_theft.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_core::generated::index;

fn morgue_theft() -> CardIndex {
    card_index("0b8a508b-20c0-4a87-a73a-08af72c39a0a")
}

#[track_caller]
fn start_flashback(engine: &mut Engine<RegistryLookup>, player: PlayerId, theft: ObjectId) {
    engine
        .apply(player, PlayerAction::CastSpell { card: theft })
        .unwrap();
    if matches!(engine.pending(), Pending::ChooseCastMode { .. }) {
        let flashback = choose_cast_kind(engine, CastModeKind::Flashback);
        engine
            .apply(player, PlayerAction::ChooseMode(flashback))
            .unwrap();
    }
}

/// For {1}{B}, only a creature card in the caster's graveyard is a legal
/// target. The chosen incarnation returns to its owner's hand, not the
/// battlefield; an opposing creature and our noncreature stay in graveyard.
#[test]
fn morgue_theft_returns_only_the_chosen_own_creature_to_hand() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp()])
        .hand(0, &[morgue_theft(), index::GRIZZLY_BEARS, lightning_bolt()])
        .hand(1, &[index::GRAY_OGRE])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let bears = hand_to_graveyard(&mut engine, p0, index::GRIZZLY_BEARS);
    let bolt = hand_to_graveyard(&mut engine, p0, lightning_bolt());
    let ogre = hand_to_graveyard(&mut engine, p1, index::GRAY_OGRE);

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, morgue_theft());
    let options = aim_at(&mut engine, p0, bears);
    assert_eq!(options, vec![bears], "only our creature is a target");
    assert!(!options.contains(&bolt) && !options.contains(&ogre));
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(in_hand(&engine, p0, index::GRIZZLY_BEARS), Some(bears));
    assert!(on_battlefield(&engine, p0, index::GRIZZLY_BEARS).is_none());
    assert!(in_graveyard(&engine, p0, index::GRIZZLY_BEARS).is_none());
    assert_eq!(engine.state().object(bolt).unwrap().zone, Zone::Graveyard);
    assert_eq!(engine.state().object(ogre).unwrap().zone, Zone::Graveyard);
    assert!(in_graveyard(&engine, p0, morgue_theft()).is_some());
}

/// Flashback replaces {1}{B} with {4}{B}. It returns the same graveyard
/// incarnation to hand, then exiles Morgue Theft rather than recurring it.
#[test]
fn morgue_theft_flashback_costs_five_returns_a_creature_and_exiles() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(); 5])
        .hand(0, &[morgue_theft(), index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let theft = hand_to_graveyard(&mut engine, p0, morgue_theft());
    let bears = hand_to_graveyard(&mut engine, p0, index::GRIZZLY_BEARS);

    tap_all_mana(&mut engine, p0);
    start_flashback(&mut engine, p0, theft);
    aim_at(&mut engine, p0, bears);
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    assert_eq!(engine.state().object(theft).unwrap().zone, Zone::Stack);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(in_hand(&engine, p0, index::GRIZZLY_BEARS), Some(bears));
    assert!(on_battlefield(&engine, p0, index::GRIZZLY_BEARS).is_none());
    assert_eq!(engine.state().object(theft).unwrap().zone, Zone::Exile);
    assert!(in_graveyard(&engine, p0, morgue_theft()).is_none());
}

/// Four black mana cannot pay {4}{B} even with a legal creature target. The
/// failed cast changes neither zone nor mana pool.
#[test]
fn morgue_theft_flashback_is_not_offered_with_only_four_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(); 4])
        .hand(0, &[morgue_theft(), index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let theft = hand_to_graveyard(&mut engine, p0, morgue_theft());
    let bears = hand_to_graveyard(&mut engine, p0, index::GRIZZLY_BEARS);
    tap_all_mana(&mut engine, p0);

    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(!legal.castable.contains(&theft));
    assert!(!legal.payable.contains(&theft));
    assert!(
        engine
            .apply(p0, PlayerAction::CastSpell { card: theft })
            .is_err()
    );
    assert_eq!(engine.state().object(theft).unwrap().zone, Zone::Graveyard);
    assert_eq!(engine.state().object(bears).unwrap().zone, Zone::Graveyard);
    assert_eq!(engine.state().players[0].mana_pool.total(), 4);
}
