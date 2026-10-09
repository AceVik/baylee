//! `cards/enchantments/mv_2/island_sanctuary.rs`, played.
//!
//! Island Sanctuary: "If you would draw a card during your draw step,
//! instead you may skip that draw. If you do, until your next turn, you
//! can't be attacked except by creatures with flying and/or islandwalk."
//!
//! A skip is a replacement effect (CR 614.10) asked at each draw its
//! controller would make in their own draw step (CR 504.1, 121.2); the
//! restriction is one on declaring attackers (CR 508.1c) and outlives the
//! card (its 2004-10-04 ruling). The mechanism has its rule tests in
//! `engine/draw_skip_tests.rs`; these play the card.
//!
//! `pass_until` answers every "you may" with yes, which would leave the "no"
//! path unplayed, so every test here drives the skip question itself:
//! `play_until` answers each one from a script and fails on a question
//! nobody scripted.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::turn::{Phase, Step};
use baylee_cards_dsl::Modifier;
use std::collections::VecDeque;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn island_sanctuary() -> CardIndex {
    card_index("7d1769d0-d942-45b3-a31c-2bbe45e68661")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn serra_angel() -> CardIndex {
    card_index("4b7ac066-e5c7-43e6-9e7e-2739b24a905d")
}

fn lord_of_atlantis() -> CardIndex {
    card_index("cc7f290f-ca00-4285-9bdb-4b4402444f30")
}

fn howling_mine() -> CardIndex {
    card_index("d26b27db-a567-4631-b4b6-7294222fbdd1")
}

fn ancestral_recall() -> CardIndex {
    card_index("550c74d4-1fcb-406a-b02a-639a760a4380")
}

/// One skip question as it was asked: the turn, the step, and whether
/// something was resolving (a draw inside a resolution) or not.
#[derive(Debug, PartialEq, Eq)]
struct Asked {
    turn: u32,
    step: Step,
    resolving: bool,
}

fn script(answers: &[bool]) -> VecDeque<bool> {
    answers.iter().copied().collect()
}

fn hand(engine: &Engine<RegistryLookup>, seat: PlayerId) -> usize {
    engine.state().zones.list(ZoneLocation::Hand(seat)).len()
}

/// The skip question: a "you may" of the Sanctuary's own, put to p0.
fn skip_offered(engine: &Engine<RegistryLookup>) -> bool {
    matches!(
        engine.pending(),
        Pending::YesNo {
            player,
            prompt: YesNoPrompt::MayDo,
            source: Some(source),
        } if *player == P0 && source.card == island_sanctuary()
    )
}

/// p0's "can't be attacked except by" is in force.
fn restricted(engine: &Engine<RegistryLookup>) -> bool {
    engine.state().effects.iter().any(|fx| {
        fx.controller == P0 && matches!(fx.modifier, Modifier::CantBeAttackedExceptBy { .. })
    })
}

/// Plays on, answering each skip question from `answers` in order and
/// passing everything else (no attackers, no blockers), until `stop` holds.
/// `stop` is tried first, so a stop on the question itself leaves it
/// unanswered. Returns the questions asked on the way.
#[track_caller]
fn play_until(
    engine: &mut Engine<RegistryLookup>,
    answers: &mut VecDeque<bool>,
    stop: impl Fn(&Engine<RegistryLookup>) -> bool,
) -> Vec<Asked> {
    let mut asked = Vec::new();
    for _ in 0..300 {
        if stop(engine) {
            return asked;
        }
        if skip_offered(engine) {
            asked.push(Asked {
                turn: engine.state().turn.number,
                step: engine.state().turn.step,
                resolving: !stack_is_empty(engine),
            });
            let answer = answers.pop_front().expect("a question nobody scripted");
            engine.apply(P0, PlayerAction::YesNo(answer)).unwrap();
            continue;
        }
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseAttackers { player, .. } => engine
                .apply(player, PlayerAction::DeclareAttackers { attackers: vec![] })
                .unwrap(),
            Pending::ChooseBlockers { player, .. } => engine
                .apply(player, PlayerAction::DeclareBlockers { blockers: vec![] })
                .unwrap(),
            other => panic!("unexpected question {other:?}"),
        }
    }
    panic!("never reached the stop");
}

