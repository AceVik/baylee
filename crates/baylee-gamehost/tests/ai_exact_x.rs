//! An X that must equal a target's mana value, announced by the house in
//! its own chair and in a seat's (`baylee_ai::noted::answer_as_seated`).
//!
//! - Spell Blast ("counter target spell with mana value X") against a Hill
//!   Giant on the stack (mana value 4): a live house announces the X the
//!   Giant has, or does not cast it (`AIProfile::EXACT_SPELL_X`). Before,
//!   the X was all the mana paid: with two Islands an X of 1, which no
//!   spell on the stack had, so the cast had no target and was taken back,
//!   and the house cast it again from the same position until a cap ended
//!   the game (the campaign's capped pairings, 10.11.2026).
//! - Kaervek's Purge ("destroy target creature with mana value X") at a
//!   Hill Giant on the battlefield: the X is the Giant's 4. A seat used to
//!   be asked the X with no note of the cast, announced 0, found no target
//!   and was taken back the same way; with the note it plans the cast's X
//!   as the table's own house does.

use baylee_ai::{AIProfile, HeuristicAgent};
use baylee_core::ids::PlayerId;
use baylee_core::preset::{DeckEntry, GamePreset};
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_engine::engine::Engine;
use baylee_engine::zone::Zone;
use baylee_gamehost::{RegistryLookup, SeatContext, owed_payment, player_view, view};

const HOUSE: PlayerId = PlayerId::new(0);
const THEM: PlayerId = PlayerId::new(1);

fn entry(name: &str) -> DeckEntry {
    DeckEntry {
        rule_print: None,
        choices: baylee_core::card_metadata::DeckChoices::default(),
        card: baylee_cards::decks::by_name(name).expect("registered fixture"),
        print: baylee_core::ids::PrintRef::new(0),
    }
}

fn entries(names: &[&str]) -> Vec<DeckEntry> {
    names.iter().map(|name| entry(name)).collect()
}

/// A table where `first` takes the first turn, each seat with the
/// battlefield and hand given.
fn table(first: PlayerId, house: (&[&str], &[&str]), them: (&[&str], &[&str])) -> GamePreset {
    let mut preset = baylee_cards::decks::probe_preset(17, entry("Island").card).unwrap();
    preset.house_rules.starting_player = Some(first.get());
    for (seat, (battlefield, hand)) in preset.seats.iter_mut().zip([house, them]) {
        seat.starting_battlefield = entries(battlefield);
        seat.starting_hand = Some(entries(hand));
    }
    preset
}

/// How the house's own chair answers: in-process from the engine's context,
/// or as a seat over a socket from the view and the question's note.
#[derive(Clone, Copy, Debug)]
enum Chair {
    InProcess,
    Seated,
}

/// Plays until the Hill Giant has left the stack or has left the
/// battlefield, both seats answered by the house at `level` and the house's
/// own seat in `chair`. Returns where the Giant went and how many times the
/// house cast `spell`.
fn play(preset: &GamePreset, level: AIProfile, chair: Chair, spell: &str) -> (Zone, u32) {
    let mut engine = Engine::new(preset, RegistryLookup).unwrap();
    let agents = [HeuristicAgent::new(level), HeuristicAgent::new(level)];
    let giant = baylee_cards::decks::by_name("Hill Giant").unwrap();
    let start = engine
        .state()
        .arena
        .iter()
        .find(|(_, o)| o.card.is_some_and(|c| c.index == giant))
        .map(|(_, o)| o.zone)
        .expect("the Giant is in the game");
    let mut casts = 0;
    let mut moved = false;
    for seq in 0..400 {
        let pending = engine.pending().clone();
        if matches!(pending, Pending::GameOver(_)) {
            break;
        }
        let zone = engine
            .state()
            .arena
            .iter()
            .find(|(_, o)| o.card.is_some_and(|c| c.index == giant))
            .map(|(_, o)| o.zone)
            .expect("the Giant is in the game");
        if moved && zone != Zone::Stack {
            return (zone, casts);
        }
        moved |= zone != start;
        let actor = engine.decision_actor().expect("somebody is asked");
        let view = player_view(
            engine.state(),
            actor,
            seq,
            Some(&pending),
            &SeatContext {
                awaiting: view::awaiting_for(&engine, actor),
                decision_player: pending.asked(),
                controlled_players: engine.controlled_players(actor),
                deciding: view::deciding(&engine),
                owed: owed_payment(&engine),
                inspected_cards: engine.inspected_cards(),
                ..Default::default()
            },
            &[],
        );
        let agent = &agents[usize::from(actor.get())];
        let action = match chair {
            Chair::Seated if actor == HOUSE => baylee_ai::noted::answer_as_seated(
                agent,
                view.clone(),
                &pending,
                view::decision_note(&engine, actor).as_ref(),
                None,
            ),
            _ => agent.act_with_context(&view, &pending, &engine.decision_context()),
        };
        if actor == HOUSE
            && let PlayerAction::CastSpell { card } = action
            && view.hand.iter().any(|c| c.id == card && c.name == spell)
        {
            casts += 1;
        }
        engine
            .apply(actor, action.clone())
            .unwrap_or_else(|err| panic!("{action:?} for {pending:?} was refused: {err}"));
    }
    panic!("the Giant never settled ({spell} cast {casts} times)");
}

#[test]
fn spell_blast_is_cast_with_the_x_of_a_spell_or_not_at_all() {
    let giant: (&[&str], &[&str]) = (&["Mountain"; 4], &["Hill Giant"]);
    for level in [AIProfile::SHARP, AIProfile::EXPERT] {
        for chair in [Chair::InProcess, Chair::Seated] {
            let two = table(THEM, (&["Island"; 2], &["Spell Blast"]), giant);
            assert_eq!(
                play(&two, level, chair, "Spell Blast"),
                (Zone::Battlefield, 0),
                "{chair:?}: with two Islands no X counters the Giant, so Spell Blast stays in hand"
            );
            let five = table(THEM, (&["Island"; 5], &["Spell Blast"]), giant);
            assert_eq!(
                play(&five, level, chair, "Spell Blast"),
                (Zone::Graveyard, 1),
                "{chair:?}: with five Islands Spell Blast is cast once, with X 4, and counters \
                 the Giant"
            );
        }
    }
}

#[test]
fn kaerveks_purge_is_cast_with_the_x_of_its_target_from_a_seat_too() {
    let lands = [
        "Swamp", "Swamp", "Swamp", "Mountain", "Mountain", "Mountain",
    ];
    let preset = table(
        HOUSE,
        (&lands, &["Kaervek's Purge"]),
        (&["Hill Giant"], &[]),
    );
    for level in [AIProfile::SHARP, AIProfile::EXPERT] {
        for chair in [Chair::InProcess, Chair::Seated] {
            assert_eq!(
                play(&preset, level, chair, "Kaervek's Purge"),
                (Zone::Graveyard, 1),
                "{chair:?}: the Purge is cast once, with X 4, and destroys the Giant"
            );
        }
    }
}
