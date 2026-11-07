//! `cards/sorceries/mv_1/chatter_of_the_squirrel.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_core::{color::Color, generated::subtypes};

fn chatter_of_the_squirrel() -> CardIndex {
    card_index("b0aaa4b1-5188-43a6-997d-7a9b2ad452bc")
}

fn assert_green_squirrel(engine: &Engine<RegistryLookup>, player: PlayerId, squirrel: ObjectId) {
    let object = engine.state().object(squirrel).unwrap();
    assert!(object.card.is_none(), "the Squirrel is a token");
    assert_eq!(object.controller, player);
    assert_eq!(pt(engine, squirrel), (1, 1));
    assert!(object.characteristics().colors.contains(Color::Green));
    assert!(
        object
            .characteristics()
            .subtypes
            .contains(subtypes::creature::SQUIRREL)
    );
}

#[track_caller]
fn start_flashback(engine: &mut Engine<RegistryLookup>, player: PlayerId, chatter: ObjectId) {
    engine
        .apply(player, PlayerAction::CastSpell { card: chatter })
        .unwrap();
    if matches!(engine.pending(), Pending::ChooseCastMode { .. }) {
        let flashback = choose_cast_kind(engine, CastModeKind::Flashback);
        engine
            .apply(player, PlayerAction::ChooseMode(flashback))
            .unwrap();
    }
}

/// One green mana creates one green 1/1 Squirrel. Flashback costs {1}{G},
/// creates a separate second Squirrel and exiles the graveyard incarnation.
#[test]
fn chatter_of_the_squirrel_creates_a_squirrel_and_flashes_back_for_two_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(); 3])
        .hand(0, &[chatter_of_the_squirrel()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, chatter_of_the_squirrel());
    pass_until(&mut engine, stack_is_empty);
    let first = tokens_of(&engine, p0);
    assert_eq!(first.len(), 1, "one spell creates exactly one Squirrel");
    assert_green_squirrel(&engine, p0, first[0]);
    assert_eq!(engine.state().players[0].mana_pool.total(), 2);
    let chatter = in_graveyard(&engine, p0, chatter_of_the_squirrel()).unwrap();

    start_flashback(&mut engine, p0, chatter);
    assert_eq!(engine.state().object(chatter).unwrap().zone, Zone::Stack);
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    pass_until(&mut engine, stack_is_empty);

    let all = tokens_of(&engine, p0);
    assert_eq!(all.len(), 2, "each resolution creates its own token");
    assert!(all.contains(&first[0]), "the first token remains in play");
    assert_ne!(all[0], all[1]);
    for squirrel in all {
        assert_green_squirrel(&engine, p0, squirrel);
    }
    assert_eq!(engine.state().object(chatter).unwrap().zone, Zone::Exile);
    assert!(in_graveyard(&engine, p0, chatter_of_the_squirrel()).is_none());
}

/// The cheaper ordinary cost is unavailable in the graveyard: one green mana
/// neither offers nor pays the {1}{G} flashback cost.
#[test]
fn chatter_of_the_squirrel_flashback_is_not_offered_with_only_one_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest()])
        .hand(0, &[chatter_of_the_squirrel()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let chatter = hand_to_graveyard(&mut engine, p0, chatter_of_the_squirrel());
    tap_all_mana(&mut engine, p0);

    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(!legal.castable.contains(&chatter));
    assert!(!legal.payable.contains(&chatter));
    assert!(
        engine
            .apply(p0, PlayerAction::CastSpell { card: chatter })
            .is_err()
    );
    assert_eq!(
        engine.state().object(chatter).unwrap().zone,
        Zone::Graveyard
    );
    assert_eq!(engine.state().players[0].mana_pool.total(), 1);
    assert!(tokens_of(&engine, p0).is_empty());
}
