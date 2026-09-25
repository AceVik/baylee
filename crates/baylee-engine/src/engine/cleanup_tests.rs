//! The cleanup step (CR 514, #287).
//!
//! Its turn-based actions come in the order the rules give them: the active
//! player discards to their maximum hand size (CR 514.1), then damage wears
//! off and "until end of turn" effects end (CR 514.2). Then the game checks
//! once (CR 514.3a): if that check performs a state-based action or finds a
//! triggered ability, the active player gets priority, and when the stack is
//! empty and everyone has passed another cleanup step begins. If it finds
//! nothing, nobody gets priority and the turn ends (CR 704.3).

use super::testkit::{
    Duel, RegistryLookup, basic_forest, card_index, cast_with_floating, in_graveyard,
    keep_mulligans, on_battlefield, pass_until, pt, stack_is_empty, tap_all_mana, walk_to_own_main,
};
use super::*;
use baylee_core::ids::CardIndex;
use baylee_core::types::TypeSet;

fn seat(n: u8) -> PlayerId {
    PlayerId::new(n)
}

fn swamp() -> CardIndex {
    card_index("56719f6a-1a6c-4c0a-8d21-18f7d7350b68")
}

/// `{B}` 1/1 "When this creature dies, target creature gets -1/-1 until end
/// of turn."
fn festering_goblin() -> CardIndex {
    card_index("66fb4764-d309-4c30-a2a4-474f9030dc87")
}

/// `{G}` "Target creature gets +3/+3 until end of turn."
fn giant_growth() -> CardIndex {
    card_index("5748ebf1-24e3-499d-ab7c-c2cebd462a24")
}

/// `{2}{B}{B}` "All creatures get -1/-1": the effect that outlasts the pump.
fn night_of_souls_betrayal() -> CardIndex {
    card_index("916bd025-c44f-49c9-8d76-4b7b2f9a8ba3")
}

/// `{1}{G}` 1/3 reach, and nothing else.
fn canopy_spider() -> CardIndex {
    card_index("37f3733e-cc4e-4d84-b29b-d474f6e254a2")
}

/// "{T}: Add {C}." / "{1}: This land becomes a 2/2 creature with all
/// creature types until end of turn. It's still a land."
fn mutavault() -> CardIndex {
    card_index("6b3cc59a-7ea5-4eb5-9bf9-5a9c07f80e2b")
}

/// `{1}` "Equipped creature gets +1/+1." / "Equip {1}".
fn leonin_scimitar() -> CardIndex {
    card_index("cde26d69-f3e7-4dd0-a53b-cd0ec812d717")
}

/// Answers a target question on the table with `target`.
fn target(engine: &mut Engine<RegistryLookup>, player: PlayerId, target: ObjectId) {
    assert!(
        matches!(engine.pending(), Pending::ChooseTargets { player: asked, options, .. }
            if *asked == player && options.contains(&target)),
        "{:?}",
        engine.pending()
    );
    engine
        .apply(
            player,
            PlayerAction::ChooseObjects {
                objects: vec![target],
            },
        )
        .unwrap();
}

/// Activates `source`'s ability number `index`, as offered.
fn activate(engine: &mut Engine<RegistryLookup>, player: PlayerId, source: ObjectId, index: u32) {
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(source, index)),
        "{:?}",
        legal.abilities
    );
    engine
        .apply(
            player,
            PlayerAction::ActivateAbility {
                source,
                ability_index: index,
            },
        )
        .unwrap();
}

/// Where the turn is: its number and step.
fn now(engine: &Engine<RegistryLookup>) -> (u32, Step) {
    (engine.state().turn.number, engine.state().turn.step)
}

/// The turn and step in which `object` was put into a graveyard, read off
/// the journal.
fn died_during(engine: &Engine<RegistryLookup>, object: ObjectId) -> Option<(u32, Step)> {
    let mut turn = 0;
    let mut step = None;
    for entry in engine.state().journal.entries() {
        match entry.event {
            GameEvent::TurnStarted { number, .. } => turn = number,
            GameEvent::StepChanged { step: s, .. } => step = Some(s),
            GameEvent::ZoneChanged {
                object: moved,
                to: Zone::Graveyard,
                ..
            } if moved == object => return step.map(|s| (turn, s)),
            _ => {}
        }
    }
    None
}

/// How many cleanup steps turn `number` had.
fn cleanup_steps_in(engine: &Engine<RegistryLookup>, number: u32) -> usize {
    let mut turn = 0;
    engine
        .state()
        .journal
        .entries()
        .iter()
        .filter(|entry| {
            if let GameEvent::TurnStarted { number, .. } = entry.event {
                turn = number;
            }
            turn == number
                && matches!(
                    entry.event,
                    GameEvent::StepChanged {
                        step: Step::Cleanup,
                        ..
                    }
                )
        })
        .count()
}

