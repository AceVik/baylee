//! `cards/instants/mv_3/corpse_dance.rs`, played.
//!
//! "Return the top creature card of your graveyard to the battlefield. That
//! creature gains haste until end of turn. Exile it at the beginning of the
//! next end step." The pile is ordered (CR 404.2), its owner orders what falls
//! together (CR 404.3), and nothing is targeted (CR 115.1). Its buyback
//! (CR 702.27a) is every other buyback's; it can't lose a target, so it can't
//! fizzle.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as kit;
use baylee_core::generated::index;

const P0: PlayerId = kit::P0;
const P1: PlayerId = kit::P1;

/// p0's main phase, with Corpse Dance in hand and `buried` in the graveyard,
/// the last of them on top of the pile.
fn over_a_pile(buried: &[CardIndex]) -> Engine<RegistryLookup> {
    let mut hand = buried.to_vec();
    hand.push(index::CORPSE_DANCE);
    let mut e = kit::game(SEED, &[], &[], &hand, &[]);
    for card in buried {
        hand_to_graveyard(&mut e, P0, *card);
    }
    e
}

/// Casts it from floating mana, answering its buyback. It asks for no target.
#[track_caller]
fn dance(e: &mut Engine<RegistryLookup>, buy_back: bool) {
    kit::float(e, P0, &[(ManaColor::Black, 5)]);
    cast_with_floating(e, P0, index::CORPSE_DANCE);
    kit::yes_no(e, buy_back);
    assert!(
        !matches!(e.pending(), Pending::ChooseTargets { .. }),
        "nothing is targeted (CR 115.1)"
    );
}

fn exiled(e: &Engine<RegistryLookup>, card: CardIndex) -> bool {
    e.state()
        .zones
        .list(ZoneLocation::Exile(P0))
        .iter()
        .any(|id| {
            e.state()
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == card))
        })
}

/// "The top creature card": the creature nearest the top, whatever lies over
/// it, and not the one under it.
#[test]
fn the_top_creature_card_comes_back_past_what_lies_above_it() {
    let mut e = over_a_pile(&[index::GRAY_OGRE, index::GRIZZLY_BEARS, index::ISLAND]);
    dance(&mut e, false);
    kit::settle(&mut e);
    assert!(on_battlefield(&e, P0, index::GRIZZLY_BEARS).is_some());
    assert!(on_battlefield(&e, P0, index::GRAY_OGRE).is_none());
    assert!(in_graveyard(&e, P0, index::GRAY_OGRE).is_some());
    assert!(in_graveyard(&e, P0, index::ISLAND).is_some());
}

/// It came back this turn and attacks all the same, and the end step takes it.
#[test]
fn that_creature_has_haste_and_is_exiled_at_the_next_end_step() {
    let mut e = over_a_pile(&[index::GRIZZLY_BEARS]);
    dance(&mut e, false);
    kit::settle(&mut e);
    let bears = on_battlefield(&e, P0, index::GRIZZLY_BEARS).expect("it came back");
    assert!(keywords(&e, bears).contains(KeywordSet::HASTE));
    assert!(
        kit::attackers_offered(&mut e).contains(&bears),
        "a creature that arrived this turn attacks with haste (CR 702.10b)"
    );
    let turn = e.state().turn.number;
    pass_until(&mut e, |e| exiled(e, index::GRIZZLY_BEARS));
    assert_eq!(e.state().turn.number, turn, "this turn's end step");
    assert_eq!(e.state().turn.step, crate::turn::Step::End);
}

