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

fn time_counters_on(view: &PlayerView, id: ObjectId, kind: baylee_view::CounterKind) -> u16 {
    view.object(id)
        .expect("object is on battlefield")
        .counters
        .iter()
        .find(|c| c.kind == kind)
        .map_or(0, |c| c.count)
}

fn advance_to_step(
    engine: &mut Engine<RegistryLookup>,
    active: PlayerId,
    turn: u32,
    step: baylee_view::Step,
) -> PlayerView {
    for seq in 0..250 {
        let pending = engine.pending().clone();
        let seat = pending_player(&pending).expect("live game");
        let view = asked_view(engine.state(), seat, seq, &pending);
        if view.turn == turn && view.active == active && view.step == step && view.stack.is_empty()
        {
            return view;
        }
        let action = match pending {
            Pending::Mulligan { .. } => PlayerAction::MulliganKeep,
            Pending::Priority { .. } => PlayerAction::PassPriority,
            Pending::ChooseAttackers { .. } => PlayerAction::DeclareAttackers { attackers: vec![] },
            Pending::ChooseBlockers { .. } => PlayerAction::DeclareBlockers { blockers: vec![] },
            other => panic!("unexpected prompt for untargeted clock test: {other:?}"),
        };
        engine.apply(seat, action).expect("legal pass");
    }
    panic!("never reached turn {turn} step {step:?}");
}

#[test]
fn infinite_hourglass_counts_upkeep_pumps_all_creatures_and_allows_upkeep_removal() {
    let (me, them) = (PlayerId::new(0), PlayerId::new(1));
    let mut preset = position(
        &[],
        &[
            "Infinite Hourglass",
            "Grizzly Bears",
            "Forest",
            "Forest",
            "Forest",
        ],
    );
    preset.seats[1].starting_battlefield = vec![
        entry("Hill Giant"),
        entry("Forest"),
        entry("Forest"),
        entry("Forest"),
    ];
    let mut engine = Engine::new(&preset, RegistryLookup).unwrap();

    let main1 = advance_to_step(&mut engine, me, 1, baylee_view::Step::Main);
    let hourglass = untapped(&main1, "Infinite Hourglass")[0];
    let bears = untapped(&main1, "Grizzly Bears")[0];
    let giant = main1
        .battlefield_of(them)
        .find(|o| o.card.is_some_and(|c| c.index == entry("Hill Giant").card))
        .expect("Hill Giant on seat 1 battlefield")
        .id;
    assert_eq!(
        time_counters_on(&main1, hourglass, baylee_view::CounterKind::Time),
        1
    );
    assert_eq!(main1.object(bears).unwrap().power, Some(3));
    assert_eq!(main1.object(giant).unwrap().power, Some(4));
    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("expected priority");
    };
    assert!(
        legal.abilities.iter().all(|(obj, _)| *obj != hourglass),
        "Infinite Hourglass may only be activated during an upkeep step"
    );

    let opp_upkeep = advance_to_step(&mut engine, them, 2, baylee_view::Step::Upkeep);
    assert_eq!(
        time_counters_on(&opp_upkeep, hourglass, baylee_view::CounterKind::Time),
        1,
        "opponent upkeep adds no time counter"
    );
    for forest in opp_upkeep
        .battlefield_of(them)
        .filter(|o| o.name == "Forest" && !o.status.contains(baylee_view::ObjectStatus::TAPPED))
        .map(|o| o.id)
        .collect::<Vec<_>>()
    {
        engine
            .apply(them, PlayerAction::ActivateManaAbility { source: forest })
            .unwrap();
    }
    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("expected opponent priority in upkeep");
    };
    let &(_, ability_index) = legal
        .abilities
        .iter()
        .find(|(obj, _)| *obj == hourglass)
        .expect("any player may activate counter removal during upkeep");
    engine
        .apply(
            them,
            PlayerAction::ActivateAbility {
                source: hourglass,
                ability_index,
            },
        )
        .unwrap();
    advance_to_step(&mut engine, them, 2, baylee_view::Step::Main);
    let after = asked_view(engine.state(), me, 0, engine.pending());
    assert_eq!(
        time_counters_on(&after, hourglass, baylee_view::CounterKind::Time),
        0
    );
    assert_eq!(after.object(bears).unwrap().power, Some(2));
    assert_eq!(after.object(giant).unwrap().power, Some(3));
}

#[test]
fn self_only_time_and_lore_counter_arrivals_never_target_clock_score() {
    probe_soultether_and_lavacore();
    probe_kes_and_scroll();
    probe_as_foretold_and_myth_realized();
}