/// Passes priority until turn `number` is over.
fn pass_out_of_turn(engine: &mut Engine<RegistryLookup>, number: u32) {
    pass_until(engine, |e| {
        e.state().turn.number > number && matches!(e.pending(), Pending::Priority { .. })
    });
}

/// Seat 0's Festering Goblin, pumped by Giant Growth and then shrunk by its
/// own Night of Souls' Betrayal: a 3/3 that the cleanup step leaves at 0/0.
/// Seat 1 has two Canopy Spiders (0/2 under the Night) for its trigger to
/// point at. `extra` goes into seat 0's hand beside the two spells.
fn goblin_board(extra: &[CardIndex]) -> (Engine<RegistryLookup>, ObjectId, [ObjectId; 2]) {
    let mut hand = vec![giant_growth(), night_of_souls_betrayal()];
    hand.extend_from_slice(extra);
    let mut engine = Duel::table(287, basic_forest(), 2)
        .battlefield(
            0,
            &[
                festering_goblin(),
                basic_forest(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
            ],
        )
        .hand(0, &hand)
        .battlefield(1, &[canopy_spider(), canopy_spider()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, seat(0)));
    let goblin = on_battlefield(&engine, seat(0), festering_goblin()).expect("seat 0's Goblin");
    let spiders: Vec<ObjectId> = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == canopy_spider()))
        })
        .collect();

    tap_all_mana(&mut engine, seat(0));
    cast_with_floating(&mut engine, seat(0), giant_growth());
    target(&mut engine, seat(0), goblin);
    pass_until(&mut engine, stack_is_empty);
    cast_with_floating(&mut engine, seat(0), night_of_souls_betrayal());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, goblin), (3, 3), "+3/+3, then -1/-1");
    assert_eq!(pt(&engine, spiders[0]), (0, 2));
    (engine, goblin, [spiders[0], spiders[1]])
}

/// The issue's three points, played. The cleanup step ends Giant Growth and
/// leaves the Goblin at 0/0: it dies in that cleanup step, not after the
/// next turn has begun. Its trigger goes on the stack and the active player
/// gets priority with it there. It resolves in the same turn, and when
/// everyone passes on the empty stack another cleanup step begins, whose
/// CR 514.2 ends the -1/-1 the trigger made. That second step finds
/// nothing, so it is the last.
#[test]
fn a_creature_the_cleanup_leaves_at_zero_toughness_dies_that_turn_and_its_trigger_resolves() {
    let (mut engine, goblin, [pointed_at, other]) = goblin_board(&[]);
    let turn = engine.state().turn.number;

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. }) || e.state().turn.number > turn
    });
    assert_eq!(
        now(&engine),
        (turn, Step::Cleanup),
        "{:?}",
        engine.pending()
    );
    assert_eq!(
        died_during(&engine, goblin),
        Some((turn, Step::Cleanup)),
        "put into its graveyard by the cleanup step's check (CR 704.5f)"
    );
    assert!(in_graveyard(&engine, seat(0), festering_goblin()).is_some());

    target(&mut engine, seat(0), pointed_at);
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == seat(0)),
        "the active player gets priority with the trigger on the stack (CR 514.3a): {:?}",
        engine.pending()
    );
    assert_eq!(now(&engine), (turn, Step::Cleanup));
    assert!(!stack_is_empty(&engine));

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        now(&engine),
        (turn, Step::Cleanup),
        "resolved in the same turn"
    );
    assert_eq!(
        (pt(&engine, pointed_at), pt(&engine, other)),
        ((-1, 1), (0, 2))
    );

    pass_out_of_turn(&mut engine, turn);
    assert_eq!(
        cleanup_steps_in(&engine, turn),
        2,
        "everyone passed on an empty stack, so another cleanup step began"
    );
    assert_eq!(
        pt(&engine, pointed_at),
        (0, 2),
        "the second cleanup step ended the -1/-1 made in the first (CR 514.2)"
    );
    assert_eq!(engine.cleanup, Cleanup::Due, "no cleanup step is under way");
}

