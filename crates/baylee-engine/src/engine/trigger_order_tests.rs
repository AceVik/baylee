//! CR 603.3b: the order a player's triggers go on the stack in, played with
//! real cards under `HouseRules::order_triggers`.
//!
//! The case that started it: Dingus Egg under Armageddon, where every land
//! that dies is a trigger about its controller, and the order decides who
//! reaches zero life first.

use super::testkit::*;
use super::*;
use crate::choice::{ArrangePlace, ArrangePrompt, PriorityHold, default_arrangement};
use crate::zone::ZoneLocation;
use baylee_core::ids::{CardIndex, ObjectId};

const SEED: u64 = 603;

fn dingus_egg() -> CardIndex {
    card_index("1973f1e9-aa14-49dc-bafe-be17b30ba288")
}

fn armageddon() -> CardIndex {
    card_index("c9ed8b01-959a-47d6-891e-0abbdccf6e4f")
}

fn stone_rain() -> CardIndex {
    card_index("6e880df6-fc00-43d2-a9c8-f575f40b78c6")
}

fn acid_rain() -> CardIndex {
    card_index("6ae3c3a2-bd17-4fea-ae27-9d2d1eaf8cc0")
}

fn soul_warden() -> CardIndex {
    card_index("f3fad295-1af2-4ecc-8546-b121ad6be27b")
}

fn souls_attendant() -> CardIndex {
    card_index("045a9d3d-c20d-427f-a77b-0bffc6f55526")
}

fn flametongue_kavu() -> CardIndex {
    card_index("b215196a-3716-451f-bdbd-d16c05ace5b4")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn blood_artist() -> CardIndex {
    card_index("310f141c-7f37-4729-aed6-dd9c09db448d")
}

fn wrath_of_god() -> CardIndex {
    card_index("34515b16-c9a4-4f98-8c77-416a7a523407")
}

const fn seat(n: u8) -> PlayerId {
    PlayerId::new(n)
}

/// A game at seat 0's first main phase, under the rule.
fn with_the_rule(table: Duel) -> Engine<RegistryLookup> {
    let mut engine = without_the_rule(table);
    engine.house_rules.order_triggers = true;
    engine
}

/// The same game as it was played before there was a question.
fn without_the_rule(table: Duel) -> Engine<RegistryLookup> {
    let mut engine = table.start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, seat(0));
    engine
}

/// Seat 0 with five Plains and a Dingus Egg, seat 1 with a Forest, and
/// Armageddon to cast.
fn egg_table() -> Duel {
    let plains = plains();
    Duel::new(SEED, plains)
        .battlefield(0, &[dingus_egg(), plains, plains, plains, plains, plains])
        .battlefield(1, &[basic_forest()])
        .hand(0, &[armageddon()])
}

fn plains() -> CardIndex {
    card_index("bc71ebf6-2056-41f7-be35-b2e5c34afa99")
}

fn mountain() -> CardIndex {
    card_index("a3fb7228-e76b-4e96-a40e-20b5fed75685")
}

fn island() -> CardIndex {
    card_index("b2c6aa39-2d2a-459c-a555-fb48ba993373")
}

fn is_order_question(engine: &Engine<RegistryLookup>) -> bool {
    matches!(
        engine.pending(),
        Pending::Arrange {
            prompt: ArrangePrompt::TriggerOrder,
            ..
        }
    )
}

/// Passes until the order question or the stack's triggers, whichever is
/// first, so a test that expects none sees what came instead.
fn cast_and_resolve(engine: &mut Engine<RegistryLookup>, spell: CardIndex) {
    cast_from_hand(engine, seat(0), spell);
    let before = engine.state().zones.list(ZoneLocation::Stack).len();
    pass_until(engine, |e| {
        !matches!(e.pending(), Pending::Priority { .. })
            || e.state().zones.list(ZoneLocation::Stack).len() != before
    });
}

