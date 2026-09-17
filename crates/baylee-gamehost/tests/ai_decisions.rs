//! Decisions tested across the real engine/view boundary.

use baylee_ai::{AIProfile, HeuristicAgent, pending_player};
use baylee_core::ids::PlayerId;
use baylee_core::preset::{DeckEntry, GamePreset};
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_engine::engine::Engine;
use baylee_gamehost::{RegistryLookup, player_view};

fn entry(name: &str) -> DeckEntry {
    DeckEntry {
        card: baylee_cards::decks::by_name(name).expect("registered fixture"),
        print: baylee_core::ids::PrintRef::new(0),
    }
}

fn position(hand: &[&str], board: &[&str]) -> GamePreset {
    let mut preset = baylee_cards::decks::probe_preset(41, entry("Forest").card).unwrap();
    for seat in &mut preset.seats {
        seat.starting_hand = Some(vec![]);
        seat.starting_battlefield.clear();
    }
    preset.seats[0].starting_hand = Some(hand.iter().map(|name| entry(name)).collect());
    preset.seats[0].starting_battlefield = board.iter().map(|name| entry(name)).collect();
    preset
}

#[test]
fn a_planned_multicolour_cast_survives_the_mana_choice_round_trip() {
    let preset = position(
        &[
            "Baleful Strix",
            "Loran of the Third Path",
            "Loran of the Third Path",
            "Loran of the Third Path",
            "Loran of the Third Path",
        ],
        &["City of Brass", "Swamp"],
    );
    for profile in [AIProfile::STEADY, AIProfile::SHARP, AIProfile::EXPERT] {
        let mut engine = Engine::new(&preset, RegistryLookup).unwrap();
        let agent = HeuristicAgent::new(profile);
        let mut cast = false;
        for seq in 0..100 {
            let Some(seat) = pending_player(engine.pending()) else {
                break;
            };
            let view = player_view(
                engine.state(),
                seat,
                Some(seat),
                seq,
                Some(engine.pending()),
                false,
            );
            if view.turn > 1 {
                break;
            }
            cast |= view.battlefield.iter().any(|o| {
                o.card
                    .is_some_and(|c| c.index == entry("Baleful Strix").card)
            });
            let action = match engine.pending() {
                Pending::Mulligan { .. } => PlayerAction::MulliganKeep,
                _ if seat == PlayerId::new(0) => agent.act(&view, engine.pending()),
                Pending::Priority { .. } => PlayerAction::PassPriority,
                _ => panic!("unexpected opposing question: {:?}", engine.pending()),
            };
            engine
                .apply(seat, action)
                .expect("every planned action is legal");
        }
        assert!(
            cast,
            "{profile:?}: two untapped sources must cast Strix despite the white-heavy hand"
        );
    }
}

struct CombatCards(Vec<&'static baylee_cards_dsl::CardDef>);

impl baylee_engine::state::CardLookup for CombatCards {
    fn card(
        &self,
        index: baylee_core::ids::CardIndex,
    ) -> Option<&'static baylee_cards_dsl::CardDef> {
        self.0
            .iter()
            .find(|c| c.index == index)
            .copied()
            .or_else(|| baylee_cards::by_index(index))
    }
}

fn combat_card(
    index: u32,
    power: i16,
    toughness: i16,
    keywords: baylee_cards_dsl::KeywordSet,
) -> &'static baylee_cards_dsl::CardDef {
    use baylee_cards_dsl::{CardDef, FaceDef};
    Box::leak(Box::new(CardDef {
        index: baylee_core::ids::CardIndex::new(index),
        faces: Box::leak(Box::new([FaceDef {
            name: "Combat fixture",
            mana_cost: baylee_core::mana::ManaCost::from_symbol_generic(2),
            types: baylee_core::types::TypeSet::CREATURE,
            power: Some(power),
            toughness: Some(toughness),
            ..FaceDef::DEFAULT
        }])),
        keywords,
        ..CardDef::DEFAULT
    }))
}

#[test]
fn combat_lifelink_block_keeps_the_player_alive_in_the_engine() {
    use baylee_cards_dsl::KeywordSet;
    let cards = vec![
        combat_card(90_000, 2, 2, KeywordSet::LIFELINK),
        combat_card(90_001, 3, 3, KeywordSet::EMPTY),
        combat_card(90_002, 8, 8, KeywordSet::FLYING),
    ];
    let engine = combat_after_expert_blocks(cards, 8);
    assert_eq!(
        engine.state().players[0].life,
        2,
        "eight flying damage is survivable only by gaining two life from the ground block"
    );
    assert!(!matches!(engine.pending(), Pending::GameOver(_)));
}

#[test]
fn combat_first_strike_must_be_blocked_before_lifelink_can_happen() {
    use baylee_cards_dsl::KeywordSet;
    let cards = vec![
        combat_card(90_000, 3, 2, KeywordSet::LIFELINK),
        combat_card(90_001, 3, 3, KeywordSet::FIRST_STRIKE),
        combat_card(90_002, 1, 20, KeywordSet::EMPTY),
    ];
    let engine = combat_after_expert_blocks(cards, 3);
    assert_eq!(engine.state().players[0].life, 2);
    assert!(!matches!(engine.pending(), Pending::GameOver(_)));
}

fn combat_after_expert_blocks(
    cards: Vec<&'static baylee_cards_dsl::CardDef>,
    life: i32,
) -> Engine<CombatCards> {
    use baylee_core::ids::{CardIndex, Defender, PrintRef};
    let mut preset = position(&[], &[]);
    let object = |index| DeckEntry {
        card: CardIndex::new(index),
        print: PrintRef::new(0),
    };
    preset.seats[0].starting_life = Some(life);
    preset.seats[0].starting_battlefield = vec![object(90_000)];
    preset.seats[1].starting_battlefield = vec![object(90_001), object(90_002)];
    let mut engine = Engine::new(&preset, CombatCards(cards)).unwrap();
    let agent = HeuristicAgent::new(AIProfile::EXPERT);
    let mut blocked = false;
    for seq in 0..200 {
        let Some(seat) = pending_player(engine.pending()) else {
            break;
        };
        let view = player_view(
            engine.state(),
            seat,
            Some(seat),
            seq,
            Some(engine.pending()),
            false,
        );
        if blocked && view.step == baylee_view::Step::CombatEnd {
            break;
        }
        let action = match engine.pending() {
            Pending::Mulligan { .. } => PlayerAction::MulliganKeep,
            Pending::Priority { .. } => PlayerAction::PassPriority,
            Pending::ChooseAttackers { attackers, .. } if seat == PlayerId::new(1) => {
                PlayerAction::DeclareAttackers {
                    attackers: attackers
                        .iter()
                        .map(|id| (*id, Defender::Player(PlayerId::new(0))))
                        .collect(),
                }
            }
            Pending::ChooseAttackers { .. } => PlayerAction::DeclareAttackers { attackers: vec![] },
            Pending::ChooseBlockers { .. } if seat == PlayerId::new(0) => {
                blocked = true;
                agent.act(&view, engine.pending())
            }
            Pending::ChooseBlockers { .. } => PlayerAction::DeclareBlockers { blockers: vec![] },
            _ => panic!("unexpected question: {:?}", engine.pending()),
        };
        engine
            .apply(seat, action)
            .expect("combat decision is legal");
    }
    assert!(blocked, "the fixture must reach the blocking choice");
    engine
}
