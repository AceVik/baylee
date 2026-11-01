//! Group decoding must respect the actor's already-announced payment source.
use super::*;
use baylee_core::ids::PrintRef;
use baylee_core::preset::{AIProfile, DeckEntry, FormatId};
use baylee_view::Phase;

const ME: PlayerId = PlayerId::new(0);

fn entry(name: &str) -> DeckEntry {
    DeckEntry {
        card: baylee_cards::decks::by_name(name).unwrap(),
        print: PrintRef::new(0),
        rule_print: None,
        choices: baylee_core::card_metadata::DeckChoices::default(),
    }
}

fn session(board: &[&str]) -> baylee_gamehost::Session {
    let deck = crate::housedeck::HouseDeck::named("victory").unwrap();
    let mut preset = crate::selfplay::table_for(12, &[&deck, &deck], &[AIProfile::STEADY; 2], &[]);
    preset.format = FormatId::Freeform;
    preset.modifiers.clear();
    for (index, seat) in preset.seats.iter_mut().enumerate() {
        seat.deck = vec![entry("Forest"); 32];
        seat.starting_hand = Some(vec![]);
        seat.commanders.clear();
        seat.sideboard.clear();
        seat.starting_battlefield = if index == 0 {
            board.iter().map(|name| entry(name)).collect()
        } else {
            vec![]
        };
    }
    let mut session = baylee_gamehost::Session::new(&preset).unwrap();
    session.take_over(ME);
    session.take_over(PlayerId::new(1));
    for _ in 0..64 {
        if session.observe_view(ME).phase == Phase::FirstMain {
            return session;
        }
        let player = session.pending().asked().unwrap();
        let action = match session.pending() {
            Pending::Mulligan { .. } => PlayerAction::MulliganKeep,
            Pending::Priority { .. } => PlayerAction::PassPriority,
            other => panic!("unexpected opening {other:?}"),
        };
        session.act(player, action).unwrap();
    }
    panic!("main phase not reached");
}

fn activation_payment() -> (baylee_gamehost::Session, ObjectId, ObjectId) {
    let mut session = session(&[
        "Kessig Wolf Run",
        "Kessig Wolf Run",
        "Forest",
        "Mountain",
        "Grizzly Bears",
    ]);
    let view = session.observe_view(ME);
    for name in ["Forest", "Mountain"] {
        let source = view
            .battlefield
            .iter()
            .find(|row| row.name == name)
            .unwrap()
            .id;
        session
            .act(ME, PlayerAction::ActivateManaAbility { source })
            .unwrap();
    }
    let (pending, view) = session.view_for_with_context(ME).unwrap();
    let bear = view
        .battlefield
        .iter()
        .find(|row| row.name == "Grizzly Bears")
        .unwrap()
        .id;
    let state = encode_state7(&view, Some(&pending), None, &[]);
    let menu = encode_candidates7_grouped(&state, &view, &pending);
    let reserved = menu
        .candidates
        .iter()
        .find_map(|candidate| match candidate.action {
            Some(PlayerAction::ActivateAbility {
                source,
                ability_index: 1,
            }) => Some(source),
            _ => None,
        })
        .expect("fixed RG is affordable before announcing X");
    session
        .act(
            ME,
            PlayerAction::ActivateAbility {
                source: reserved,
                ability_index: 1,
            },
        )
        .unwrap();
    for _ in 0..8 {
        let action = match session.pending() {
            Pending::ChooseNumber { .. } => PlayerAction::ChooseNumber(1),
            Pending::ChooseTargets { .. } => PlayerAction::ChooseTargets {
                objects: vec![bear],
                players: vec![],
            },
            Pending::Priority { .. } => break,
            other => panic!("unexpected activation question {other:?}"),
        };
        session.act(ME, action).unwrap();
    }
    let view = session.observe_view_with_context(ME);
    assert!(view.owed.is_some());
    assert!(view.stack.is_empty(), "activation is not yet on stack");
    assert!(
        view.casting.is_none(),
        "spell-only context cannot identify activation"
    );
    (session, reserved, bear)
}

