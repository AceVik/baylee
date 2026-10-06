use super::*;

/// A counter names the spell aimed at this seat, player or permanent,
/// over one of the same cost aimed at nobody of this seat's (#226). The
/// view's `targets` is what tells them apart: both are hostile, both cost
/// one, and neither is a cantrip.
#[test]
fn a_counter_names_the_spell_aimed_at_this_seat() {
    use baylee_engine::engine::DecisionContext;
    let (me, them) = (PlayerId::new(0), PlayerId::new(1));
    let spell = |id: u32, name: &str, at: baylee_view::TargetRef| {
        let mut o = permanent(obj(id), them, 0);
        o.card = Some(hand_card(id, name).card);
        o.types = TypeSet::INSTANT;
        o.power = None;
        o.toughness = None;
        o.stack_item = Some(baylee_view::StackItem::Spell);
        o.targets = vec![at];
        o
    };
    let effects = [baylee_cards_dsl::Effect::CounterTargetSpell];
    let counterspell = DecisionContext {
        effects: &effects,
        ..Default::default()
    };
    let pending = Pending::ChooseTargets {
        player: me,
        options: vec![obj(1), obj(2)],
        player_options: vec![],
        min: 1,
        max: 1,
        reason: baylee_engine::choice::TargetPrompt::Targets,
    };
    let mut v = view(
        0,
        &[20, 20],
        vec![permanent(obj(9), me, 2), permanent(obj(8), them, 2)],
    );
    for (aimed, named, why) in [
        (
            baylee_view::TargetRef::Player(me),
            obj(2),
            "the Bolt at this seat",
        ),
        (
            baylee_client_core::test_support::target(obj(9)),
            obj(2),
            "the Bolt at this seat's creature",
        ),
        // Saving an opponent's creature is worth nothing: a tie, and the
        // first listed is named.
        (
            baylee_client_core::test_support::target(obj(8)),
            obj(1),
            "not the Bolt at an opponent's creature",
        ),
    ] {
        // The one aimed at nobody of mine is listed first, so a tie
        // names it.
        v.stack = vec![
            spell(1, "Ancestral Recall", baylee_view::TargetRef::Player(them)),
            spell(2, "Lightning Bolt", aimed),
        ];
        baylee_client_core::test_support::project_current_targets(&mut v);
        assert_eq!(
            agent().act_with_context(&v, &pending, &counterspell),
            PlayerAction::ChooseTargets {
                objects: vec![named],
                players: vec![]
            },
            "{why} is named"
        );
    }
}

/// Which of the pool's spells the counter gate lets resolve (#226), by
/// name. The predicate is a positive list over effects, so a card written
/// later with only those effects joins it without anybody deciding it
/// should, and this list moves. Ancestral Recall (it draws three), Night's
/// Whisper (it costs life) and every permanent spell stay out.
///
/// Three are `Partial`, and are here because the engine does only what
/// is written: Borne Upon a Wind's flash, Gitaxian Probe's look at a hand
/// and Open Communications' Beam me up are not, so each draws one card.
/// Writing that half adds an effect the list does not name, and the card
/// leaves.
#[test]
fn the_counter_gate_lets_only_a_cantrip_resolve() {
    let mut held: Vec<&str> = baylee_cards::all()
        .filter(|def| {
            let mut o = permanent(obj(1), PlayerId::new(1), 0);
            o.card = Some(baylee_view::CardIdentity {
                index: def.index,
                print: baylee_core::ids::PrintRef::new(0),
                face: 0,
            });
            o.types = def.faces[0].types;
            crate::tactics::only_replaces_itself(&o)
        })
        .map(baylee_cards_dsl::CardDef::name)
        .collect();
    held.sort_unstable();

    // An ability on the stack arrives with no types and its source's
    // card: Opt's card with an ability's blank base is not Opt.
    let mut ability = permanent(obj(1), PlayerId::new(1), 0);
    ability.card = Some(hand_card(1, "Opt").card);
    ability.types = TypeSet::EMPTY;
    ability.stack_item = Some(baylee_view::StackItem::Ability {
        token: None,
        source: obj(2),
        ability: None,
        text: None,
        rules: None,
    });
    assert!(
        !crate::tactics::only_replaces_itself(&ability),
        "an ability was read as the spell its source card prints"
    );

    assert_eq!(
        held,
        [
            "Borne Upon a Wind",
            "Brainstorm",
            "Gitaxian Probe",
            "Open Communications",
            "Opt",
            "Reach Through Mists",
            "Serum Visions",
        ]
    );
}

