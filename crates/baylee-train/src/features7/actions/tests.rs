//! Real rules plumbing; explicit legal choices, not learned tactical strength.
use super::*;
use crate::features4::{TeacherTarget4, decode_action4, encode_state4};
use crate::housedeck::HouseDeck;
use baylee_core::ids::{CardIndex, ObjectId, PlayerId, PrintRef};
use baylee_core::preset::{AIProfile, DeckEntry, FormatId};
use baylee_gamehost::Session;
use baylee_view::Phase;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn entry(name: &str) -> DeckEntry {
    let card: CardIndex = baylee_cards::all()
        .find(|card| card.name() == name)
        .unwrap()
        .index;
    DeckEntry {
        card,
        print: PrintRef::new(0),
        rule_print: None,
        choices: baylee_core::card_metadata::DeckChoices::default(),
    }
}

fn game(hands: [&[&str]; 2], boards: [&[&str]; 2]) -> Session {
    let deck = HouseDeck::named("victory").unwrap();
    let mut preset = crate::selfplay::table_for(12, &[&deck, &deck], &[AIProfile::STEADY; 2], &[]);
    preset.format = FormatId::Freeform;
    preset.modifiers.clear();
    for (index, seat) in preset.seats.iter_mut().enumerate() {
        seat.deck = vec![entry("Forest"); 32];
        seat.commanders.clear();
        seat.sideboard.clear();
        seat.starting_hand = Some(hands[index].iter().map(|name| entry(name)).collect());
        seat.starting_battlefield = boards[index].iter().map(|name| entry(name)).collect();
        seat.starting_life = Some(20);
    }
    let mut session = Session::new(&preset).unwrap();
    assert!(session.take_over(P0));
    assert!(session.take_over(P1));
    session
}

fn apply(session: &mut Session, action: PlayerAction) {
    let player = session.pending().asked().unwrap();
    let (pending, view) = session.view_for_with_context(player).unwrap();
    let original = serde_json::to_value(&pending).unwrap();
    let state = encode_state4(&view, Some(&pending), None, &[]);
    let menu = encode_candidates7(&state, &view, &pending).unwrap();
    let index = menu
        .candidates
        .iter()
        .position(|candidate| candidate.action.as_ref() == Some(&action))
        .unwrap_or_else(|| panic!("missing {action:?} in {pending:?}"));
    let target = TeacherTarget4 {
        single_idx: u16::try_from(index).unwrap(),
        ..TeacherTarget4::default()
    };
    let decoded = decode_action4(&menu, &view, &pending, &target).unwrap();
    assert_eq!(decoded, action);
    if decoded == PlayerAction::CancelCast {
        // Engine::apply checks this owner-only sideband before Pending's
        // ordinary answer contract (engine/actions.rs). The context is the
        // offered permission; successful Session::act below is authoritative.
        assert!(view.casting.is_some());
        assert_eq!(pending.asked(), Some(player));
    } else {
        assert!(pending.answer_fault(&decoded).is_none());
    }
    assert_eq!(serde_json::to_value(&pending).unwrap(), original);
    session.act(player, action).unwrap();
}

fn main_phase(session: &mut Session) {
    for _ in 0..64 {
        if session.observe_view(P0).phase == Phase::FirstMain {
            return;
        }
        let action = match session.pending() {
            Pending::Mulligan { .. } => PlayerAction::MulliganKeep,
            Pending::Priority { .. } => PlayerAction::PassPriority,
            other => panic!("unexpected opening {other:?}"),
        };
        apply(session, action);
    }
    panic!("main phase not reached");
}

fn announce(session: &mut Session, name: &str) -> ObjectId {
    let player = session.pending().asked().unwrap();
    let (pending, view) = session.view_for_with_context(player).unwrap();
    assert_eq!(view.seats[usize::from(player.get())].mana_pool.total(), 0);
    let spell = view.hand.iter().find(|card| card.name == name).unwrap().id;
    let Pending::Priority { legal, .. } = &pending else {
        panic!("cast priority");
    };
    assert!(legal.payable.contains(&spell));
    assert!(!legal.castable.contains(&spell));
    let state = encode_state4(&view, Some(&pending), None, &[]);
    let old = encode_candidates4(&state, &view, &pending);
    let new = encode_candidates7(&state, &view, &pending).unwrap();
    let old_metadata = serde_json::to_value(&old).unwrap();
    let new_metadata = serde_json::to_value(&new).unwrap();
    for field in [
        "head",
        "pending_kind",
        "min_choices",
        "max_choices",
        "pairs",
        "pair_objects",
    ] {
        assert_eq!(
            old_metadata[field], new_metadata[field],
            "unchanged legal metadata {field}"
        );
    }
    assert_eq!(
        serde_json::to_value(&new.candidates[..old.candidates.len()]).unwrap(),
        serde_json::to_value(&old.candidates).unwrap(),
        "legacy menu is an exact prefix"
    );
    assert!(
        !old.candidates
            .iter()
            .any(|candidate| candidate.action == Some(PlayerAction::CastSpell { card: spell }))
    );
    apply(session, PlayerAction::CastSpell { card: spell });
    while matches!(session.pending(), Pending::ChooseCastMode { .. }) {
        apply(session, PlayerAction::ChooseMode(0));
    }
    spell
}

