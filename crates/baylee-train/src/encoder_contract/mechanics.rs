//! The questions PR #410 added, asked by the engine on real tables and
//! answered through every encoder: the candidate is there, it is marked as
//! the slot it borrows is, a teacher's answer encodes to it, it decodes back
//! to that answer, and the engine takes the decoded answer.
//!
//! - Foretell, the special action (CR 702.143a), is a Priority candidate
//!   marked as suspend is, on the card's own row.
//! - The foretell cost is an alternative cost: cast mode slot 28.
//! - "Choose a card type" (Serra's Emissary) is a type choice: the subtype
//!   question's kind, one candidate per offered type in the engine's order.
//! - "Choose a nonland card name" (Cabal Therapy) offers, and so decodes, no
//!   land's name.

use baylee_core::mana::ManaCost;
use baylee_core::types::TypeSet;
use baylee_engine::choice::{CastModeDesc, CastModeKind, Pending, PlayerAction};

use super::{Encoder, P0, P1, card, scenes};
use crate::features4::{
    CAND_FEAT_WIDTH, EncodedCandidates4, HeadKind4, PendingKind4, TeacherTarget4, decode_action4,
    encode_executed_action4, encode_teacher_action4,
};
use crate::features7::encode_candidates7;

/// A candidate's features: zero but for `marks`.
fn marked(marks: &[(usize, f32)]) -> Vec<f32> {
    let mut feat = vec![0.0; CAND_FEAT_WIDTH];
    for &(slot, value) in marks {
        feat[slot] = value;
    }
    feat
}

/// A teacher's `action`, which must be a candidate.
fn target_for(
    candidates: &EncodedCandidates4,
    pending: &Pending,
    action: &PlayerAction,
) -> TeacherTarget4 {
    encode_teacher_action4(candidates, pending, action).expect("the action is a candidate")
}

fn foretell_is_offered_like_suspend_and_the_engine_takes_it(encoder: Encoder) {
    let (mut table, card) = scenes::foretell_on_offer();
    let (pending, view) = encoder.view_for(&table.session, P0);
    let Pending::Priority { legal, .. } = &pending else {
        panic!("priority, not {pending:?}")
    };
    assert_eq!(legal.foretellable, [card]);
    let (state, candidates) = encoder.encode(&view, &pending, None);
    let foretell = PlayerAction::Foretell { card };
    let index = candidates
        .candidates
        .iter()
        .position(|candidate| candidate.action.as_ref() == Some(&foretell))
        .expect("foretelling is a candidate");
    let candidate = &candidates.candidates[index];
    let row = state.find_entity(card).expect("the card has a row");
    assert_eq!(candidate.feat, marked(&[(3, 1.0), (18, 1.0)]));
    assert_eq!(candidate.entity_idx, row as i16);
    assert_eq!(candidate.card_id, state.entity_card_ids[row]);
    assert_eq!(state.entity_feat[row][27], 1.0, "the row is offered");

    let target = target_for(&candidates, &pending, &foretell);
    assert_eq!(usize::from(target.single_idx), index);
    assert_eq!(
        encode_executed_action4(&candidates, &pending, &foretell),
        Some(target.clone())
    );
    let decoded = decode_action4(&candidates, &view, &pending, &target).expect("decodes");
    assert_eq!(decoded, foretell);
    if encoder != Encoder::V4 {
        let announced = encode_candidates7(&state, &view, &pending).expect("the menu fits");
        assert!(
            announced
                .candidates
                .iter()
                .any(|candidate| candidate.action.as_ref() == Some(&foretell))
        );
    }
    assert_eq!(pending.answer_fault(&decoded), None);
    table
        .session
        .act(P0, decoded)
        .expect("the engine takes the decoded foretell");
    assert!(
        table
            .session
            .observe_view(P0)
            .hand
            .iter()
            .all(|object| object.id != card),
        "the card left the hand"
    );
}

