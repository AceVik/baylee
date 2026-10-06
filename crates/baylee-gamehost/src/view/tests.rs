mod controlled;

use super::stack::stack_text;
use super::*;
use baylee_cards::by_oracle_id;
use baylee_cards::dsl::AbilityDef;
use baylee_core::ids::{CardIndex, PrintRef};
use baylee_core::mana::ManaColor;
use baylee_core::preset::{
    AIProfile, DeckEntry, Finish, FormatId, GamePreset, HouseRules, PrintInfo, SeatController,
    SeatSpec,
};
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_engine::engine::Engine;
use baylee_engine::object::PrintedFace;
use baylee_engine::state::CardLookup;
use baylee_view::{RulesFace, TargetRef};

mod announced;
mod casting;
mod commanders;
mod grants;
mod hidden;
mod historical;
mod log;
mod mana;
mod objects;
mod printings;
mod seats;
mod shared_hand;

struct Registry;
impl CardLookup for Registry {
    fn card(&self, index: CardIndex) -> Option<&'static baylee_cards_dsl::CardDef> {
        baylee_cards::by_index(index)
    }
}

fn island() -> CardIndex {
    by_oracle_id("b2c6aa39-2d2a-459c-a555-fb48ba993373")
        .unwrap()
        .index
}

fn print_info(lang: &str, finish: Finish) -> PrintInfo {
    PrintInfo {
        scryfall_id: uuid::Uuid::nil(),
        lang: lang.to_string(),
        finish,
    }
}

/// A deck holding the same card in three different printings, plus one
/// copy of each already on the battlefield.
fn mixed_print_preset() -> GamePreset {
    let mut deck: Vec<DeckEntry> = Vec::new();
    for i in 0..60u16 {
        deck.push(DeckEntry {
            card: island(),
            print: PrintRef::new(i % 3),
        });
    }
    let seat = |battlefield: Vec<DeckEntry>| SeatSpec {
        controller: SeatController::Ai(AIProfile::default()),
        capabilities: baylee_core::preset::SeatCapabilities {
            dev_commands: true,
            see_hidden: false,
        },
        deck: deck.clone(),
        sideboard: vec![],
        commanders: vec![],
        starting_life: None,
        starting_hand: Some(vec![
            DeckEntry {
                card: island(),
                print: PrintRef::new(0),
            },
            DeckEntry {
                card: island(),
                print: PrintRef::new(2),
            },
        ]),
        starting_battlefield: battlefield,
        emblems: vec![],
        team: None,
    };
    GamePreset {
        format: FormatId::Freeform,
        seed: 5,
        house_rules: HouseRules::default(),
        modifiers: vec![],
        prints: vec![
            print_info("EN", Finish::Normal),
            print_info("DE", Finish::Foil),
            print_info("JA", Finish::Etched),
        ],
        seats: vec![
            seat(vec![
                DeckEntry {
                    card: island(),
                    print: PrintRef::new(1),
                },
                DeckEntry {
                    card: island(),
                    print: PrintRef::new(2),
                },
            ]),
            seat(vec![]),
        ],
    }
}

fn teferi_time_raveler() -> CardIndex {
    by_oracle_id("ae7604bb-4818-45a3-960c-cf3d83f15964")
        .unwrap()
        .index
}

/// `seat`'s view, with its context built the way a session builds it.
fn seen_by(engine: &Engine<Registry>, seat: PlayerId) -> baylee_view::PlayerView {
    let ctx = SeatContext {
        awaiting: awaiting_for(engine, seat),
        deciding: deciding(engine),
        decision_player: decision_player_for(engine, seat),
        controlled_players: engine.controlled_players(seat),
        library_reveal_blocked: engine.library_reveal_blocked(),
        ..SeatContext::default()
    };
    player_view(
        engine.state(),
        seat,
        1,
        engine.information_pending_for(seat),
        &ctx,
        &[],
    )
}

/// The first `n` cards of a seat's library, as object ids.
fn library(engine: &Engine<Registry>, seat: PlayerId, n: usize) -> Vec<ObjectId> {
    engine
        .state()
        .zones
        .list(ZoneLocation::Library(seat))
        .iter()
        .take(n)
        .copied()
        .collect()
}

/// Katara, the Fearless — a legendary creature, so a legal commander.
fn katara() -> CardIndex {
    by_oracle_id("0972d46e-423b-454e-87c7-a2d40fb6fb6d")
        .unwrap()
        .index
}

