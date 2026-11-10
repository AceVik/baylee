//! `cards/enchantments/auras/mv_1/apathy.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::GRIZZLY_BEARS;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn apathy() -> CardIndex {
    card_index("09ac5adf-f722-417e-b0fc-7ed0d8c9abd6")
}

/// P0 enchants P1's tapped Bears with Apathy; P1 holds one Forest. Returns the
/// engine at P0's main phase, the Aura resolved.
fn enchanted_board(p1_hand: &[CardIndex]) -> (Engine<RegistryLookup>, ObjectId) {
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island()])
        .battlefield(1, &[GRIZZLY_BEARS])
        .hand(0, &[apathy()])
        .hand(1, p1_hand)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let bears = on_battlefield(&engine, P1, GRIZZLY_BEARS).unwrap();
    engine
        .dev_state_mut(P0)
        .unwrap()
        .object_mut(bears)
        .unwrap()
        .status
        .insert(Status::TAPPED);

    cast_from_hand(&mut engine, P0, apathy());
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![bears],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    (engine, bears)
}

fn is_tapped(engine: &Engine<RegistryLookup>, id: ObjectId) -> bool {
    engine
        .state()
        .object(id)
        .unwrap()
        .status
        .contains(Status::TAPPED)
}

/// Walks to the question Apathy asks at the enchanted creature's controller's
/// upkeep and returns who is asked.
fn to_the_question(engine: &mut Engine<RegistryLookup>) -> PlayerId {
    pass_until(engine, |e| matches!(e.pending(), Pending::YesNo { .. }));
    let Pending::YesNo { player, .. } = engine.pending().clone() else {
        unreachable!()
    };
    player
}

/// `Apathy` ({U}, Enchantment — Aura): "Enchanted creature doesn't untap during
/// its controller's untap step." The enchanted creature stays tapped through
/// its controller's untap step.
#[test]
fn apathy_prevents_enchanted_creature_from_untapping() {
    let (mut engine, bears) = enchanted_board(&[]);

    reach_their_main_phase(&mut engine, P1);
    assert!(is_tapped(&engine, bears), "the untap step did not untap it");
}

/// "That player may discard a card at random. If the player does, untap that
/// creature." The creature's controller (not the Aura's) is asked, the discard
/// is theirs, and the creature untaps.
#[test]
fn apathy_lets_the_creatures_controller_discard_at_random_to_untap_it() {
    let (mut engine, bears) = enchanted_board(&[forest()]);

    let asked = to_the_question(&mut engine);
    assert_eq!(asked, P1, "the enchanted creature's controller decides");
    assert!(is_tapped(&engine, bears), "still tapped when asked");
    engine.apply(P1, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(!is_tapped(&engine, bears), "untapped by the discard");
    assert!(
        in_graveyard(&engine, P1, forest()).is_some(),
        "P1 discarded their Forest"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(P0)).len(),
        0,
        "the Aura's controller discards nothing"
    );
}

/// Declined: the card stays in hand and the creature stays tapped.
#[test]
fn apathy_declined_leaves_the_creature_tapped_and_the_hand_alone() {
    let (mut engine, bears) = enchanted_board(&[forest()]);

    assert_eq!(to_the_question(&mut engine), P1);
    engine.apply(P1, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(is_tapped(&engine, bears));
    assert!(in_hand(&engine, P1, forest()).is_some());
    assert!(in_graveyard(&engine, P1, forest()).is_none());
}

/// "If the player does": with no card to discard the creature stays tapped
/// even when the player says yes.
#[test]
fn apathy_with_an_empty_hand_does_not_untap_the_creature() {
    let (mut engine, bears) = enchanted_board(&[]);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
            || (e.state().turn.active == P1 && matches!(e.state().turn.phase, Phase::FirstMain))
    });
    if matches!(engine.pending(), Pending::YesNo { .. }) {
        engine.apply(P1, PlayerAction::YesNo(true)).unwrap();
    }
    pass_until(&mut engine, stack_is_empty);

    assert!(
        is_tapped(&engine, bears),
        "nothing was discarded, nothing untaps"
    );
}