fn pay(session: &mut Session, spell: ObjectId) {
    let player = session.pending().asked().unwrap();
    let view = session.observe_view_with_context(player);
    assert_eq!(view.casting, Some(spell));
    assert!(view.owed.is_some());
    let Pending::Priority { legal, .. } = session.pending() else {
        panic!("payment priority");
    };
    let sources = legal.mana_abilities.clone();
    assert!(!sources.is_empty());
    for source in sources {
        apply(session, PlayerAction::ActivateManaAbility { source });
    }
    if session.observe_view_with_context(player).owed.is_some() {
        apply(session, PlayerAction::PassPriority);
    }
    let view = session.observe_view_with_context(player);
    assert!(view.owed.is_none());
    assert!(view.casting.is_none());
    assert_eq!(view.seats[usize::from(player.get())].mana_pool.total(), 0);
    assert!(view.stack.iter().any(|row| row.id == spell));
}

fn resolve(session: &mut Session) {
    for _ in 0..16 {
        if session.observe_view(P0).stack.is_empty() {
            return;
        }
        apply(session, PlayerAction::PassPriority);
    }
    panic!("stack did not resolve");
}

fn graveyard(session: &Session, player: PlayerId, name: &str) -> bool {
    session.observe_view(P0).graveyards[usize::from(player.get())]
        .iter()
        .any(|card| card.name == name)
}

#[test]
fn announce_creature_before_tapping_then_pay_and_resolve() {
    let mut session = game([&["Llanowar Elves"], &[]], [&["Forest"], &[]]);
    main_phase(&mut session);
    let spell = announce(&mut session, "Llanowar Elves");
    pay(&mut session, spell);
    resolve(&mut session);
    assert!(
        session
            .observe_view(P0)
            .battlefield
            .iter()
            .any(|row| row.id == spell)
    );
}

#[test]
fn opponent_turn_instant_announces_targets_then_makes_mana() {
    let mut session = game([&[], &["Shock"]], [&[], &["Mountain"]]);
    main_phase(&mut session);
    apply(&mut session, PlayerAction::PassPriority);
    assert_eq!(session.pending().asked(), Some(P1));
    assert_eq!(session.observe_view(P1).active, P0);
    let spell = announce(&mut session, "Shock");
    apply(
        &mut session,
        PlayerAction::ChooseTargets {
            objects: vec![],
            players: vec![P0],
        },
    );
    pay(&mut session, spell);
    resolve(&mut session);
    assert_eq!(session.observe_view(P0).seats[0].life, 18);
    assert!(graveyard(&session, P1, "Shock"));
}

#[test]
fn counterspell_announces_before_two_islands_targets_stack_and_counters() {
    let mut session = game(
        [&["Llanowar Elves"], &["Counterspell"]],
        [&["Forest"], &["Island", "Island"]],
    );
    main_phase(&mut session);
    let creature = announce(&mut session, "Llanowar Elves");
    pay(&mut session, creature);
    apply(&mut session, PlayerAction::PassPriority);
    let counter = announce(&mut session, "Counterspell");
    apply(
        &mut session,
        PlayerAction::ChooseTargets {
            objects: vec![creature],
            players: vec![],
        },
    );
    pay(&mut session, counter);
    resolve(&mut session);
    assert!(graveyard(&session, P0, "Llanowar Elves"));
    assert!(graveyard(&session, P1, "Counterspell"));
    assert!(
        !session
            .observe_view(P0)
            .battlefield
            .iter()
            .any(|row| row.id == creature)
    );
}

#[test]
fn cancel_is_encoded_during_payment_and_returns_announced_card() {
    let mut session = game([&["Llanowar Elves"], &[]], [&["Forest"], &[]]);
    main_phase(&mut session);
    let spell = announce(&mut session, "Llanowar Elves");
    assert_eq!(session.observe_view_with_context(P0).casting, Some(spell));
    apply(&mut session, PlayerAction::CancelCast);
    assert!(session.observe_view_with_context(P0).casting.is_none());
    assert!(session.observe_view_with_context(P0).owed.is_none());
    assert!(
        session
            .observe_view(P0)
            .hand
            .iter()
            .any(|row| row.id == spell)
    );
}