fn first_main_of(seat: PlayerId, turn: u32) -> impl Fn(&Engine<RegistryLookup>) -> bool {
    move |e| {
        e.state().turn.number == turn
            && e.state().turn.active == seat
            && matches!(e.state().turn.phase, Phase::FirstMain)
    }
}

/// p0's draw step with the turn-based draw done and the stack empty: the
/// moment a spell can be cast in the step.
fn p0_draw_step_priority(e: &Engine<RegistryLookup>) -> bool {
    e.state().turn.active == P0
        && e.state().turn.step == Step::Draw
        && stack_is_empty(e)
        && matches!(e.pending(), Pending::Priority { player, .. } if *player == P0)
}

/// A table with the Sanctuary for p0 and `p0` (more of p0's permanents),
/// `p1` (p1's), started and past the mulligans. The player on the play has
/// no turn-1 draw step (CR 103.8a), so p0's first draw step is turn 3.
fn table(seed: u64, p0: &[CardIndex], p1: &[CardIndex]) -> Engine<RegistryLookup> {
    let mut mine = vec![island_sanctuary()];
    mine.extend_from_slice(p0);
    let mut engine = Duel::new(seed, plains())
        .battlefield(0, &mine)
        .battlefield(1, p1)
        .start();
    keep_mulligans(&mut engine);
    engine
}

/// p1's creatures: a Bears (neither ability), a Serra Angel (flying) and two
/// Lords of Atlantis, which give each other islandwalk.
fn p1_attackers() -> [CardIndex; 4] {
    [
        grizzly_bears(),
        serra_angel(),
        lord_of_atlantis(),
        lord_of_atlantis(),
    ]
}

fn p1_creatures(engine: &Engine<RegistryLookup>, card: CardIndex) -> Vec<ObjectId> {
    engine
        .state()
        .battlefield_view()
        .into_iter()
        .filter(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.controller == P1 && o.card.map(|c| c.index) == Some(card))
        })
        .collect()
}

/// Stops on p1's declare-attackers choice of a turn after p0's first draw
/// step (turn 2's, before any question, is not it).
fn p1_declares_attackers(e: &Engine<RegistryLookup>) -> bool {
    e.state().turn.number >= 3
        && matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == P1)
}

/// The creatures p1 is offered as attackers; the engine is at the choice.
fn offered_attackers(engine: &Engine<RegistryLookup>) -> Vec<ObjectId> {
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        panic!("not at p1's attack: {:?}", engine.pending())
    };
    attackers
}

/// Declares one attacker at p0.
fn attack_p0_with(
    engine: &mut Engine<RegistryLookup>,
    attacker: ObjectId,
) -> Result<(), crate::engine::EngineError> {
    engine
        .apply(
            P1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(attacker, Defender::Player(P0))],
            },
        )
        .map(drop)
}

// ---------------------------------------------------------------------------
// 1. The turn-based draw
// ---------------------------------------------------------------------------

/// Yes: the turn-based draw is not made and the restriction is. It is the
/// only question of the step, asked outside any resolution, on turn 3.
#[test]
fn island_sanctuary_yes_skips_the_turn_based_draw_and_makes_the_restriction() {
    let mut engine = table(7126, &[], &[]);
    let early = play_until(&mut engine, &mut script(&[]), skip_offered);
    assert!(early.is_empty(), "nothing was asked before turn 3");
    let before = hand(&engine, P0);
    assert!(!restricted(&engine));

    let asked = play_until(&mut engine, &mut script(&[true]), first_main_of(P0, 3));
    assert_eq!(
        asked,
        vec![Asked {
            turn: 3,
            step: Step::Draw,
            resolving: false
        }],
        "one question, for the turn-based draw"
    );
    assert_eq!(hand(&engine, P0), before, "the draw was skipped");
    assert!(restricted(&engine), "and the skip made the restriction");
}

/// No: the card is drawn as it would have been and nothing is restricted.
#[test]
fn island_sanctuary_no_draws_the_card_and_restricts_nothing() {
    let mut engine = table(7127, &[], &[]);
    play_until(&mut engine, &mut script(&[]), skip_offered);
    let before = hand(&engine, P0);

    let asked = play_until(&mut engine, &mut script(&[false]), first_main_of(P0, 3));
    assert_eq!(asked.len(), 1, "{asked:?}");
    assert_eq!(hand(&engine, P0), before + 1, "drawn after all");
    assert!(!restricted(&engine));
}

