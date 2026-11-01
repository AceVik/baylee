//! Actual E7 grouped menus must retain the executed attacker/defender mask.
use super::*;
use baylee_core::ids::Defender;
use baylee_core::preset::{AIProfile, FormatId};

const ME: PlayerId = PlayerId::new(0);

fn attack() -> (
    baylee_gamehost::Session,
    Pending,
    PlayerView,
    EncodedCandidates4,
) {
    let deck = crate::housedeck::HouseDeck::named("victory").unwrap();
    let mut preset =
        crate::selfplay::table_for(912, &[&deck, &deck, &deck], &[AIProfile::STEADY; 3], &[]);
    preset.format = FormatId::Freeform;
    preset.modifiers.clear();
    let mut entry = preset.seats[0].deck[0];
    entry.card = baylee_cards::decks::by_name("Grizzly Bears").unwrap();
    entry.rule_print = None;
    entry.choices = baylee_core::card_metadata::DeckChoices::default();
    for (seat, spec) in preset.seats.iter_mut().enumerate() {
        spec.commanders.clear();
        spec.starting_hand = Some(vec![]);
        spec.starting_battlefield = if seat == 0 { vec![entry; 2] } else { vec![] };
    }
    let mut session =
        baylee_gamehost::Session::new_recorded(&preset, "grouped-policy-test").unwrap();
    for seat in 0..3 {
        session.take_over(PlayerId::new(seat));
    }
    for _ in 0..128 {
        if matches!(session.pending(), Pending::ChooseAttackers { player, attackers, .. }
            if *player == ME && attackers.len() == 2)
        {
            let (pending, view) = session.view_for_with_context(ME).unwrap();
            let state = encode_state7(&view, Some(&pending), None, &[]);
            let menu = encode_candidates7_grouped(&state, &view, &pending);
            assert!(
                menu.candidates
                    .iter()
                    .any(|candidate| candidate.feat[47] > 0.0)
            );
            return (session, pending, view, menu);
        }
        let player = session.pending().asked().unwrap();
        let action = match session.pending() {
            Pending::Mulligan { .. } => PlayerAction::MulliganKeep,
            Pending::Priority { .. } => PlayerAction::PassPriority,
            // The engine asks every active player for a declaration, also
            // one with nothing to attack with.
            Pending::ChooseAttackers { .. } => PlayerAction::DeclareAttackers {
                attackers: Vec::new(),
            },
            other => panic!("unexpected setup decision {other:?}"),
        };
        session.act(player, action).unwrap();
    }
    panic!("attack not reached");
}

fn sample(
    menu: &EncodedCandidates4,
    pending: &Pending,
    view: &PlayerView,
    chosen: &[(ObjectId, Defender)],
) -> (PlayerAction, TeacherTarget4) {
    let logits: Vec<_> = menu
        .candidates
        .iter()
        .map(|candidate| {
            if chosen.iter().any(|&(id, defender)| {
                candidate.object_id == Some(id) && candidate.defender == Some(defender)
            }) {
                10.0
            } else {
                -10.0
            }
        })
        .collect();
    let mut rng = Rng::new(417);
    let (action, _, lp, target) = PolicySampler4 {
        mode: PolicyMode4::Raw,
        temperature: 0.0,
        rng: &mut rng,
        active_source: None,
    }
    .select(
        menu,
        view,
        pending,
        PolicyOutputs4 {
            cand_logits: &logits,
            pair_logits: &[],
            scalar_pred: 0.0,
            value: 0.5,
        },
    )
    .expect("legal grouped attack must retain its raw policy decision");
    assert_eq!(lp, 0.0);
    assert_eq!(
        action,
        PlayerAction::DeclareAttackers {
            attackers: chosen.to_vec()
        }
    );
    assert!(pending.answer_fault(&action).is_none());
    (action, target)
}

#[test]
fn raw_grouped_attack_accepts_second_member_without_changing_bc_canonicalization() {
    let (mut session, pending, view, menu) = attack();
    let Pending::ChooseAttackers { attackers, .. } = &pending else {
        unreachable!()
    };
    let (action, target) = sample(
        &menu,
        &pending,
        &view,
        &[(attackers[1], Defender::Player(PlayerId::new(1)))],
    );
    let teacher = crate::features4::encode_teacher_action4(&menu, &pending, &action).unwrap();
    assert_ne!(
        target.multi_mask, teacher.multi_mask,
        "BC remains canonical"
    );
    assert!(decoded_matches_sample(&menu, &pending, &action, &target));
    session
        .act(ME, action)
        .expect("actual engine accepts the selected second member");
}

#[test]
fn raw_grouped_attack_accepts_distinct_defenders_for_identical_members() {
    let (mut session, pending, view, menu) = attack();
    let Pending::ChooseAttackers { attackers, .. } = &pending else {
        unreachable!()
    };
    let chosen = [
        (attackers[0], Defender::Player(PlayerId::new(1))),
        (attackers[1], Defender::Player(PlayerId::new(2))),
    ];
    let (action, target) = sample(&menu, &pending, &view, &chosen);
    assert_eq!(target.multi_mask.iter().filter(|&&bit| bit != 0).count(), 2);
    session
        .act(ME, action)
        .expect("both actual attackers can attack different opponents");
}