/// The stack from its top, as the player the top resolves against would
/// read it: whose land each Egg trigger is about.
fn stack_about(engine: &Engine<RegistryLookup>) -> Vec<PlayerId> {
    engine
        .state()
        .zones
        .list(ZoneLocation::Stack)
        .iter()
        .rev()
        .filter_map(|id| {
            engine
                .state()
                .object(*id)?
                .riders
                .iter()
                .find_map(|r| match r {
                    crate::object::Rider::EventDeparture(controller, _) => Some(*controller),
                    _ => None,
                })
        })
        .collect()
}

fn life(engine: &Engine<RegistryLookup>, player: PlayerId) -> i32 {
    engine.state().players[usize::from(player.get())].life
}

/// The handle of the first offered trigger about `player`.
fn about(engine: &Engine<RegistryLookup>, player: PlayerId) -> ObjectId {
    engine
        .queued_triggers()
        .iter()
        .find(|t| t.about_player == Some(player))
        .map(|t| t.handle)
        .expect("a trigger about that player is offered")
}

/// `first` on top of the stack and the rest as they were offered.
fn top_first(engine: &Engine<RegistryLookup>, first: ObjectId) -> PlayerAction {
    let Pending::Arrange { cards, .. } = engine.pending() else {
        panic!("no arrangement asked: {:?}", engine.pending())
    };
    let mut pile = vec![first];
    pile.extend(cards.iter().copied().filter(|id| *id != first));
    PlayerAction::Arrange { piles: vec![pile] }
}

fn answer_with_the_offer(engine: &mut Engine<RegistryLookup>, player: PlayerId) {
    let answer = crate::choice::timeout_answer(engine.pending()).expect("an answer with no wish");
    engine
        .apply(player, answer)
        .expect("the offer is an answer");
}

/// Eighteen Dingus Egg triggers lost a real game for want of this: the
/// player asked resolves the one about their opponent first.
#[test]
fn the_first_of_the_answer_resolves_first() {
    let mut engine = with_the_rule(egg_table().life(1, 2));
    let start = life(&engine, seat(0));
    cast_and_resolve(&mut engine, armageddon());
    assert!(is_order_question(&engine), "{:?}", engine.pending());
    let Pending::Arrange {
        player,
        piles,
        cards,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("the question was just matched")
    };
    assert_eq!(player, seat(0), "the Egg's controller is asked");
    assert_eq!(cards.len(), 6, "one trigger per land");
    assert_eq!(piles.len(), 1);
    assert_eq!(piles[0].place, ArrangePlace::Stack);
    assert!(
        stack_is_empty(&engine),
        "nothing is on the stack before the answer"
    );

    let opponent_first = top_first(&engine, about(&engine, seat(1)));
    engine.apply(seat(0), opponent_first).expect("an order");
    assert_eq!(stack_about(&engine)[0], seat(1), "on top, as asked");
    assert_eq!(engine.state().zones.list(ZoneLocation::Stack).len(), 6);

    pass_until(&mut engine, |e| matches!(e.pending(), Pending::GameOver(_)));
    assert_eq!(
        life(&engine, seat(0)),
        start,
        "seat 1 was dead before any other Egg"
    );
}

/// With no wish the answer is the order a game without the question plays.
#[test]
fn the_order_offered_is_the_order_the_timestamps_give() {
    let plain = {
        let mut engine = without_the_rule(egg_table());
        cast_and_resolve(&mut engine, armageddon());
        stack_about(&engine)
    };
    let mut engine = with_the_rule(egg_table());
    cast_and_resolve(&mut engine, armageddon());
    answer_with_the_offer(&mut engine, seat(0));
    assert_eq!(stack_about(&engine), plain);
    assert_eq!(plain.len(), 6);
}

/// The rule off: no question, in a game where it would have been asked.
#[test]
fn a_game_without_the_rule_asks_nothing() {
    let mut engine = without_the_rule(egg_table());
    cast_and_resolve(&mut engine, armageddon());
    assert!(!is_order_question(&engine), "{:?}", engine.pending());
    assert_eq!(engine.state().zones.list(ZoneLocation::Stack).len(), 6);
    assert!(engine.queued_triggers().is_empty());
}