/// The question comes every one of p0's turns, not once: skipped on turn 3,
/// drawn on turn 5, skipped again on turn 7.
#[test]
fn island_sanctuary_is_asked_again_on_every_turn_of_its_controller() {
    let mut engine = table(7128, &[], &[]);
    let mut answers = script(&[true, false, true]);
    let asked = play_until(&mut engine, &mut answers, first_main_of(P0, 7));
    let turns: Vec<_> = asked.iter().map(|a| a.turn).collect();
    assert_eq!(turns, [3, 5, 7]);
    assert!(answers.is_empty());
    assert!(restricted(&engine), "the last answer was yes");
}

// ---------------------------------------------------------------------------
// 2. Howling Mine in its controller's own draw step
// ---------------------------------------------------------------------------

/// A Howling Mine's extra card in p0's draw step is a draw of its own,
/// asked as the Mine's trigger resolves, after the turn-based draw's
/// question. Each answer decides its own card: the hand count is exact.
#[test]
fn island_sanctuary_asks_about_a_howling_mines_card_after_the_turn_based_one() {
    // (turn-based, Mine's card) -> cards drawn
    for (answers, drawn) in [
        ([false, false], 2),
        ([false, true], 1),
        ([true, false], 1),
        ([true, true], 0),
    ] {
        let mut engine = table(7129, &[howling_mine()], &[]);
        play_until(&mut engine, &mut script(&[]), skip_offered);
        let before = hand(&engine, P0);
        let asked = play_until(&mut engine, &mut script(&answers), first_main_of(P0, 3));
        assert_eq!(
            asked,
            vec![
                Asked {
                    turn: 3,
                    step: Step::Draw,
                    resolving: false
                },
                Asked {
                    turn: 3,
                    step: Step::Draw,
                    resolving: true
                },
            ],
            "{answers:?}: the turn-based draw, then the Mine's card as its trigger resolves"
        );
        assert_eq!(hand(&engine, P0) - before, drawn, "{answers:?}");
        assert_eq!(
            restricted(&engine),
            answers.contains(&true),
            "{answers:?}: a skip makes the restriction"
        );
    }
}

/// A Howling Mine of the opponent's is no different: the card is still p0's
/// draw in p0's draw step.
#[test]
fn island_sanctuary_asks_about_an_opponents_howling_mine_card_too() {
    let mut engine = table(7130, &[], &[howling_mine()]);
    play_until(&mut engine, &mut script(&[]), skip_offered);
    let before = hand(&engine, P0);
    let asked = play_until(
        &mut engine,
        &mut script(&[false, true]),
        first_main_of(P0, 3),
    );
    assert_eq!(asked.len(), 2, "{asked:?}");
    assert!(asked[1].resolving);
    assert_eq!(hand(&engine, P0), before + 1, "one drawn, one skipped");
}

// ---------------------------------------------------------------------------
// 3. Ancestral Recall in the draw step
// ---------------------------------------------------------------------------

/// p0 skips the turn-based draw and casts Ancestral Recall at themself in
/// their draw step; returns the questions the three cards raised and the
/// change in hand size from just after the cast.
fn recall_in_the_draw_step(seed: u64, answers: [bool; 3]) -> (Vec<Asked>, usize) {
    let mut engine = Duel::new(seed, plains())
        .battlefield(0, &[island_sanctuary(), island()])
        .hand(0, &[ancestral_recall()])
        .start();
    keep_mulligans(&mut engine);
    play_until(&mut engine, &mut script(&[true]), p0_draw_step_priority);
    assert_eq!(engine.state().turn.number, 3);
    cast_from_hand(&mut engine, P0, ancestral_recall());
    engine.apply(P0, PlayerAction::ChoosePlayer(P0)).unwrap();
    let before = hand(&engine, P0);
    let mut answers = script(&answers);
    let asked = play_until(&mut engine, &mut answers, |e| {
        stack_is_empty(e) && !skip_offered(e)
    });
    assert!(answers.is_empty(), "every scripted answer was used");
    (asked, hand(&engine, P0) - before)
}