fn foretold_card_is_cast_for_its_alternative_cost(encoder: Encoder) {
    let (mut table, card) = scenes::foretold_and_castable();
    let (pending, view) = encoder.view_for(&table.session, P0);
    let (_, candidates) = encoder.encode(&view, &pending, None);
    let cast = PlayerAction::CastSpell { card };
    let target = encode_teacher_action4(&candidates, &pending, &cast)
        .expect("the foretold card is a cast candidate");
    let decoded = decode_action4(&candidates, &view, &pending, &target).expect("decodes");
    assert_eq!(decoded, cast);

    // Where the foretell cost is one way of several to cast the card, it is
    // the alternative cost it is: the same features as an alternative at
    // the same price, but for the option's index (slot 15). The card
    // structure catalog has no foretell entry, so it names no extra rule.
    let cost = ManaCost::parse("{1}{W}{W}");
    let modes = Pending::ChooseCastMode {
        player: P0,
        object: card,
        options: vec![
            CastModeDesc {
                index: 0,
                kind: CastModeKind::Foretell,
                cost,
            },
            CastModeDesc {
                index: 1,
                kind: CastModeKind::Alternative(0),
                cost,
            },
        ],
    };
    let (_, options) = encoder.encode(&view, &modes, None);
    assert_eq!(options.pending_kind, PendingKind4::ChooseCastMode);
    let [foretold, alternative] = options.candidates.as_slice() else {
        panic!("two cast options, not {}", options.candidates.len())
    };
    assert_eq!(foretold.feat[14], 1.0);
    assert_eq!(foretold.feat[28], 1.0);
    let unindexed = |feat: &[f32]| {
        let mut feat = feat.to_vec();
        feat[15] = 0.0;
        feat
    };
    assert_eq!(unindexed(&foretold.feat), unindexed(&alternative.feat));
    assert!(foretold.extra_rules.is_empty());
    let choose = PlayerAction::ChooseMode(0);
    let mode = target_for(&options, &modes, &choose);
    assert_eq!(mode.single_idx, 0);
    assert_eq!(decode_action4(&options, &view, &modes, &mode), Some(choose));

    // The engine takes the decoded cast. Foretell alone grants one way to
    // cast the card, so it asks no mode; were it to, the answer is the
    // foretell option.
    table.session.act(P0, decoded).expect("the engine casts it");
    if let Pending::ChooseCastMode { options, .. } = table.session.pending().clone() {
        let index = options
            .iter()
            .position(|option| option.kind == CastModeKind::Foretell)
            .expect("the foretell option");
        table
            .session
            .act(P0, PlayerAction::ChooseMode(index))
            .expect("the foretell cost");
    }
    assert!(
        table
            .session
            .observe_view(P0)
            .stack
            .iter()
            .any(|object| object.id == card),
        "Doomskar is on the stack"
    );
}

fn card_type_question_offers_the_fifteen_types(encoder: Encoder) {
    let mut table = scenes::card_type_asked();
    let (pending, view) = encoder.view_for(&table.session, P0);
    let Pending::ChooseCardType { options, .. } = &pending else {
        panic!("a card type, not {pending:?}")
    };
    assert_eq!(options.as_slice(), TypeSet::CARD_TYPES.as_slice());
    let (state, candidates) = encoder.encode(&view, &pending, None);
    // The subtype question's kind, in the candidates and in the state.
    assert_eq!(candidates.pending_kind, PendingKind4::ChooseSubtype);
    assert_eq!(candidates.head, HeadKind4::SingleChoice);
    let subtype = Pending::ChooseSubtype {
        player: P0,
        options: Vec::new(),
    };
    assert_eq!(
        state.global_feat[34],
        encoder.state(&view, Some(&subtype), None).global_feat[34]
    );
    assert_eq!(candidates.candidates.len(), 15);
    for (index, (candidate, &card_type)) in (0_u8..).zip(candidates.candidates.iter().zip(options))
    {
        let answer = PlayerAction::ChooseCardType(card_type);
        assert_eq!(candidate.action.as_ref(), Some(&answer));
        assert_eq!(
            candidate.feat,
            marked(&[(15, 1.0), (18, f32::from(index) / 16.0)])
        );
        assert_eq!(candidate.entity_idx, -1);
        let target = target_for(&candidates, &pending, &answer);
        assert_eq!(target.single_idx, u16::from(index));
        let decoded = decode_action4(&candidates, &view, &pending, &target).expect("decodes");
        assert_eq!(decoded, answer);
        assert_eq!(pending.answer_fault(&decoded), None);
    }
    let creature = PlayerAction::ChooseCardType(TypeSet::CREATURE);
    let decoded = decode_action4(
        &candidates,
        &view,
        &pending,
        &target_for(&candidates, &pending, &creature),
    )
    .expect("decodes");
    table
        .session
        .act(P0, decoded)
        .expect("the engine takes the decoded type");
    assert!(!matches!(
        table.session.pending(),
        Pending::ChooseCardType { .. }
    ));
}

