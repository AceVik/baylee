//! `cards/instants/mv_6/cauldron_dance.rs`, played.
//!
//! The two creatures have different destinations at the next end step: the
//! reanimated target returns to hand, while the optional creature from hand is
//! sacrificed by its controller.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as kit;
use baylee_core::generated::index;

const P0: PlayerId = kit::P0;
const P1: PlayerId = kit::P1;

fn cauldron_dance() -> CardIndex {
    card_index("e9784eb4-96f2-409e-a7b5-eb6478644943")
}

/// Player 0's first main phase, with a legal graveyard target, a creature in
/// hand, Cauldron Dance, and exactly enough mana already floating.
fn at_main() -> Engine<RegistryLookup> {
    let mut engine = kit::game(
        SEED,
        &[],
        &[],
        &[index::GRAY_OGRE, index::GRIZZLY_BEARS, cauldron_dance()],
        &[],
    );
    hand_to_graveyard(&mut engine, P0, index::GRAY_OGRE);
    kit::float(
        &mut engine,
        P0,
        &[(ManaColor::Black, 5), (ManaColor::Red, 1)],
    );
    engine
}

fn during_combat() -> Engine<RegistryLookup> {
    let mut engine = at_main();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::Combat
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == P0)
    });
    // Mana pools empty as the game leaves the main phase.
    kit::float(
        &mut engine,
        P0,
        &[(ManaColor::Black, 5), (ManaColor::Red, 1)],
    );
    engine
}

#[track_caller]
fn cast_at_the_ogre(engine: &mut Engine<RegistryLookup>) {
    cast_with_floating(engine, P0, cauldron_dance());
    let ogre = in_graveyard(engine, P0, index::GRAY_OGRE).expect("the graveyard target");
    kit::aim(engine, &[ogre], &[]);
}

#[track_caller]
fn optional_hand_choice(engine: &mut Engine<RegistryLookup>) -> (Vec<ObjectId>, u8, u8) {
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("the predicate stopped at the optional hand choice")
    };
    assert_eq!(player, P0);
    (options, min, max)
}

/// "Cast this spell only during combat": sufficient mana and a legal target do
/// not make the spell available in a main phase.
#[test]
fn cauldron_dance_cannot_be_cast_in_a_main_phase() {
    let mut engine = at_main();
    let spell = in_hand(&engine, P0, cauldron_dance()).expect("the spell is in hand");
    assert!(
        engine
            .apply(P0, PlayerAction::CastSpell { card: spell })
            .is_err()
    );
    assert!(in_hand(&engine, P0, cauldron_dance()).is_some());
}

/// Both creatures enter and can attack immediately despite entering this turn.
/// At the next end step the targeted creature returns to hand and the creature
/// chosen from hand is sacrificed, each still tracked as its own object.
#[test]
fn cauldron_dance_returns_one_creature_and_sacrifices_the_other_at_the_end_step() {
    let mut engine = during_combat();
    cast_at_the_ogre(&mut engine);
    let bears_in_hand = in_hand(&engine, P0, index::GRIZZLY_BEARS).expect("Bears in hand");
    let (options, min, max) = optional_hand_choice(&mut engine);
    assert_eq!((min, max), (0, 1), "the second creature is optional");
    assert_eq!(options, vec![bears_in_hand]);
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![bears_in_hand],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let ogre = on_battlefield(&engine, P0, index::GRAY_OGRE).expect("the target returned");
    let bears = on_battlefield(&engine, P0, index::GRIZZLY_BEARS).expect("the hand choice entered");
    for creature in [ogre, bears] {
        assert!(keywords(&engine, creature).contains(KeywordSet::HASTE));
    }
    let attackers = kit::attackers_offered(&mut engine);
    assert!(attackers.contains(&ogre) && attackers.contains(&bears));

    pass_until(&mut engine, |e| e.state().turn.active == P1);
    assert!(in_hand(&engine, P0, index::GRAY_OGRE).is_some());
    assert!(in_graveyard(&engine, P0, index::GRIZZLY_BEARS).is_some());
}

/// Declining the optional hand choice neither moves that creature nor creates
/// a delayed sacrifice for it; the reanimated target still returns to hand.
#[test]
fn cauldron_dance_allows_declining_the_optional_creature_from_hand() {
    let mut engine = during_combat();
    cast_at_the_ogre(&mut engine);
    let bears_in_hand = in_hand(&engine, P0, index::GRIZZLY_BEARS).expect("Bears in hand");
    let (options, min, max) = optional_hand_choice(&mut engine);
    assert_eq!((min, max), (0, 1));
    assert_eq!(options, vec![bears_in_hand]);
    engine
        .apply(P0, PlayerAction::ChooseObjects { objects: vec![] })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P0, index::GRAY_OGRE).is_some());
    assert!(in_hand(&engine, P0, index::GRIZZLY_BEARS).is_some());
    pass_until(&mut engine, |e| e.state().turn.active == P1);
    assert!(in_hand(&engine, P0, index::GRAY_OGRE).is_some());
    assert!(in_hand(&engine, P0, index::GRIZZLY_BEARS).is_some());
    assert!(in_graveyard(&engine, P0, index::GRIZZLY_BEARS).is_none());
}
