use super::*;
use baylee_core::preset::AIProfile;
use baylee_engine::choice::{BlockOption, NumberPrompt};

fn view() -> PlayerView {
    let deck = crate::housedeck::HouseDeck::named("victory").unwrap();
    let mut preset = crate::selfplay::table_for(12, &[&deck, &deck], &[AIProfile::STEADY; 2], &[]);
    preset.format = baylee_core::preset::FormatId::Freeform;
    preset.modifiers.clear();
    let spell = baylee_cards::all()
        .find(|card| card.name() == "Shock")
        .unwrap()
        .index;
    for seat in &mut preset.seats {
        let mut entry = seat.deck[0];
        entry.card = spell;
        entry.rule_print = None;
        entry.choices = baylee_core::card_metadata::DeckChoices::default();
        seat.starting_hand = Some(vec![entry; 3]);
    }
    baylee_gamehost::Session::new(&preset)
        .unwrap()
        .observe_view(PlayerId::new(0))
}

fn menu(view: &PlayerView, pending: &Pending) -> EncodedCandidates4 {
    let state = encode_state4(view, Some(pending), None, &[]);
    encode_candidates4(&state, view, pending)
}

fn select(
    mode: PolicyMode4,
    view: &PlayerView,
    pending: &Pending,
    cands: &EncodedCandidates4,
    logits: &[f32],
    scalar: f32,
) -> (PlayerAction, f32, TeacherTarget4) {
    let mut rng = Rng::new(77);
    let (action, _, lp, target) = PolicySampler4 {
        mode,
        temperature: 0.8,
        rng: &mut rng,
        active_source: None,
    }
    .select(
        cands,
        view,
        pending,
        PolicyOutputs4 {
            cand_logits: logits,
            pair_logits: logits,
            scalar_pred: scalar,
            value: 0.4,
        },
    )
    .unwrap();
    (action, lp, target)
}

#[test]
fn tactical_demonstrations_are_house_never_ppo_and_raw_greedy_scalar_are_not_stochastic() {
    for head in [
        HeadKind4::SingleChoice,
        HeadKind4::SubsetPick,
        HeadKind4::PairPick,
        HeadKind4::ScalarValue,
    ] {
        let tactical = PolicyMode4::Tactical.provenance(0.8, 4, head, -2.0);
        assert_eq!(tactical.source, SourceKind4::House);
        assert!(!tactical.has_logprob);
        assert_eq!(tactical.behavior_logprob, 0.0);
        let greedy = PolicyMode4::Raw.provenance(0.0, 4, head, -2.0);
        assert_eq!(greedy.source, SourceKind4::Net);
        assert!(!greedy.has_logprob);
        let stochastic = PolicyMode4::Raw.provenance(0.8, 4, head, -2.0);
        assert_eq!(stochastic.has_logprob, head != HeadKind4::ScalarValue);
        if stochastic.has_logprob {
            assert_eq!(stochastic.behavior_logprob, -2.0);
        }
    }
    for tau in [-1.0, 0.00001, 0.2, 0.35, f32::NAN, f32::INFINITY] {
        assert!(!valid_raw_temperature(tau));
    }
    assert!(valid_raw_temperature(0.0));
    assert!(valid_raw_temperature(0.35001));
}

#[test]
fn raw_single_bypasses_mulligan_rules_and_matches_raw_categorical_probability() {
    let view = view();
    let pending = Pending::Mulligan {
        player: view.seat,
        taken: 0,
        next_is_free: false,
        can_take: true,
    };
    let cands = menu(&view, &pending);
    let logits = [20.0, -20.0];
    let (raw, lp, _) = select(PolicyMode4::Raw, &view, &pending, &cands, &logits, 0.2);
    let (tactical, _, _) = select(PolicyMode4::Tactical, &view, &pending, &cands, &logits, 0.2);
    assert_eq!(raw, PlayerAction::MulliganKeep);
    assert_eq!(tactical, PlayerAction::MulliganTake);
    assert!(lp.abs() < 1e-6);
    let logits = [0.4, -0.7];
    let (_, lp, target) = select(PolicyMode4::Raw, &view, &pending, &cands, &logits, 0.2);
    let chosen = usize::from(target.single_idx);
    let expected = logits[chosen] / 0.8 - ((logits[0] / 0.8).exp() + (logits[1] / 0.8).exp()).ln();
    assert!((lp - expected).abs() < 1e-6);
}

