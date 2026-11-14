//! Asymmetric Tower opening resources obey the ordinary London procedure.

use super::testkit::{RegistryLookup, basic_forest, keep_mulligans, pass_until};
use super::*;
use baylee_core::ids::PrintRef;
use baylee_core::preset::{
    AIProfile, DeckEntry, Finish, FormatId, PrintInfo, SeatController, SeatOpening, SeatSpec,
};

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn preset(opening: Vec<SeatOpening>) -> GamePreset {
    let seat = SeatSpec {
        controller: SeatController::Ai(AIProfile::default()),
        capabilities: baylee_core::preset::SeatCapabilities::default(),
        deck: vec![
            DeckEntry {
                card: basic_forest(),
                print: PrintRef::new(0),
                rule_print: None,
                choices: baylee_core::card_metadata::DeckChoices::default()
            };
            60
        ],
        sideboard: vec![],
        commanders: vec![],
        starting_life: None,
        starting_hand: None,
        starting_battlefield: vec![],
        emblems: vec![],
        team: None,
    };
    GamePreset {
        rules: baylee_core::card_metadata::RulesMetadata::default(),
        format: FormatId::Custom,
        seed: 0x7801,
        house_rules: HouseRules {
            starting_player: Some(0),
            opening,
            ..HouseRules::default()
        },
        modifiers: vec![],
        prints: vec![PrintInfo {
            scryfall_id: uuid::Uuid::nil(),
            lang: "EN".into(),
            finish: Finish::Normal,
        }],
        seats: vec![seat.clone(), seat],
    }
}

fn bonus() -> SeatOpening {
    SeatOpening {
        seat: 0,
        cards: 8,
        maximum_hand: 8,
        free_mulligans: 3,
    }
}

fn hand(e: &Engine<RegistryLookup>, player: PlayerId) -> Vec<ObjectId> {
    e.state().zones.list(ZoneLocation::Hand(player)).clone()
}

#[test]
fn opening_bonus_redraws_eight_and_costs_cards_only_after_three_free_mulligans() {
    let mut e = Engine::new(&preset(vec![bonus()]), RegistryLookup).unwrap();
    assert_eq!(hand(&e, P0).len(), 8);
    assert_eq!(hand(&e, P1).len(), 7);
    for taken in 1..=4 {
        e.apply(P0, PlayerAction::MulliganTake).unwrap();
        assert_eq!(hand(&e, P0).len(), 8);
        assert!(
            matches!(e.pending_for(P0), Some(Pending::Mulligan { next_is_free, .. }) if *next_is_free == (taken < 3))
        );
    }
    e.apply(P0, PlayerAction::MulliganKeep).unwrap();
    assert!(matches!(
        e.pending_for(P0),
        Some(Pending::MulliganBottom { count: 1, .. })
    ));
    e.apply(
        P0,
        PlayerAction::ChooseObjects {
            objects: hand(&e, P0)[..1].to_vec(),
        },
    )
    .unwrap();
    assert_eq!(hand(&e, P0).len(), 7);
    e.apply(P1, PlayerAction::MulliganTake).unwrap();
    e.apply(P1, PlayerAction::MulliganTake).unwrap();
    e.apply(P1, PlayerAction::MulliganKeep).unwrap();
    assert!(matches!(
        e.pending_for(P1),
        Some(Pending::MulliganBottom { count: 1, .. })
    ));
}

#[test]
fn opening_limit_refuses_an_extra_redraw_without_changing_state() {
    let rule = SeatOpening {
        cards: 2,
        free_mulligans: 0,
        ..bonus()
    };
    let mut e = Engine::new(&preset(vec![rule]), RegistryLookup).unwrap();
    for _ in 0..2 {
        e.apply(P0, PlayerAction::MulliganTake).unwrap();
    }
    assert!(matches!(
        e.pending_for(P0),
        Some(Pending::Mulligan {
            can_take: false,
            ..
        })
    ));
    let before = e.fingerprint();
    assert!(e.apply(P0, PlayerAction::MulliganTake).is_err());
    assert_eq!(before, e.fingerprint());
    e.apply(P0, PlayerAction::MulliganKeep).unwrap();
    e.apply(
        P0,
        PlayerAction::ChooseObjects {
            objects: hand(&e, P0),
        },
    )
    .unwrap();
    assert!(hand(&e, P0).is_empty());
}

#[test]
fn opening_bonus_changes_cleanup_limit_only_for_its_owner() {
    let mut e = Engine::new(&preset(vec![bonus()]), RegistryLookup).unwrap();
    keep_mulligans(&mut e);
    assert_eq!(e.state().maximum_hand_size(P0), Some(8));
    assert_eq!(e.state().maximum_hand_size(P1), Some(7));
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::DiscardChoice { .. }) || e.state().turn.number > 1
    });
    assert_eq!(
        e.state().turn.number,
        2,
        "eight cards need no cleanup discard"
    );
    assert_eq!(hand(&e, P0).len(), 8);
}

#[test]
fn opening_allowance_is_replay_deterministic_and_part_of_continuation_hash() {
    let p = preset(vec![bonus()]);
    let mut a = Engine::new(&p, RegistryLookup).unwrap();
    let mut b = Engine::new(&p, RegistryLookup).unwrap();
    for action in [
        PlayerAction::MulliganTake,
        PlayerAction::MulliganTake,
        PlayerAction::MulliganKeep,
    ] {
        a.apply(P0, action.clone()).unwrap();
        b.apply(P0, action).unwrap();
        assert_eq!(a.snapshot_hash(), b.snapshot_hash());
    }
    let altered = preset(vec![SeatOpening {
        free_mulligans: 2,
        ..bonus()
    }]);
    let a = Engine::new(&p, RegistryLookup).unwrap();
    let b = Engine::new(&altered, RegistryLookup).unwrap();
    assert_eq!(a.state().snapshot_hash(), b.state().snapshot_hash());
    assert_ne!(a.snapshot_hash(), b.snapshot_hash());
}
