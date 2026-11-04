//! Buyback, read from the rule and not from one card (CR 702.27a).
//!
//! "You may pay an additional [cost] as you cast this spell" and "if the
//! buyback cost was paid, put this spell into its owner's hand instead of
//! into that player's graveyard as it resolves". Each card's own scenario is
//! its file under `card_tests`; what is here is the rule's edges: what the
//! question asks and when, what the total comes to (CR 601.2f), the ways a
//! spell leaves the stack without resolving (CR 701.6a, 608.2b), and what
//! else wants the card (CR 702.34a, 702.88a, 707.10).

#[allow(clippy::wildcard_imports)] // the shared duel plumbing, as every card test takes it
use super::testkit::*;

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;

use crate::choice::YesNoPrompt;
use baylee_core::ids::{CardIndex, ObjectId};
use baylee_core::mana::ManaCost;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn island() -> CardIndex {
    card_index("b2c6aa39-2d2a-459c-a555-fb48ba993373")
}

fn swamp() -> CardIndex {
    card_index("56719f6a-1a6c-4c0a-8d21-18f7d7350b68")
}

fn ojer_pakpatiq() -> CardIndex {
    card_index("34ef174e-1b3d-43d5-9f72-3d35befbdd7f")
}

fn mountain() -> CardIndex {
    card_index("a3fb7228-e76b-4e96-a40e-20b5fed75685")
}

fn capsize() -> CardIndex {
    card_index("77637eff-2963-4402-88f3-ca346f762fc8")
}

fn counterspell() -> CardIndex {
    card_index("cc187110-1148-4090-bbb8-e205694a39f5")
}

fn fork() -> CardIndex {
    card_index("50c53ae0-51ba-4046-ac74-87c65e688032")
}

fn snapcaster_mage() -> CardIndex {
    card_index("2bb2eda7-3b38-4c56-870f-c3218a1056f5")
}

fn mind_peel() -> CardIndex {
    card_index("4f424d94-1544-4086-a57d-2add17332094")
}

fn thunderscape_familiar() -> CardIndex {
    card_index("40f6ea6f-71e1-46bd-8d9a-ccb5f437892c")
}

fn slaughter() -> CardIndex {
    card_index("37de575d-15d4-42a2-a3d2-3855fb6ab110")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// p0 holds Capsize over `lands` Islands, and p1 a Grizzly Bears to aim it
/// at. p0's main phase.
fn capsize_over(lands: usize) -> Engine<RegistryLookup> {
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &vec![island(); lands])
        .battlefield(1, &[grizzly_bears()])
        .hand(0, &[capsize()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    engine
}

/// Taps `n` of p0's lands for mana, which is what a seat does before it casts.
fn tap_lands(engine: &mut Engine<RegistryLookup>, n: usize) {
    let lands: Vec<ObjectId> = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|id| {
            engine.state().object(*id).is_some_and(|object| {
                object.controller == P0
                    && object
                        .characteristics()
                        .types
                        .contains(baylee_core::types::TypeSet::LAND)
                    && !object.status.contains(crate::object::Status::TAPPED)
            })
        })
        .take(n)
        .collect();
    tap_mana_where(engine, P0, |id| lands.contains(&id));
}

fn answer(engine: &mut Engine<RegistryLookup>, yes: bool) {
    engine.apply(P0, PlayerAction::YesNo(yes)).unwrap();
}

fn aim(engine: &mut Engine<RegistryLookup>, target: ObjectId) {
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![target],
                players: Vec::new(),
            },
        )
        .unwrap();
}

fn buyback_cost(engine: &Engine<RegistryLookup>) -> Option<ManaCost> {
    match engine.pending() {
        Pending::YesNo {
            prompt: YesNoPrompt::Buyback { cost, .. },
            ..
        } => Some(*cost),
        _ => None,
    }
}