/// Nothing to order: one trigger.
#[test]
fn one_trigger_asks_nothing() {
    let mut engine = with_the_rule(
        Duel::new(SEED, plains())
            .battlefield(0, &[dingus_egg(), mountain(), mountain(), mountain()])
            .battlefield(1, &[basic_forest()])
            .hand(0, &[stone_rain()]),
    );
    let forest = on_battlefield(&engine, seat(1), basic_forest()).expect("the Forest");
    tap_all_mana(&mut engine, seat(0));
    let spell = in_hand(&engine, seat(0), stone_rain()).expect("the Rain");
    engine
        .apply(seat(0), PlayerAction::CastSpell { card: spell })
        .expect("cast");
    engine
        .apply(
            seat(0),
            PlayerAction::ChooseTargets {
                objects: vec![forest],
                players: vec![],
            },
        )
        .expect("the Forest");
    pass_until(&mut engine, |e| {
        !matches!(e.pending(), Pending::Priority { .. }) || stack_about(e).len() == 1
    });
    assert_eq!(stack_about(&engine), vec![seat(1)]);
    assert!(!is_order_question(&engine));
}

/// Two Eggs and one land are two triggers nobody could tell apart.
#[test]
fn identical_triggers_from_twin_sources_ask_nothing() {
    let mut engine = with_the_rule(
        Duel::new(SEED, plains())
            .battlefield(
                0,
                &[
                    dingus_egg(),
                    dingus_egg(),
                    mountain(),
                    mountain(),
                    mountain(),
                ],
            )
            .battlefield(1, &[basic_forest()])
            .hand(0, &[stone_rain()]),
    );
    let forest = on_battlefield(&engine, seat(1), basic_forest()).expect("the Forest");
    tap_all_mana(&mut engine, seat(0));
    let spell = in_hand(&engine, seat(0), stone_rain()).expect("the Rain");
    engine
        .apply(seat(0), PlayerAction::CastSpell { card: spell })
        .expect("cast");
    engine
        .apply(
            seat(0),
            PlayerAction::ChooseTargets {
                objects: vec![forest],
                players: vec![],
            },
        )
        .expect("the Forest");
    pass_until(&mut engine, |e| {
        !matches!(e.pending(), Pending::Priority { .. }) || stack_about(e).len() == 2
    });
    assert_eq!(stack_about(&engine), vec![seat(1), seat(1)]);
    assert!(!is_order_question(&engine));
}

/// Two lands of one player are one thing to "that land's controller".
#[test]
fn triggers_about_one_player_ask_nothing_whichever_land_died() {
    let mut engine = with_the_rule(
        Duel::new(SEED, plains())
            .battlefield(0, &[dingus_egg(), island(), island(), island(), island()])
            .battlefield(1, &[basic_forest(), basic_forest(), basic_forest()])
            .hand(0, &[acid_rain()]),
    );
    cast_and_resolve(&mut engine, acid_rain());
    assert!(!is_order_question(&engine), "{:?}", engine.pending());
    assert_eq!(stack_about(&engine), vec![seat(1); 3]);
}

/// An ability that reads nothing of its event is the same ability whatever
/// died: a board wipe is a question about each target, and not about order.
#[test]
fn triggers_that_read_nothing_of_their_event_ask_nothing() {
    let mut engine = with_the_rule(
        Duel::new(SEED, plains())
            .battlefield(
                0,
                &[
                    blood_artist(),
                    grizzly_bears(),
                    plains(),
                    plains(),
                    plains(),
                    plains(),
                ],
            )
            .battlefield(1, &[grizzly_bears()])
            .hand(0, &[wrath_of_god()]),
    );
    cast_and_resolve(&mut engine, wrath_of_god());
    assert!(
        matches!(engine.pending(), Pending::ChooseTargets { .. }),
        "three triggers, each asking its own target: {:?}",
        engine.pending()
    );
}