/// The exile is about the creature as it came back (CR 603.7c): out of play
/// and back in hand it is a new object (CR 400.7), and the end step leaves it.
#[test]
fn a_creature_that_left_before_the_end_step_is_not_exiled() {
    let mut e = over_a_pile(&[index::GRIZZLY_BEARS]);
    dance(&mut e, false);
    kit::settle(&mut e);
    let bears = on_battlefield(&e, P0, index::GRIZZLY_BEARS).expect("it came back");
    e.dev_state_mut(P0)
        .unwrap()
        .move_object(
            bears,
            ZoneLocation::Hand(P0),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .unwrap();
    e.refresh_offer();
    pass_until(&mut e, |e| e.state().turn.active == P1);
    assert!(in_hand(&e, P0, index::GRIZZLY_BEARS).is_some());
    assert!(!exiled(&e, index::GRIZZLY_BEARS));
}

/// With nothing to return the spell still resolves and its buyback still pays,
/// and the end step's exile has nothing to be about: above all not the spell
/// itself, safely back in hand.
#[test]
fn with_no_creature_card_nothing_comes_back_and_nothing_is_exiled() {
    let mut e = over_a_pile(&[index::ISLAND]);
    dance(&mut e, true);
    kit::settle(&mut e);
    assert!(in_hand(&e, P0, index::CORPSE_DANCE).is_some());
    pass_until(&mut e, |e| e.state().turn.active == P1);
    assert!(in_hand(&e, P0, index::CORPSE_DANCE).is_some());
    assert!(!exiled(&e, index::CORPSE_DANCE));
    assert!(e.state().delayed.is_empty());
}

/// Paid, the card comes home and dances again; unpaid it stays in the
/// graveyard, on top of the pile and no creature.
#[test]
fn its_buyback_brings_it_home_and_only_if_paid() {
    let mut e = over_a_pile(&[index::GRAY_OGRE, index::GRIZZLY_BEARS]);
    dance(&mut e, true);
    kit::settle(&mut e);
    assert!(in_hand(&e, P0, index::CORPSE_DANCE).is_some());
    assert!(on_battlefield(&e, P0, index::GRIZZLY_BEARS).is_some());
    dance(&mut e, false);
    kit::settle(&mut e);
    assert!(on_battlefield(&e, P0, index::GRAY_OGRE).is_some());
    assert!(in_graveyard(&e, P0, index::CORPSE_DANCE).is_some());
}

/// Countered, it never resolved: nothing comes back and the buyback buys
/// nothing (CR 701.6a).
#[test]
fn a_countered_dance_returns_nothing_and_stays_in_the_graveyard() {
    let mut e = kit::game(
        SEED,
        &[],
        &[],
        &[index::GRIZZLY_BEARS, index::CORPSE_DANCE],
        &[index::COUNTERSPELL],
    );
    hand_to_graveyard(&mut e, P0, index::GRIZZLY_BEARS);
    dance(&mut e, true);
    let spell = e.state().zones.list(ZoneLocation::Stack)[0];
    pass_until(
        &mut e,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == P1),
    );
    kit::float(&mut e, P1, &[(ManaColor::Blue, 2)]);
    cast_with_floating(&mut e, P1, index::COUNTERSPELL);
    kit::aim(&mut e, &[spell], &[]);
    kit::settle(&mut e);
    assert!(in_graveyard(&e, P0, index::CORPSE_DANCE).is_some());
    assert!(in_hand(&e, P0, index::CORPSE_DANCE).is_none());
    assert!(in_graveyard(&e, P0, index::GRIZZLY_BEARS).is_some());
}

/// Two creatures that die together are ordered by their owner (CR 404.3), and
/// only because a card in the game reads the order: the top is what they said,
/// each way, with the Wrath itself lying over both.
#[test]
fn the_owner_of_a_pile_that_fell_together_says_which_is_on_top() {
    for top in [index::GRAY_OGRE, index::GRIZZLY_BEARS] {
        let mut e = kit::game(
            SEED,
            &[index::GRAY_OGRE, index::GRIZZLY_BEARS],
            &[],
            &[index::WRATH_OF_GOD, index::CORPSE_DANCE],
            &[],
        );
        kit::float(&mut e, P0, &[(ManaColor::White, 4)]);
        cast_with_floating(&mut e, P0, index::WRATH_OF_GOD);
        pass_until(&mut e, |e| matches!(e.pending(), Pending::Arrange { .. }));
        let Pending::Arrange { player, cards, .. } = e.pending().clone() else {
            unreachable!("pass_until stopped on it");
        };
        assert_eq!(player, P0);
        let mut order = cards;
        order.sort_by_key(|id| {
            e.state()
                .object(*id)
                .is_none_or(|o| o.card.is_none_or(|c| c.index != top))
        });
        e.apply(P0, PlayerAction::Arrange { piles: vec![order] })
            .unwrap();
        kit::settle(&mut e);
        dance(&mut e, false);
        kit::settle(&mut e);
        assert!(on_battlefield(&e, P0, top).is_some(), "{top:?} was on top");
    }
}

/// "At the beginning of the next end step": cast during an end step, the next
/// is the following turn's, because this one has already begun (CR 603.7).
/// The creature stays through p1's turn and goes at p1's end step.
#[test]
fn a_dance_in_the_end_step_exiles_at_the_end_step_after_it() {
    let mut e = over_a_pile(&[index::GRIZZLY_BEARS]);
    pass_until(&mut e, |e| {
        e.state().turn.step == crate::turn::Step::End
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == P0)
    });
    let turn = e.state().turn.number;
    dance(&mut e, false);
    kit::settle(&mut e);
    assert!(on_battlefield(&e, P0, index::GRIZZLY_BEARS).is_some());

    pass_until(&mut e, |e| e.state().turn.number > turn);
    assert!(
        on_battlefield(&e, P0, index::GRIZZLY_BEARS).is_some(),
        "this end step was already under way when it was cast"
    );
    pass_until(&mut e, |e| exiled(e, index::GRIZZLY_BEARS));
    assert_eq!(e.state().turn.number, turn + 1);
    assert_eq!(e.state().turn.step, crate::turn::Step::End);
}