fn probe_soultether_and_lavacore() {
    let (me, them) = (PlayerId::new(0), PlayerId::new(1));
    let p1 = position(
        &["Soultether Golem", "Llanowar Elves"],
        &["Forest", "Forest", "Forest", "Forest"],
    );
    let mut e1 = Engine::new(&p1, RegistryLookup).unwrap();
    let v1 = advance_to_step(&mut e1, me, 1, baylee_view::Step::Main);
    float(&mut e1, &untapped(&v1, "Forest")[..3]);
    let golem_card = v1
        .hand
        .iter()
        .find(|c| c.name == "Soultether Golem")
        .unwrap()
        .id;
    let elf_card = v1
        .hand
        .iter()
        .find(|c| c.name == "Llanowar Elves")
        .unwrap()
        .id;
    e1.apply(me, PlayerAction::CastSpell { card: golem_card })
        .unwrap();
    settle(&mut e1, &[]);
    let v1 = asked_view(e1.state(), me, 0, e1.pending());
    let golem = untapped(&v1, "Soultether Golem")[0];
    assert_eq!(
        time_counters_on(&v1, golem, baylee_view::CounterKind::Time),
        1
    );
    float(&mut e1, &untapped(&v1, "Forest")[..1]);
    e1.apply(me, PlayerAction::CastSpell { card: elf_card })
        .unwrap();
    settle(&mut e1, &[]);
    let v1 = asked_view(e1.state(), me, 0, e1.pending());
    assert_eq!(
        time_counters_on(&v1, golem, baylee_view::CounterKind::Time),
        2
    );

    let p2 = position(
        &["Lavacore Elemental"],
        &["Grizzly Bears", "Mountain", "Forest", "Forest"],
    );
    let mut e2 = Engine::new(&p2, RegistryLookup).unwrap();
    let v2 = advance_to_step(&mut e2, me, 1, baylee_view::Step::Main);
    for forest_or_mtn in untapped(&v2, "Forest")
        .into_iter()
        .chain(untapped(&v2, "Mountain"))
    {
        e2.apply(
            me,
            PlayerAction::ActivateManaAbility {
                source: forest_or_mtn,
            },
        )
        .unwrap();
    }
    e2.apply(
        me,
        PlayerAction::CastSpell {
            card: v2.hand[0].id,
        },
    )
    .unwrap();
    settle(&mut e2, &[]);
    let v2 = advance_to_step(&mut e2, me, 1, baylee_view::Step::DeclareAttackers);
    let bears = untapped(&v2, "Grizzly Bears")[0];
    e2.apply(
        me,
        PlayerAction::DeclareAttackers {
            attackers: vec![(bears, baylee_core::ids::Defender::Player(them))],
        },
    )
    .unwrap();
    let v2 = advance_to_step(&mut e2, me, 1, baylee_view::Step::End);
    let elem = v2
        .battlefield_of(me)
        .find(|o| {
            o.card
                .is_some_and(|c| c.index == entry("Lavacore Elemental").card)
        })
        .unwrap()
        .id;
    assert_eq!(
        time_counters_on(&v2, elem, baylee_view::CounterKind::Time),
        2
    );
}

fn probe_kes_and_scroll() {
    let (me, them) = (PlayerId::new(0), PlayerId::new(1));
    let p3 = position(&[], &["Kes, Evolving Telepath"]);
    let mut e3 = Engine::new(&p3, RegistryLookup).unwrap();
    let v3 = advance_to_step(&mut e3, me, 1, baylee_view::Step::Main);
    let hand_before = v3.hand.len();
    let v3_opp = advance_to_step(&mut e3, them, 2, baylee_view::Step::Main);
    let kes = v3_opp
        .battlefield_of(me)
        .find(|o| {
            o.card
                .is_some_and(|c| c.index == entry("Kes, Evolving Telepath").card)
        })
        .unwrap()
        .id;
    assert_eq!(
        time_counters_on(&v3_opp, kes, baylee_view::CounterKind::Time),
        1
    );
    let hand_after = asked_view(e3.state(), me, 0, e3.pending()).hand.len();
    assert_eq!(hand_after, hand_before + 1);

    let p4 = position(
        &["Scroll of the Masters", "Sol Ring"],
        &[
            "Grizzly Bears",
            "Forest",
            "Forest",
            "Forest",
            "Forest",
            "Forest",
            "Forest",
        ],
    );
    let mut e4 = Engine::new(&p4, RegistryLookup).unwrap();
    let v4 = advance_to_step(&mut e4, me, 1, baylee_view::Step::Main);
    float(&mut e4, &untapped(&v4, "Forest")[..3]);
    let scroll_card = v4
        .hand
        .iter()
        .find(|c| c.name == "Scroll of the Masters")
        .unwrap()
        .id;
    let ring_card = v4.hand.iter().find(|c| c.name == "Sol Ring").unwrap().id;
    e4.apply(me, PlayerAction::CastSpell { card: scroll_card })
        .unwrap();
    settle(&mut e4, &[]);
    e4.apply(me, PlayerAction::CastSpell { card: ring_card })
        .unwrap();
    settle(&mut e4, &[]);
    let v4 = asked_view(e4.state(), me, 0, e4.pending());
    let scroll = untapped(&v4, "Scroll of the Masters")[0];
    let bears = untapped(&v4, "Grizzly Bears")[0];
    assert_eq!(
        time_counters_on(&v4, scroll, baylee_view::CounterKind::Lore),
        1
    );
    float(&mut e4, &untapped(&v4, "Forest")[..3]);
    e4.apply(
        me,
        PlayerAction::ActivateAbility {
            source: scroll,
            ability_index: 1,
        },
    )
    .unwrap();
    e4.apply(
        me,
        PlayerAction::ChooseObjects {
            objects: vec![bears],
        },
    )
    .unwrap();
    settle(&mut e4, &[]);
    let v4 = asked_view(e4.state(), me, 0, e4.pending());
    assert_eq!(v4.object(bears).unwrap().power, Some(3));
}