/// Three cards, three questions as the spell resolves (CR 121.2), and the
/// hand count follows the answers exactly.
#[test]
fn island_sanctuary_asks_three_times_about_ancestral_recall_in_its_draw_step() {
    for (seed, answers, drawn) in [
        (7131, [false, true, false], 2),
        (7132, [true, false, true], 1),
        (7133, [false, false, false], 3),
        (7134, [true, true, true], 0),
    ] {
        let (asked, got) = recall_in_the_draw_step(seed, answers);
        assert_eq!(asked.len(), 3, "{answers:?}: one question per card");
        assert!(
            asked.iter().all(|a| a.resolving && a.step == Step::Draw),
            "{answers:?}: asked while the spell resolves, in the draw step: {asked:?}"
        );
        assert_eq!(got, drawn, "{answers:?}");
    }
}

/// A skip while the Recall resolves makes the restriction even when the
/// turn-based draw was taken.
#[test]
fn island_sanctuary_skipping_a_recall_card_makes_the_restriction() {
    let mut engine = Duel::new(7135, plains())
        .battlefield(0, &[island_sanctuary(), island()])
        .hand(0, &[ancestral_recall()])
        .start();
    keep_mulligans(&mut engine);
    play_until(&mut engine, &mut script(&[false]), p0_draw_step_priority);
    assert!(!restricted(&engine), "the turn-based card was drawn");
    cast_from_hand(&mut engine, P0, ancestral_recall());
    engine.apply(P0, PlayerAction::ChoosePlayer(P0)).unwrap();
    play_until(
        &mut engine,
        &mut script(&[false, false, true]),
        stack_is_empty,
    );
    assert!(restricted(&engine));
}

// ---------------------------------------------------------------------------
// 4 & 5. Draws that are not "during your draw step"
// ---------------------------------------------------------------------------

/// The same Recall in p0's main phase: it is not p0's draw step, so the
/// three cards come without a question.
#[test]
fn island_sanctuary_asks_nothing_about_a_draw_in_the_main_phase() {
    let mut engine = Duel::new(7136, plains())
        .battlefield(0, &[island_sanctuary(), island()])
        .hand(0, &[ancestral_recall()])
        .start();
    keep_mulligans(&mut engine);
    // The turn-based draw is declined; the main phase is what is under test.
    play_until(&mut engine, &mut script(&[false]), first_main_of(P0, 3));
    cast_from_hand(&mut engine, P0, ancestral_recall());
    engine.apply(P0, PlayerAction::ChoosePlayer(P0)).unwrap();
    let before = hand(&engine, P0);
    let asked = play_until(&mut engine, &mut script(&[]), stack_is_empty);
    assert!(asked.is_empty(), "{asked:?}");
    assert_eq!(hand(&engine, P0), before + 3);
    assert!(!restricted(&engine));
}

/// p1's draw step, with a Howling Mine out: p1 draws two cards and p0, who
/// has the Sanctuary, is asked nothing ("your draw step"). Turn 2 is p1's
/// first, and p0's first question is turn 3's.
#[test]
fn island_sanctuary_asks_nothing_in_the_opponents_draw_step() {
    let mut engine = table(7137, &[howling_mine()], &[]);
    let before = hand(&engine, P1);
    let asked = play_until(&mut engine, &mut script(&[]), first_main_of(P1, 2));
    assert!(asked.is_empty(), "{asked:?}");
    assert_eq!(
        hand(&engine, P1),
        before + 2,
        "p1 drew the turn's card and the Mine's"
    );
    assert!(!restricted(&engine));
}

/// An Ancestral Recall that p0 casts at the opponent in p0's own draw step:
/// the cards are p1's draws, and p0 is asked nothing about them.
#[test]
fn island_sanctuary_asks_nothing_about_cards_the_opponent_draws() {
    let mut engine = Duel::new(7138, plains())
        .battlefield(0, &[island_sanctuary(), island()])
        .hand(0, &[ancestral_recall()])
        .start();
    keep_mulligans(&mut engine);
    play_until(&mut engine, &mut script(&[false]), p0_draw_step_priority);
    cast_from_hand(&mut engine, P0, ancestral_recall());
    engine.apply(P0, PlayerAction::ChoosePlayer(P1)).unwrap();
    let before = hand(&engine, P1);
    let asked = play_until(&mut engine, &mut script(&[]), stack_is_empty);
    assert!(asked.is_empty(), "{asked:?}");
    assert_eq!(hand(&engine, P1), before + 3);
}

// ---------------------------------------------------------------------------
// 6. The restriction
// ---------------------------------------------------------------------------

