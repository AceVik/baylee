use super::*;

// ---- a card an effect reveals is shown to every seat (CR 701.20a)

use crate::log::GameLog;
use baylee_view::{LogEvent, LogObject};

fn entry(name: &str) -> DeckEntry {
    DeckEntry {
        card: baylee_cards::generated::ALL
            .iter()
            .find(|(_, card)| card.name() == name)
            .unwrap_or_else(|| panic!("{name} is in the pool"))
            .1
            .index,
        print: PrintRef::new(0),
    }
}

/// Three seats; seat 0 holds Mystical Tutor beside an Island, and its
/// library is Islands with one Lightning Bolt in it.
fn a_tutor_table() -> Engine<Registry> {
    let mut preset = mixed_print_preset();
    preset.seats.push(preset.seats[1].clone());
    let mut deck: Vec<DeckEntry> = (0..59).map(|_| entry("Island")).collect();
    deck.insert(20, entry("Lightning Bolt"));
    preset.seats[0].deck = deck;
    preset.seats[0].starting_hand = Some(vec![entry("Mystical Tutor")]);
    preset.seats[0].starting_battlefield = vec![entry("Island")];
    Engine::new(&preset, Registry).unwrap()
}

/// Seat 0 casts Mystical Tutor and finds the Bolt. Returns the log from
/// before the cast, where the lines about it begin, and the Bolt.
fn tutor_for_the_bolt(engine: &mut Engine<Registry>) -> (GameLog, usize, ObjectId) {
    let me = PlayerId::new(0);
    let view = settle(engine, None);
    let mut log = GameLog::new(engine.state());
    let from = log.len();
    engine
        .apply(
            me,
            PlayerAction::ActivateManaAbility {
                source: view
                    .battlefield
                    .iter()
                    .find(|o| o.controller == me)
                    .unwrap()
                    .id,
            },
        )
        .unwrap();
    let tutor = view
        .hand
        .iter()
        .find(|o| o.name == "Mystical Tutor")
        .unwrap()
        .id;
    engine
        .apply(me, PlayerAction::CastSpell { card: tutor })
        .unwrap();
    for _ in 0..8 {
        if let Pending::Priority { player, .. } = engine.pending() {
            engine.apply(*player, PlayerAction::PassPriority).unwrap();
        } else {
            break;
        }
    }
    let Pending::ChooseCards {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!("Mystical Tutor asks its caster, got {:?}", engine.pending());
    };
    assert_eq!(player, me);
    let bolt = *options.first().expect("the Bolt is offered");
    engine
        .apply(
            me,
            PlayerAction::ChooseObjects {
                objects: vec![bolt],
            },
        )
        .unwrap();
    log.consume(engine.state());
    (log, from, bolt)
}

/// Every seat's log names the card the tutor revealed, by its handle and
/// its card, and the caster's too.
#[test]
fn a_tutors_reveal_names_the_found_card_to_every_seat() {
    let mut engine = a_tutor_table();
    let (log, from, bolt) = tutor_for_the_bolt(&mut engine);
    let bolt_index = engine.state().object(bolt).unwrap().card.unwrap().index;
    for seat in (0..3).map(PlayerId::new) {
        let told: Vec<LogEvent> = log
            .told(seat, from, log.len())
            .into_iter()
            .map(|line| line.event)
            .collect();
        let shown: Vec<&LogObject> = told
            .iter()
            .filter_map(|event| match event {
                LogEvent::Revealed { cards, .. } => Some(cards.iter()),
                _ => None,
            })
            .flatten()
            .collect();
        assert!(
            matches!(
                shown[..],
                [LogObject::Known { id, card: Some(card), .. }] if *id == bolt && card.index == bolt_index
            ),
            "{seat:?} is told which card was revealed: {told:?}"
        );
    }
}

/// The reveal is over once the tutor has resolved: the library was
/// shuffled, which ends it (CR 701.20d), and putting the card on top does
/// not turn it face up (CR 701.20b: revealing never moved it, and nothing
/// reveals it again). No seat but its owner's search was ever asked about
/// it, so no view names it: not as a library top, not as a card being
/// looked at, not anywhere else. What the table saw stays in the log.
#[test]
fn the_revealed_card_is_hidden_again_once_it_is_back_in_the_library() {
    let mut engine = a_tutor_table();
    let (_, _, bolt) = tutor_for_the_bolt(&mut engine);
    let me = PlayerId::new(0);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Library(me)).last(),
        Some(&bolt),
        "the Bolt is on top"
    );
    for seat in (0..3).map(PlayerId::new) {
        let view = seen_by(&engine, seat);
        assert!(view.library_tops.is_empty(), "{seat:?}: no public top");
        assert!(view.looking_at.is_empty(), "{seat:?}: nothing shown");
        let named = view
            .battlefield
            .iter()
            .chain(&view.stack)
            .chain(view.graveyards.iter().flatten())
            .chain(view.exile.iter().flatten())
            .chain(view.command.iter().flatten())
            .any(|o| o.id == bolt)
            || view.hand.iter().any(|o| o.id == bolt);
        assert!(!named, "{seat:?}'s view names the Bolt");
    }
}