fn payment_menu(
    session: &baylee_gamehost::Session,
    reserved: ObjectId,
) -> (Pending, PlayerView, EncodedCandidates4, usize) {
    let (pending, view) = session.view_for_with_context(ME).unwrap();
    let state = encode_state7(&view, Some(&pending), None, &[]);
    let menu = encode_candidates7_grouped(&state, &view, &pending);
    let index = menu
        .candidates
        .iter()
        .position(|candidate| {
            candidate.group_object_ids.len() == 2
                && candidate.action
                    == Some(PlayerAction::ActivateAbility {
                        source: reserved,
                        ability_index: 0,
                    })
        })
        .expect("two untapped mana sources collapse onto reserved canonical member");
    (pending, view, menu, index)
}

#[test]
fn grouped_mana_preserves_announced_tap_self_activation_in_both_modes() {
    for mode in [PolicyMode4::Raw, PolicyMode4::Tactical] {
        let (mut session, reserved, bear) = activation_payment();
        let (pending, view, menu, group_index) = payment_menu(&session, reserved);
        let mut logits = vec![-2.0; menu.candidates.len()];
        logits[group_index] = 2.0;
        let mut rng = Rng::new(77);
        let (action, _, lp, target) = PolicySampler4 {
            mode,
            temperature: 0.8,
            rng: &mut rng,
            active_source: Some((reserved, Some(1))),
        }
        .select(
            &menu,
            &view,
            &pending,
            PolicyOutputs4 {
                cand_logits: &logits,
                pair_logits: &[],
                scalar_pred: 0.5,
                value: 0.5,
            },
        )
        .unwrap();
        assert_eq!(usize::from(target.single_idx), group_index);
        assert_eq!(
            crate::features4::encode_teacher_action4(&menu, &pending, &action)
                .expect("executed member maps back to the same recorded group")
                .single_idx,
            target.single_idx
        );
        let PlayerAction::ActivateAbility {
            source: used,
            ability_index: 0,
        } = action
        else {
            panic!("grouped mana action");
        };
        assert_ne!(
            used, reserved,
            "announced TapSelf source must remain available"
        );
        if mode == PolicyMode4::Raw {
            let mut rng = Rng::new(77);
            let (index, expected_lp) = sample_categorical(&logits, 0.8, &mut rng);
            assert_eq!(index, group_index);
            assert_eq!(lp, expected_lp, "member decode preserves group probability");
        }
        session
            .act(
                ME,
                PlayerAction::ActivateAbility {
                    source: used,
                    ability_index: 0,
                },
            )
            .unwrap();
        session.act(ME, PlayerAction::PassPriority).unwrap();
        for _ in 0..8 {
            if session.observe_view(ME).stack.is_empty() {
                break;
            }
            let player = session.pending().asked().unwrap();
            session.act(player, PlayerAction::PassPriority).unwrap();
        }
        assert_eq!(
            session.observe_view(ME).object(bear).unwrap().power,
            Some(3),
            "X=1 pump resolved"
        );
        assert!(
            session
                .observe_view(ME)
                .object(reserved)
                .unwrap()
                .status
                .contains(baylee_view::ObjectStatus::TAPPED)
        );
    }
}

#[test]
fn tapping_canonical_reserved_member_reverses_the_real_activation() {
    let (mut session, reserved, bear) = activation_payment();
    let (_, _, menu, index) = payment_menu(&session, reserved);
    let action = menu.candidates[index].action.clone().unwrap();
    session.act(ME, action).unwrap();
    session.act(ME, PlayerAction::PassPriority).unwrap();
    let view = session.observe_view(ME);
    assert!(view.stack.is_empty());
    assert_eq!(
        view.object(bear).unwrap().power,
        Some(2),
        "announced pump was reversed"
    );
    assert!(
        !view
            .object(reserved)
            .unwrap()
            .status
            .contains(baylee_view::ObjectStatus::TAPPED)
    );
}

#[test]
fn empty_pool_mind_stone_draw_is_not_an_offered_activation() {
    let session = session(&["Mind Stone", "Mind Stone"]);
    let (pending, view) = session.view_for_with_context(ME).unwrap();
    let source = view
        .battlefield
        .iter()
        .find(|row| row.name == "Mind Stone")
        .unwrap()
        .id;
    let action = PlayerAction::ActivateAbility {
        source,
        ability_index: 1,
    };
    assert!(
        pending.answer_fault(&action).is_some(),
        "unpaid_abilities are only planning hints"
    );
    let state = encode_state7(&view, Some(&pending), None, &[]);
    let menu = encode_candidates7_grouped(&state, &view, &pending);
    assert!(
        !menu
            .candidates
            .iter()
            .any(|candidate| candidate.action == Some(action.clone()))
    );
}
