//! Brainstorm and Temporal Mastery: a miracle drawn on an opponent's turn,
//! paid for in the window the "yes" opens.
//!
//! Miracle (CR 702.94a): "You may reveal this card from your hand as you
//! draw it if it's the first card you've drawn this turn. When you reveal
//! this card this way, you may cast it by paying [cost] rather than its
//! mana cost." The first card a player draws on someone else's turn counts,
//! so an instant that draws in the opponent's upkeep is how the card is
//! played at its best. Temporal Mastery: `{5}{U}{U}` sorcery, "Take an
//! extra turn after this one. Exile Temporal Mastery. Miracle {1}{U}".
//!
//! The "yes" opens a mana window owing the miracle cost
//! (`Engine::payment_window`, which the view projects as
//! `PlayerView::owed`), and the player makes the mana there one tap at a
//! time (CR 601.2g). Passing with the cost made casts it. Passing short
//! casts nothing, and the play that was begun is reversed (CR 732.1): the
//! card stays in hand, and the lands tapped for it are untapped with their
//! mana taken back. Before the reversal they stayed tapped and their mana
//! floated until the step ended, which is what the owner asked to change.
//!
//! Every library is Temporal Mastery, so each of Brainstorm's draws is one;
//! seat 1's own miracle offers on the way are declined. The bystander is the
//! third Island: tapped for Brainstorm before the window, it must stay
//! tapped when the window is given back.

use super::*;
use crate::zone::Zone;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaPayment;

fn brainstorm() -> CardIndex {
    card_index("36cd2364-d113-47d1-b2c4-b088d9eb88dd")
}

fn temporal_mastery() -> CardIndex {
    card_index("5c58b8e6-c572-461e-893e-a8c05f20ba17")
}

fn seat(n: u8) -> PlayerId {
    PlayerId::new(n)
}

fn is_tapped(engine: &Engine<RegistryLookup>, id: ObjectId) -> bool {
    engine
        .state()
        .object(id)
        .is_some_and(|o| o.status.contains(crate::object::Status::TAPPED))
}

fn pool(engine: &Engine<RegistryLookup>) -> u64 {
    engine.state().players[0].mana_pool.total()
}

/// Seat 0, holding Brainstorm and two Islands beside three Islands on the
/// battlefield, casts Brainstorm in seat 1's upkeep with the first Island,
/// puts the two Islands from hand back, and is offered the miracle of the
/// Temporal Mastery it drew first. Returns the engine at that offer, the
/// miracle card, and the Islands: the one Brainstorm was paid with, then
/// the two left untapped.
fn offered_on_their_upkeep(seed: u64) -> (Engine<RegistryLookup>, ObjectId, [ObjectId; 3]) {
    let mut engine = Duel::new(seed, temporal_mastery())
        .battlefield(0, &[island(), island(), island()])
        .hand(0, &[brainstorm(), island(), island()])
        .start();
    keep_mulligans(&mut engine);
    let lands: Vec<ObjectId> = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.controller == seat(0))
        })
        .collect();
    let [paid, first, second] = lands[..] else {
        panic!("three Islands: {lands:?}")
    };
    let mut cast = false;
    for _ in 0..200 {
        let pending = engine.pending().clone();
        match pending {
            Pending::YesNo {
                player,
                prompt: YesNoPrompt::Miracle { card },
                ..
            } if player == seat(0) => {
                assert!(cast, "the miracle is drawn by Brainstorm");
                assert_eq!(engine.state().turn.active, seat(1), "on seat 1's turn");
                return (engine, card, [paid, first, second]);
            }
            Pending::YesNo { player, .. } => {
                engine.apply(player, PlayerAction::YesNo(false)).unwrap();
            }
            Pending::Priority { player, .. }
                if player == seat(0)
                    && !cast
                    && engine.state().turn.active == seat(1)
                    && engine.state().turn.step == crate::turn::Step::Upkeep =>
            {
                engine
                    .apply(player, PlayerAction::ActivateManaAbility { source: paid })
                    .unwrap();
                let card = in_hand(&engine, seat(0), brainstorm()).expect("Brainstorm in hand");
                engine
                    .apply(player, PlayerAction::CastSpell { card })
                    .unwrap();
                cast = true;
            }
            Pending::ChooseCards {
                player, options, ..
            } => {
                // Brainstorm's put-back: the two Islands, so every Temporal
                // Mastery stays in hand.
                let islands: Vec<ObjectId> = options
                    .iter()
                    .copied()
                    .filter(|id| {
                        engine
                            .state()
                            .object(*id)
                            .and_then(|o| o.card)
                            .is_some_and(|c| c.index == island())
                    })
                    .collect();
                assert_eq!(islands.len(), 2, "two Islands to put back: {options:?}");
                engine
                    .apply(player, PlayerAction::ChooseObjects { objects: islands })
                    .unwrap();
            }
            Pending::ChooseAttackers { player, .. } => {
                engine
                    .apply(player, PlayerAction::DeclareAttackers { attackers: vec![] })
                    .unwrap();
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected before the miracle: {other:?}"),
        }
    }
    panic!("no miracle offer on seat 1's upkeep");
}

