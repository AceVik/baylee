//! `cards/creatures/mv_4/scarwood_bandits.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn scarwood_bandits() -> CardIndex {
    card_index("7fcc8aab-6f7b-4474-b4e4-58cf4a119b22")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn float(engine: &mut Engine<RegistryLookup>, seat: usize, mana: &[(ManaColor, u32)]) {
    let state = engine
        .dev_state_mut(PlayerId::new(0))
        .expect("the harness may set boards up");
    for (color, n) in mana {
        state.players[seat].mana_pool.add(*color, *n);
    }
    engine.refresh_offer();
}

/// Scarwood Bandits (7fcc8aab-6f7b-4474-b4e4-58cf4a119b22) — "Forestwalk
/// (This creature can't be blocked as long as defending player controls a
/// Forest.)"
#[test]
fn scarwood_bandits_cannot_be_blocked_while_the_defender_has_a_forest() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[scarwood_bandits()])
        .battlefield(1, &[grizzly_bears(), forest()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    let bandits = on_battlefield(&engine, p0, scarwood_bandits()).expect("the Bandits are out");
    assert!(keywords(&engine, bandits).contains(KeywordSet::FORESTWALK));
    let blocks = attack_and_collect_blocks(&mut engine, bandits, p1);
    assert!(
        blocks.is_empty(),
        "the defender controls a Forest, so nothing may block: {blocks:?}"
    );
}

#[test]
fn scarwood_bandits_can_be_blocked_without_a_forest_to_walk_over() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[scarwood_bandits()])
        .battlefield(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    let bandits = on_battlefield(&engine, p0, scarwood_bandits()).expect("the Bandits are out");
    let bears = on_battlefield(&engine, p1, grizzly_bears()).expect("their Bears are out");
    let blocks = attack_and_collect_blocks(&mut engine, bandits, p1);
    assert!(
        blocks
            .iter()
            .any(|b| b.blocker == bears && b.attackers.contains(&bandits)),
        "no Forest: an ordinary block is legal: {blocks:?}"
    );
}

/// An opponent's artifact, and p0 with the mana to try for it.
fn heist() -> (Engine<RegistryLookup>, ObjectId, ObjectId) {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[scarwood_bandits()])
        .battlefield(1, &[quiet_artifact()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    let bandits = on_battlefield(&engine, p0, scarwood_bandits()).expect("the Bandits are out");
    let artifact =
        on_battlefield(&engine, PlayerId::new(1), quiet_artifact()).expect("their artifact");
    float(&mut engine, 0, &[(ManaColor::Green, 3)]);
    activate(&mut engine, p0, scarwood_bandits(), 0);
    let options = aim_at(&mut engine, p0, artifact);
    assert_eq!(options, vec![artifact], "only the artifact can be named");
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayTax { .. },
                ..
            }
        )
    });
    let Pending::YesNo {
        player,
        prompt: YesNoPrompt::PayTax { mana },
        ..
    } = engine.pending().clone()
    else {
        unreachable!("the pass waited for exactly this")
    };
    assert_eq!(player, PlayerId::new(1), "an opponent is asked");
    assert_eq!(mana, 2);
    assert!(is_tapped(&engine, bandits), "{{T}} is part of the cost");
    (engine, bandits, artifact)
}

/// "{2}{G}, {T}: Unless an opponent pays {2}, gain control of target
/// artifact for as long as this creature remains on the battlefield."
#[test]
fn scarwood_bandits_steals_the_artifact_when_the_opponent_does_not_pay() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let (mut engine, bandits, artifact) = heist();
    engine
        .apply(p1, PlayerAction::YesNo(false))
        .expect("declines");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine
            .state()
            .object(artifact)
            .expect("the artifact is out")
            .controller,
        p0,
        "stolen"
    );

    kill(&mut engine, bandits);
    assert!(in_graveyard(&engine, p0, scarwood_bandits()).is_some());
    assert_eq!(
        engine
            .state()
            .object(artifact)
            .expect("the artifact is out")
            .controller,
        p1,
        "it goes home when the Bandits leave"
    );
}

#[test]
fn scarwood_bandits_leaves_the_artifact_when_the_opponent_pays_two() {
    let p1 = PlayerId::new(1);
    let (mut engine, _bandits, artifact) = heist();
    float(&mut engine, 1, &[(ManaColor::Colorless, 2)]);
    engine.apply(p1, PlayerAction::YesNo(true)).expect("pays");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine
            .state()
            .object(artifact)
            .expect("the artifact is out")
            .controller,
        p1,
        "paid for: still theirs"
    );
    assert_eq!(
        engine.state().players[1].mana_pool.total(),
        0,
        "{{2}} was spent"
    );
}