/// The question, and both its answers, with the whole price floating.
#[test]
fn the_cost_is_asked_as_the_spell_is_cast_and_paid_on_top_of_it() {
    for (paid, returns) in [(true, true), (false, false)] {
        let mut engine = capsize_over(6);
        let bears = on_battlefield(&engine, P1, grizzly_bears()).unwrap();
        cast_from_hand(&mut engine, P0, capsize());
        assert_eq!(
            buyback_cost(&engine),
            Some(ManaCost::parse("{3}")),
            "asked before any target (CR 601.2b, 601.2c), got {:?}",
            engine.pending()
        );
        answer(&mut engine, paid);
        aim(&mut engine, bears);
        let left = engine.state().players[0].mana_pool.total();
        assert_eq!(left, if paid { 0 } else { 3 }, "{{3}} more when paid");
        pass_until(&mut engine, stack_is_empty);
        assert_eq!(in_hand(&engine, P0, capsize()).is_some(), returns);
        assert_eq!(in_graveyard(&engine, P0, capsize()).is_none(), returns);
        assert!(in_hand(&engine, P1, grizzly_bears()).is_some());
    }
}

/// "Additional costs don't change a spell's mana cost" (CR 118.8d).
#[test]
fn paying_it_leaves_the_mana_value_alone() {
    let mut engine = capsize_over(6);
    let bears = on_battlefield(&engine, P1, grizzly_bears()).unwrap();
    cast_from_hand(&mut engine, P0, capsize());
    answer(&mut engine, true);
    aim(&mut engine, bears);
    let spell = on_stack(&engine, capsize()).unwrap();
    let object = engine.state().object(spell).unwrap();
    assert_eq!(object.characteristics().mana_value(), 3);
}

/// With nothing left to pay it with, there is no question: the mana is what
/// is short, and it cannot be made.
#[test]
fn a_cost_nothing_can_pay_is_not_asked() {
    let mut engine = capsize_over(3);
    let bears = on_battlefield(&engine, P1, grizzly_bears()).unwrap();
    cast_from_hand(&mut engine, P0, capsize());
    assert!(
        matches!(engine.pending(), Pending::ChooseTargets { .. }),
        "got {:?}",
        engine.pending()
    );
    aim(&mut engine, bears);
    pass_until(&mut engine, stack_is_empty);
    assert!(in_graveyard(&engine, P0, capsize()).is_some());
}

/// A land left to tap may still pay it (CR 601.2g), so it is asked, and the
/// window a yes opens is where the mana is made.
#[test]
fn mana_still_to_be_made_pays_in_the_window_a_yes_opens() {
    let mut engine = capsize_over(6);
    let bears = on_battlefield(&engine, P1, grizzly_bears()).unwrap();
    tap_lands(&mut engine, 3);
    cast_with_floating(&mut engine, P0, capsize());
    assert_eq!(buyback_cost(&engine), Some(ManaCost::parse("{3}")));
    answer(&mut engine, true);
    aim(&mut engine, bears);
    assert!(
        on_stack(&engine, capsize()).is_none(),
        "the {{3}} is not floating yet"
    );
    tap_all_mana(&mut engine, P0);
    engine.apply(P0, PlayerAction::PassPriority).unwrap();
    assert!(on_stack(&engine, capsize()).is_some());
    pass_until(&mut engine, stack_is_empty);
    assert!(in_hand(&engine, P0, capsize()).is_some());
}

/// A yes the lands turn out not to cover reverses the whole cast (CR 601.2h,
/// 733.1): the card is back in hand, the bears are where they were.
#[test]
fn a_yes_the_mana_cannot_reach_reverses_the_cast() {
    let mut engine = capsize_over(4);
    let bears = on_battlefield(&engine, P1, grizzly_bears()).unwrap();
    tap_lands(&mut engine, 3);
    cast_with_floating(&mut engine, P0, capsize());
    answer(&mut engine, true);
    aim(&mut engine, bears);
    tap_all_mana(&mut engine, P0);
    engine.apply(P0, PlayerAction::PassPriority).unwrap();
    assert!(on_stack(&engine, capsize()).is_none());
    assert!(in_hand(&engine, P0, capsize()).is_some());
    assert!(on_battlefield(&engine, P1, grizzly_bears()).is_some());
}

