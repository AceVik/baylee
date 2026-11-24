//! `cards/instants/mv_2/chaotic_strike.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index::GRIZZLY_BEARS;

fn chaotic_strike() -> CardIndex {
    card_index("1f0b3549-2d24-49e8-85bc-55e3a5e5f6b8")
}

/// Reaches p0's priority after blockers with an attacking Grizzly Bears. Both
/// earlier windows refuse a real cast even with its exact mana already floating.
fn after_blockers(seed: u64) -> (Engine<RegistryLookup>, ObjectId) {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(seed, mountain())
        .hand(0, &[chaotic_strike()])
        .battlefield(0, &[GRIZZLY_BEARS, mountain(), mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let spell = in_hand(&engine, p0, chaotic_strike()).unwrap();
    let attacker = on_battlefield(&engine, p0, GRIZZLY_BEARS).unwrap();

    lk::float(
        &mut engine,
        p0,
        &[(ManaColor::Red, 1), (ManaColor::Colorless, 1)],
    );
    let before = engine.state().snapshot_hash();
    assert!(
        engine
            .apply(p0, PlayerAction::CastSpell { card: spell })
            .is_err(),
        "not during combat"
    );
    assert_eq!(engine.state().snapshot_hash(), before);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(attacker, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.step == baylee_engine::turn::Step::DeclareAttackers
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    lk::float(
        &mut engine,
        p0,
        &[(ManaColor::Red, 1), (ManaColor::Colorless, 1)],
    );
    let before = engine.state().snapshot_hash();
    assert!(
        engine
            .apply(p0, PlayerAction::CastSpell { card: spell })
            .is_err(),
        "before blockers are declared"
    );
    assert_eq!(engine.state().snapshot_hash(), before);

    pass_until(&mut engine, |e| {
        e.state().turn.step == baylee_engine::turn::Step::DeclareBlockers
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    (engine, attacker)
}

#[test]
fn chaotic_strike_is_castable_after_blockers() {
    let p0 = PlayerId::new(0);
    let (mut engine, attacker) = after_blockers(SEED);
    cast_from_hand(&mut engine, p0, chaotic_strike());
    aim_at(&mut engine, p0, attacker);
    assert!(on_stack(&engine, chaotic_strike()).is_some());
}

/// Plays one flip and returns whether it was won. Both paths draw immediately;
/// only the win pumps the target, and that pump wears off at the turn's end.
fn chaotic_strike_game(seed: u64) -> bool {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let (mut engine, attacker) = after_blockers(seed);
    cast_from_hand(&mut engine, p0, chaotic_strike());
    aim_at(&mut engine, p0, attacker);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1,
        "draw a card after either outcome"
    );
    let flips: Vec<bool> = engine
        .state()
        .journal
        .entries()
        .iter()
        .filter_map(|entry| match entry.event {
            baylee_engine::event::GameEvent::CoinFlipped { won, .. } => Some(won),
            _ => None,
        })
        .collect();
    assert_eq!(flips.len(), 1, "one flip");
    let won = flips[0];
    assert_eq!(
        pt(&engine, attacker),
        if won { (3, 3) } else { (2, 2) },
        "+1/+1 only when the flip is won"
    );

    reach_their_main_phase(&mut engine, p1);
    assert_eq!(pt(&engine, attacker), (2, 2), "only until end of turn");
    won
}

#[test]
fn chaotic_strike_pumps_on_a_win_and_draws_on_either_outcome() {
    let results: Vec<bool> = (1..=16u64).map(chaotic_strike_game).collect();
    assert!(
        results.contains(&true) && results.contains(&false),
        "both outcomes of the flip were played: {results:?}"
    );
}
