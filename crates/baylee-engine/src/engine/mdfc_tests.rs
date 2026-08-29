//! MDFC tests: land-face choice (pathways), auto back-face land play
//! (Glasspool Shore), and back-face casting (The True Scriptures).

use super::*;
use crate::choice::CastModeKind;
use baylee_core::ids::{CardIndex, PrintRef};
use baylee_core::preset::{
    AIProfile, DeckEntry, Finish, FormatId, GamePreset, PrintInfo, SeatController, SeatSpec,
};

struct RegistryLookup;
impl CardLookup for RegistryLookup {
    fn card(&self, index: CardIndex) -> Option<&'static baylee_cards_dsl::CardDef> {
        baylee_cards::by_index(index)
    }
}

fn card_index(oracle_id: &str) -> CardIndex {
    baylee_cards::by_oracle_id(oracle_id)
        .expect("card exists")
        .index
}

fn island() -> CardIndex {
    card_index("b2c6aa39-2d2a-459c-a555-fb48ba993373")
}
fn swamp() -> CardIndex {
    card_index("56719f6a-1a6c-4c0a-8d21-18f7d7350b68")
}
fn brightclimb() -> CardIndex {
    card_index("1c633e02-95ef-445e-b4e0-fbfbc5ed9cc9")
}
fn glasspool_mimic() -> CardIndex {
    card_index("c178953c-3888-4edd-9d0c-265bd82b1d24")
}
fn sheoldred() -> CardIndex {
    card_index("97652492-7906-4d79-983c-fa1dc1239eba")
}

fn entry(card: CardIndex) -> DeckEntry {
    DeckEntry {
        card,
        print: PrintRef::new(0),
    }
}

fn preset(seed: u64, hand0: Vec<CardIndex>, bf0: Vec<CardIndex>) -> GamePreset {
    let deck: Vec<DeckEntry> = (0..60).map(|_| entry(island())).collect();
    let mk = |hand: Vec<CardIndex>, bf: Vec<CardIndex>| SeatSpec {
        controller: SeatController::Ai(AIProfile::default()),
        deck: deck.clone(),
        starting_life: None,
        starting_hand: Some(hand.into_iter().map(entry).collect()),
        starting_battlefield: bf.into_iter().map(entry).collect(),
        emblems: vec![],
        team: None,
    };
    GamePreset {
        format: FormatId::Freeform,
        seed,
        dev_mode: false,
        house_rules: HouseRules::default(),
        modifiers: vec![],
        prints: vec![PrintInfo {
            scryfall_id: uuid::Uuid::nil(),
            lang: "EN".into(),
            finish: Finish::Normal,
        }],
        seats: vec![mk(hand0, bf0), mk(vec![], vec![])],
    }
}

fn keep_mulligans(engine: &mut Engine<RegistryLookup>) {
    for _ in 0..2 {
        match engine.pending().clone() {
            Pending::Mulligan { player, .. } => {
                engine.apply(player, PlayerAction::MulliganKeep).unwrap();
            }
            other => panic!("expected mulligan, got {other:?}"),
        }
    }
}

/// Advances until p0 has priority with a playable land, then plays it.
fn play_land(engine: &mut Engine<RegistryLookup>, p0: PlayerId, card_name_idx: CardIndex) {
    let mut guard = 0;
    loop {
        guard += 1;
        assert!(guard < 400, "no land-play window found");
        match engine.pending().clone() {
            Pending::Priority { player, legal } if player == p0 => {
                if let Some(&card) = legal.lands.iter().find(|id| {
                    engine.state().object(**id).unwrap().card.unwrap().index == card_name_idx
                }) {
                    engine
                        .apply(player, PlayerAction::PlayLand { card })
                        .unwrap();
                    return;
                }
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected pending: {other:?}"),
        }
    }
}

fn battlefield_card(engine: &Engine<RegistryLookup>, idx: CardIndex) -> ObjectId {
    engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Battlefield)
        .iter()
        .copied()
        .find(|id| {
            engine
                .state()
                .object(*id)
                .and_then(|o| o.card)
                .is_some_and(|c| c.index == idx)
        })
        .expect("card on the battlefield")
}