/// The window opened by the "yes" owes the miracle cost and offers only
/// mana; seat 0 taps one Island by hand, then the other, and passing casts
/// Temporal Mastery for `{1}{U}`, which resolves into an extra turn.
#[test]
fn a_miracle_drawn_on_their_turn_is_paid_in_its_window_and_cast() {
    let (mut engine, card, [paid, first, second]) = offered_on_their_upkeep(702);
    engine.apply(seat(0), PlayerAction::YesNo(true)).unwrap();
    assert_eq!(
        engine.payment_window(),
        Some((seat(0), ManaPayment::Fixed("{1}{U}".parse().unwrap()))),
        "the window owes the miracle cost"
    );
    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!("the window is a priority offer: {:?}", engine.pending())
    };
    assert_eq!(player, seat(0));
    assert!(
        legal.castable.is_empty() && legal.lands.is_empty(),
        "only mana is made in the window"
    );
    assert!(
        legal.mana_abilities.contains(&first) && legal.mana_abilities.contains(&second),
        "both untapped Islands are offered: {:?}",
        legal.mana_abilities
    );
    assert!(
        !legal.mana_abilities.contains(&paid),
        "the tapped one is not"
    );

    engine
        .apply(seat(0), PlayerAction::ActivateManaAbility { source: first })
        .unwrap();
    assert_eq!(
        engine.payment_window(),
        Some((seat(0), ManaPayment::Fixed("{1}{U}".parse().unwrap()))),
        "one tap in, the window still stands"
    );
    engine
        .apply(
            seat(0),
            PlayerAction::ActivateManaAbility { source: second },
        )
        .unwrap();
    engine.apply(seat(0), PlayerAction::PassPriority).unwrap();

    assert_eq!(engine.payment_window(), None, "passing closed the window");
    assert_eq!(
        engine.state().object(card).map(|o| o.zone),
        Some(Zone::Stack)
    );
    assert_eq!(pool(&engine), 0, "the {{1}}{{U}} paid for it");
    pass_until(&mut engine, |e| !e.state().extra_turns.is_empty());
    assert_eq!(
        engine.state().object(card).map(|o| o.zone),
        Some(Zone::Exile),
        "\"Exile Temporal Mastery\""
    );
}

/// Passing after one tap of two leaves the miracle uncast and the card in
/// hand, and the play is reversed (CR 732.1): the Island tapped for it is
/// untapped and its {U} taken back. The Island Brainstorm was paid with
/// stays tapped: that mana was spent before the window opened.
#[test]
fn a_miracle_window_closed_short_gives_back_what_was_tapped_for_it() {
    let (mut engine, card, [paid, first, second]) = offered_on_their_upkeep(703);
    engine.apply(seat(0), PlayerAction::YesNo(true)).unwrap();
    engine
        .apply(seat(0), PlayerAction::ActivateManaAbility { source: first })
        .unwrap();
    assert!(is_tapped(&engine, first));
    assert_eq!(pool(&engine), 1, "the Island's {{U}} is in the pool");

    engine.apply(seat(0), PlayerAction::PassPriority).unwrap();

    assert_eq!(engine.payment_window(), None, "passing closed the window");
    assert_eq!(
        engine.state().object(card).map(|o| o.zone),
        Some(Zone::Hand),
        "nothing was cast"
    );
    assert!(
        !is_tapped(&engine, first),
        "the Island tapped for it untaps"
    );
    assert_eq!(pool(&engine), 0, "and its mana is taken back");
    assert!(is_tapped(&engine, paid), "Brainstorm's Island stays tapped");
    assert!(!is_tapped(&engine, second));
    assert!(engine.state().extra_turns.is_empty());
    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!("play goes on: {:?}", engine.pending())
    };
    assert!(
        player != seat(0) || legal.mana_abilities.contains(&first),
        "the untapped Island can be tapped again"
    );
}