/// CR 101.4, 603.3b: the active player's go on first, so they resolve last,
/// and each player is asked about their own, in that order.
#[test]
fn each_player_orders_their_own_in_apnap_order() {
    let mut engine = with_the_rule(
        Duel::new(SEED, plains())
            .battlefield(0, &[dingus_egg(), plains(), plains(), plains(), plains()])
            .battlefield(1, &[dingus_egg(), basic_forest()])
            .hand(0, &[armageddon()]),
    );
    cast_and_resolve(&mut engine, armageddon());
    assert!(is_order_question(&engine), "{:?}", engine.pending());
    assert_eq!(
        engine.pending().asked(),
        Some(seat(0)),
        "the active player first"
    );
    assert!(stack_is_empty(&engine));
    answer_with_the_offer(&mut engine, seat(0));

    assert!(is_order_question(&engine), "{:?}", engine.pending());
    assert_eq!(engine.pending().asked(), Some(seat(1)));
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Stack).len(),
        5,
        "the active player's are on the stack as the next is asked"
    );
    let first = about(&engine, seat(1));
    let answer = top_first(&engine, first);
    engine.apply(seat(1), answer).expect("an order");
    let stack = stack_about(&engine);
    assert_eq!(stack.len(), 10);
    assert_eq!(stack[0], seat(1), "the last to choose resolves first");
    let controllers: Vec<PlayerId> = engine
        .state()
        .zones
        .list(ZoneLocation::Stack)
        .iter()
        .rev()
        .map(|id| engine.state().object(*id).expect("stack object").controller)
        .collect();
    assert_eq!(&controllers[..5], &[seat(1); 5]);
    assert_eq!(&controllers[5..], &[seat(0); 5]);
}

/// CR 603.3d: targets are chosen as each goes on the stack, so the order is
/// asked first and the target after it, in that order.
#[test]
fn the_order_comes_before_the_targets() {
    let mut engine = with_the_rule(
        Duel::new(SEED, plains())
            .battlefield(
                0,
                &[
                    soul_warden(),
                    mountain(),
                    mountain(),
                    mountain(),
                    mountain(),
                    mountain(),
                ],
            )
            .battlefield(1, &[grizzly_bears()])
            .hand(0, &[flametongue_kavu()]),
    );
    let start = life(&engine, seat(0));
    cast_and_resolve(&mut engine, flametongue_kavu());
    assert!(is_order_question(&engine), "{:?}", engine.pending());
    let rows = engine.queued_triggers();
    assert_eq!(rows.len(), 2);
    let aimed = rows.iter().find(|t| t.asks_targets).expect("the Kavu's");
    assert_eq!(rows.iter().filter(|t| t.asks_targets).count(), 1);

    // The Kavu's on top goes on last: the Warden goes on, then the target
    // is asked for the Kavu with the Warden already below it.
    let answer = top_first(&engine, aimed.handle);
    engine.apply(seat(0), answer).expect("an order");
    assert!(
        matches!(engine.pending(), Pending::ChooseTargets { .. }),
        "{:?}",
        engine.pending()
    );
    assert_eq!(engine.state().zones.list(ZoneLocation::Stack).len(), 1);
    let bears = on_battlefield(&engine, seat(1), grizzly_bears()).expect("the Bears");
    engine
        .apply(
            seat(0),
            PlayerAction::ChooseTargets {
                objects: vec![bears],
                players: vec![],
            },
        )
        .expect("a target");
    assert_eq!(engine.state().zones.list(ZoneLocation::Stack).len(), 2);
    pass_until(&mut engine, |e| {
        e.state().zones.list(ZoneLocation::Stack).len() == 1
    });
    assert!(in_graveyard(&engine, seat(1), grizzly_bears()).is_some());
    assert_eq!(
        life(&engine, seat(0)),
        start,
        "the Warden has not resolved yet"
    );
}

