//! Adaptive thinking on the decisions of a real game. Needs an exported model
//! with the deliberation block whose depths answer differently:
//! `BAYLEE_TEST_THINK_MODEL=DIR` (for plumbing, `attach_thinking5.py
//! --random-correction` and `export_onnx4.py` give one). Run with `--ignored`.
use super::*;
use baylee_core::preset::AIProfile;

fn model() -> Option<NetPlayer4> {
    let dir = std::env::var("BAYLEE_TEST_THINK_MODEL").ok()?;
    Some(
        NetPlayer4::load_with(Path::new(&dir), CatalogPolicy4::Drift)
            .unwrap()
            .with_policy_mode(PolicyMode4::Raw)
            .with_temperature(0.0),
    )
}

/// The first `limit` answers of a 1v1 game `net` plays for both seats, each
/// with the depth its provenance names.
fn answers(net: &mut NetPlayer4, seed: u64, limit: usize) -> Vec<(String, u8)> {
    let victory = crate::housedeck::HouseDeck::named("victory").unwrap();
    let allytifact = crate::housedeck::HouseDeck::named("allytifact").unwrap();
    let preset =
        crate::selfplay::table_for(seed, &[&victory, &allytifact], &[AIProfile::SHARP; 2], &[]);
    let mut session = baylee_gamehost::Session::new(&preset).unwrap();
    for seat in 0..2 {
        session.take_over(PlayerId::new(seat));
    }
    net.reseed(seed);
    net.init_game(&preset);
    let teams = [None, None];
    let mut taken = Vec::new();
    while taken.len() < limit {
        let Some(player) = session.pending().asked() else {
            break;
        };
        let (pending, view) = session.view_for_with_context(player).unwrap();
        net.observe(&view).unwrap();
        let answer = net.act_detailed(&view, &pending, &teams);
        taken.push((
            format!("{:?}", answer.action),
            answer.provenance.thinking_steps,
        ));
        if session.act(player, answer.action).is_err() {
            break;
        }
    }
    taken
}

#[test]
#[ignore = "needs BAYLEE_TEST_THINK_MODEL, an export with the deliberation block"]
fn a_second_pass_answers_as_the_deep_graph_and_none_as_the_shallow_one() {
    let (Some(shallow), Some(deep), Some(always), Some(never)) =
        (model(), model(), model(), model())
    else {
        return;
    };
    let mut shallow = shallow.with_thinking_steps(1);
    let mut deep = deep.with_thinking_steps(4);
    let mut always = always
        .with_thinking_steps(1)
        .with_adaptive_thinking(4, 1.0)
        .unwrap();
    let mut never = never
        .with_thinking_steps(1)
        .with_adaptive_thinking(4, f32::MIN_POSITIVE)
        .unwrap();
    let quick = answers(&mut shallow, 4711, 160);
    let slow = answers(&mut deep, 4711, 160);
    let again = answers(&mut always, 4711, 160);
    let once = answers(&mut never, 4711, 160);
    assert!(
        quick.len() >= 40,
        "the game ended after {} answers",
        quick.len()
    );

    // Never thinking again is the shallow player, answer for answer.
    assert_eq!(once, quick);
    let unused = never.take_thinking_use();
    assert!(unused.decisions > 0);
    assert_eq!(unused.escalated, 0);

    // Always thinking again answers as the deep graph does. The depth of an
    // answer is the pass that gave it: the deep one wherever there was a
    // choice among candidates, the first pass elsewhere.
    let actions = |taken: &[(String, u8)]| taken.iter().map(|a| a.0.clone()).collect::<Vec<_>>();
    assert_eq!(actions(&again), actions(&slow));
    let used = always.take_thinking_use();
    assert!(used.decisions > 0);
    assert_eq!(used.escalated, used.decisions);
    assert!(again.iter().any(|(_, depth)| *depth == 4));
    assert!(again.iter().all(|(_, depth)| matches!(depth, 0 | 1 | 4)));
    assert_eq!(always.take_thinking_use(), ThinkingUse4::default());

    let differing = quick.iter().zip(&slow).filter(|(a, b)| a.0 != b.0).count();
    eprintln!(
        "{} answers; depth 1 and depth 4 part ways after {} shared answers ({} of the \
         compared differ); {} of {} choices thought through again",
        quick.len(),
        quick
            .iter()
            .zip(&slow)
            .take_while(|(a, b)| a.0 == b.0)
            .count(),
        differing,
        used.escalated,
        used.decisions
    );
}

#[test]
fn a_model_without_the_block_refuses_adaptive_thinking() {
    let net = NetPlayer4::without_graphs(10, 8);
    let error = net.with_adaptive_thinking(4, 0.7).err().unwrap();
    assert!(error.to_string().contains("state a confidence"), "{error}");
}