/// One object named, as a target answer.
fn named(id: ObjectId) -> PlayerAction {
    PlayerAction::ChooseTargets {
        objects: vec![id],
        players: vec![],
    }
}

/// A redirect's two questions (#226). Which spell, as it is cast: an
/// opponent's removal aimed at this seat's creature, over this seat's own
/// aimed at the same creature and over an opponent's aimed elsewhere,
/// however they are listed; the gate that casts it asks the same, and
/// finds nothing in an opponent's gift to this seat. What it becomes, as
/// it resolves: a turned Path is removal and goes to the bigger of two
/// creatures across the table, the one listed second, unless another
/// Path is already on its way there; a pump is not. The questions are
/// told apart by the stack, which holds the resolving redirect.
/// Hydroelectric Specimen's trigger prints its redirect behind a "you
/// may", and is read through it.
#[test]
fn a_redirect_turns_the_spell_aimed_at_this_seat_onto_the_best_target() {
    use baylee_engine::engine::DecisionContext;
    let (me, them) = (PlayerId::new(0), PlayerId::new(1));
    let effects = [baylee_cards_dsl::Effect::ChangeTarget {
        to: &baylee_cards_dsl::Filter::Any,
    }];
    let misdirection = DecisionContext {
        source: Some(obj(4)),
        effects: &effects,
        ..Default::default()
    };
    // Hydroelectric Specimen's trigger says it behind a "you may".
    let may = [baylee_cards_dsl::Effect::MayDo {
        effects: &[baylee_cards_dsl::Effect::ChangeTarget {
            to: &baylee_cards_dsl::Filter::This,
        }],
    }];
    let specimen = DecisionContext {
        source: Some(obj(20)),
        effects: &may,
        ..Default::default()
    };
    let ask = |options: Vec<ObjectId>| Pending::ChooseTargets {
        player: me,
        options,
        player_options: vec![],
        min: 1,
        max: 1,
        reason: baylee_engine::choice::TargetPrompt::Targets,
    };
    let mut v = view(
        0,
        &[20, 20],
        vec![
            permanent(obj(9), me, 1),
            permanent(obj(8), them, 1),
            permanent(obj(7), them, 5),
        ],
    );
    let paths = [
        stack_spell(1, "Path to Exile", me, obj(9)),
        stack_spell(2, "Path to Exile", them, obj(8)),
        stack_spell(3, "Path to Exile", them, obj(9)),
    ];
    let mut gift = stack_spell(5, "Ancestral Recall", them, obj(5));
    gift.targets = vec![baylee_view::TargetRef::Player(me)];
    let card = hand_card(4, "Misdirection").card;
    v.stack = vec![paths[0].clone(), paths[1].clone(), gift];
    baylee_client_core::test_support::project_current_targets(&mut v);
    assert!(
        agent().spell_score(&v, card) < 0,
        "nothing of this seat's is under attack from an opponent"
    );
    for order in [[1, 2, 3], [3, 2, 1], [2, 3, 1]] {
        v.stack = order
            .iter()
            .filter_map(|&i| paths.iter().find(|p| p.id == obj(i)).cloned())
            .collect();
        baylee_client_core::test_support::project_current_targets(&mut v);
        assert!(agent().spell_score(&v, card) > 0, "listed {order:?}");
        for (context, card) in [(&misdirection, "Misdirection"), (&specimen, "the Specimen")] {
            assert_eq!(
                agent().act_with_context(&v, &ask(order.map(obj).to_vec()), context),
                named(obj(3)),
                "{card}: the Path at this seat's creature, listed {order:?}"
            );
        }
    }
    v.stack = vec![paths[2].clone(), stack_spell(4, "Misdirection", me, obj(3))];
    baylee_client_core::test_support::project_current_targets(&mut v);
    assert_eq!(
        agent().act_with_context(&v, &ask(vec![obj(8), obj(7)]), &misdirection),
        named(obj(7)),
        "the turned Path goes to the bigger creature"
    );
    // A pump on the bigger one does not save it from the Path.
    v.stack
        .insert(1, stack_spell(6, "Giant Growth", them, obj(7)));
    assert_eq!(
        agent().act_with_context(&v, &ask(vec![obj(8), obj(7)]), &misdirection),
        named(obj(7)),
        "a gift on the stack is not a creature already on its way out"
    );
    // A Path already on its way to the bigger one, this seat's or an
    // opponent's, has it covered.
    for caster in [me, them] {
        v.stack[1] = stack_spell(6, "Path to Exile", caster, obj(7));
        assert_eq!(
            agent().act_with_context(&v, &ask(vec![obj(8), obj(7)]), &misdirection),
            named(obj(8)),
            "the turned Path goes where {caster:?}'s is not"
        );
    }
}