/// A "may" and a "must": the order is asked of both, and each is then asked
/// or done where the answer put it.
#[test]
fn optional_and_mandatory_triggers_are_ordered_alike() {
    let table = || {
        Duel::new(SEED, plains())
            .battlefield(
                0,
                &[
                    soul_warden(),
                    souls_attendant(),
                    basic_forest(),
                    basic_forest(),
                ],
            )
            .hand(0, &[grizzly_bears()])
    };
    let mut engine = with_the_rule(table());
    let start = life(&engine, seat(0));
    cast_and_resolve(&mut engine, grizzly_bears());
    assert!(is_order_question(&engine), "{:?}", engine.pending());
    let rows = engine.queued_triggers();
    assert_eq!(rows.len(), 2);
    assert_ne!(rows[0].twin, rows[1].twin, "a may is not a must");

    let attendant = on_battlefield(&engine, seat(0), souls_attendant()).expect("the Attendant");
    let may = rows
        .iter()
        .find(|t| t.loc.source == attendant)
        .expect("the Attendant's");
    let answer = top_first(&engine, may.handle);
    engine.apply(seat(0), answer).expect("an order");
    pass_until(&mut engine, |e| {
        !matches!(e.pending(), Pending::Priority { .. })
    });
    assert!(
        matches!(engine.pending(), Pending::YesNo { .. }),
        "the may resolves first: {:?}",
        engine.pending()
    );
    assert_eq!(
        life(&engine, seat(0)),
        start,
        "and the must is still waiting"
    );
}

/// A hold on priority and standing answers are about priority and yes-or-no
/// questions. Neither answers this one.
#[test]
fn automation_does_not_answer_the_question() {
    let mut engine = with_the_rule(egg_table());
    let turn = engine.state().turn.number;
    cast_from_hand(&mut engine, seat(0), armageddon());
    for player in [seat(1), seat(0)] {
        engine
            .apply(
                player,
                PlayerAction::SetPriorityHold(PriorityHold::UntilEndOfTurn { turn }),
            )
            .expect("a hold");
    }
    pass_until(&mut engine, |e| {
        !matches!(e.pending(), Pending::Priority { .. })
    });
    assert!(is_order_question(&engine), "{:?}", engine.pending());
}

/// The answer's refusals are the arrangement's: the question says all of them.
#[test]
fn an_order_that_loses_a_trigger_is_refused_and_asked_again() {
    let mut engine = with_the_rule(egg_table());
    cast_and_resolve(&mut engine, armageddon());
    let Pending::Arrange { cards, piles, .. } = engine.pending().clone() else {
        panic!("no question: {:?}", engine.pending())
    };
    let answer = default_arrangement(&cards, &piles).expect("an arrangement");
    let mut short = answer.clone();
    short[0].pop();
    assert!(
        engine
            .apply(seat(0), PlayerAction::Arrange { piles: short })
            .is_err()
    );
    let mut twice = answer.clone();
    twice[0][0] = twice[0][1];
    assert!(
        engine
            .apply(seat(0), PlayerAction::Arrange { piles: twice })
            .is_err()
    );
    assert!(is_order_question(&engine));
    assert!(
        engine
            .apply(
                seat(1),
                PlayerAction::Arrange {
                    piles: answer.clone()
                }
            )
            .is_err(),
        "the other player has no say"
    );
    engine
        .apply(seat(0), PlayerAction::Arrange { piles: answer })
        .expect("the whole order");
    assert!(!is_order_question(&engine));
}

/// The asked player leaves: their triggers go with them (CR 800.4d), the
/// next player is asked, and the game goes on.
#[test]
fn the_asked_player_leaving_leaves_no_question_behind() {
    let mut engine = with_the_rule(
        Duel::table(SEED, plains(), 3)
            .battlefield(0, &[dingus_egg(), plains(), plains(), plains(), plains()])
            .battlefield(1, &[dingus_egg(), basic_forest()])
            .battlefield(2, &[basic_forest()])
            .hand(0, &[armageddon()]),
    );
    cast_and_resolve(&mut engine, armageddon());
    assert_eq!(engine.pending().asked(), Some(seat(0)));
    assert!(is_order_question(&engine));
    engine
        .apply(seat(0), PlayerAction::Concede)
        .expect("a concession");
    assert!(
        is_order_question(&engine),
        "seat 1's triggers are next: {:?}",
        engine.pending()
    );
    assert_eq!(engine.pending().asked(), Some(seat(1)));
    assert!(stack_is_empty(&engine), "seat 0's never went on");
    answer_with_the_offer(&mut engine, seat(1));
    assert_eq!(stack_about(&engine).len(), 6);
}

