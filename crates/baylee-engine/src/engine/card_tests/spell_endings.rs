//! Who decides where a resolved spell goes when more than one "instead" wants
//! it (CR 608.2n, 616.1, 616.1e, 616.1f), played with the cards that do it.
//!
//! Buyback (CR 702.27a), rebound (702.88a), an Adventure (715.3d) and an
//! effect that replaces the graveyard each replace the same move, so the
//! spell's controller chooses one. Flashback is the exception: it takes the
//! card from every route (702.34a), so there is nothing to ask, which
//! `buyback_tests::a_flashed_back_spell_is_exiled_with_its_buyback_paid` plays.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::legends_kit as kit;
use baylee_cards_dsl::counters::VOID;
use baylee_core::generated::index;
use baylee_engine::choice::{CastModeKind, SpellEnding};

const P0: PlayerId = kit::P0;
const P1: PlayerId = kit::P1;
const P2: PlayerId = PlayerId::new(2);

/// What the question on the table offers; empty for any other question.
fn endings(e: &Engine<RegistryLookup>) -> Vec<SpellEnding> {
    let Pending::ChooseCastMode { options, .. } = e.pending() else {
        return Vec::new();
    };
    options
        .iter()
        .filter_map(|option| match option.kind {
            CastModeKind::SpellEnding(ending) => Some(ending),
            _ => None,
        })
        .collect()
}

#[track_caller]
fn choose(e: &mut Engine<RegistryLookup>, seat: PlayerId, ending: SpellEnding) {
    let at = endings(e)
        .iter()
        .position(|offered| *offered == ending)
        .unwrap_or_else(|| panic!("{ending:?} is not offered: {:?}", e.pending()));
    e.apply(seat, PlayerAction::ChooseMode(at)).unwrap();
}

/// Runs the stack down to the question.
#[track_caller]
fn to_the_question(e: &mut Engine<RegistryLookup>) {
    pass_until(e, |e| !endings(e).is_empty());
}

fn exiled(e: &Engine<RegistryLookup>, seat: PlayerId, card: CardIndex) -> Option<ObjectId> {
    e.state()
        .zones
        .list(ZoneLocation::Exile(seat))
        .iter()
        .copied()
        .find(|id| {
            e.state()
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == card))
        })
}

/// `seat` pays Capsize's buyback and aims it at `target`.
#[track_caller]
fn capsize_with_buyback(e: &mut Engine<RegistryLookup>, seat: PlayerId, target: ObjectId) {
    kit::float(e, seat, &[(ManaColor::Blue, 6)]);
    cast_with_floating(e, seat, index::CAPSIZE);
    kit::yes_no(e, true);
    kit::aim(e, &[target], &[]);
}

/// p0 with Ojer Pakpatiq casts Capsize from hand with its buyback paid, at
/// p1's Grizzly Bears: it has rebound, and it was bought back.
fn bought_back_under_ojer() -> Engine<RegistryLookup> {
    let mut e = kit::game(
        SEED,
        &[index::OJER_PAKPATIQ_DEEPEST_EPOCH],
        &[index::GRIZZLY_BEARS],
        &[index::CAPSIZE],
        &[],
    );
    let bears = kit::obj(&e, P1, index::GRIZZLY_BEARS);
    capsize_with_buyback(&mut e, P0, bears);
    e
}

/// The question comes last: the spell has done everything it does and is
/// still on the stack (CR 608.2n), and it is its controller who is asked.
#[test]
fn a_bought_back_spell_with_rebound_asks_its_controller_once_it_has_done_its_work() {
    let mut e = bought_back_under_ojer();
    to_the_question(&mut e);
    assert_eq!(endings(&e), [SpellEnding::Hand, SpellEnding::Rebound]);
    assert_eq!(e.pending().asked(), Some(P0));
    assert!(
        in_hand(&e, P1, index::GRIZZLY_BEARS).is_some(),
        "Capsize has already bounced the Bears"
    );
    assert_eq!(
        e.state().zones.list(ZoneLocation::Stack),
        e.decision_context().source.as_slice(),
        "and is still on the stack, the one thing the question is about"
    );
}

/// "Put this spell into its owner's hand instead" is chosen (CR 702.27a), and
/// that leaves no move to the graveyard for rebound to replace (CR 616.1f).
#[test]
fn choosing_the_hand_over_rebound_brings_the_card_home_and_nothing_rebounds() {
    let mut e = bought_back_under_ojer();
    to_the_question(&mut e);
    choose(&mut e, P0, SpellEnding::Hand);
    assert!(in_hand(&e, P0, index::CAPSIZE).is_some());
    assert!(exiled(&e, P0, index::CAPSIZE).is_none());
    assert!(e.state().delayed.is_empty(), "nothing rebounds");
    assert!(stack_is_empty(&e));
    assert!(
        !e.state()
            .object(in_hand(&e, P0, index::CAPSIZE).unwrap())
            .unwrap()
            .riders
            .contains(&baylee_engine::object::Rider::Rebound)
    );
}

