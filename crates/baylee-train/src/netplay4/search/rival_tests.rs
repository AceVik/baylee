//! A measuring search ([`Selection4::Measure`]) and the tactical layer's
//! answer for a rival ([`Rival4::Distill`]), with the stand-in for the net:
//! what is played, what is compared, and what the label keeps.
use super::tests::{ME, Mock, SEED, config, flat, land_playable, lands, passes, root, table};
use super::*;
use crate::netplay4::TacticRules4;
use crate::netplay4::distill::Distill4;

/// The first land the seat may play, as the layer would answer.
fn first_land(_: &PlayerView, pending: &Pending) -> Option<PlayerAction> {
    match pending {
        Pending::Priority { legal, .. } => legal
            .lands
            .first()
            .map(|&card| PlayerAction::PlayLand { card }),
        _ => None,
    }
}

/// The layer answers as the policy does: a pass.
#[allow(
    clippy::unnecessary_wraps,
    reason = "One signature for every stand-in policy"
)]
fn layer_passes(_: &PlayerView, _: &Pending) -> Option<PlayerAction> {
    Some(PlayerAction::PassPriority)
}

/// A stand-in that passes at the land-drop root, its labeller in place and
/// `rival` for the layer.
fn labelled(
    critic: fn(&PlayerView) -> f32,
    rival: Option<fn(&PlayerView, &Pending) -> Option<PlayerAction>>,
) -> Mock {
    let mut player = Mock::new(passes, critic);
    player.rival = rival;
    player.labels = Some(Distill4::new(TacticRules4::parse("react").unwrap()));
    player
}

fn probe_config(particles: usize, horizon: usize) -> SearchConfig4 {
    config(2, particles, horizon)
        .with_selection(Selection4::Measure)
        .unwrap()
        .with_rival(Rival4::Distill)
}

#[test]
fn a_measuring_search_plays_the_policys_answer_as_it_gave_it() {
    let mut searched_table = table(22);
    let mut plain_table = table(22);
    let (pending, view) = root(&mut searched_table, land_playable);
    let _ = root(&mut plain_table, land_playable);
    let mut searched = Mock::new(passes, lands);
    let mut plain = Mock::new(passes, lands);
    searched.observe(&view).unwrap();
    plain.observe(&view).unwrap();
    let measure = config(SearchConfig4::MAX_CANDIDATES, 1, 0)
        .with_selection(Selection4::Measure)
        .unwrap();
    let result = search_decision(
        &mut searched,
        &KnownDecks(&searched_table),
        &view,
        &pending,
        &[],
        measure,
        SEED,
    )
    .unwrap();
    let decision = plain.decide(&view, &pending, &[]);
    plain.conclude(&view, &decision, &decision.answer.action);

    let SearchOutcome4::Searched(report) = &result.outcome else {
        panic!("the root is searched: {:?}", result.outcome);
    };
    // The land drop the policy passed on is worth 0.1 more, and it is
    // measured, not played.
    assert!(
        report.diffs.iter().any(|d| (d - 0.1).abs() < 1e-6),
        "{report:?}"
    );
    assert_eq!((report.chosen, report.changed), (0, false));
    assert_eq!(result.answer.action, decision.answer.action);
    assert_eq!(result.answer.provenance, decision.answer.provenance);
    assert_eq!(result.answer.provenance.source, SourceKind4::Net);
    assert_eq!(result.answer.target, decision.answer.target);
    assert!(result.follow.is_none() && result.script.is_none());
    // Nothing counts the decision as searched: the player is where
    // answering without a search would have left it.
    assert_eq!(searched.state, plain.state);

    searched_table
        .act_slice(ME, result.answer.action, 0)
        .unwrap();
    plain_table
        .act_slice(ME, decision.answer.action, 0)
        .unwrap();
    assert_eq!(searched_table.take_record(), plain_table.take_record());
}

#[test]
fn the_layers_answer_is_the_one_rival_and_the_label_keeps_its_worlds() {
    let mut session = table(22);
    let (pending, view) = root(&mut session, land_playable);
    let mut player = labelled(lands, Some(first_land));
    player.observe(&view).unwrap();
    let result = search_decision(
        &mut player,
        &KnownDecks(&session),
        &view,
        &pending,
        &[],
        probe_config(3, 0),
        SEED,
    )
    .unwrap();
    let SearchOutcome4::Searched(report) = &result.outcome else {
        panic!("the root is searched: {:?}", result.outcome);
    };
    let [own, land] = report.actions.as_slice() else {
        panic!("the policy's answer, then the layer's alone: {report:?}");
    };
    assert_eq!(*own, PlayerAction::PassPriority);
    assert!(matches!(land, PlayerAction::PlayLand { .. }), "{land:?}");
    assert_eq!(report.rounds, 3);
    assert_eq!(result.answer.action, PlayerAction::PassPriority);
    assert_eq!(result.answer.provenance.source, SourceKind4::Net);

    let probes = std::mem::take(&mut player.probes);
    let [probe] = probes.as_slice() else {
        panic!("one probe kept: {probes:?}");
    };
    assert_eq!(probe.worlds.len(), 3);
    assert_eq!(probe.ends.len(), 3);
    assert!(probe.ends.iter().flatten().all(|end| end == "immediate"));
    let ranking = player
        .decide(&view, &pending, &[])
        .ranking
        .expect("the policy ranks the root");
    let index_of = |action: &PlayerAction| {
        ranking
            .iter()
            .find(|r| r.action == *action)
            .map(|r| u16::try_from(r.index).unwrap())
    };
    assert_eq!(Some(probe.own_index), index_of(&PlayerAction::PassPriority));
    assert_eq!(Some(probe.rival_index), index_of(land));
    let (mean, se) = probe.advantage().unwrap();
    assert!((mean - 0.1).abs() < 1e-6, "{probe:?}");
    assert!(se.unwrap() < 1e-6, "every world agrees");
    assert!(player.labels.is_some(), "the labeller is back in place");
}