/// Somebody else leaving does not touch the question or its triggers.
#[test]
fn another_player_leaving_leaves_the_question_standing() {
    let mut engine = with_the_rule(
        Duel::table(SEED, plains(), 3)
            .battlefield(0, &[dingus_egg(), plains(), plains(), plains(), plains()])
            .battlefield(1, &[basic_forest()])
            .battlefield(2, &[basic_forest()])
            .hand(0, &[armageddon()]),
    );
    cast_and_resolve(&mut engine, armageddon());
    assert_eq!(engine.pending().asked(), Some(seat(0)));
    let offered = engine.queued_triggers().len();
    engine
        .apply(seat(2), PlayerAction::Concede)
        .expect("a concession");
    assert!(is_order_question(&engine), "{:?}", engine.pending());
    assert_eq!(engine.queued_triggers().len(), offered);
    let first = about(&engine, seat(1));
    let answer = top_first(&engine, first);
    engine.apply(seat(0), answer).expect("an order");
    assert_eq!(stack_about(&engine)[0], seat(1));
}

/// The house rule on loops takes the queue as the answer arrives, and a loop
/// through this question is found at an answer (`Engine::apply`): the answer
/// is taken, orders nothing, and the game goes on.
#[test]
fn an_answer_a_broken_loop_got_to_first_orders_nothing() {
    let mut engine = with_the_rule(egg_table());
    cast_and_resolve(&mut engine, armageddon());
    let answer = crate::choice::timeout_answer(engine.pending()).expect("an answer");
    engine.breaking_loop = true;
    engine.trigger_queue.clear();
    assert!(engine.queued_triggers().is_empty());
    engine.apply(seat(0), answer).expect("the answer stands");
    assert!(!is_order_question(&engine), "{:?}", engine.pending());
    assert!(stack_is_empty(&engine));
}

/// The rule writes nothing into a game that does not have it, and adds a
/// question to one that does, so it has to tell the two apart.
#[test]
fn the_rule_is_in_the_continuation_hash_only_when_on() {
    let plain = without_the_rule(egg_table());
    let ruled = with_the_rule(egg_table());
    assert_eq!(plain.state().snapshot_hash(), ruled.state().snapshot_hash());
    assert_ne!(plain.snapshot_hash(), ruled.snapshot_hash());
}

/// A handle is no object, and names its place back.
#[test]
fn a_queued_trigger_handle_names_its_place_and_no_object() {
    let engine = without_the_rule(egg_table());
    for place in [0, 1, 17, 4096] {
        let handle = ObjectId::queued_trigger(place);
        assert_eq!(handle.queued_trigger_place(), Some(place));
        assert!(engine.state().object(handle).is_none());
    }
    assert_eq!(ObjectId::NO_SOURCE.queued_trigger_place(), None);
    let plains = on_battlefield(&engine, seat(0), plains()).expect("a Plains");
    assert_eq!(plains.queued_trigger_place(), None);
}

/// A token that has ceased to exist (CR 704.5d) is named in the order
/// question by what it last was (CR 113.7a, 111.4): Carbon block 3,
/// attempt 20, q140, two rows read "#T6 · about you" with no name, a
/// Pest's "When this creature dies, you gain 1 life" beside Blood Artist's.
#[test]
fn a_gone_token_s_trigger_is_named_in_the_order_question() {
    let plains = plains();
    let mut engine = with_the_rule(
        Duel::new(SEED, plains)
            .battlefield(0, &[blood_artist(), plains, plains, plains, plains])
            .hand(0, &[wrath_of_god()]),
    );
    crate::resolve::tokens::create_tokens(
        &mut engine.state,
        seat(0),
        &baylee_cards::generated_tokens::PEST_1_1_BLACK_GREEN,
        None,
        1,
    );
    engine.state.invalidate_projections();
    engine.refresh_offer();
    cast_and_resolve(&mut engine, wrath_of_god());
    assert!(is_order_question(&engine), "{:?}", engine.pending());
    let names: Vec<String> = engine
        .queued_triggers()
        .into_iter()
        .map(|row| row.name)
        .collect();
    assert!(names.iter().all(|name| !name.is_empty()), "{names:?}");
    assert!(names.iter().any(|name| name == "Pest"), "{names:?}");
}
