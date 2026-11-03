//! Attack-declaration sacrifice choices: atomic answers, continuation hashes,
//! mana windows, forks, and a player leaving before any cost is paid.

use super::testkit::*;
use super::*;
use crate::engine::{DeckKnowledge, ForkRefusal};
use baylee_core::generated::index;
use baylee_core::ids::{CardIndex, Defender};
use baylee_core::mana::ManaColor;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn game(seats: usize, with_brainwash: bool) -> Engine<RegistryLookup> {
    let mut duel = Duel::table(508_001, index::FOREST, seats)
        .battlefield(
            0,
            &[
                index::GRIZZLY_BEARS,
                index::LLANOWAR_ELVES,
                index::FOREST,
                index::FOREST,
                index::FOREST,
            ],
        )
        .battlefield(1, &[index::FLOODED_WOODLANDS]);
    if with_brainwash {
        duel = duel.hand(0, &[index::BRAINWASH]);
    }
    let mut engine = duel.start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    engine
}

fn all_of(engine: &Engine<RegistryLookup>, player: PlayerId, card: CardIndex) -> Vec<ObjectId> {
    engine
        .state()
        .battlefield_seen()
        .filter(|id| {
            engine.state().object(*id).is_some_and(|object| {
                object.controller == player && object.card.is_some_and(|c| c.index == card)
            })
        })
        .collect()
}

fn creature(engine: &Engine<RegistryLookup>, card: CardIndex) -> ObjectId {
    on_battlefield(engine, P0, card).expect("the creature is on the battlefield")
}

fn declare(engine: &mut Engine<RegistryLookup>, attackers: &[ObjectId]) {
    pass_until(engine, |engine| {
        matches!(
            engine.pending(),
            Pending::ChooseAttackers { player: P0, .. }
        )
    });
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: attackers
                    .iter()
                    .map(|id| (*id, Defender::Player(P1)))
                    .collect(),
            },
        )
        .expect("the declaration can pay its sacrifice choices");
}

fn sacrifice_options(engine: &Engine<RegistryLookup>) -> Vec<ObjectId> {
    let Pending::ChooseCards {
        player: P0,
        options,
        min: 1,
        max: 1,
        prompt: crate::choice::ChoicePrompt::CostSacrifice,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "a single-land sacrifice choice was expected: {:?}",
            engine.pending()
        );
    };
    options
}

fn choose(engine: &mut Engine<RegistryLookup>, objects: Vec<ObjectId>) {
    engine
        .apply(P0, PlayerAction::ChooseObjects { objects })
        .expect("the choice pays the current sacrifice cost");
}

fn zone(engine: &Engine<RegistryLookup>, object: ObjectId) -> Zone {
    engine.state().object(object).expect("object exists").zone
}

fn published(engine: &Engine<RegistryLookup>) -> (u64, usize, String) {
    (
        engine.snapshot_hash(),
        engine.state().journal.len(),
        format!("{:?}", engine.pending()),
    )
}

#[test]
fn invalid_answers_are_atomic_and_the_unpaid_choice_cannot_be_forked() {
    let mut engine = game(2, false);
    let bears = creature(&engine, index::GRIZZLY_BEARS);
    let elves = creature(&engine, index::LLANOWAR_ELVES);
    let forests = all_of(&engine, P0, index::FOREST);
    declare(&mut engine, &[bears, elves]);
    let before = published(&engine);

    for objects in [Vec::new(), vec![forests[0], forests[1]]] {
        assert!(
            engine
                .apply(P0, PlayerAction::ChooseObjects { objects })
                .is_err(),
            "only exactly one offered land answers each cost"
        );
        assert_eq!(published(&engine), before);
    }
    assert_eq!(zone(&engine, forests[0]), Zone::Battlefield);
    assert!(!engine.state().combat.is_attacking(bears));
    assert!(!engine.state().combat.is_attacking(elves));
    assert_eq!(
        engine.fork_for(P0, DeckKnowledge::Known, 1).err(),
        Some(ForkRefusal::Continuation("attack_sacrifice")),
        "a simulation cannot skip an unfinished declaration"
    );
}