#[test]
fn pathway_face_choice_plays_back_face() {
    let mut engine = Engine::new(&preset(7, vec![brightclimb()], vec![]), RegistryLookup).unwrap();
    keep_mulligans(&mut engine);
    let p0 = PlayerId::new(0);
    play_land(&mut engine, p0, brightclimb());
    // The engine asks which land face to play.
    let Pending::ChooseCastMode { player, options } = engine.pending().clone() else {
        panic!("expected face choice, got {:?}", engine.pending());
    };
    assert_eq!(player, p0);
    assert_eq!(options.len(), 2);
    assert!(matches!(options[1].kind, CastModeKind::PlayLandFace(1)));
    engine.apply(player, PlayerAction::ChooseMode(1)).unwrap();
    let land = battlefield_card(&engine, brightclimb());
    let obj = engine.state().object(land).unwrap();
    assert_eq!(obj.face_index, 1);
    assert_eq!(
        engine.state().names.get(obj.characteristics().name),
        "Grimclimb Pathway"
    );
    assert!(
        obj.characteristics()
            .types
            .contains(baylee_core::types::TypeSet::LAND)
    );
}

#[test]
fn glasspool_shore_plays_as_back_face_land_without_choice() {
    let mut engine =
        Engine::new(&preset(11, vec![glasspool_mimic()], vec![]), RegistryLookup).unwrap();
    keep_mulligans(&mut engine);
    let p0 = PlayerId::new(0);
    play_land(&mut engine, p0, glasspool_mimic());
    // Only one land face: enters directly as Glasspool Shore.
    let land = battlefield_card(&engine, glasspool_mimic());
    let obj = engine.state().object(land).unwrap();
    assert_eq!(obj.face_index, 1);
    assert_eq!(
        engine.state().names.get(obj.characteristics().name),
        "Glasspool Shore"
    );
    assert!(
        obj.characteristics()
            .types
            .contains(baylee_core::types::TypeSet::LAND)
    );
}

#[test]
fn true_scriptures_casts_as_back_face() {
    let mut engine = Engine::new(
        &preset(
            13,
            vec![sheoldred()],
            vec![swamp(), swamp(), swamp(), swamp()],
        ),
        RegistryLookup,
    )
    .unwrap();
    keep_mulligans(&mut engine);
    let p0 = PlayerId::new(0);
    // Reach p0's main phase and start the cast.
    let mut guard = 0;
    loop {
        guard += 1;
        assert!(guard < 400, "no cast window found");
        match engine.pending().clone() {
            Pending::Priority { player, legal } if player == p0 => {
                // Tap all available mana first.
                if let Some(&source) = legal.mana_abilities.first() {
                    engine
                        .apply(player, PlayerAction::ActivateManaAbility { source })
                        .unwrap();
                    continue;
                }
                if let Some(&card) = legal.castable.iter().find(|id| {
                    engine.state().object(**id).unwrap().card.unwrap().index == sheoldred()
                }) {
                    engine
                        .apply(player, PlayerAction::CastSpell { card })
                        .unwrap();
                    break;
                }
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseAttackers { player } => {
                engine
                    .apply(player, PlayerAction::DeclareAttackers { attackers: vec![] })
                    .unwrap();
            }
            Pending::ChooseBlockers { player, .. } => {
                engine
                    .apply(player, PlayerAction::DeclareBlockers { blockers: vec![] })
                    .unwrap();
            }
            Pending::DiscardChoice { player, count } => {
                let hand: Vec<_> = engine
                    .state()
                    .zones
                    .list(crate::zone::ZoneLocation::Hand(player))
                    .clone();
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: hand[..count as usize].to_vec(),
                        },
                    )
                    .unwrap();
            }
            other => panic!("unexpected pending: {other:?}"),
        }
    }
    // The wizard offers normal (front) and back-face options.
    let Pending::ChooseCastMode { player, options } = engine.pending().clone() else {
        panic!("expected cast options, got {:?}", engine.pending());
    };
    let face_option = options
        .iter()
        .position(|o| matches!(o.kind, CastModeKind::Face(1)))
        .expect("back-face option offered");
    engine
        .apply(player, PlayerAction::ChooseMode(face_option))
        .unwrap();
    // The spell on the stack is The True Scriptures.
    let stack = engine.state().zones.list(crate::zone::ZoneLocation::Stack);
    let spell = stack.last().copied().expect("spell on the stack");
    let obj = engine.state().object(spell).unwrap();
    assert_eq!(obj.face_index, 1);
    assert_eq!(
        engine.state().names.get(obj.characteristics().name),
        "The True Scriptures"
    );
    assert!(
        obj.characteristics()
            .types
            .contains(baylee_core::types::TypeSet::ENCHANTMENT)
    );
}
