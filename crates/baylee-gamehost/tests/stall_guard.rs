//! The house does not retry an action the engine took back
//! (`baylee_ai`'s `stall`, `docs/house-ai.md` §"The frozen reference").
//!
//! `stall/purge-x0.jsonl` is the start of a capped game of the campaign's
//! test pairings (10.11.2026): the house in a seat's chair, answering with
//! no note of its cast, tapped three lands for Kaervek's Purge ("destroy
//! target creature with mana value X") and announced X 0. No creature had
//! mana value 0, so the cast was taken back, and the house cast it again
//! from the same position 2,450 times until the cap ended the game. The
//! record is cut where the house holds priority with the mana up.
//!
//! The note now explains the cast to a seat (`ai_exact_x`), but whatever
//! takes an action back, the house must not give it again from the same
//! position: here it is asked as that seat was, and at every level, the
//! frozen reference included.

use baylee_ai::{AIProfile, HeuristicAgent};
use baylee_core::ids::PlayerId;
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_engine::engine::Engine;
use baylee_gamehost::{RegistryLookup, SeatContext, owed_payment, player_view, view};

const HOUSE: PlayerId = PlayerId::new(0);

/// The engine where the record ends: the house to act, the Purge in hand
/// and three mana floating.
fn position() -> Engine<RegistryLookup> {
    let record = include_bytes!("stall/purge-x0.jsonl");
    baylee_gamehost::record::replay(record)
        .expect("the record replays on this build")
        .engine
}

/// `seat`'s view of the question it is asked, as a seat is sent it.
fn seen(engine: &Engine<RegistryLookup>, seat: PlayerId, seq: u64) -> baylee_view::PlayerView {
    let pending = engine.pending();
    player_view(
        engine.state(),
        seat,
        seq,
        Some(pending),
        &SeatContext {
            awaiting: view::awaiting_for(engine, seat),
            decision_player: pending.asked(),
            controlled_players: engine.controlled_players(seat),
            deciding: view::deciding(engine),
            owed: owed_payment(engine),
            inspected_cards: engine.inspected_cards(),
            ..Default::default()
        },
        &[],
    )
}

#[test]
fn a_cast_the_engine_took_back_is_not_cast_again_from_the_same_position() {
    for level in [
        AIProfile::SHARP_2026_11,
        AIProfile::STEADY,
        AIProfile::SHARP,
        AIProfile::EXPERT,
    ] {
        let mut engine = position();
        let start = engine.snapshot_hash();
        let house = HeuristicAgent::new(level);
        let other = HeuristicAgent::new(level);
        let mut casts = 0;
        let mut taken_back = 0;
        let mut passed = false;
        for seq in 0..200 {
            if matches!(engine.pending(), Pending::GameOver(_)) {
                break;
            }
            let actor = engine.decision_actor().expect("somebody is asked");
            let pending = engine.pending().clone();
            let view = seen(&engine, actor, seq);
            // The seat as it was: the question with no note of the cast.
            let action = if actor == HOUSE {
                house.act(&view, &pending)
            } else {
                other.act_with_context(&view, &pending, &engine.decision_context())
            };
            if actor == HOUSE && matches!(pending, Pending::Priority { .. }) {
                match action {
                    PlayerAction::CastSpell { .. } => casts += 1,
                    PlayerAction::PassPriority => {
                        passed = true;
                        break;
                    }
                    _ => {}
                }
            }
            engine
                .apply(actor, action.clone())
                .unwrap_or_else(|err| panic!("{action:?} for {pending:?} was refused: {err}"));
            taken_back += u32::from(engine.snapshot_hash() == start);
        }
        assert!(
            passed,
            "{level:?}: the house never passed ({casts} casts, {taken_back} taken back)"
        );
        assert!(
            casts <= 1 && taken_back <= 1,
            "{level:?}: the house cast {casts} times and was taken back {taken_back} times"
        );
    }
}

fn entry(name: &str) -> baylee_core::preset::DeckEntry {
    baylee_core::preset::DeckEntry {
        rule_print: None,
        choices: baylee_core::card_metadata::DeckChoices::default(),
        card: baylee_cards::decks::by_name(name).expect("registered fixture"),
        print: baylee_core::ids::PrintRef::new(0),
    }
}

/// An ability used again on purpose is not a stall: each use changes the
/// position once it resolves, so the guard lets the next one through. The
/// hardest case is a free one, which spends no mana: Flowstone Hellion's
/// "{0}: +1/+0 and -0/-1 until end of turn". The house's 3/3 Hellion faces
/// a player at 5 life with nothing to block it, pumps, waits for the pump to
/// resolve, pumps again from the 4/2 it is now, and attacks as a 5/1 for
/// exactly the 5. Between the two pumps only the Hellion's power and
/// toughness differ.
#[test]
fn an_ability_used_again_on_purpose_is_used_again() {
    let them = PlayerId::new(1);
    let mut preset = baylee_cards::decks::probe_preset(23, entry("Mountain").card).unwrap();
    preset.house_rules.starting_player = Some(HOUSE.get());
    for seat in &mut preset.seats {
        seat.starting_hand = Some(vec![]);
        seat.starting_battlefield.clear();
    }
    preset.seats[usize::from(HOUSE.get())].starting_battlefield = vec![entry("Flowstone Hellion")];
    preset.seats[usize::from(them.get())].starting_life = Some(5);
    for level in [
        AIProfile::SHARP_2026_11,
        AIProfile::SHARP,
        AIProfile::EXPERT,
    ] {
        let mut engine = Engine::new(&preset, RegistryLookup).unwrap();
        let agents = [HeuristicAgent::new(level), HeuristicAgent::new(level)];
        let hellion = engine
            .state()
            .arena
            .iter()
            .find(|(_, o)| {
                o.card
                    .is_some_and(|c| c.index == entry("Flowstone Hellion").card)
            })
            .map(|(id, _)| id)
            .expect("the Hellion is on the battlefield");
        let mut pumps = 0;
        let mut trail = Vec::new();
        for seq in 0..300 {
            if matches!(engine.pending(), Pending::GameOver(_)) {
                break;
            }
            let actor = engine.decision_actor().expect("somebody is asked");
            let pending = engine.pending().clone();
            let view = seen(&engine, actor, seq);
            let action = agents[usize::from(actor.get())].act_with_context(
                &view,
                &pending,
                &engine.decision_context(),
            );
            pumps += u32::from(matches!(
                action,
                PlayerAction::ActivateAbility { source, .. } if source == hellion
            ));
            if actor == HOUSE && !matches!(action, PlayerAction::PassPriority) {
                trail.push(format!("t{} {:?}: {action:?}", view.turn, view.step));
            }
            engine
                .apply(actor, action.clone())
                .unwrap_or_else(|err| panic!("{action:?} for {pending:?} was refused: {err}"));
        }
        assert!(
            matches!(engine.pending(), Pending::GameOver(_))
                && engine.state().players[usize::from(them.get())].life <= 0,
            "{level:?}: the Hellion did not pump them out ({pumps} pumps, life {})",
            engine.state().players[usize::from(them.get())].life
        );
        assert_eq!(pumps, 2, "{level:?}: two pumps, one at a time: {trail:#?}");
    }
}
