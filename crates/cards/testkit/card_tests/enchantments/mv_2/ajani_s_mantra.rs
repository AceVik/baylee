//! Oracle: “At the beginning of your upkeep, you may gain 1 life.”
#[allow(clippy::wildcard_imports)]
use super::*;

fn ajanis_mantra() -> CardIndex {
    card_index("2f47661e-a107-4d98-8773-0a068a63df49")
}

/// Drive through the first upkeep and answer the Mantra's optional trigger.
fn answer_first_upkeep(engine: &mut Engine<RegistryLookup>, accept: bool) {
    let p0 = PlayerId::new(0);
    let mut saw_choice = false;
    for _ in 0..100 {
        if matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0)
            && engine.state().turn.phase == Phase::FirstMain
        {
            assert!(saw_choice, "the upkeep trigger offered its optional gain");
            return;
        }
        match engine.pending().clone() {
            Pending::Priority { player, .. } => engine
                .apply(player, PlayerAction::PassPriority)
                .expect("priority advances through the upkeep trigger"),
            Pending::YesNo {
                player,
                prompt: YesNoPrompt::MayDo,
                ..
            } => {
                assert_eq!(player, p0);
                assert!(!saw_choice, "one choice for one upkeep trigger");
                saw_choice = true;
                engine.apply(player, PlayerAction::YesNo(accept)).unwrap();
            }
            Pending::ChooseAttackers { player, .. } => engine
                .apply(player, PlayerAction::DeclareAttackers { attackers: vec![] })
                .unwrap(),
            Pending::ChooseBlockers { player, .. } => engine
                .apply(player, PlayerAction::DeclareBlockers { blockers: vec![] })
                .unwrap(),
            other => panic!("unexpected while resolving the upkeep trigger: {other:?}"),
        }
    }
    panic!("did not reach the first main phase");
}

/// Checklist: one trigger per controller upkeep; accepting gains one and
/// declining gains zero.
#[test]
fn ajanis_mantra_honors_accepting_and_declining_its_optional_life() {
    let p0 = PlayerId::new(0);
    for (accept, expected_life) in [(true, 21), (false, 20)] {
        let mut game = Duel::new(SEED, plains())
            .battlefield(0, &[ajanis_mantra()])
            .life(0, 20)
            .start();
        keep_mulligans(&mut game);
        answer_first_upkeep(&mut game, accept);
        assert_eq!(game.state().players[0].life, expected_life);
        assert!(on_battlefield(&game, p0, ajanis_mantra()).is_some());
    }
}
