//! The policy answers PR #410's questions itself: when the net's logits
//! favour an answer, both sampling modes play it, decoded from every
//! encoder's candidates, and no tactical guard masks it. Nothing reaches the
//! house fallback.
use super::*;
use crate::encoder_contract::{Encoder, P0, scenes};

/// What the policy plays, greedily, when the logits favour `answer`.
fn favoured(
    mode: PolicyMode4,
    view: &PlayerView,
    pending: &Pending,
    candidates: &EncodedCandidates4,
    answer: &PlayerAction,
) -> PlayerAction {
    let index = candidates
        .candidates
        .iter()
        .position(|candidate| candidate.action.as_ref() == Some(answer))
        .expect("the answer is a candidate");
    let mut logits = vec![0.0; candidates.candidates.len()];
    logits[index] = 8.0;
    let mut rng = Rng::new(410);
    let (action, _, _, target) = PolicySampler4 {
        mode,
        temperature: 0.0,
        rng: &mut rng,
        active_source: None,
    }
    .select(
        candidates,
        view,
        pending,
        PolicyOutputs4 {
            cand_logits: &logits,
            pair_logits: &[],
            scalar_pred: 0.0,
            value: 0.5,
        },
    )
    .expect("the policy answers");
    assert_eq!(usize::from(target.single_idx), index);
    action
}

/// Every encoder and mode plays `answer` to the question `table` is asked.
fn plays(table: &crate::encoder_contract::Table, answer: &PlayerAction) {
    for encoder in Encoder::ALL {
        let (pending, view) = encoder.view_for(&table.session, P0);
        let (_, candidates) = encoder.encode(&view, &pending, None);
        for mode in [PolicyMode4::Raw, PolicyMode4::Tactical] {
            assert_eq!(
                &favoured(mode, &view, &pending, &candidates, answer),
                answer,
                "{encoder:?}, {mode:?}"
            );
        }
    }
}

#[test]
fn the_policy_foretells() {
    let (table, card) = scenes::foretell_on_offer();
    plays(&table, &PlayerAction::Foretell { card });
}

#[test]
fn the_policy_chooses_a_card_type() {
    let table = scenes::card_type_asked();
    plays(
        &table,
        &PlayerAction::ChooseCardType(baylee_core::types::TypeSet::CREATURE),
    );
}

#[test]
fn the_policy_names_a_nonland_card() {
    let table = scenes::nonland_name_asked();
    let bears = baylee_cards::all()
        .find(|card| card.name() == "Grizzly Bears")
        .expect("Grizzly Bears")
        .index;
    plays(
        &table,
        &PlayerAction::ChooseCardName {
            card: bears,
            face: 0,
        },
    );
}
