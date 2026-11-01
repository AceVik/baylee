//! `cards/instants/mv_2/aleatory.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::GRIZZLY_BEARS;

fn aleatory() -> CardIndex {
    card_index("da4cab2d-0068-440f-ba7c-f2d4e5fa976e")
}

#[test]
fn aleatory_flips_coin_after_blockers_and_draws_next_upkeep() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[aleatory()])
        .battlefield(0, &[GRIZZLY_BEARS, mountain(), mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let attacker = on_battlefield(&engine, p0, GRIZZLY_BEARS).unwrap();

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
        e.state().turn.step == crate::turn::Step::DeclareBlockers
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });

    cast_from_hand(&mut engine, p0, aleatory());
    aim_at(&mut engine, p0, attacker);
    pass_until(&mut engine, stack_is_empty);

    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    reach_their_main_phase(&mut engine, p1);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1,
        "Aleatory should draw a card at the beginning of the next turn's upkeep"
    );
}

/// Whether the flip of the game's Aleatory was won, with the pump checked
/// against it: +1/+1 on a win, nothing on a loss.
fn aleatory_game(seed: u64) -> bool {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(seed, mountain())
        .hand(0, &[aleatory()])
        .battlefield(0, &[GRIZZLY_BEARS, mountain(), mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let attacker = on_battlefield(&engine, p0, GRIZZLY_BEARS).unwrap();
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
        e.state().turn.step == crate::turn::Step::DeclareBlockers
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    cast_from_hand(&mut engine, p0, aleatory());
    aim_at(&mut engine, p0, attacker);
    pass_until(&mut engine, stack_is_empty);
    let flips: Vec<bool> = engine
        .state()
        .journal
        .entries()
        .iter()
        .filter_map(|entry| match entry.event {
            crate::event::GameEvent::CoinFlipped { won, .. } => Some(won),
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
    won
}

#[test]
fn aleatory_pumps_only_when_the_flip_is_won() {
    let results: Vec<bool> = (1..=16u64).map(aleatory_game).collect();
    assert!(
        results.contains(&true) && results.contains(&false),
        "both outcomes of the flip were played: {results:?}"
    );
}

/// Whether the player holding priority may cast `spell` right now: in
/// `legal.castable` when the mana already floats, in `legal.payable` when
/// every other rule holds and the lands still have to be tapped. Only the
/// union answers "may cast", and a negative asserted on `castable` alone
/// would pass for the wrong reason (no mana floating).
fn castable(engine: &Engine<RegistryLookup>, spell: ObjectId) -> bool {
    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("priority, got {:?}", engine.pending())
    };
    legal.castable.contains(&spell) || legal.payable.contains(&spell)
}

#[test]
fn aleatory_is_castable_only_in_combat_after_blockers_are_declared() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[aleatory()])
        .battlefield(0, &[GRIZZLY_BEARS, mountain(), mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let spell = in_hand(&engine, p0, aleatory()).unwrap();
    let attacker = on_battlefield(&engine, p0, GRIZZLY_BEARS).unwrap();
    assert!(!castable(&engine, spell), "not in a main phase");
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
    assert!(
        !castable(&engine, spell),
        "not before blockers are declared"
    );
    pass_until(&mut engine, |e| {
        e.state().turn.step == crate::turn::Step::DeclareBlockers
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    assert!(castable(&engine, spell), "after blockers are declared");
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    assert!(!castable(&engine, spell), "combat is over");
}