#[test]
fn overlapping_offers_deduplicate_and_cap_errors_instead_of_truncating() {
    let mut session = game([&["Llanowar Elves"], &[]], [&["Forest"], &[]]);
    main_phase(&mut session);
    let (mut pending, view) = session.view_for_with_context(P0).unwrap();
    let Pending::Priority { legal, .. } = &mut pending else {
        panic!("priority");
    };
    let spell = legal.payable[0];
    legal.castable.push(spell);
    legal.payable.push(spell);
    let state = encode_state4(&view, Some(&pending), None, &[]);
    let menu = encode_candidates7(&state, &view, &pending).unwrap();
    assert_eq!(
        menu.candidates
            .iter()
            .filter(|candidate| candidate.action == Some(PlayerAction::CastSpell { card: spell }))
            .count(),
        1
    );
    let Pending::Priority { legal, .. } = &mut pending else {
        unreachable!()
    };
    legal.castable = vec![spell; MAX_CANDIDATES];
    assert!(encode_candidates7(&state, &view, &pending).is_err());
}

#[test]
fn non_priority_menu_is_exactly_legacy() {
    let session = game([&["Llanowar Elves"], &[]], [&["Forest"], &[]]);
    let (pending, view) = session.view_for_with_context(P0).unwrap();
    assert!(matches!(pending, Pending::Mulligan { .. }));
    let state = encode_state4(&view, Some(&pending), None, &[]);
    assert_eq!(
        serde_json::to_value(encode_candidates4(&state, &view, &pending)).unwrap(),
        serde_json::to_value(encode_candidates7(&state, &view, &pending).unwrap()).unwrap()
    );
}

#[test]
fn target_question_cancellation_is_legal_sideband_but_not_a_fake_target() {
    let mut session = game([&["Shock"], &[]], [&["Mountain"], &[]]);
    main_phase(&mut session);
    let spell = announce(&mut session, "Shock");
    let (pending, view) = session.view_for_with_context(P0).unwrap();
    assert!(matches!(pending, Pending::ChooseTargets { .. }));
    assert_eq!(view.casting, Some(spell));
    let state = encode_state4(&view, Some(&pending), None, &[]);
    let menu = encode_candidates7(&state, &view, &pending).unwrap();
    assert!(
        !menu
            .candidates
            .iter()
            .any(|candidate| candidate.action == Some(PlayerAction::CancelCast))
    );
    assert!(pending.answer_fault(&PlayerAction::CancelCast).is_some());
    // This verifies the engine's separate permission; it does not claim the
    // current neural target head can select the cancellation sideband.
    session.act(P0, PlayerAction::CancelCast).unwrap();
    let view = session.observe_view_with_context(P0);
    assert!(view.casting.is_none());
    assert!(view.hand.iter().any(|row| row.id == spell));
}

#[test]
fn encode_state7_groups_identical_permanents_and_deduplicates_candidates() {
    use crate::features4::{EntityZone4, encode_teacher_action4};
    use crate::features7::{
        ENTITY_FEAT_WIDTH7, GLOBAL_FEAT_WIDTH7, encode_candidates7_grouped, encode_state7,
    };

    let mut session = game(
        [&["Llanowar Elves"], &[]],
        [
            &["Forest", "Forest", "Forest", "Forest", "Forest"],
            &["Mountain", "Mountain", "Mountain"],
        ],
    );
    main_phase(&mut session);
    let (pending, view) = session.view_for_with_context(P0).unwrap();
    let state = encode_state7(&view, Some(&pending), None, &[]);
    assert_eq!(state.global_feat.len(), GLOBAL_FEAT_WIDTH7);
    assert!(
        state
            .entity_feat
            .iter()
            .all(|f| f.len() == ENTITY_FEAT_WIDTH7)
    );

    // 5 identical Forests + 3 identical Mountains = 2 battlefield entity tokens (instead of 8).
    let bf_indices: Vec<usize> = state
        .entity_zone_ids
        .iter()
        .enumerate()
        .filter_map(|(i, &z)| (z == EntityZone4::Battlefield as u8).then_some(i))
        .collect();
    assert_eq!(bf_indices.len(), 2);

    let forest_ent_idx = bf_indices
        .iter()
        .copied()
        .find(|&i| state.entity_group_members[i].len() == 5)
        .expect("grouped Forest entity");
    assert!((state.entity_feat[forest_ent_idx][122] - 5.0).abs() < 1e-6);

    let menu = encode_candidates7_grouped(&state, &view, &pending);
    let mana_cands: Vec<_> = menu
        .candidates
        .iter()
        .enumerate()
        .filter(|(_, c)| matches!(c.action, Some(PlayerAction::ActivateManaAbility { .. })))
        .collect();
    assert_eq!(
        mana_cands.len(),
        1,
        "5 identical Forests must deduplicate to 1 ActivateManaAbility candidate"
    );
    let (mana_cand_idx, mana_cand) = mana_cands[0];
    assert_eq!(mana_cand.group_object_ids.len(), 5);

    // Teacher action for ANY of the 5 Forest ObjectIds must map to `mana_cand_idx`.
    for &forest_id in &mana_cand.group_object_ids {
        let teacher = encode_teacher_action4(
            &menu,
            &pending,
            &PlayerAction::ActivateManaAbility { source: forest_id },
        )
        .expect("teacher action maps to grouped candidate");
        assert_eq!(usize::from(teacher.single_idx), mana_cand_idx);
    }
}