fn probe_as_foretold_and_myth_realized() {
    let me = PlayerId::new(0);
    // As Foretold adds a self-only time counter on upkeep (no targeting) and
    // lets its controller cast a spell with mana value <= 1 for {0}.
    let p5 = position(&["Sol Ring"], &["As Foretold"]);
    let mut e5 = Engine::new(&p5, RegistryLookup).unwrap();
    let v5 = advance_to_step(&mut e5, me, 1, baylee_view::Step::Main);
    let foretold = untapped(&v5, "As Foretold")[0];
    assert_eq!(
        time_counters_on(&v5, foretold, baylee_view::CounterKind::Time),
        1
    );
    let ring_card = v5.hand.iter().find(|c| c.name == "Sol Ring").unwrap().id;
    e5.apply(me, PlayerAction::CastSpell { card: ring_card })
        .unwrap();
    settle(&mut e5, &[]);
    let v5_after = asked_view(e5.state(), me, 0, e5.pending());
    assert!(
        v5_after.battlefield_of(me).any(|o| o.name == "Sol Ring"),
        "As Foretold's self-only time counter pays for a 1-MV spell without targeting"
    );

    // Myth Realized puts self-only lore counters on itself via both its
    // noncreature spell trigger and its {2}{W} activation (no targeting).
    let p6 = position(
        &["Sol Ring"],
        &["Myth Realized", "Plains", "Plains", "Plains", "Plains"],
    );
    let mut e6 = Engine::new(&p6, RegistryLookup).unwrap();
    let v6 = advance_to_step(&mut e6, me, 1, baylee_view::Step::Main);
    float(&mut e6, &untapped(&v6, "Plains")[..1]);
    let ring6 = v6.hand.iter().find(|c| c.name == "Sol Ring").unwrap().id;
    e6.apply(me, PlayerAction::CastSpell { card: ring6 })
        .unwrap();
    settle(&mut e6, &[]);
    let v6 = asked_view(e6.state(), me, 0, e6.pending());
    let myth = untapped(&v6, "Myth Realized")[0];
    assert_eq!(
        time_counters_on(&v6, myth, baylee_view::CounterKind::Lore),
        1
    );
    float(&mut e6, &untapped(&v6, "Plains")[..3]);
    e6.apply(
        me,
        PlayerAction::ActivateAbility {
            source: myth,
            ability_index: 1,
        },
    )
    .unwrap();
    settle(&mut e6, &[]);
    let v6 = asked_view(e6.state(), me, 0, e6.pending());
    assert_eq!(
        time_counters_on(&v6, myth, baylee_view::CounterKind::Lore),
        2
    );
}