/// "A countered spell is put into its owner's graveyard" (CR 701.6a): it did
/// not resolve, so the replacement has nothing to replace, and the cost is
/// not refunded (CR 701.6b).
#[test]
fn a_countered_spell_goes_to_the_graveyard_with_its_cost_spent() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &vec![island(); 6])
        .battlefield(1, &[grizzly_bears(), island(), island()])
        .hand(0, &[capsize()])
        .hand(1, &[counterspell()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    let bears = on_battlefield(&engine, P1, grizzly_bears()).unwrap();
    cast_from_hand(&mut engine, P0, capsize());
    answer(&mut engine, true);
    aim(&mut engine, bears);
    let spell = on_stack(&engine, capsize()).unwrap();
    engine.apply(P0, PlayerAction::PassPriority).unwrap();
    cast_from_hand(&mut engine, P1, counterspell());
    engine
        .apply(
            P1,
            PlayerAction::ChooseTargets {
                objects: vec![spell],
                players: Vec::new(),
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(in_graveyard(&engine, P0, capsize()).is_some());
    assert!(in_hand(&engine, P0, capsize()).is_none());
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
}

/// A spell with no legal target left "doesn't resolve" (CR 608.2b): the same
/// answer by the other door.
#[test]
fn a_spell_that_loses_its_target_goes_to_the_graveyard() {
    let mut engine = capsize_over(6);
    let bears = on_battlefield(&engine, P1, grizzly_bears()).unwrap();
    cast_from_hand(&mut engine, P0, capsize());
    answer(&mut engine, true);
    aim(&mut engine, bears);
    crate::sba::destroy(engine.dev_state_mut(P0).unwrap(), bears);
    pass_until(&mut engine, stack_is_empty);
    assert!(in_graveyard(&engine, P0, capsize()).is_some());
    assert!(in_hand(&engine, P0, capsize()).is_none());
}

/// A copy has the decision (CR 707.10) and nothing to send anywhere: it
/// ceases to exist (CR 704.5e) and the card comes home once.
#[test]
fn a_copy_of_a_bought_back_spell_ceases_to_exist() {
    let mut lands = vec![island(); 6];
    lands.extend([mountain(), mountain()]);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &lands)
        .battlefield(1, &[grizzly_bears(), island()])
        .hand(0, &[capsize(), fork()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    let bears = on_battlefield(&engine, P1, grizzly_bears()).unwrap();
    tap_lands(&mut engine, 6);
    cast_with_floating(&mut engine, P0, capsize());
    answer(&mut engine, true);
    aim(&mut engine, bears);
    let spell = on_stack(&engine, capsize()).unwrap();
    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, fork());
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![spell],
                players: Vec::new(),
            },
        )
        .unwrap();
    // Elsewhere, or the copy's bounce would leave the spell with no target.
    pass_until(&mut engine, |engine| {
        matches!(engine.pending(), Pending::ChooseTargets { .. })
    });
    let their_island = on_battlefield(&engine, P1, island()).unwrap();
    aim(&mut engine, their_island);
    let copy = engine
        .state()
        .zones
        .list(ZoneLocation::Stack)
        .last()
        .copied()
        .filter(|id| *id != spell)
        .expect("Fork left a copy on top of the spell");
    assert!(
        engine
            .state()
            .object(copy)
            .unwrap()
            .riders
            .contains(&crate::object::Rider::Buyback),
        "a copy carries the additional costs decided for the spell (CR 707.10)"
    );
    pass_until(&mut engine, stack_is_empty);
    let hand = engine.state().zones.list(ZoneLocation::Hand(P0));
    let held = hand
        .iter()
        .filter(|id| {
            engine
                .state()
                .object(**id)
                .and_then(|o| o.card)
                .is_some_and(|c| c.index == capsize())
        })
        .count();
    assert_eq!(held, 1, "the copy did not become a second card");
}

/// Flashback exiles the card "instead of putting it anywhere else any time
/// it would leave the stack" (CR 702.34a), and that is the last word.
#[test]
fn a_flashed_back_spell_is_exiled_with_its_buyback_paid() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &vec![island(); 11])
        .battlefield(1, &[grizzly_bears()])
        .hand(0, &[capsize(), snapcaster_mage()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    let bears = on_battlefield(&engine, P1, grizzly_bears()).unwrap();
    tap_lands(&mut engine, 3);
    cast_with_floating(&mut engine, P0, capsize());
    answer(&mut engine, false);
    aim(&mut engine, bears);
    pass_until(&mut engine, stack_is_empty);
    let card = in_graveyard(&engine, P0, capsize()).unwrap();

    tap_lands(&mut engine, 2);
    cast_with_floating(&mut engine, P0, snapcaster_mage());
    pass_until(&mut engine, |engine| {
        matches!(engine.pending(), Pending::ChooseTargets { .. })
    });
    aim(&mut engine, card);
    pass_until(&mut engine, stack_is_empty);

    let land = on_battlefield(&engine, P0, island()).unwrap();
    tap_all_mana(&mut engine, P0);
    engine.apply(P0, PlayerAction::CastSpell { card }).unwrap();
    assert_eq!(buyback_cost(&engine), Some(ManaCost::parse("{3}")));
    answer(&mut engine, true);
    aim(&mut engine, land);
    pass_until(&mut engine, stack_is_empty);
    assert!(in_hand(&engine, P0, capsize()).is_none());
    assert!(in_graveyard(&engine, P0, capsize()).is_none());
    assert_eq!(
        engine.state().object(card).map(|o| o.zone),
        Some(crate::zone::Zone::Exile)
    );
}

/// The total is the mana cost plus the additional cost less the reductions
/// (CR 601.2f), so a reduction a printed `{B}` has no generic for comes off
/// the buyback's. Thunderscape Familiar takes {1} off a black spell.
#[test]
fn a_reduction_comes_off_the_whole_total() {
    let mut board = vec![swamp(); 5];
    board.push(thunderscape_familiar());
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &board)
        .hand(0, &[mind_peel()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, mind_peel());
    answer(&mut engine, true);
    engine.apply(P0, PlayerAction::ChoosePlayer(P1)).unwrap();
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "{{B}} and {{2}}{{B}}{{B}} less {{1}} is four of the five floating"
    );
    pass_until(&mut engine, stack_is_empty);
    assert!(in_hand(&engine, P0, mind_peel()).is_some());
}

