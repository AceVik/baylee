//! The human's one-shot auto distribution through the real rules engine.
use baylee_client_core::combat_auto::CombatAuto;
use baylee_core::{
    ids::{Defender, PlayerId, PrintRef},
    preset::DeckEntry,
};
use baylee_engine::{
    choice::{NumberPrompt, Pending, PlayerAction},
    engine::Engine,
};
use baylee_gamehost::{RegistryLookup, SeatContext, player_view};

/// CR 702.19b through the real engine: auto gives each blocker lethal, the
/// last's `min` holds, and the rest goes over them, which is what a game that
/// does not ask plays.
#[test]
fn auto_gives_a_tramplers_blockers_lethal_and_sends_the_rest_on() {
    let entry = |name| DeckEntry {
        rule_print: None,
        choices: baylee_core::card_metadata::DeckChoices::default(),
        card: baylee_cards::decks::by_name(name).unwrap(),
        print: PrintRef::new(0),
    };
    let mut preset = baylee_cards::decks::probe_preset(73, entry("Forest").card).unwrap();
    for seat in &mut preset.seats {
        seat.starting_hand = Some(vec![]);
        seat.starting_battlefield.clear();
    }
    preset.seats[0].starting_battlefield = vec![entry("Rootbreaker Wurm")];
    preset.seats[1].starting_battlefield = vec![entry("Grizzly Bears"); 2];
    preset.house_rules.divide_trample = true;
    let mut engine = Engine::new(&preset, RegistryLookup).unwrap();
    let them = PlayerId::new(1);
    let mut auto: Option<CombatAuto> = None;
    let mut asked = vec![];
    for seq in 0..80 {
        let pending = engine.pending().clone();
        let seat = baylee_ai::pending_player(&pending).unwrap();
        let view = player_view(
            engine.state(),
            seat,
            seq,
            Some(&pending),
            &SeatContext::default(),
            &[],
        );
        if !asked.is_empty() && !matches!(pending, Pending::ChooseNumber { .. }) {
            assert_eq!(asked, vec![(0, 2), (2, 2)], "each `min` and what auto sent");
            assert_eq!(engine.state().players[1].life, 18, "6 less 2 and 2");
            assert!(
                view.battlefield.iter().all(|o| o.name != "Grizzly Bears"),
                "both took lethal"
            );
            return;
        }
        let action = match &pending {
            Pending::Mulligan { .. } => PlayerAction::MulliganKeep,
            Pending::Priority { .. } => PlayerAction::PassPriority,
            Pending::ChooseAttackers { attackers, .. } => PlayerAction::DeclareAttackers {
                attackers: vec![(attackers[0], Defender::Player(them))],
            },
            Pending::ChooseBlockers { blockers, .. } => PlayerAction::DeclareBlockers {
                blockers: blockers
                    .iter()
                    .map(|option| (option.blocker, option.attackers[0]))
                    .collect(),
            },
            Pending::ChooseNumber { min, .. } => {
                let auto = auto.get_or_insert_with(|| CombatAuto::begin(&view, &pending).unwrap());
                let action = auto.answer(&view, &pending).unwrap();
                if let PlayerAction::ChooseNumber(n) = action {
                    asked.push((*min, n));
                }
                action
            }
            other => panic!("unexpected: {other:?}"),
        };
        engine.apply(seat, action).unwrap();
    }
    panic!("the combat did not finish");
}

#[test]
fn manual_first_share_then_auto_finishes_only_the_remaining_damage() {
    let entry = |name| DeckEntry {
        rule_print: None,
        choices: baylee_core::card_metadata::DeckChoices::default(),
        card: baylee_cards::decks::by_name(name).unwrap(),
        print: PrintRef::new(0),
    };
    let mut preset = baylee_cards::decks::probe_preset(73, entry("Forest").card).unwrap();
    for seat in &mut preset.seats {
        seat.starting_hand = Some(vec![]);
        seat.starting_battlefield.clear();
    }
    preset.seats[0].starting_battlefield = vec![entry("Craw Wurm")];
    preset.seats[1].starting_battlefield = vec![entry("Grizzly Bears"); 4];
    let mut engine = Engine::new(&preset, RegistryLookup).unwrap();
    let me = PlayerId::new(0);
    let them = PlayerId::new(1);
    let mut manually_assigned = false;
    let mut auto: Option<CombatAuto> = None;
    let mut automatic_shares = vec![];
    for seq in 0..80 {
        let pending = engine.pending().clone();
        let seat = baylee_ai::pending_player(&pending).unwrap();
        let view = player_view(
            engine.state(),
            seat,
            seq,
            Some(&pending),
            &SeatContext::default(),
            &[],
        );
        if manually_assigned && !matches!(pending, Pending::ChooseNumber { .. }) {
            assert_eq!(automatic_shares, vec![2, 2]);
            let bears: Vec<_> = view
                .battlefield
                .iter()
                .filter(|o| o.name == "Grizzly Bears")
                .collect();
            assert_eq!(
                bears.len(),
                2,
                "the first and last each received only one damage"
            );
            assert!(bears.iter().all(|o| o.damage == 1));
            assert!(!auto.as_ref().unwrap().accepts(&view, &pending));
            return;
        }
        let action = match &pending {
            Pending::Mulligan { .. } => PlayerAction::MulliganKeep,
            Pending::Priority { .. } => PlayerAction::PassPriority,
            Pending::ChooseAttackers { attackers, .. } => PlayerAction::DeclareAttackers {
                attackers: vec![(attackers[0], Defender::Player(them))],
            },
            Pending::ChooseBlockers { blockers, .. } => PlayerAction::DeclareBlockers {
                blockers: blockers
                    .iter()
                    .map(|option| (option.blocker, option.attackers[0]))
                    .collect(),
            },
            Pending::ChooseNumber {
                reason: NumberPrompt::CombatDamage { index, .. },
                ..
            } => {
                assert_eq!(seat, me);
                if *index == 0 {
                    manually_assigned = true;
                    PlayerAction::ChooseNumber(1)
                } else {
                    let auto =
                        auto.get_or_insert_with(|| CombatAuto::begin(&view, &pending).unwrap());
                    let action = auto.answer(&view, &pending).unwrap();
                    if let PlayerAction::ChooseNumber(n) = action {
                        automatic_shares.push(n);
                    }
                    action
                }
            }
            other => panic!("unexpected: {other:?}"),
        };
        engine.apply(seat, action).unwrap();
    }
    panic!("the combat did not finish");
}