fn adapt_and_infect_turn(
    engine: &mut Engine<RegistryLookup>,
    active: PlayerId,
    defender: PlayerId,
    turn: u32,
) {
    let view = advance_to_step(engine, active, turn, baylee_view::Step::Main);
    let myr = view
        .battlefield_of(active)
        .find(|o| o.name == "Knighted Myr")
        .expect("Knighted Myr on active side")
        .id;
    for plains in view
        .battlefield_of(active)
        .filter(|o| o.name == "Plains" && !o.status.contains(baylee_view::ObjectStatus::TAPPED))
        .take(3)
        .map(|o| o.id)
        .collect::<Vec<_>>()
    {
        engine
            .apply(active, PlayerAction::ActivateManaAbility { source: plains })
            .unwrap();
    }
    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("expected priority for Adapt");
    };
    let &(_, ability_index) = legal
        .abilities
        .iter()
        .find(|(obj, _)| *obj == myr)
        .expect("Adapt 1 ability legal");
    engine
        .apply(
            active,
            PlayerAction::ActivateAbility {
                source: myr,
                ability_index,
            },
        )
        .unwrap();
    let atk_view = advance_to_step(engine, active, turn, baylee_view::Step::DeclareAttackers);
    let stinger = atk_view
        .battlefield_of(active)
        .find(|o| o.name == "Plague Stinger")
        .expect("Plague Stinger on active side")
        .id;
    engine
        .apply(
            active,
            PlayerAction::DeclareAttackers {
                attackers: vec![(stinger, baylee_core::ids::Defender::Player(defender))],
            },
        )
        .unwrap();
    advance_to_step(engine, active, turn, baylee_view::Step::End);
}

#[test]
fn proliferate_treats_poison_as_hostile_and_plus_one_counters_as_friendly() {
    let (me, them) = (PlayerId::new(0), PlayerId::new(1));
    let mut preset = position(
        &["Experimental Augury"],
        &[
            "Plague Stinger",
            "Knighted Myr",
            "Plains",
            "Plains",
            "Plains",
            "Island",
            "Island",
        ],
    );
    preset.seats[1].starting_battlefield = vec![
        entry("Plague Stinger"),
        entry("Knighted Myr"),
        entry("Plains"),
        entry("Plains"),
        entry("Plains"),
    ];
    let mut engine = Engine::new(&preset, RegistryLookup).unwrap();
    adapt_and_infect_turn(&mut engine, me, them, 1);
    adapt_and_infect_turn(&mut engine, them, me, 2);

    let main3 = advance_to_step(&mut engine, me, 3, baylee_view::Step::Main);
    let my_myr = find_named_on_side(&main3, me, "Knighted Myr");
    let their_myr = find_named_on_side(&main3, them, "Knighted Myr");
    assert_eq!(main3.seats[0].poison, 1);
    assert_eq!(main3.seats[1].poison, 1);
    assert_eq!(p1p1_on(&main3, my_myr), 1);
    assert_eq!(p1p1_on(&main3, their_myr), 1);

    for island in main3
        .battlefield_of(me)
        .filter(|o| o.name == "Island" && !o.status.contains(baylee_view::ObjectStatus::TAPPED))
        .take(2)
        .map(|o| o.id)
        .collect::<Vec<_>>()
    {
        engine
            .apply(me, PlayerAction::ActivateManaAbility { source: island })
            .unwrap();
    }
    let augury = main3
        .hand
        .iter()
        .find(|c| c.name == "Experimental Augury")
        .unwrap()
        .id;
    engine
        .apply(me, PlayerAction::CastSpell { card: augury })
        .unwrap();

    let agent = HeuristicAgent::new(AIProfile::EXPERT);
    for seq in 0..50 {
        let pending = engine.pending().clone();
        let seat = pending_player(&pending).expect("live game");
        let view = asked_view(engine.state(), seat, seq, &pending);
        if matches!(pending, Pending::Priority { .. }) && view.stack.is_empty() {
            break;
        }
        let action = match pending {
            Pending::Priority { .. } => PlayerAction::PassPriority,
            _ => agent.act_with_context(&view, &pending, &engine.decision_context()),
        };
        engine.apply(seat, action).unwrap();
    }

    let after = asked_view(engine.state(), me, 0, engine.pending());
    assert!(after.stack.is_empty(), "Experimental Augury must resolve");
    assert_eq!(
        p1p1_on(&after, my_myr),
        2,
        "friendly +1/+1 counter proliferated"
    );
    assert_eq!(
        p1p1_on(&after, their_myr),
        1,
        "opposing +1/+1 counter untouched"
    );
    assert_eq!(
        after.seats[1].poison, 2,
        "hostile poison counter proliferated"
    );
    assert_eq!(after.seats[0].poison, 1, "own poison counter avoided");
}

fn find_named_on_side(view: &PlayerView, side: PlayerId, name: &str) -> ObjectId {
    view.battlefield_of(side)
        .find(|o| o.name == name)
        .expect("expected permanent")
        .id
}

fn p1p1_on(view: &PlayerView, id: ObjectId) -> u16 {
    time_counters_on(
        view,
        id,
        baylee_view::CounterKind::Plus {
            power: 1,
            toughness: 1,
        },
    )
}