#[test]
fn raw_subset_and_pair_preserve_unmodified_sampler_masks_and_logprobs() {
    let view = view();
    let subset = Pending::DiscardChoice {
        player: view.seat,
        count: 1,
    };
    let subset_menu = menu(&view, &subset);
    let subset_logits = vec![0.4; subset_menu.candidates.len()];
    let mut rng = Rng::new(77);
    let (mask, free, seq, lp) =
        sample_subset_constrained(&subset_logits, &subset_menu, &subset, 0.8, &mut rng);
    let (_, raw_lp, target) = select(
        PolicyMode4::Raw,
        &view,
        &subset,
        &subset_menu,
        &subset_logits,
        0.2,
    );
    assert_eq!(
        (target.multi_mask, target.free_mask, target.pick_sequence),
        (mask, free, seq)
    );
    assert!((raw_lp - lp).abs() < 1e-6);
    let a = ObjectId::new(111, 0);
    let b = ObjectId::new(112, 0);
    let pair = Pending::ChooseBlockers {
        player: view.seat,
        attacker: PlayerId::new(1),
        blockers: vec![BlockOption {
            blocker: a,
            attackers: vec![b],
        }],
        capacity: vec![],
        obeying: vec![],
        demands: vec![],
        bounds: vec![],
    };
    let pair_menu = menu(&view, &pair);
    let pair_logits = vec![0.4; pair_menu.pairs.len()];
    let mut rng = Rng::new(77);
    let (mask, free, seq, lp) = sample_pairs_constrained(&pair_logits, &pair_menu, 0.8, &mut rng);
    let (_, raw_lp, target) = select(
        PolicyMode4::Raw,
        &view,
        &pair,
        &pair_menu,
        &pair_logits,
        0.2,
    );
    assert_eq!(
        (target.multi_mask, target.free_mask, target.pick_sequence),
        (mask, free, seq)
    );
    assert!((raw_lp - lp).abs() < 1e-6);
}

#[test]
fn raw_scalar_uses_prediction_instead_of_max_x_override() {
    let view = view();
    let pending = Pending::ChooseNumber {
        player: view.seat,
        min: 0,
        max: 10,
        reason: NumberPrompt::X,
    };
    let mut cands = menu(&view, &pending);
    cands.min_choices = 0;
    cands.max_choices = 10;
    let (raw, _, _) = select(PolicyMode4::Raw, &view, &pending, &cands, &[], 0.2);
    let (tactical, _, _) = select(PolicyMode4::Tactical, &view, &pending, &cands, &[], 0.2);
    assert_eq!(raw, PlayerAction::ChooseNumber(2));
    assert_eq!(tactical, PlayerAction::ChooseNumber(10));
}

#[test]
fn raw_sampler_refuses_unsupported_temperatures_and_nonfinite_outputs() {
    let view = view();
    let pending = Pending::Mulligan {
        player: view.seat,
        taken: 0,
        next_is_free: false,
        can_take: true,
    };
    let cands = menu(&view, &pending);
    for (temperature, logits) in [(0.2, [0.0, 1.0]), (0.8, [f32::NAN, 0.0])] {
        let mut rng = Rng::new(1);
        let result = PolicySampler4 {
            mode: PolicyMode4::Raw,
            temperature,
            rng: &mut rng,
            active_source: None,
        }
        .select(
            &cands,
            &view,
            &pending,
            PolicyOutputs4 {
                cand_logits: &logits,
                pair_logits: &[],
                scalar_pred: 0.5,
                value: 0.5,
            },
        );
        assert!(result.is_err());
    }
}

#[test]
fn categorical_frequency_and_logprob_describe_the_same_raw_distribution() {
    let logits = [0.0, 1.0];
    let tau = 0.8;
    let expected = 1.0 / (1.0 + (-1.0_f32 / tau).exp());
    let mut rng = Rng::new(9123);
    let mut selected = 0_u32;
    for _ in 0..20_000 {
        let (index, lp) = sample_categorical(&logits, tau, &mut rng);
        let p = if index == 1 { expected } else { 1.0 - expected };
        assert!((lp - p.ln()).abs() < 1e-6);
        selected += u32::from(index == 1);
    }
    assert!((f64::from(selected) / 20_000.0 - f64::from(expected)).abs() < 0.015);
}

#[test]
fn raw_reencode_gate_rejects_decoder_minimum_fill_including_forced_bits() {
    let view = view();
    let pending = Pending::DiscardChoice {
        player: view.seat,
        count: 2,
    };
    let cands = menu(&view, &pending);
    let sampled = TeacherTarget4 {
        multi_mask: vec![0; cands.candidates.len()],
        ..TeacherTarget4::default()
    };
    let repaired = decode_action4(&cands, &view, &pending, &sampled).unwrap();
    assert!(pending.answer_fault(&repaired).is_none());
    assert!(!decoded_matches_sample(
        &cands, &pending, &repaired, &sampled
    ));
    let valid = crate::features4::encode_teacher_action4(&cands, &pending, &repaired).unwrap();
    assert!(decoded_matches_sample(&cands, &pending, &repaired, &valid));
}