/// Rebound is chosen, and the buyback paid for buys nothing: the card is
/// exiled, and p0's next upkeep casts it free (CR 702.88a). That cast is not
/// from hand, so it neither rebounds nor asks, and ends in the graveyard.
#[test]
fn choosing_rebound_over_the_hand_exiles_it_and_casts_it_free_next_upkeep() {
    let mut e = bought_back_under_ojer();
    to_the_question(&mut e);
    choose(&mut e, P0, SpellEnding::Rebound);
    assert!(in_hand(&e, P0, index::CAPSIZE).is_none());
    assert!(exiled(&e, P0, index::CAPSIZE).is_some());
    assert_eq!(e.state().delayed.len(), 1, "the free cast waits");

    let turn = e.state().turn.number;
    pass_until(&mut e, |e| {
        e.state().turn.number > turn
            && e.state().turn.active == P0
            && matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    assert_eq!(e.state().turn.step, baylee_engine::turn::Step::Upkeep);
    let ojer = kit::obj(&e, P0, index::OJER_PAKPATIQ_DEEPEST_EPOCH);
    kit::aim(&mut e, &[ojer], &[]);
    kit::settle(&mut e);
    assert!(in_hand(&e, P0, index::OJER_PAKPATIQ_DEEPEST_EPOCH).is_some());
    assert!(
        in_graveyard(&e, P0, index::CAPSIZE).is_some(),
        "cast from exile, so no rebound, and no buyback was paid this time"
    );
}

/// Nothing is offered to anybody else, and a position past the end is no
/// answer. Neither loses the question.
#[test]
fn a_wrong_answer_leaves_the_question_standing() {
    let mut e = bought_back_under_ojer();
    to_the_question(&mut e);
    assert!(e.apply(P1, PlayerAction::ChooseMode(0)).is_err());
    assert!(e.apply(P0, PlayerAction::ChooseMode(2)).is_err());
    assert_eq!(endings(&e), [SpellEnding::Hand, SpellEnding::Rebound]);
    choose(&mut e, P0, SpellEnding::Hand);
    assert!(in_hand(&e, P0, index::CAPSIZE).is_some());
}

/// One "instead" is no choice: rebound takes the card when no buyback was paid.
#[test]
fn rebound_alone_asks_nothing() {
    let mut e = kit::game(
        SEED,
        &[index::OJER_PAKPATIQ_DEEPEST_EPOCH],
        &[index::GRIZZLY_BEARS],
        &[index::CAPSIZE],
        &[],
    );
    let bears = kit::obj(&e, P1, index::GRIZZLY_BEARS);
    kit::float(&mut e, P0, &[(ManaColor::Blue, 3)]);
    cast_with_floating(&mut e, P0, index::CAPSIZE);
    kit::aim(&mut e, &[bears], &[]);
    kit::settle(&mut e);
    assert!(exiled(&e, P0, index::CAPSIZE).is_some());
    assert_eq!(e.state().delayed.len(), 1);
}

/// A spell that does not resolve reaches no ending to choose (CR 701.6a):
/// countered, it goes to the graveyard with both still on it.
#[test]
fn a_countered_spell_is_not_asked_where_it_goes() {
    let mut e = kit::game(
        SEED,
        &[index::OJER_PAKPATIQ_DEEPEST_EPOCH],
        &[index::GRIZZLY_BEARS],
        &[index::CAPSIZE],
        &[index::COUNTERSPELL],
    );
    let bears = kit::obj(&e, P1, index::GRIZZLY_BEARS);
    capsize_with_buyback(&mut e, P0, bears);
    // Ojer's trigger gives it rebound first, on top of it.
    pass_until(&mut e, |e| {
        e.state().zones.list(ZoneLocation::Stack).len() == 1
    });
    let capsize = e.state().zones.list(ZoneLocation::Stack)[0];
    pass_until(
        &mut e,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == P1),
    );
    kit::float(&mut e, P1, &[(ManaColor::Blue, 2)]);
    cast_with_floating(&mut e, P1, index::COUNTERSPELL);
    kit::aim(&mut e, &[capsize], &[]);
    kit::settle(&mut e);
    assert!(in_graveyard(&e, P0, index::CAPSIZE).is_some());
    assert!(e.state().delayed.is_empty());
}

/// Nor does a spell with no legal target left (CR 608.2b).
#[test]
fn a_spell_that_loses_its_target_is_not_asked_where_it_goes() {
    let mut e = bought_back_under_ojer();
    pass_until(&mut e, |e| {
        e.state().zones.list(ZoneLocation::Stack).len() == 1
    });
    let bears = kit::obj(&e, P1, index::GRIZZLY_BEARS);
    e.dev_state_mut(P0)
        .unwrap()
        .move_object(
            bears,
            ZoneLocation::Hand(P1),
            baylee_engine::zone::ZonePosition::Top,
            baylee_engine::event::Cause::Effect,
        )
        .unwrap();
    e.refresh_offer();
    kit::settle(&mut e);
    assert!(in_graveyard(&e, P0, index::CAPSIZE).is_some());
    assert!(e.state().delayed.is_empty());
}

/// A copy has no card to send anywhere (CR 707.10), so only the original asks.
#[test]
fn a_copy_of_the_spell_is_not_asked() {
    let mut e = kit::game(
        SEED,
        &[index::OJER_PAKPATIQ_DEEPEST_EPOCH],
        &[index::GRIZZLY_BEARS],
        &[index::CAPSIZE, index::FORK],
        &[],
    );
    let bears = kit::obj(&e, P1, index::GRIZZLY_BEARS);
    capsize_with_buyback(&mut e, P0, bears);
    pass_until(&mut e, |e| {
        e.state().zones.list(ZoneLocation::Stack).len() == 1
    });
    let capsize = e.state().zones.list(ZoneLocation::Stack)[0];
    kit::float(&mut e, P0, &[(ManaColor::Red, 4)]);
    cast_with_floating(&mut e, P0, index::FORK);
    kit::aim(&mut e, &[capsize], &[]);
    // Fork asks nothing of itself; an instant from hand gained rebound too,
    // and it has no other "instead" to meet.
    pass_until(&mut e, |e| {
        e.state().zones.list(ZoneLocation::Stack).len() == 2
            || !endings(e).is_empty()
            || matches!(e.pending(), Pending::YesNo { .. })
    });
    assert!(endings(&e).is_empty(), "got {:?}", e.pending());
}

/// Yawgmoth's Will replaces the same move ("if a card would be put into your
/// graveyard from anywhere this turn, exile that card instead"), so a spell
/// bought back under it is asked as well. Both answers.
#[test]
fn yawgmoths_will_and_buyback_are_the_casters_to_choose_between() {
    for (ending, home) in [(SpellEnding::Hand, true), (SpellEnding::Exile, false)] {
        let mut e = kit::game(
            SEED,
            &[],
            &[index::GRIZZLY_BEARS],
            &[index::YAWGMOTH_S_WILL, index::CAPSIZE],
            &[],
        );
        kit::float(&mut e, P0, &[(ManaColor::Black, 3)]);
        cast_with_floating(&mut e, P0, index::YAWGMOTH_S_WILL);
        kit::settle(&mut e);
        let bears = kit::obj(&e, P1, index::GRIZZLY_BEARS);
        capsize_with_buyback(&mut e, P0, bears);
        to_the_question(&mut e);
        assert_eq!(endings(&e), [SpellEnding::Hand, SpellEnding::Exile]);
        choose(&mut e, P0, ending);
        assert_eq!(in_hand(&e, P0, index::CAPSIZE).is_some(), home);
        assert_eq!(exiled(&e, P0, index::CAPSIZE).is_some(), !home);
    }
}

/// p1's Dauthi Voidwalker wants p0's bought-back spell. Chosen, it is exiled
/// with a void counter: the effect's own move, not a plain exile. Chosen
/// against, the card is home and carries nothing.
#[test]
fn dauthi_voidwalker_and_buyback_are_the_casters_to_choose_between() {
    for (ending, void) in [(SpellEnding::Hand, 0), (SpellEnding::Exile, 1)] {
        let mut e = kit::game(
            SEED,
            &[],
            &[index::DAUTHI_VOIDWALKER, index::GRIZZLY_BEARS],
            &[index::CAPSIZE],
            &[],
        );
        let bears = kit::obj(&e, P1, index::GRIZZLY_BEARS);
        capsize_with_buyback(&mut e, P0, bears);
        to_the_question(&mut e);
        assert_eq!(endings(&e), [SpellEnding::Hand, SpellEnding::Exile]);
        choose(&mut e, P0, ending);
        let card = exiled(&e, P0, index::CAPSIZE).or_else(|| in_hand(&e, P0, index::CAPSIZE));
        assert_eq!(
            e.state().object(card.unwrap()).unwrap().counters.get(VOID),
            void
        );
    }
}

/// Ephemerate prints rebound, and p1's Voidwalker wants the same move: the
/// free cast, or the void counter that lets p1 play it.
#[test]
fn dauthi_voidwalker_and_rebound_are_the_casters_to_choose_between() {
    for (ending, void, free_casts) in [(SpellEnding::Rebound, 0, 1), (SpellEnding::Exile, 1, 0)] {
        let mut e = kit::game(
            SEED,
            &[index::GRIZZLY_BEARS],
            &[index::DAUTHI_VOIDWALKER],
            &[index::EPHEMERATE],
            &[],
        );
        let bears = kit::obj(&e, P0, index::GRIZZLY_BEARS);
        kit::float(&mut e, P0, &[(ManaColor::White, 1)]);
        cast_with_floating(&mut e, P0, index::EPHEMERATE);
        kit::aim(&mut e, &[bears], &[]);
        to_the_question(&mut e);
        assert_eq!(endings(&e), [SpellEnding::Rebound, SpellEnding::Exile]);
        choose(&mut e, P0, ending);
        let card = exiled(&e, P0, index::EPHEMERATE).expect("exiled either way");
        assert_eq!(e.state().object(card).unwrap().counters.get(VOID), void);
        assert_eq!(e.state().delayed.len(), free_casts);
    }
}

/// p0 with Ojer casts Swift End from hand at p1's Bears: an Adventure spell
/// that has gained rebound. CR 716.3d and 702.88a both replace its move.
fn swift_end_under_ojer() -> Engine<RegistryLookup> {
    let mut e = kit::game(
        SEED,
        &[index::OJER_PAKPATIQ_DEEPEST_EPOCH],
        &[index::GRIZZLY_BEARS],
        &[index::MURDEROUS_RIDER],
        &[],
    );
    let bears = kit::obj(&e, P1, index::GRIZZLY_BEARS);
    let card = in_hand(&e, P0, index::MURDEROUS_RIDER).unwrap();
    kit::float(&mut e, P0, &[(ManaColor::Black, 3)]);
    e.apply(P0, PlayerAction::CastSpell { card }).unwrap();
    let Pending::ChooseCastMode { options, .. } = e.pending().clone() else {
        panic!("creature or Adventure, got {:?}", e.pending())
    };
    let adventure = options
        .iter()
        .position(|o| matches!(o.kind, CastModeKind::Face(1)))
        .expect("Swift End is offered");
    e.apply(P0, PlayerAction::ChooseMode(adventure)).unwrap();
    kit::aim(&mut e, &[bears], &[]);
    to_the_question(&mut e);
    assert_eq!(endings(&e), [SpellEnding::Rebound, SpellEnding::Adventure]);
    e
}

/// On an adventure: it may be played from exile at its price, and nothing
/// waits for an upkeep (CR 716.3d).
#[test]
fn choosing_the_adventure_over_rebound_leaves_the_creature_castable() {
    let mut e = swift_end_under_ojer();
    choose(&mut e, P0, SpellEnding::Adventure);
    let card = exiled(&e, P0, index::MURDEROUS_RIDER).expect("on an adventure");
    let object = e.state().object(card).unwrap();
    assert_eq!(object.face_index, 0, "a creature card in exile (CR 716.4)");
    assert!(
        object
            .riders
            .iter()
            .any(|r| matches!(r, baylee_engine::object::Rider::Adventure(_)))
    );
    assert!(e.state().delayed.is_empty(), "nothing rebounds");
}

/// Rebound: exile, as the card it is out of the stack (CR 716.4), with no
/// adventure to come back from, and the free cast at the next upkeep.
#[test]
fn choosing_rebound_over_the_adventure_casts_it_free_next_upkeep() {
    let mut e = swift_end_under_ojer();
    choose(&mut e, P0, SpellEnding::Rebound);
    let card = exiled(&e, P0, index::MURDEROUS_RIDER).expect("exiled");
    let object = e.state().object(card).unwrap();
    assert_eq!(object.face_index, 0, "a creature card in exile (CR 716.4)");
    assert!(
        !object
            .riders
            .iter()
            .any(|r| matches!(r, baylee_engine::object::Rider::Adventure(_))),
        "it is not on an adventure"
    );
    assert_eq!(e.state().delayed.len(), 1, "the free cast waits");
    // "Cast this card": out of the stack it is the creature, and no land of
    // p0's pays for it.
    let turn = e.state().turn.number;
    pass_until(&mut e, |e| {
        on_battlefield(e, P0, index::MURDEROUS_RIDER).is_some()
    });
    assert_eq!(e.state().turn.active, P0);
    assert_eq!(e.state().turn.number, turn + 2, "p0's next turn");
    assert_eq!(e.state().turn.step, baylee_engine::turn::Step::Upkeep);
}

/// A controlled player's choices are their controller's (CR 724.5), and this
/// is one: p1's spell, p1 asked, p0 answering, and p1 refused.
#[test]
fn the_controller_of_a_controlled_turn_answers_it() {
    let mut e = kit::game(
        SEED,
        &[
            index::MINDSLAVER,
            index::ISLAND,
            index::ISLAND,
            index::ISLAND,
            index::ISLAND,
        ],
        &[index::OJER_PAKPATIQ_DEEPEST_EPOCH, index::GRIZZLY_BEARS],
        &[],
        &[index::CAPSIZE],
    );
    let islands = all_on_battlefield(&e, P0, index::ISLAND);
    tap_mana_where(&mut e, P0, |id| islands.contains(&id));
    activate(&mut e, P0, index::MINDSLAVER, 0);
    e.apply(
        P0,
        PlayerAction::ChooseTargets {
            objects: vec![],
            players: vec![P1],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    reach_their_main_phase(&mut e, P1);
    assert_eq!(e.decision_actor(), Some(P0), "a controlled turn");

    let bears = kit::obj(&e, P1, index::GRIZZLY_BEARS);
    let card = in_hand(&e, P1, index::CAPSIZE).unwrap();
    kit::float(&mut e, P1, &[(ManaColor::Blue, 6)]);
    e.apply(P0, PlayerAction::CastSpell { card }).unwrap();
    e.apply(P0, PlayerAction::YesNo(true)).unwrap();
    e.apply(
        P0,
        PlayerAction::ChooseTargets {
            objects: vec![bears],
            players: vec![],
        },
    )
    .unwrap();
    to_the_question(&mut e);
    assert_eq!(endings(&e), [SpellEnding::Hand, SpellEnding::Rebound]);
    assert_eq!(
        (e.pending().asked(), e.decision_actor()),
        (Some(P1), Some(P0))
    );
    assert!(
        e.apply(P1, PlayerAction::ChooseMode(0)).is_err(),
        "the subject has no say (CR 723.5)"
    );
    choose(&mut e, P0, SpellEnding::Rebound);
    assert!(exiled(&e, P1, index::CAPSIZE).is_some());
}

/// Three seats; p0 buys Capsize back with `p2_board` on the other side.
fn three_seats(p2_board: &[CardIndex]) -> Engine<RegistryLookup> {
    let mut e = Duel::table(SEED, forest(), 3)
        .battlefield(0, &[index::OJER_PAKPATIQ_DEEPEST_EPOCH])
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .battlefield(2, p2_board)
        .hand(0, &[index::CAPSIZE])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    let bears = kit::obj(&e, P1, index::GRIZZLY_BEARS);
    capsize_with_buyback(&mut e, P0, bears);
    to_the_question(&mut e);
    e
}

/// Somebody else leaving takes nothing from the question.
#[test]
fn a_bystander_leaving_leaves_the_question_as_it_was() {
    let mut e = three_seats(&[]);
    e.apply(P2, PlayerAction::Concede).unwrap();
    assert_eq!(endings(&e), [SpellEnding::Hand, SpellEnding::Rebound]);
    assert_eq!(e.pending().asked(), Some(P0));
    choose(&mut e, P0, SpellEnding::Hand);
    assert!(in_hand(&e, P0, index::CAPSIZE).is_some());
}

/// A competitor's source leaving takes that option with it, and the question
/// is asked again of what is left: with Ojer's rebound there are still two.
#[test]
fn a_competitor_leaving_takes_its_option_away() {
    let mut e = three_seats(&[index::DAUTHI_VOIDWALKER]);
    assert_eq!(
        endings(&e),
        [SpellEnding::Hand, SpellEnding::Rebound, SpellEnding::Exile]
    );
    e.apply(P2, PlayerAction::Concede).unwrap();
    assert_eq!(endings(&e), [SpellEnding::Hand, SpellEnding::Rebound]);
}

/// The player asked leaves: their spell leaves the game with them (CR 800.4a),
/// so there is nothing left to choose, and the game goes on.
#[test]
fn the_player_asked_leaving_ends_the_question_and_the_game_goes_on() {
    let mut e = three_seats(&[]);
    e.apply(P0, PlayerAction::Concede).unwrap();
    assert!(endings(&e).is_empty(), "got {:?}", e.pending());
    assert!(stack_is_empty(&e));
    assert!(e.pending().asked().is_some(), "the game is still running");
}