/// After a skip only creatures with flying or islandwalk can attack p0. The
/// Bears have neither: they are not offered, and a declaration naming them
/// is refused. The Angel (flying) and each Lord (islandwalk from the other)
/// are offered and may attack.
#[test]
fn island_sanctuary_after_a_skip_only_flyers_and_islandwalkers_attack() {
    let mut engine = table(7139, &[], &p1_attackers());
    play_until(&mut engine, &mut script(&[true]), p1_declares_attackers);
    assert_eq!(engine.state().turn.number, 4);
    assert!(restricted(&engine));

    let bears = p1_creatures(&engine, grizzly_bears())[0];
    let angel = p1_creatures(&engine, serra_angel())[0];
    let lords = p1_creatures(&engine, lord_of_atlantis());
    let offered = offered_attackers(&engine);
    assert!(
        !offered.contains(&bears),
        "the Bears have neither: {offered:?}"
    );
    assert!(offered.contains(&angel), "flying");
    assert!(lords.iter().all(|l| offered.contains(l)), "islandwalk");

    assert!(
        attack_p0_with(&mut engine, bears).is_err(),
        "the Bears can't attack p0"
    );
    attack_p0_with(&mut engine, angel).expect("the Angel flies over");
}

/// Both a flyer and an islandwalker may attack together; the declaration is
/// not limited to one.
#[test]
fn island_sanctuary_lets_a_flyer_and_an_islandwalker_attack_together() {
    let mut engine = table(7140, &[], &p1_attackers());
    play_until(&mut engine, &mut script(&[true]), p1_declares_attackers);
    let angel = p1_creatures(&engine, serra_angel())[0];
    let lord = p1_creatures(&engine, lord_of_atlantis())[0];
    engine
        .apply(
            P1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(angel, Defender::Player(P0)), (lord, Defender::Player(P0))],
            },
        )
        .expect("a flyer and an islandwalker");
}

/// Declined, nothing is restricted: the Bears attack.
#[test]
fn island_sanctuary_declined_leaves_the_bears_free_to_attack() {
    let mut engine = table(7141, &[], &p1_attackers());
    play_until(&mut engine, &mut script(&[false]), p1_declares_attackers);
    let bears = p1_creatures(&engine, grizzly_bears())[0];
    assert!(offered_attackers(&engine).contains(&bears));
    attack_p0_with(&mut engine, bears).expect("nothing stops the Bears");
}

/// "Until your next turn": the restriction holds through p1's turn 4, is
/// gone as p0's turn 5 begins (before its draw step, so before the
/// Sanctuary is asked again), and p1's turn 6 attacks freely.
#[test]
fn island_sanctuary_restriction_ends_at_its_controllers_next_turn() {
    let mut engine = table(7142, &[], &p1_attackers());
    play_until(&mut engine, &mut script(&[true]), p1_declares_attackers);
    assert!(restricted(&engine), "through turn 4");
    let bears = p1_creatures(&engine, grizzly_bears())[0];
    assert!(!offered_attackers(&engine).contains(&bears));
    engine
        .apply(P1, PlayerAction::DeclareAttackers { attackers: vec![] })
        .unwrap();

    // Turn 5: the question is the first thing the restriction is gone before.
    play_until(&mut engine, &mut script(&[]), skip_offered);
    assert_eq!(engine.state().turn.number, 5);
    assert_eq!(engine.state().turn.step, Step::Draw);
    assert!(!restricted(&engine), "ended with p0's next turn began");

    // Declined this time: turn 6 is unrestricted.
    play_until(&mut engine, &mut script(&[false]), p1_declares_attackers);
    assert_eq!(engine.state().turn.number, 6);
    assert!(offered_attackers(&engine).contains(&bears));
}

// ---------------------------------------------------------------------------
// 7. The restriction outlives the Sanctuary
// ---------------------------------------------------------------------------