/// A copy's two questions (#226). Which spell, as the trigger is put on
/// the stack: the one whose copy is worth most to this seat, which is
/// its own Path over an opponent's Brainstorm, though the Brainstorm has
/// the lower id, which a tie would name. Where the copy goes, as the trigger resolves: the copy
/// is the top of the stack and starts with the original's target (CR
/// 707.10), which the original already exiles, so it goes to the other
/// creature across the table, though the spent one is listed first.
#[test]
fn a_copy_is_not_aimed_where_the_original_is() {
    use baylee_engine::engine::DecisionContext;
    let (me, them) = (PlayerId::new(0), PlayerId::new(1));
    let effects = [baylee_cards_dsl::Effect::CopyTargetSpell { mods: &[] }];
    let dualcaster = DecisionContext {
        source: Some(obj(20)),
        effects: &effects,
        ..Default::default()
    };
    let ask = |options: Vec<ObjectId>| Pending::ChooseTargets {
        player: me,
        options,
        player_options: vec![],
        min: 1,
        max: 1,
        reason: baylee_engine::choice::TargetPrompt::Targets,
    };
    let mut v = view(
        0,
        &[20, 20],
        vec![
            permanent(obj(9), me, 1),
            permanent(obj(8), them, 1),
            permanent(obj(7), them, 5),
        ],
    );
    let mut brainstorm = stack_spell(1, "Brainstorm", them, obj(1));
    brainstorm.targets.clear();
    let path = stack_spell(6, "Path to Exile", me, obj(7));
    v.stack = vec![brainstorm.clone(), path.clone()];
    baylee_client_core::test_support::project_current_targets(&mut v);
    assert_eq!(
        agent().act_with_context(&v, &ask(vec![obj(1), obj(6)]), &dualcaster),
        named(obj(6)),
        "the copy of this seat's own Path is worth more"
    );
    let mut trigger = permanent(obj(2), me, 0);
    trigger.types = TypeSet::EMPTY;
    trigger.stack_item = Some(baylee_view::StackItem::Ability {
        token: None,
        source: obj(20),
        ability: None,
        text: None,
        rules: None,
    });
    trigger.targets = vec![baylee_client_core::test_support::target(obj(6))];
    let copy = stack_spell(3, "Path to Exile", me, obj(7));
    v.stack = vec![brainstorm, path, trigger, copy];
    baylee_client_core::test_support::project_current_targets(&mut v);
    assert_eq!(
        agent().act_with_context(&v, &ask(vec![obj(9), obj(7), obj(8)]), &dualcaster),
        named(obj(8)),
        "the copy goes where the original is not"
    );
}

/// A replicate copy's new target (CR 707.10c). The trigger's source is
/// this seat's Lose Focus, still on the stack below it and aimed at the
/// opponent's second Ritual; the copy starts there too. It is offered
/// the first Ritual and this seat's own Lose Focus, and turns onto the
/// Ritual, as a counterspell should. With only its own Lose Focus to
/// turn onto it keeps its target: "you may choose new targets" is
/// answered by naming nothing (min 0), and a copy aimed at its own
/// original would counter it.
#[test]
fn a_replicate_copy_counters_their_other_spell_and_never_its_own() {
    use baylee_engine::engine::DecisionContext;
    let (me, them) = (PlayerId::new(0), PlayerId::new(1));
    let effects = [baylee_cards_dsl::Effect::CopyThisSpell];
    let replicate = DecisionContext {
        source: Some(obj(6)),
        effects: &effects,
        ..Default::default()
    };
    let ask = |options: Vec<ObjectId>| Pending::ChooseTargets {
        player: me,
        options,
        player_options: vec![],
        min: 0,
        max: 1,
        reason: baylee_engine::choice::TargetPrompt::Targets,
    };
    let mut v = view(0, &[20, 20], vec![]);
    let mut first = stack_spell(1, "Dark Ritual", them, obj(1));
    first.targets.clear();
    let mut second = stack_spell(2, "Dark Ritual", them, obj(2));
    second.targets.clear();
    let focus = stack_spell(6, "Lose Focus", me, obj(2));
    let mut trigger = permanent(obj(7), me, 0);
    trigger.types = TypeSet::EMPTY;
    trigger.stack_item = Some(baylee_view::StackItem::Ability {
        token: None,
        source: obj(6),
        ability: None,
        text: None,
        rules: None,
    });
    trigger.targets = vec![baylee_client_core::test_support::target(obj(6))];
    let copy = stack_spell(8, "Lose Focus", me, obj(2));
    v.stack = vec![first, second, focus, trigger, copy];
    baylee_client_core::test_support::project_current_targets(&mut v);
    assert_eq!(
        agent().act_with_context(&v, &ask(vec![obj(6), obj(1)]), &replicate),
        named(obj(1)),
        "the copy counters the Ritual the original does not"
    );
    assert_eq!(
        agent().act_with_context(&v, &ask(vec![obj(6)]), &replicate),
        PlayerAction::ChooseTargets {
            objects: vec![],
            players: vec![],
        },
        "never its own Lose Focus: the copy keeps its target"
    );
}

