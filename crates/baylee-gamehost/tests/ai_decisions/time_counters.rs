#[allow(clippy::wildcard_imports)] // shared integration fixture vocabulary
use super::*;

/// Pass through a real turn, declining combat, until the named main phase.
/// Time Bomb's upkeep never asks for targets: an unexpected question fails.
fn time_bomb_main(engine: &mut Engine<RegistryLookup>, active: PlayerId, turn: u32) -> PlayerView {
    for seq in 0..200 {
        let pending = engine.pending();
        let seat = pending_player(pending).expect("the fixture stays live");
        let view = asked_view(engine.state(), seat, seq, pending);
        if view.turn == turn
            && view.active == active
            && view.phase == baylee_view::Phase::FirstMain
            && view.stack.is_empty()
        {
            return view;
        }
        let action = match pending {
            Pending::Mulligan { .. } => PlayerAction::MulliganKeep,
            Pending::Priority { .. } => PlayerAction::PassPriority,
            Pending::ChooseAttackers { .. } => PlayerAction::DeclareAttackers { attackers: vec![] },
            Pending::ChooseBlockers { .. } => PlayerAction::DeclareBlockers { blockers: vec![] },
            other => panic!("Time Bomb should need no target or choice: {other:?}"),
        };
        engine
            .apply(seat, action)
            .expect("a legal pass through the turn");
    }
    panic!("Time Bomb never reached turn {turn}'s main phase");
}

fn time_on_bomb(view: &PlayerView, bomb: ObjectId) -> u16 {
    view.object(bomb)
        .expect("Time Bomb is still on the battlefield")
        .counters
        .iter()
        .find(|c| c.kind == baylee_view::CounterKind::Time)
        .map_or(0, |c| c.count)
}

/// #166's second named arrival counts up, without a target, on its own
/// upkeeps only. Sacrifice clears its counters before resolution, so both
/// damage sentences must read the departed source's last battlefield state.
#[test]
fn time_bomb_counts_its_own_upkeeps_and_uses_sacrificed_source_counters_for_damage() {
    let (me, them) = (PlayerId::new(0), PlayerId::new(1));
    for counters in 1_u16..=2 {
        let mut preset = position(
            &[],
            &["Time Bomb", "Forest", "Llanowar Elves", "Hill Giant"],
        );
        preset.seats[0].starting_life = Some(20);
        preset.seats[1].starting_life = Some(20);
        preset.seats[1].starting_battlefield = ["Grizzly Bears", "Hill Giant"].map(entry).to_vec();
        let mut engine = Engine::new(&preset, RegistryLookup).unwrap();
        let mut view = time_bomb_main(&mut engine, me, 1);
        let bomb = untapped(&view, "Time Bomb")[0];
        assert_eq!(time_on_bomb(&view, bomb), 1, "first upkeep adds one");
        if counters == 2 {
            view = time_bomb_main(&mut engine, them, 2);
            assert_eq!(time_on_bomb(&view, bomb), 1, "opponent upkeep adds nothing");
            view = time_bomb_main(&mut engine, me, 3);
            assert_eq!(time_on_bomb(&view, bomb), 2, "second own upkeep adds one");
        }
        float(&mut engine, &untapped(&view, "Forest")[..1]);
        let Pending::Priority { legal, .. } = engine.pending() else {
            panic!("mana activation must leave priority");
        };
        assert!(
            legal.abilities.contains(&(bomb, 1)),
            "the paid activation is offered"
        );
        engine
            .apply(
                me,
                PlayerAction::ActivateAbility {
                    source: bomb,
                    ability_index: 1,
                },
            )
            .expect("one mana, tap and sacrifice pay for the sweep");
        let paid = asked_view(engine.state(), me, 0, engine.pending());
        assert!(
            paid.battlefield.iter().all(|o| o.id != bomb),
            "sacrificed as a cost"
        );
        let departed = paid.graveyards[0]
            .iter()
            .find(|o| o.name == "Time Bomb")
            .expect("Time Bomb is already in the graveyard");
        assert!(
            departed.counters.is_empty(),
            "the new zone has no counters to read"
        );
        assert_eq!(paid.stack.len(), 1, "the sweep has not resolved yet");
        assert!(
            paid.stack[0].targets.is_empty(),
            "the sweep targets nothing"
        );
        assert!(
            paid.seats.iter().all(|s| s.life == 20),
            "costs deal no damage"
        );

        settle(&mut engine, &[]);
        let resolved = asked_view(engine.state(), me, 0, engine.pending());
        assert!(
            resolved
                .seats
                .iter()
                .all(|s| s.life == 20 - i32::from(counters)),
            "each player takes the departed source's {counters} damage"
        );
        assert!(
            resolved.graveyards[0]
                .iter()
                .any(|o| o.name == "Llanowar Elves"),
            "the controller's creature also takes damage"
        );
        assert_eq!(
            resolved.graveyards[1]
                .iter()
                .any(|o| o.name == "Grizzly Bears"),
            counters == 2,
            "lethal damage follows the accumulated count"
        );
        let giants: Vec<_> = resolved
            .battlefield
            .iter()
            .filter(|o| o.name == "Hill Giant")
            .collect();
        assert_eq!(
            giants.len(),
            2,
            "nonlethal damage leaves a creature on each side"
        );
        assert!(
            giants.iter().all(|o| o.damage == counters),
            "each surviving creature takes damage"
        );
        assert!(
            resolved.battlefield.iter().any(|o| o.name == "Forest"),
            "lands are unaffected"
        );
    }
}

/// Time Bomb is not a suspend clock and neither ability chooses targets.
/// The AI currently cannot price its counter-dependent sweep, even with the
/// mana floated and a board it would profitably clear. Pin that limitation
/// rather than claiming the new pool name makes clock targeting reachable.
#[test]
fn the_ai_declines_time_bombs_unread_sweep_even_with_mana_and_enemy_creatures() {
    let me = PlayerId::new(0);
    let mut preset = position(&[], &["Time Bomb", "Forest"]);
    preset.seats[1].starting_battlefield = vec![entry("Llanowar Elves"); 4];
    let mut engine = Engine::new(&preset, RegistryLookup).unwrap();
    let view = time_bomb_main(&mut engine, me, 1);
    let bomb = untapped(&view, "Time Bomb")[0];
    assert_eq!(time_on_bomb(&view, bomb), 1);
    assert_eq!(view.battlefield_of(PlayerId::new(1)).count(), 4);
    float(&mut engine, &untapped(&view, "Forest"));
    let view = asked_view(engine.state(), me, 0, engine.pending());
    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("the agent is asked at priority");
    };
    assert_eq!(
        legal.abilities,
        vec![(bomb, 1)],
        "the sweep is payable and the only activation"
    );
    let agent = HeuristicAgent::new(AIProfile::EXPERT);
    let answer = agent.act_with_context(&view, engine.pending(), &engine.decision_context());
    assert_eq!(
        answer,
        PlayerAction::PassPriority,
        "the unsupported sweep is left alone"
    );
    engine.apply(me, answer).expect("the AI's answer is legal");
    assert_eq!(
        time_on_bomb(&asked_view(engine.state(), me, 0, engine.pending()), bomb),
        1
    );
}

// Infinite Hourglass is held back after the Argon review: its earlier
// implementation counted time without the printed creature-power payoff.
// Its canonical stub has no live clock ability to exercise; restore its
// card-specific AI regression when the complete card is implemented.