/// "The effect will continue until your next turn even if this card leaves
/// the battlefield."
#[test]
fn island_sanctuary_restriction_outlives_the_sanctuary() {
    let mut engine = table(7143, &[], &p1_attackers());
    play_until(&mut engine, &mut script(&[true]), first_main_of(P0, 3));
    assert!(restricted(&engine));
    let sanctuary = on_battlefield(&engine, P0, island_sanctuary()).expect("in play");
    engine
        .dev_state_mut(P0)
        .unwrap()
        .move_object(
            sanctuary,
            ZoneLocation::Graveyard(P0),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .unwrap();
    engine.refresh_offer();
    assert!(
        on_battlefield(&engine, P0, island_sanctuary()).is_none(),
        "the Sanctuary is gone"
    );
    assert!(restricted(&engine), "the restriction is not");

    play_until(&mut engine, &mut script(&[]), p1_declares_attackers);
    assert_eq!(engine.state().turn.number, 4);
    let bears = p1_creatures(&engine, grizzly_bears())[0];
    let angel = p1_creatures(&engine, serra_angel())[0];
    let offered = offered_attackers(&engine);
    assert!(!offered.contains(&bears), "{offered:?}");
    assert!(offered.contains(&angel));
    assert!(attack_p0_with(&mut engine, bears).is_err());
}

// ---------------------------------------------------------------------------
// 8. Two Sanctuaries
// ---------------------------------------------------------------------------

/// Two Sanctuaries give one opportunity each at a draw (CR 614.5), not one
/// each per question: two questions for the turn-based draw. Declined by
/// both, the card is drawn once, not twice.
#[test]
fn two_island_sanctuaries_ask_once_each_per_draw() {
    let mut engine = table(7144, &[island_sanctuary()], &[]);
    play_until(&mut engine, &mut script(&[]), skip_offered);
    let before = hand(&engine, P0);
    let mut answers = script(&[false, false]);
    let asked = play_until(&mut engine, &mut answers, first_main_of(P0, 3));
    assert_eq!(asked.len(), 2, "one per Sanctuary: {asked:?}");
    assert!(answers.is_empty());
    assert_eq!(hand(&engine, P0), before + 1, "one draw, drawn once");
    assert!(!restricted(&engine));
}

/// The second Sanctuary's yes skips a draw the first let through.
#[test]
fn two_island_sanctuaries_the_second_may_skip_what_the_first_declined() {
    let mut engine = table(7145, &[island_sanctuary()], &[]);
    play_until(&mut engine, &mut script(&[]), skip_offered);
    let before = hand(&engine, P0);
    let mut answers = script(&[false, true]);
    let asked = play_until(&mut engine, &mut answers, first_main_of(P0, 3));
    assert_eq!(asked.len(), 2, "{asked:?}");
    assert_eq!(hand(&engine, P0), before, "skipped by the second");
    assert!(restricted(&engine));
}

/// With a Howling Mine as well, each of the two draws is put to each
/// Sanctuary once: four questions, and only the last answer skips.
#[test]
fn two_island_sanctuaries_ask_once_each_at_a_howling_mines_card_too() {
    let mut engine = table(7146, &[island_sanctuary(), howling_mine()], &[]);
    play_until(&mut engine, &mut script(&[]), skip_offered);
    let before = hand(&engine, P0);
    let mut answers = script(&[false, false, false, true]);
    let asked = play_until(&mut engine, &mut answers, first_main_of(P0, 3));
    assert_eq!(asked.len(), 4, "two Sanctuaries, two draws: {asked:?}");
    assert!(answers.is_empty());
    assert_eq!(hand(&engine, P0), before + 1);
    assert!(restricted(&engine));
}

/// A skip ends the draw it replaces, so the second Sanctuary is not asked
/// about it: one question for the turn-based draw. At the Mine's card the
/// first declines and the second skips, a second skip in the step; it makes
/// no second restriction.
#[test]
fn two_island_sanctuaries_a_second_skip_makes_no_second_restriction() {
    let mut engine = table(7147, &[island_sanctuary(), howling_mine()], &[]);
    play_until(&mut engine, &mut script(&[]), skip_offered);
    let before = hand(&engine, P0);
    let mut answers = script(&[true, false, true]);
    let asked = play_until(&mut engine, &mut answers, first_main_of(P0, 3));
    assert_eq!(
        asked.len(),
        3,
        "a skipped draw is not asked again: {asked:?}"
    );
    assert!(answers.is_empty());
    assert_eq!(hand(&engine, P0), before, "both cards skipped");
    let restrictions = engine
        .state()
        .effects
        .iter()
        .filter(|fx| fx.controller == P0)
        .filter(|fx| matches!(fx.modifier, Modifier::CantBeAttackedExceptBy { .. }))
        .count();
    assert_eq!(restrictions, 1, "one restriction for two skips");
}