#[test]
fn raw_grouped_attack_still_rejects_decoder_repair_of_duplicate_attacker_assignment() {
    let (_session, pending, view, menu) = attack();
    let Pending::ChooseAttackers { attackers, .. } = &pending else {
        unreachable!()
    };
    let mut target = TeacherTarget4 {
        multi_mask: vec![0; menu.candidates.len()],
        ..TeacherTarget4::default()
    };
    for (index, candidate) in menu.candidates.iter().enumerate() {
        if candidate.object_id == Some(attackers[0]) {
            target.multi_mask[index] = 1;
        }
    }
    assert_eq!(target.multi_mask.iter().filter(|&&bit| bit != 0).count(), 2);
    let decoded = decode_action4(&menu, &view, &pending, &target).unwrap();
    assert!(!decoded_matches_sample(&menu, &pending, &decoded, &target));
    let invalid = PlayerAction::DeclareAttackers {
        attackers: vec![
            (attackers[0], Defender::Player(PlayerId::new(1))),
            (attackers[0], Defender::Player(PlayerId::new(2))),
        ],
    };
    assert!(pending.answer_fault(&invalid).is_some());
}

#[test]
fn recorded_grouped_attack_preserves_exact_neural_mask_and_lp_but_house_labels_stay_canonical() {
    use crate::convert4::{RolloutStepSidecar4, extract_game4_with_sidecar};
    use baylee_gamehost::record::{Line, Source};
    let (mut session, pending, view, menu) = attack();
    let Pending::ChooseAttackers { attackers, .. } = &pending else {
        unreachable!()
    };
    let action = PlayerAction::DeclareAttackers {
        attackers: vec![(attackers[1], Defender::Player(PlayerId::new(1)))],
    };
    let index = menu
        .candidates
        .iter()
        .position(|candidate| {
            candidate.object_id == Some(attackers[1])
                && candidate.defender == Some(Defender::Player(PlayerId::new(1)))
        })
        .unwrap();
    let mut logits = vec![-3.0; menu.candidates.len()];
    logits[index] = 3.0;
    let mut rng = Rng::new(417);
    let (mask, free, sequence, lp) =
        sample_subset_constrained(&logits, &menu, &pending, 0.8, &mut rng);
    assert_eq!(mask.iter().filter(|&&bit| bit != 0).count(), 1);
    assert_eq!(mask[index], 1);
    assert!(lp.is_finite() && lp < 0.0);
    let sampled = TeacherTarget4 {
        multi_mask: mask.clone(),
        free_mask: free.clone(),
        pick_sequence: sequence.clone(),
        ..TeacherTarget4::default()
    };
    assert_eq!(
        decode_action4(&menu, &view, &pending, &sampled).unwrap(),
        action
    );
    session.act(ME, action.clone()).unwrap();
    // Explicit administrative concessions finish this synthetic record; it is no strength fixture.
    while !matches!(session.pending(), Pending::GameOver(_)) {
        let player = session.pending().asked().unwrap();
        session.act(player, PlayerAction::Concede).unwrap();
    }
    let record = session.take_record();
    let mut sidecar = Vec::new();
    for line in record
        .split(|&byte| byte == b'\n')
        .filter(|line| !line.is_empty())
    {
        if let Line::Input {
            by: Source::Seat,
            action: executed,
            ..
        } = serde_json::from_slice::<Line>(line).unwrap()
        {
            let entry = if executed == action {
                RolloutStepSidecar4 {
                    provenance: StepProvenance4 {
                        source: SourceKind4::Net,
                        temperature: 0.8,
                        thinking_steps: 2,
                        behavior_logprob: lp,
                        has_logprob: true,
                    },
                    free_mask: free.clone(),
                    pick_sequence: sequence.clone(),
                }
            } else {
                RolloutStepSidecar4::default()
            };
            sidecar.push(entry);
        }
    }
    let game = extract_game4_with_sidecar(&record, 1, true, Some(&sidecar)).unwrap();
    let step = game
        .steps
        .iter()
        .find(|step| step.provenance.source == SourceKind4::Net)
        .unwrap();
    assert_eq!(step.target.multi_mask, mask);
    assert_eq!(step.target.free_mask, free);
    assert_eq!(step.target.pick_sequence, sequence);
    assert_eq!(step.provenance.behavior_logprob, lp);
    assert!(step.provenance.has_logprob);
    let canonical = crate::features4::encode_teacher_action4(&menu, &pending, &action).unwrap();
    assert_ne!(canonical.multi_mask, mask);
    let entry = sidecar
        .iter_mut()
        .find(|entry| entry.provenance.source == SourceKind4::Net)
        .unwrap();
    entry.provenance.source = SourceKind4::House;
    entry.provenance.has_logprob = false;
    entry.provenance.behavior_logprob = 0.0;
    let house = extract_game4_with_sidecar(&record, 1, true, Some(&sidecar)).unwrap();
    let labelled = house
        .steps
        .iter()
        .find(|step| step.is_decision && step.head == 1 && step.seat == 0)
        .unwrap();
    assert_eq!(labelled.target.multi_mask, canonical.multi_mask);
    assert_eq!(labelled.provenance.source, SourceKind4::House);
    assert!(!labelled.provenance.has_logprob);
}