/// CR 514.1 before CR 514.2: while the active player is asked to discard,
/// Giant Growth still applies and the Goblin is a 3/3 on the battlefield.
/// Only after the discard does the pump end, and the check that follows
/// puts the Goblin into the graveyard in the same turn.
#[test]
fn the_discard_to_hand_size_comes_before_the_turns_effects_end() {
    let forests = [basic_forest(); 8];
    let (mut engine, goblin, _) = goblin_board(&forests);
    let turn = engine.state().turn.number;
    // Eight Forests, and a ninth card if seat 0 drew this turn.
    let over = engine.state().zones.list(ZoneLocation::Hand(seat(0))).len() - 7;
    assert!(over >= 1);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::DiscardChoice { .. }) || e.state().turn.number > turn
    });
    assert!(
        matches!(engine.pending(), Pending::DiscardChoice { player, count }
            if *player == seat(0) && usize::from(*count) == over),
        "{:?}",
        engine.pending()
    );
    assert_eq!(now(&engine), (turn, Step::Cleanup));
    assert!(
        engine
            .state()
            .effects
            .iter()
            .any(|fx| fx.duration == baylee_cards_dsl::Duration::UntilEndOfTurn),
        "Giant Growth has not ended while the discard is asked for"
    );
    // Read through a fresh projection: the cached one would still say 3/3
    // after the pump had ended.
    engine
        .dev_state_mut(seat(0))
        .unwrap()
        .refresh_characteristics();
    assert_eq!(pt(&engine, goblin), (3, 3));

    let cards = engine.state().zones.list(ZoneLocation::Hand(seat(0)))[..over].to_vec();
    engine
        .apply(seat(0), PlayerAction::ChooseObjects { objects: cards })
        .unwrap();
    assert!(
        matches!(engine.pending(), Pending::ChooseTargets { player, .. } if *player == seat(0)),
        "the Goblin's trigger, from the check after the discard: {:?}",
        engine.pending()
    );
    assert_eq!(died_during(&engine, goblin), Some((turn, Step::Cleanup)));
}

/// A state-based action that moves nothing opens the window too. Mutavault
/// animated and wearing Leonin Scimitar stops being a creature at CR 514.2,
/// and the Scimitar comes off it (CR 704.5n). That is the check performing a
/// state-based action, so the active player gets priority on an empty stack.
/// Nothing is journaled for it, so the step can't be read off the journal.
/// When everyone passes, a second cleanup step ends the turn.
#[test]
fn an_equipment_falling_off_at_cleanup_gives_the_active_player_priority() {
    let mut engine = Duel::table(287, basic_forest(), 2)
        .battlefield(
            0,
            &[mutavault(), basic_forest(), basic_forest(), basic_forest()],
        )
        .hand(0, &[leonin_scimitar()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, seat(0)));
    let vault = on_battlefield(&engine, seat(0), mutavault()).expect("seat 0's Mutavault");

    tap_all_mana(&mut engine, seat(0));
    activate(&mut engine, seat(0), vault, 1);
    pass_until(&mut engine, stack_is_empty);
    cast_with_floating(&mut engine, seat(0), leonin_scimitar());
    pass_until(&mut engine, stack_is_empty);
    let scimitar = on_battlefield(&engine, seat(0), leonin_scimitar()).expect("the Scimitar");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("{:?}", engine.pending())
    };
    let (_, equip) = legal
        .abilities
        .iter()
        .copied()
        .find(|(source, _)| *source == scimitar)
        .expect("Equip {1} is the Scimitar's one activated ability");
    activate(&mut engine, seat(0), scimitar, equip);
    target(&mut engine, seat(0), vault);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(scimitar).and_then(|o| o.attached_to),
        Some(vault)
    );
    assert_eq!(pt(&engine, vault), (3, 3));
    let turn = engine.state().turn.number;

    pass_until(&mut engine, |e| {
        e.state().turn.step == Step::Cleanup || e.state().turn.number > turn
    });
    assert_eq!(now(&engine), (turn, Step::Cleanup));
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == seat(0)),
        "{:?}",
        engine.pending()
    );
    assert!(
        stack_is_empty(&engine),
        "no trigger: the window is the SBA's"
    );
    assert!(
        !engine
            .state()
            .object(vault)
            .is_some_and(|o| o.characteristics().types.contains(TypeSet::CREATURE))
    );
    assert_eq!(
        engine.state().object(scimitar).and_then(|o| o.attached_to),
        None
    );

    // The check and the window close differently, so a replay has to tell
    // them apart.
    let open = engine.snapshot_hash();
    engine.cleanup = Cleanup::Checking;
    assert_ne!(engine.snapshot_hash(), open);
    engine.cleanup = Cleanup::Open;

    pass_out_of_turn(&mut engine, turn);
    assert_eq!(cleanup_steps_in(&engine, turn), 2);
}