/// Elesh Norn, Mother of Machines — seat 0's *second* commander.
///
/// The second one is the fixture and not decoration. At a table where
/// every seat has exactly one commander, a list indexed by seat and a
/// list indexed by commander have the same length and the same order,
/// and no assertion can tell the fix from the bug it replaced.
fn elesh_norn() -> CardIndex {
    by_oracle_id("5ade11c0-41dd-4b6a-9f5b-c5903a3a0d7f")
        .unwrap()
        .index
}

/// A Commander table: seat 0 with partners, seat 1 with one commander,
/// and an Island on each battlefield to contrast against.
fn commander_preset() -> GamePreset {
    let deck: Vec<DeckEntry> = (0..60)
        .map(|_| DeckEntry {
            card: island(),
            print: PrintRef::new(0),
        })
        .collect();
    let seat = |commanders: Vec<CardIndex>| SeatSpec {
        controller: SeatController::Ai(AIProfile::default()),
        capabilities: baylee_core::preset::SeatCapabilities::default(),
        deck: deck.clone(),
        sideboard: vec![],
        commanders: commanders
            .into_iter()
            .map(|card| DeckEntry {
                card,
                print: PrintRef::new(0),
            })
            .collect(),
        starting_life: None,
        starting_hand: Some(vec![]),
        starting_battlefield: vec![DeckEntry {
            card: island(),
            print: PrintRef::new(0),
        }],
        emblems: vec![],
        team: None,
    };
    GamePreset {
        format: FormatId::Commander,
        seed: 5,
        house_rules: HouseRules::default(),
        modifiers: vec![],
        prints: vec![print_info("EN", Finish::Normal)],
        seats: vec![seat(vec![katara(), elesh_norn()]), seat(vec![katara()])],
    }
}

/// An ability on the stack, as `push_ability_to_stack` leaves one:
/// taken from `source`'s card, carrying `list` printed on `printed`.
fn stacked(
    state: &mut GameState,
    source_card: CardIndex,
    index: u32,
    list: baylee_engine::object::AbilityList,
) -> GameObject {
    let name = state.names.intern("ability");
    let base = state.bare_base(name);
    let mut obj = GameObject::new_ability_on_stack(
        ObjectId::new(900, 0),
        PlayerId::new(0),
        baylee_engine::object::AbilityLoc {
            card: Some(source_card),
            index,
            source: ObjectId::new(901, 0),
        },
        std::iter::empty().collect(),
        base,
    );
    obj.take_abilities(list);
    obj
}

/// The four cards whose mana nothing but a board can name.
fn card_named(oracle: &str) -> CardIndex {
    by_oracle_id(oracle).expect("the card is in the pool").index
}

fn forest() -> CardIndex {
    card_named("b34bb2dc-c1af-4d77-b0b3-a0fb342a5fc6")
}

fn opt() -> CardIndex {
    by_oracle_id("713332c1-5bd8-400f-bfff-c1ca0697a043")
        .unwrap()
        .index
}

/// Answers everything until seat 0 holds priority in its own first main
/// phase with the stack empty, and returns seat 0's view there. A target
/// question gets `aim`, a scry keeps its card on top, nobody attacks, and
/// a Cavern of Souls names Bird.
fn settle(engine: &mut Engine<Registry>, aim: Option<ObjectId>) -> PlayerView {
    let me = PlayerId::new(0);
    for _ in 0..100 {
        let (player, action) = match engine.pending().clone() {
            Pending::Mulligan { player, .. } => (player, PlayerAction::MulliganKeep),
            Pending::Priority { player, .. } => {
                let view = player_view(engine.state(), me, 0, None, &SeatContext::default(), &[]);
                if player == me
                    && view.stack.is_empty()
                    && view.active == me
                    && view.phase == Phase::FirstMain
                {
                    return view;
                }
                (player, PlayerAction::PassPriority)
            }
            Pending::ChooseTargets { player, .. } => (
                player,
                PlayerAction::ChooseObjects {
                    objects: aim.into_iter().collect(),
                },
            ),
            Pending::Arrange { player, cards, .. } => (
                player,
                PlayerAction::Arrange {
                    piles: vec![cards, vec![]],
                },
            ),
            Pending::ChooseAttackers { player, .. } => {
                (player, PlayerAction::DeclareAttackers { attackers: vec![] })
            }
            Pending::ChooseSubtype { player, .. } => (
                player,
                PlayerAction::ChooseSubtype(baylee_core::generated::subtypes::creature::BIRD),
            ),
            other => panic!("unexpected question: {other:?}"),
        };
        engine.apply(player, action).expect("a legal answer");
    }
    panic!("seat 0 never came back to its main phase");
}