#[test]
fn the_snapshot_distinguishes_choices_while_the_board_is_still_identical() {
    let mut first = game(2, false);
    let mut second = game(2, false);
    for engine in [&mut first, &mut second] {
        let bears = creature(engine, index::GRIZZLY_BEARS);
        let elves = creature(engine, index::LLANOWAR_ELVES);
        declare(engine, &[bears, elves]);
    }
    let options = sacrifice_options(&first);
    choose(&mut first, vec![options[0]]);
    choose(&mut second, vec![options[1]]);

    assert_eq!(
        first.state().snapshot_hash(),
        second.state().snapshot_hash(),
        "choices alone have not moved a permanent"
    );
    assert_ne!(
        first.snapshot_hash(),
        second.snapshot_hash(),
        "the suspended declaration records which payment was chosen"
    );
    assert!(!sacrifice_options(&first).contains(&options[0]));
    assert!(!sacrifice_options(&second).contains(&options[1]));
}

#[test]
fn chosen_sacrifices_survive_a_mana_window_and_a_short_payment_changes_nothing() {
    let mut engine = game(2, true);
    let bears = creature(&engine, index::GRIZZLY_BEARS);
    let forests = all_of(&engine, P0, index::FOREST);
    engine.state.players[0].mana_pool.add(ManaColor::White, 1);
    engine.refresh_offer();
    cast_with_floating(&mut engine, P0, index::BRAINWASH);
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![bears],
                players: Vec::new(),
            },
        )
        .expect("Brainwash enchants the Bears");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);

    declare(&mut engine, &[bears]);
    let chosen = forests[1];
    choose(&mut engine, vec![chosen]);
    assert!(
        engine.payment_window().is_some(),
        "Brainwash's {{3}} is owed"
    );
    assert_eq!(zone(&engine, chosen), Zone::Battlefield);
    assert!(!engine.state().combat.is_attacking(bears));
    let before = published(&engine);

    assert!(
        engine
            .apply(P0, PlayerAction::PayWindow { steps: Vec::new() })
            .is_err(),
        "an empty pool cannot pay Brainwash"
    );
    assert_eq!(published(&engine), before);
    assert_eq!(zone(&engine, chosen), Zone::Battlefield);

    engine
        .apply(
            P0,
            PlayerAction::PayWindow {
                steps: forests
                    .iter()
                    .map(|source| PlayerAction::ActivateManaAbility { source: *source })
                    .collect(),
            },
        )
        .expect("the three Forests pay Brainwash after the land was chosen");
    assert_eq!(zone(&engine, chosen), Zone::Graveyard);
    assert_eq!(zone(&engine, forests[0]), Zone::Battlefield);
    assert_eq!(zone(&engine, forests[2]), Zone::Battlefield);
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    assert!(engine.state().combat.is_attacking(bears));
}

#[test]
fn the_choosers_concession_discards_the_unpaid_declaration() {
    let mut engine = game(3, false);
    let bears = creature(&engine, index::GRIZZLY_BEARS);
    let elves = creature(&engine, index::LLANOWAR_ELVES);
    declare(&mut engine, &[bears, elves]);
    let forests = sacrifice_options(&engine);
    choose(&mut engine, vec![forests[0]]);
    let mark = engine.state().journal.len();
    assert_eq!(zone(&engine, forests[0]), Zone::Battlefield);

    engine.apply(P0, PlayerAction::Concede).unwrap();

    assert!(engine.attack_sacrifice.is_none());
    assert!(!matches!(engine.pending(), Pending::GameOver(_)));
    assert!(engine.state().players[0].has_lost());
    assert!(
        !engine.state().journal.entries()[mark..]
            .iter()
            .any(|entry| matches!(
                entry.event,
                GameEvent::ZoneChanged {
                    from: Zone::Battlefield,
                    to: Zone::Graveyard,
                    cause: Cause::TurnBased,
                    ..
                } | GameEvent::BecameAttacker { .. }
            )),
        "neither a stored sacrifice nor an attacker is committed on departure"
    );
}