/// A spell cast with its buyback paid goes home whatever else wants it. Ojer
/// Pakpatiq gives an instant cast from hand rebound, and the two "instead"s
/// are the caster's to order (CR 616.1e): the engine orders them for the one
/// who paid.
#[test]
fn a_rebounding_spell_with_its_buyback_paid_comes_home() {
    let mut board = vec![island(); 6];
    board.push(ojer_pakpatiq());
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &board)
        .battlefield(1, &[grizzly_bears()])
        .hand(0, &[capsize()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    let bears = on_battlefield(&engine, P1, grizzly_bears()).unwrap();
    cast_from_hand(&mut engine, P0, capsize());
    answer(&mut engine, true);
    aim(&mut engine, bears);
    pass_until(&mut engine, stack_is_empty);
    assert!(in_hand(&engine, P0, capsize()).is_some());
    assert!(engine.state().delayed.is_empty(), "nothing rebounds");
}

/// A cost in life is offered to a player who has it, and charged as the spell
/// is cast (CR 118.3b); a player who has not is not asked (CR 118.3).
#[test]
fn a_life_cost_is_paid_and_refused_as_life() {
    for (life, asked) in [(5, true), (3, false)] {
        let mut engine = Duel::new(SEED, swamp())
            .battlefield(0, &vec![swamp(); 4])
            .battlefield(1, &[grizzly_bears()])
            .hand(0, &[slaughter()])
            .life(0, life)
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, P0);
        let bears = on_battlefield(&engine, P1, grizzly_bears()).unwrap();
        cast_from_hand(&mut engine, P0, slaughter());
        assert_eq!(
            buyback_cost(&engine).is_some(),
            asked,
            "at {life} life, got {:?}",
            engine.pending()
        );
        if asked {
            answer(&mut engine, true);
        }
        aim(&mut engine, bears);
        let paid = if asked { 4 } else { 0 };
        assert_eq!(engine.state().players[0].life, life - paid);
        pass_until(&mut engine, stack_is_empty);
        assert_eq!(in_hand(&engine, P0, slaughter()).is_some(), asked);
    }
}
