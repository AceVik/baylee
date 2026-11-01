use super::*;
use baylee_core::ids::{PlayerId, PrintRef};
use baylee_core::preset::{AIProfile, DeckEntry, FormatId};
use baylee_gamehost::Session;
use baylee_view::Phase;

fn entry(name: &str) -> DeckEntry {
    DeckEntry {
        card: baylee_cards::all()
            .find(|card| card.name() == name)
            .unwrap()
            .index,
        print: PrintRef::new(0),
        rule_print: None,
        choices: baylee_core::card_metadata::DeckChoices::default(),
    }
}

fn priority(payer: PlayerId, spell: bool) -> Session {
    let deck = crate::housedeck::HouseDeck::named("victory").unwrap();
    let mut preset = crate::selfplay::table_for(12, &[&deck, &deck], &[AIProfile::STEADY; 2], &[]);
    preset.format = FormatId::Freeform;
    preset.modifiers.clear();
    for (index, seat) in preset.seats.iter_mut().enumerate() {
        seat.deck = vec![entry("Forest"); 32];
        seat.commanders.clear();
        seat.sideboard.clear();
        seat.starting_hand = Some(if spell && index == usize::from(payer.get()) {
            vec![entry("Vitalizing Cascade")]
        } else {
            vec![]
        });
        seat.starting_battlefield = ["Forest", "Forest", "Plains"].map(entry).to_vec();
        seat.starting_life = Some(20);
    }
    let mut session = Session::new(&preset).unwrap();
    for seat in [PlayerId::new(0), PlayerId::new(1)] {
        assert!(session.take_over(seat));
    }
    for _ in 0..64 {
        if session.observe_view(payer).phase == Phase::FirstMain
            && session.pending().asked() == Some(payer)
        {
            return session;
        }
        let seat = session.pending().asked().unwrap();
        let action = match session.pending() {
            Pending::Mulligan { .. } => PlayerAction::MulliganKeep,
            Pending::Priority { .. } => PlayerAction::PassPriority,
            other => panic!("unexpected opening {other:?}"),
        };
        session.act(seat, action).unwrap();
    }
    panic!("first main priority not reached");
}

#[test]
fn ordinary_idle_pass_keeps_live_and_llm_weights() {
    for (payer, live) in [(PlayerId::new(0), 0.15), (PlayerId::new(1), 0.0)] {
        let session = priority(payer, false);
        let (pending, view) = session.view_for_with_context(payer).unwrap();
        assert!(is_idle_pass(&pending, &view, &PlayerAction::PassPriority));
        assert_eq!(
            live_weight(&pending, &view, &PlayerAction::PassPriority),
            live
        );
        assert_eq!(
            llm_weight(&pending, &view, &PlayerAction::PassPriority),
            0.25
        );
    }
}

#[test]
fn real_payment_settlement_keeps_full_bc_weight_including_off_turn() {
    for payer in [PlayerId::new(0), PlayerId::new(1)] {
        let mut session = priority(payer, true);
        let source = session
            .observe_view(payer)
            .hand
            .iter()
            .find(|card| card.name == "Vitalizing Cascade")
            .unwrap()
            .id;
        session
            .act(payer, PlayerAction::CastSpell { card: source })
            .unwrap();
        while matches!(session.pending(), Pending::ChooseCastMode { .. }) {
            session.act(payer, PlayerAction::ChooseMode(0)).unwrap();
        }
        assert!(matches!(session.pending(), Pending::ChooseNumber { .. }));
        session.act(payer, PlayerAction::ChooseNumber(1)).unwrap();
        let Pending::Priority { legal, .. } = session.pending() else {
            panic!("payment priority")
        };
        let mana_sources = legal.mana_abilities.clone();
        assert_eq!(mana_sources.len(), 3);
        for source in mana_sources {
            session
                .act(payer, PlayerAction::ActivateManaAbility { source })
                .unwrap();
        }
        for encoder in [7, 8] {
            let (pending, view) = if encoder == 8 {
                session.view_for_with_payment_context(payer)
            } else {
                session.view_for_with_context(payer)
            }
            .unwrap();
            assert_eq!(view.casting, Some(source));
            assert!(view.owed.is_some());
            assert!(view.stack.is_empty());
            assert_eq!(view.active != payer, payer == PlayerId::new(1));
            assert!(!is_idle_pass(&pending, &view, &PlayerAction::PassPriority));
            assert_eq!(
                live_weight(&pending, &view, &PlayerAction::PassPriority),
                1.0
            );
            assert_eq!(
                llm_weight(&pending, &view, &PlayerAction::PassPriority),
                1.0
            );
        }
        session.act(payer, PlayerAction::PassPriority).unwrap();
        let view = session.observe_view_with_context(payer);
        assert!(view.stack.iter().any(|object| object.id == source));
        assert!(view.owed.is_none());
        assert!(view.casting.is_none());
    }
}

/// A card that could be foretold is a play declined, as a suspendable one is:
/// the pass keeps its full weight. The classification reads only that a
/// foretell is offered, so any object stands in for the card.
#[test]
fn passing_with_a_foretell_on_offer_is_not_idle() {
    let payer = PlayerId::new(0);
    let session = priority(payer, false);
    let (mut pending, view) = session.view_for_with_context(payer).unwrap();
    assert!(is_idle_pass(&pending, &view, &PlayerAction::PassPriority));
    let Pending::Priority { legal, .. } = &mut pending else {
        panic!("first main priority")
    };
    legal
        .foretellable
        .push(view.battlefield_of(payer).next().unwrap().id);
    assert!(!is_idle_pass(&pending, &view, &PlayerAction::PassPriority));
    assert_eq!(
        live_weight(&pending, &view, &PlayerAction::PassPriority),
        1.0
    );
    assert_eq!(
        llm_weight(&pending, &view, &PlayerAction::PassPriority),
        1.0
    );
}
