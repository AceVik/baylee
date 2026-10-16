//! `cards/artifacts/mv_4/trading_post.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn trading_post() -> CardIndex {
    card_index("63788566-e25a-44bb-bb55-197e1b93b3e8")
}

/// A Duel with the Post, a Forest for the {1}, and `extra` besides.
fn table(extra: &[CardIndex], hand: &[CardIndex]) -> Engine<RegistryLookup> {
    let p0 = PlayerId::new(0);
    let mut board = vec![trading_post(), forest()];
    board.extend_from_slice(extra);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &board)
        .hand(0, hand)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    engine
}

fn pay_one_and_press(engine: &mut Engine<RegistryLookup>, line: u32) {
    let p0 = PlayerId::new(0);
    let post = on_battlefield(engine, p0, trading_post()).unwrap();
    tap_mana_except(engine, p0, post);
    activate(engine, p0, trading_post(), line);
}

fn choose(engine: &mut Engine<RegistryLookup>, object: ObjectId) {
    engine
        .apply(
            PlayerId::new(0),
            PlayerAction::ChooseObjects {
                objects: vec![object],
            },
        )
        .expect("a listed card");
}

/// "{1}, {T}, Discard a card: You gain 4 life."
#[test]
fn discarding_a_card_gains_four_life() {
    let p0 = PlayerId::new(0);
    let mut engine = table(&[], &[plains()]);
    pay_one_and_press(&mut engine, 0);
    let card = in_hand(&engine, p0, plains()).unwrap();
    choose(&mut engine, card);
    assert!(in_graveyard(&engine, p0, plains()).is_some());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(life_of(&engine, p0), 24);
}

/// "{1}, {T}, Pay 1 life: Create a 0/1 white Goat creature token."
#[test]
fn paying_a_life_makes_a_goat() {
    let p0 = PlayerId::new(0);
    let mut engine = table(&[], &[]);
    pay_one_and_press(&mut engine, 1);
    assert_eq!(life_of(&engine, p0), 19, "the life is a cost");
    pass_until(&mut engine, stack_is_empty);
    let goats = tokens_of(&engine, p0);
    assert_eq!(goats.len(), 1);
    assert_eq!(pt(&engine, goats[0]), (0, 1));
    let chars = engine.state().object(goats[0]).unwrap().characteristics().clone();
    assert!(chars.types.contains(TypeSet::CREATURE));
    assert!(
        chars
            .subtypes
            .contains(baylee_core::generated::subtypes::creature::GOAT)
    );
}

/// "{1}, {T}, Sacrifice a creature: Return target artifact card from your
/// graveyard to your hand."
#[test]
fn sacrificing_a_creature_returns_an_artifact_card() {
    let p0 = PlayerId::new(0);
    let mut engine = table(&[llanowar_elves()], &[quiet_artifact(), plains()]);
    hand_to_graveyard(&mut engine, p0, quiet_artifact());
    hand_to_graveyard(&mut engine, p0, plains());
    let elf = on_battlefield(&engine, p0, llanowar_elves()).unwrap();
    pay_one_and_press(&mut engine, 2);
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("the creature to sacrifice, got {:?}", engine.pending())
    };
    assert_eq!(options, vec![elf], "the only creature is the one on the menu");
    choose(&mut engine, elf);
    assert!(in_graveyard(&engine, p0, llanowar_elves()).is_some());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("the artifact card, got {:?}", engine.pending())
    };
    let rock = in_graveyard(&engine, p0, quiet_artifact()).unwrap();
    let land = in_graveyard(&engine, p0, plains()).unwrap();
    assert!(options.contains(&rock), "{options:?}");
    assert!(!options.contains(&land), "an artifact card only: {options:?}");
    aim_at(&mut engine, p0, rock);
    pass_until(&mut engine, stack_is_empty);
    assert!(in_hand(&engine, p0, quiet_artifact()).is_some());
}

/// "{1}, {T}, Sacrifice an artifact: Draw a card."
#[test]
fn sacrificing_an_artifact_draws_a_card() {
    let p0 = PlayerId::new(0);
    let mut engine = table(&[quiet_artifact()], &[]);
    let rock = on_battlefield(&engine, p0, quiet_artifact()).unwrap();
    let hand = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    pay_one_and_press(&mut engine, 3);
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("the artifact to sacrifice, got {:?}", engine.pending())
    };
    assert!(options.contains(&rock), "{options:?}");
    choose(&mut engine, rock);
    pass_until(&mut engine, stack_is_empty);
    assert!(in_graveyard(&engine, p0, quiet_artifact()).is_some());
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand + 1
    );
}