fn nonland_naming_never_offers_or_decodes_a_land(encoder: Encoder) {
    let mut table = scenes::nonland_name_asked();
    let (pending, view) = encoder.view_for(&table.session, P0);
    let Pending::ChooseCardName {
        types,
        excluded_types,
        ..
    } = &pending
    else {
        panic!("a card name, not {pending:?}")
    };
    assert_eq!((*types, *excluded_types), (TypeSet::EMPTY, TypeSet::LAND));
    let (_, candidates) = encoder.encode(&view, &pending, None);
    assert_eq!(candidates.pending_kind, PendingKind4::ChooseCardName);
    let nonland_faces: usize = baylee_cards::all()
        .map(|def| {
            def.faces
                .iter()
                .filter(|face| !face.types.intersects(TypeSet::LAND))
                .count()
        })
        .sum();
    assert_eq!(candidates.candidates.len(), nonland_faces);
    for index in 0..candidates.candidates.len() {
        let target = TeacherTarget4 {
            single_idx: u16::try_from(index).expect("fits the head"),
            ..TeacherTarget4::default()
        };
        let Some(PlayerAction::ChooseCardName { card, face }) =
            decode_action4(&candidates, &view, &pending, &target)
        else {
            panic!("candidate {index} decodes to a name")
        };
        let printed = &baylee_cards::by_index(card).expect("a pool card").faces[usize::from(face)];
        assert!(
            !printed.types.intersects(TypeSet::LAND),
            "{} decodes",
            printed.name
        );
    }
    let forest = PlayerAction::ChooseCardName {
        card: card("Forest").card,
        face: 0,
    };
    assert_eq!(encode_teacher_action4(&candidates, &pending, &forest), None);
    assert_eq!(
        encode_executed_action4(&candidates, &pending, &forest),
        None
    );
    // What the candidates leave out is what the engine refuses, and the
    // refusal leaves the question open.
    assert!(
        table.session.act(P0, forest).is_err(),
        "the engine refuses a land"
    );
    assert!(matches!(
        table.session.pending(),
        Pending::ChooseCardName { .. }
    ));

    let bears = PlayerAction::ChooseCardName {
        card: card("Grizzly Bears").card,
        face: 0,
    };
    let decoded = decode_action4(
        &candidates,
        &view,
        &pending,
        &target_for(&candidates, &pending, &bears),
    )
    .expect("decodes");
    assert_eq!(decoded, bears);
    table
        .session
        .act(P0, decoded)
        .expect("the engine takes the decoded name");
    assert!(
        table
            .session
            .observe_view(P1)
            .hand
            .iter()
            .all(|object| object.name != "Grizzly Bears"),
        "the named card was discarded"
    );
}

/// One test per encoder and question, so a failure names both.
macro_rules! per_encoder {
    ($($module:ident => $encoder:expr),* $(,)?) => {$(
        mod $module {
            use super::*;

            #[test]
            fn foretell_is_offered_like_suspend_and_the_engine_takes_it() {
                super::foretell_is_offered_like_suspend_and_the_engine_takes_it($encoder);
            }

            #[test]
            fn foretold_card_is_cast_for_its_alternative_cost() {
                super::foretold_card_is_cast_for_its_alternative_cost($encoder);
            }

            #[test]
            fn card_type_question_offers_the_fifteen_types() {
                super::card_type_question_offers_the_fifteen_types($encoder);
            }

            #[test]
            fn nonland_naming_never_offers_or_decodes_a_land() {
                super::nonland_naming_never_offers_or_decodes_a_land($encoder);
            }
        }
    )*};
}

per_encoder!(
    encoder4 => Encoder::V4,
    encoder7 => Encoder::V7,
    encoder8 => Encoder::V8,
    encoder9 => Encoder::V9,
);