#[test]
fn a_filtered_inspection_shows_all_six_only_to_the_chooser_then_reveals_only_the_pick() {
    use baylee_engine::{
        event::{Cause, GameEvent},
        zone::{ZoneLocation, ZonePosition},
    };
    let me = PlayerId::new(0);
    let them = PlayerId::new(1);
    let mut preset = mixed_print_preset();
    preset.seats[0].starting_hand = Some(vec![entry("Silundi Vision"), entry("Lightning Bolt")]);
    preset.seats[0].starting_battlefield = vec![entry("Island"); 3];
    let mut engine = Engine::new(&preset, Registry).unwrap();
    let view = settle(&mut engine, None);
    let bolt = view
        .hand
        .iter()
        .find(|o| o.name == "Lightning Bolt")
        .unwrap()
        .id;
    engine
        .dev_state_mut(me)
        .unwrap()
        .move_object(
            bolt,
            ZoneLocation::Library(me),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    let source = view
        .hand
        .iter()
        .find(|o| o.name == "Silundi Vision")
        .unwrap()
        .id;
    for land in view.battlefield.iter().filter(|o| o.controller == me) {
        engine
            .apply(me, PlayerAction::ActivateManaAbility { source: land.id })
            .unwrap();
    }
    engine
        .apply(me, PlayerAction::CastSpell { card: source })
        .unwrap();
    for _ in 0..8 {
        if let Pending::Priority { player, .. } = engine.pending() {
            engine.apply(*player, PlayerAction::PassPriority).unwrap();
        } else {
            break;
        }
    }
    let Pending::ChooseCards { options, .. } = engine.pending() else {
        panic!("filtered inspection")
    };
    assert_eq!(options, &vec![bolt]);
    assert_eq!(seen_by(&engine, me).looking_at.len(), 6);
    assert!(seen_by(&engine, them).looking_at.is_empty());
    let hash = engine.snapshot_hash();
    let wrong = *engine
        .inspected_cards()
        .iter()
        .find(|id| **id != bolt)
        .unwrap();
    assert!(
        engine
            .apply(
                me,
                PlayerAction::ChooseObjects {
                    objects: vec![wrong]
                }
            )
            .is_err()
    );
    assert_eq!(engine.snapshot_hash(), hash);
    let seq = engine.state().journal.last_seq();
    engine
        .apply(
            me,
            PlayerAction::ChooseObjects {
                objects: vec![bolt],
            },
        )
        .unwrap();
    assert!(seen_by(&engine, me).looking_at.is_empty());
    assert!(seen_by(&engine, them).looking_at.is_empty());
    let reveals: Vec<_> = engine
        .state()
        .journal
        .entries()
        .iter()
        .filter(|e| e.seq > seq)
        .filter_map(|e| {
            if let GameEvent::Revealed { cards, .. } = &e.event {
                Some(cards.clone())
            } else {
                None
            }
        })
        .collect();
    assert_eq!(reveals, vec![vec![bolt]]);
}

#[test]
fn mindcensor_search_exposes_only_the_top_four_to_the_searcher_including_nonmatches() {
    use baylee_engine::{event::Cause, zone::ZonePosition};
    let me = PlayerId::new(0);
    let them = PlayerId::new(1);
    let mut preset = mixed_print_preset();
    preset.seats[0].deck = vec![entry("Island"); 60];
    preset.seats[0].starting_hand = Some(vec![entry("Lightning Bolt")]);
    preset.seats[0].starting_battlefield = vec![entry("Evolving Wilds")];
    preset.seats[1].starting_battlefield = vec![entry("Aven Mindcensor")];
    let mut engine = Engine::new(&preset, Registry).unwrap();
    let view = settle(&mut engine, None);
    let bolt = view
        .hand
        .iter()
        .find(|o| o.name == "Lightning Bolt")
        .unwrap()
        .id;
    let source = view
        .battlefield
        .iter()
        .find(|o| o.controller == me)
        .unwrap()
        .id;
    engine
        .dev_state_mut(me)
        .unwrap()
        .move_object(
            bolt,
            ZoneLocation::Library(me),
            ZonePosition::Top,
            Cause::Setup,
        )
        .unwrap();
    engine
        .apply(
            me,
            PlayerAction::ActivateAbility {
                source,
                ability_index: 0,
            },
        )
        .unwrap();
    for _ in 0..8 {
        let Pending::Priority { player, .. } = engine.pending() else {
            break;
        };
        engine.apply(*player, PlayerAction::PassPriority).unwrap();
    }
    let Pending::ChooseCards { options, .. } = engine.pending() else {
        panic!("search");
    };
    assert_eq!(options.len(), 3);
    assert!(!options.contains(&bolt));
    let view = seen_by(&engine, me);
    assert_eq!(view.looking_at.len(), 4);
    assert!(
        view.looking_at
            .iter()
            .any(|o| o.id == bolt && o.name == "Lightning Bolt")
    );
    assert!(seen_by(&engine, them).looking_at.is_empty());
    engine
        .apply(me, PlayerAction::ChooseObjects { objects: vec![] })
        .unwrap();
    assert!(seen_by(&engine, me).looking_at.is_empty());
}