#[test]
fn score_noise_changes_choices_without_changing_replays() {
    let mut v = view(0, &[20, 20], vec![]);
    v.phase = baylee_view::Phase::FirstMain;
    v.seats[0].mana_pool.blue = 2;
    v.hand = vec![hand_card(1, "Brainstorm"), hand_card(2, "Brainstorm")];
    let pending = Pending::Priority {
        player: v.seat,
        legal: Box::new(baylee_engine::choice::LegalActions {
            can_pass: true,
            castable: vec![obj(1), obj(2)],
            ..Default::default()
        }),
    };
    let agent = HeuristicAgent::new(AIProfile::NOVICE);
    let first = agent.act(&v, &pending);
    let mut different = false;
    for seq in 0..64 {
        v.seq = seq;
        let action = agent.act(&v, &pending);
        assert_eq!(action, agent.act(&v, &pending));
        different |= action != first;
    }
    assert!(different);
}

#[test]
fn holding_up_interaction_changes_a_tap_out() {
    let mut v = view(0, &[20, 20], vec![permanent(obj(1), PlayerId::new(0), 2)]);
    v.phase = baylee_view::Phase::FirstMain;
    v.seats[0].mana_pool.blue = 2;
    v.hand = vec![hand_card(2, "Sol Ring"), hand_card(3, "Mana Drain")];
    let pending = Pending::Priority {
        player: v.seat,
        legal: Box::new(baylee_engine::choice::LegalActions {
            can_pass: true,
            castable: vec![obj(2)],
            ..Default::default()
        }),
    };
    assert_eq!(
        HeuristicAgent::new(AIProfile::STEADY).act(&v, &pending),
        PlayerAction::PassPriority
    );
    assert_eq!(
        HeuristicAgent::new(AIProfile {
            hold_up: baylee_core::preset::HoldUp::None,
            ..AIProfile::STEADY
        })
        .act(&v, &pending),
        PlayerAction::CastSpell { card: obj(2) }
    );
    // Threat-aware releases the reserve at an empty opposing table.
    assert_eq!(
        HeuristicAgent::new(AIProfile::SHARP).act(&v, &pending),
        PlayerAction::CastSpell { card: obj(2) }
    );
}

#[test]
fn search_is_repeatable_bounded_and_preserves_unseen_identities() {
    let mut v = view(
        0,
        &[20, 20],
        (1..=6)
            .map(|i| permanent(obj(i), PlayerId::new(0), 2))
            .chain((10..=15).map(|i| permanent(obj(i), PlayerId::new(1), 3)))
            .collect(),
    );
    let squad: Vec<_> = (1..=6).map(obj).collect();
    let victim = PlayerId::new(1);
    for (_, profile) in AIProfile::NAMED {
        let first = search::attackers(&v, &squad, victim, profile);
        assert!(first.nodes <= profile.node_budget());
        for _ in 0..3 {
            let again = search::attackers(&v, &squad, victim, profile);
            assert_eq!(first.attackers, again.attackers);
            assert_eq!(first.nodes, again.nodes);
        }
    }
    // All identities here are None, including the opposing creatures.
    // Presentation changes cannot supply a hidden rules identity.
    let before = search::attackers(&v, &squad, victim, AIProfile::EXPERT).attackers;
    for o in &mut v.battlefield {
        o.name = "unseen".into();
    }
    assert_eq!(
        before,
        search::attackers(&v, &squad, victim, AIProfile::EXPERT).attackers
    );
}