/// The ranking keeps one entry of each action. A probe names the one the
/// policy took, which is the label's `N` also where two entries of one action
/// have equal logits.
#[test]
fn a_probe_names_the_menu_entry_the_policy_took() {
    let mut session = table(22);
    let (pending, view) = root(&mut session, land_playable);
    let mut player = labelled(lands, Some(first_land));
    player.taken_at = Some(41);
    player.observe(&view).unwrap();
    search_decision(
        &mut player,
        &KnownDecks(&session),
        &view,
        &pending,
        &[],
        probe_config(3, 0),
        SEED,
    )
    .unwrap();
    let [probe] = player.probes.as_slice() else {
        panic!("one probe kept: {:?}", player.probes);
    };
    assert_eq!(probe.own_index, 41);
}

#[test]
fn no_rollout_labels_a_decision() {
    let mut session = table(22);
    let (pending, view) = root(&mut session, land_playable);
    let mut player = labelled(flat, Some(first_land));
    player.observe(&view).unwrap();
    let result = search_decision(
        &mut player,
        &KnownDecks(&session),
        &view,
        &pending,
        &[],
        probe_config(2, 6),
        SEED,
    )
    .unwrap();
    let SearchOutcome4::Searched(report) = &result.outcome else {
        panic!("the root is searched: {:?}", result.outcome);
    };
    assert!(report.rollout_steps > 0, "the worlds are played on");
    assert_eq!(player.labelled_in_rollouts, 0);
    assert!(player.labels.is_some());
    assert_eq!(player.probes.len(), 1);
}

#[test]
fn a_decision_without_a_differing_rival_is_not_searched() {
    for rival in [None, Some(layer_passes as fn(&_, &_) -> _)] {
        let mut session = table(22);
        let (pending, view) = root(&mut session, land_playable);
        let mut player = labelled(lands, rival);
        player.observe(&view).unwrap();
        let result = search_decision(
            &mut player,
            &KnownDecks(&session),
            &view,
            &pending,
            &[],
            probe_config(2, 0),
            SEED,
        )
        .unwrap();
        assert_eq!(
            result.outcome,
            SearchOutcome4::Skipped(SearchSkip4::NoRival)
        );
        assert_eq!(result.answer.action, PlayerAction::PassPriority);
        assert!(player.probes.is_empty());
        assert!(player.labels.is_some(), "the labeller is back in place");
    }
}

#[test]
fn rival_roots_take_every_phase_and_leave_mana_alone() {
    let mut session = table(22);
    let (pending, mut view) = root(&mut session, land_playable);
    let mut player = Mock::new(passes, flat);
    let decision = player.decide(&view, &pending, &[]);
    let ranking = decision.ranking.clone().unwrap();
    let land = ranking
        .iter()
        .find(|r| matches!(r.action, PlayerAction::PlayLand { .. }))
        .unwrap()
        .clone();
    let rival = (land.index, land.action.clone());
    assert_eq!(rival_skip(&view, &pending, &decision, Some(&rival)), None);
    // End steps and upkeeps are where instants answer: rivals there too.
    view.phase = Phase::Ending;
    assert_eq!(rival_skip(&view, &pending, &decision, Some(&rival)), None);
    assert_eq!(
        root_skip(&view, &pending, &decision, 2),
        Some(SearchSkip4::Phase),
        "a ranking search still skips them"
    );
    assert_eq!(
        rival_skip(&view, &pending, &decision, None),
        Some(SearchSkip4::NoRival)
    );
    // Two mana abilities: the same mana by another source or order.
    let tap = |slot| PlayerAction::ActivateManaAbility {
        source: baylee_core::ids::ObjectId::new(slot, 0),
    };
    let mana = RootDecision4 {
        answer: Answer4 {
            action: tap(7),
            ..decision.answer.clone()
        },
        ranking: Some(vec![
            Ranked4 {
                index: 0,
                action: tap(7),
                logit: 1.0,
            },
            Ranked4 {
                index: 1,
                action: tap(8),
                logit: 0.0,
            },
        ]),
        notes_answer: true,
    };
    assert_eq!(
        rival_skip(&view, &pending, &mana, Some(&(1, tap(8)))),
        Some(SearchSkip4::ManaOnly)
    );
    assert_eq!(
        rival_skip(&view, &pending, &mana, Some(&(5, tap(9)))),
        Some(SearchSkip4::NoAlternative),
        "an answer the policy did not rank"
    );
}

#[test]
fn a_measurement_never_overrides() {
    let worlds = vec![vec![0.2_f32, 0.9], vec![0.3, 0.8]];
    let choice = select::choose_with(Selection4::Measure, &worlds, &[0.5, 0.5], true);
    assert_eq!(choice.chosen, 0);
    assert!((choice.diffs[1] - 0.6).abs() < 1e-6);
    assert_eq!(
        select::choose_with(Selection4::Measure, &worlds, &[0.5, 0.5], false).chosen,
        0,
        "not even where the table took the policy's answer back"
    );
    assert!(Selection4::Measure.validate().is_ok());
}
