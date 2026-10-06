use super::*;

#[test]
fn third_iteration_burn_finishes_the_player_before_killing_a_creature() {
    use baylee_cards_dsl::{Amount, Effect, TargetSpec};
    let v = view(0, &[20, 3], vec![permanent(obj(1), PlayerId::new(1), 1)]);
    for effect in [
        Effect::DealDamage {
            amount: Amount::Fixed(3),
            target: TargetSpec::AnyTarget,
        },
        Effect::DealDamageWithCappedLifeGain {
            amount: Amount::Fixed(3),
        },
    ] {
        let effects = [effect];
        let context = baylee_engine::engine::DecisionContext {
            effects: &effects,
            ..Default::default()
        };
        let pending = Pending::ChooseTargets {
            player: v.seat,
            options: vec![obj(1)],
            player_options: vec![v.seat, PlayerId::new(1)],
            min: 1,
            max: 1,
            reason: baylee_engine::choice::TargetPrompt::Targets,
        };
        assert_eq!(
            HeuristicAgent::new(AIProfile::EXPERT).act_with_context(&v, &pending, &context),
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![PlayerId::new(1)]
            }
        );
    }
}

/// #87. The policy seed is a recorded derivation, not a hash of the day.
///
/// Four things at once, because they fail separately: it is stable
/// across runs and toolchains (the pinned value is the whole point of
/// calling a seed *recorded* — a replay has to reproduce the chair as
/// well as the shuffle), two tables differ, two seats at one table
/// differ, and the length is part of the input so one table's chair
/// cannot collide with another's by the two strings running together.
#[test]
fn a_policy_seed_is_recorded_and_belongs_to_one_seat_at_one_table() {
    assert_eq!(
        policy_seed("g1", 0),
        0xa485_49e4_7ab1_f0a8,
        "the derivation changed without POLICY_SEED_VERSION changing \
         with it, so every recorded game replays a different chair"
    );
    assert_eq!(policy_seed("g1", 0), policy_seed("g1", 0), "not stable");
    // Every pair of a small grid is its own value, which is the property
    // the one-byte suffix buys: no table's chair is another's.
    let grid: Vec<u64> = ["", "g1", "g2", "a game with spaces"]
        .iter()
        .flat_map(|game| (0..8u8).map(move |seat| policy_seed(game, seat)))
        .collect();
    let mut seen = grid.clone();
    seen.sort_unstable();
    seen.dedup();
    assert_eq!(
        seen.len(),
        grid.len(),
        "two seats or two tables were handed one stream of randomness"
    );
}

#[test]
fn third_iteration_seeded_variation_is_repeatable_and_breaks_equal_spell_ties() {
    let mut v = view(0, &[20, 20], vec![]);
    v.hand = vec![hand_card(1, "Brainstorm"), hand_card(2, "Brainstorm")];
    let pending = Pending::Priority {
        player: v.seat,
        legal: Box::new(baylee_engine::choice::LegalActions {
            castable: vec![obj(1), obj(2)],
            ..Default::default()
        }),
    };
    for profile in [AIProfile::SHARP, AIProfile::EXPERT] {
        let choices: Vec<_> = (0..32)
            .map(|seed| {
                let agent = HeuristicAgent::new(profile).with_seed(seed);
                let action = agent.act(&v, &pending);
                for _ in 0..5 {
                    assert_eq!(action, agent.act(&v, &pending));
                }
                action
            })
            .collect();
        assert!(choices.iter().any(|a| *a != choices[0]));
    }
}

#[test]
fn third_iteration_scouted_sweeper_changes_deployment_but_not_the_base_agent() {
    use intelligence::{DeckIntel, ScoutedSeat, ScoutingReport};
    let mut v = view(
        0,
        &[20, 20],
        vec![
            permanent(obj(1), PlayerId::new(0), 2),
            permanent(obj(2), PlayerId::new(0), 2),
        ],
    );
    v.hand = vec![hand_card(3, "Baleful Strix"), hand_card(4, "Brainstorm")];
    let own = DeckIntel::new(vec![v.hand[0].card.index; 20], vec![]);
    let enemy = DeckIntel::new(
        vec![baylee_cards::decks::by_name("Toxic Deluge").unwrap(); 20],
        vec![],
    );
    let report = ScoutingReport {
        seats: vec![
            ScoutedSeat {
                player: v.seat,
                deck: &own,
                hand: None,
                library: None,
                sideboard: None,
            },
            ScoutedSeat {
                player: PlayerId::new(1),
                deck: &enemy,
                hand: Some(enemy.cards[..1].to_vec()),
                library: None,
                sideboard: None,
            },
        ],
    };
    let pending = Pending::Priority {
        player: v.seat,
        legal: Box::new(baylee_engine::choice::LegalActions {
            castable: vec![obj(3), obj(4)],
            ..Default::default()
        }),
    };
    let agent = HeuristicAgent::new(AIProfile::EXPERT);
    let ordinary = agent.act(&v, &pending);
    assert_eq!(ordinary, PlayerAction::CastSpell { card: obj(3) });
    assert_eq!(
        agent.act_with_scouting(
            &v,
            &pending,
            &baylee_engine::engine::DecisionContext::default(),
            &report
        ),
        PlayerAction::CastSpell { card: obj(4) }
    );
    assert_eq!(agent.act(&v, &pending), ordinary);
}

#[test]
fn third_iteration_commander_damage_is_a_separate_loss_condition() {
    let mut commander = permanent(obj(1), PlayerId::new(1), 2);
    commander.commander = true;
    let mut v = view(
        0,
        &[40, 40],
        vec![commander, permanent(obj(2), PlayerId::new(0), 1)],
    );
    v.seats[0].commander_damage = vec![baylee_view::CommanderDamage {
        source: obj(1),
        amount: 19,
    }];
    v.combat.attackers = vec![baylee_view::AttackerView {
        creature: obj(1),
        defending: Defender::Player(v.seat),
        blocked: false,
    }];
    let pending = Pending::ChooseBlockers {
        demands: Vec::new(),
        player: v.seat,
        attacker: PlayerId::new(1),
        blockers: vec![baylee_engine::choice::BlockOption {
            blocker: obj(2),
            attackers: vec![obj(1)],
        }],
        capacity: Vec::new(),
        obeying: Vec::new(),
        bounds: Vec::new(),
    };
    let agent = HeuristicAgent::new(AIProfile::EXPERT);
    assert_eq!(
        agent.act(&v, &pending),
        PlayerAction::DeclareBlockers {
            blockers: vec![(obj(2), obj(1))]
        }
    );
    v.seats[0].commander_damage[0].source = obj(99);
    assert_eq!(
        agent.act(&v, &pending),
        PlayerAction::DeclareBlockers { blockers: vec![] },
        "a different commander's damage must not combine with this one"
    );
}

#[test]
fn third_iteration_x_uses_affordable_coloured_mana() {
    let mut v = view(0, &[20, 20], vec![]);
    v.seats[0].mana_pool.blue = 3;
    let context = baylee_engine::engine::DecisionContext {
        cost: Some("{X}{U}".parse().unwrap()),
        ..Default::default()
    };
    let pending = Pending::ChooseNumber {
        player: v.seat,
        min: 0,
        max: 50,
        reason: baylee_engine::choice::NumberPrompt::X,
    };
    assert_eq!(
        HeuristicAgent::new(AIProfile::EXPERT).act_with_context(&v, &pending, &context),
        PlayerAction::ChooseNumber(2)
    );
}

/// Replicate's count (CR 702.56a) is every payment offered: the engine
/// bounds it by the floating pool, so the house pays for each copy it
/// can. Read as X it was `min`, because Lose Focus's cost has no X, and
/// the house never replicated anything.
#[test]
fn replicate_pays_for_every_copy_the_pool_covers() {
    let mut v = view(0, &[20, 20], vec![]);
    v.seats[0].mana_pool.blue = 4;
    let context = baylee_engine::engine::DecisionContext {
        cost: Some("{1}{U}".parse().unwrap()),
        ..Default::default()
    };
    let pending = Pending::ChooseNumber {
        player: v.seat,
        min: 0,
        max: 2,
        reason: baylee_engine::choice::NumberPrompt::Replicate {
            cost: "{U}".parse().unwrap(),
        },
    };
    assert_eq!(
        HeuristicAgent::new(AIProfile::EXPERT).act_with_context(&v, &pending, &context),
        PlayerAction::ChooseNumber(2)
    );
}

#[test]
fn optional_counter_refill_is_valuable_only_below_its_cap() {
    use crate::worth::{Aim, Origin};
    use baylee_cards_dsl::{Amount, Effect};
    let kind = baylee_cards_dsl::CounterKind::Plus {
        power: 1,
        toughness: 0,
    };
    for (existing, expected) in [(6, 110), (7, 0), (9, 0)] {
        let mut creature = permanent(obj(1), PlayerId::new(0), 3);
        creature.counters.push(CounterEntry {
            kind: CounterKind::Plus {
                power: 1,
                toughness: 0,
            },
            count: existing,
        });
        let v = view(0, &[20, 20], vec![creature]);
        assert_eq!(
            agent().effects_worth(
                &v,
                Origin::of(&v, obj(1)),
                &[Effect::AddCountersUpTo {
                    kind,
                    amount: Amount::X,
                    maximum: 7
                }],
                Aim::Source,
                4
            ),
            Some(expected)
        );
    }
}

#[test]
fn resolving_counter_amount_uses_value_not_the_already_spent_mana() {
    let v = view(0, &[20, 20], vec![permanent(obj(1), PlayerId::new(0), 3)]);
    for (kind, expected) in [
        (
            baylee_cards_dsl::CounterKind::Plus {
                power: 1,
                toughness: 0,
            },
            4,
        ),
        (
            baylee_cards_dsl::CounterKind::Minus {
                power: 1,
                toughness: 1,
            },
            0,
        ),
    ] {
        let pending = Pending::ChooseNumber {
            player: v.seat,
            min: 0,
            max: 4,
            reason: baylee_engine::choice::NumberPrompt::Counters {
                target: obj(1),
                kind,
            },
        };
        assert_eq!(
            HeuristicAgent::new(AIProfile::EXPERT).act_with_context(
                &v,
                &pending,
                &baylee_engine::engine::DecisionContext::default()
            ),
            PlayerAction::ChooseNumber(expected)
        );
    }
}

#[test]
fn miracle_counts_untapped_sources_and_requires_the_right_color() {
    let mut v = view(
        0,
        &[20, 20],
        vec![
            carded(
                permanent(obj(2), PlayerId::new(0), 0),
                "Island",
                TypeSet::LAND,
            ),
            carded(
                permanent(obj(3), PlayerId::new(0), 0),
                "Island",
                TypeSet::LAND,
            ),
        ],
    );
    for land in &mut v.battlefield {
        land.subtypes
            .insert(baylee_core::generated::subtypes::land::ISLAND);
    }
    v.hand = vec![hand_card(1, "Temporal Mastery")];
    let pending = Pending::YesNo {
        player: v.seat,
        prompt: YesNoPrompt::Miracle { card: obj(1) },
        source: None,
    };
    let agent = HeuristicAgent::new(AIProfile::EXPERT);
    assert_eq!(agent.act(&v, &pending), PlayerAction::YesNo(true));
    for land in &mut v.battlefield {
        land.status = ObjectStatus::TAPPED;
    }
    assert_eq!(agent.act(&v, &pending), PlayerAction::YesNo(false));
    v.seats[0].mana_pool.black = 2;
    assert_eq!(agent.act(&v, &pending), PlayerAction::YesNo(false));
    v.seats[0].mana_pool.black = 0;
    v.seats[0].mana_pool.blue = 2;
    assert_eq!(agent.act(&v, &pending), PlayerAction::YesNo(true));
}

#[test]
fn miracle_x_counts_the_sources_its_payment_window_will_offer() {
    let mut land = carded(
        permanent(obj(2), PlayerId::new(0), 0),
        "Island",
        TypeSet::LAND,
    );
    land.subtypes
        .insert(baylee_core::generated::subtypes::land::ISLAND);
    let mut v = view(0, &[20, 20], vec![land]);
    v.seats[0].mana_pool.blue = 2;
    let mut context = baylee_engine::engine::DecisionContext {
        cost: Some("{X}{U}".parse().unwrap()),
        cast_mode: Some(baylee_engine::choice::CastModeKind::Miracle),
        ..Default::default()
    };
    let pending = Pending::ChooseNumber {
        player: v.seat,
        min: 0,
        max: 50,
        reason: baylee_engine::choice::NumberPrompt::X,
    };
    let agent = HeuristicAgent::new(AIProfile::EXPERT);
    assert_eq!(
        agent.act_with_context(&v, &pending, &context),
        PlayerAction::ChooseNumber(2)
    );
    context.cast_mode = Some(baylee_engine::choice::CastModeKind::Normal);
    assert_eq!(
        agent.act_with_context(&v, &pending, &context),
        PlayerAction::ChooseNumber(1)
    );
}

#[test]
fn a_counterspell_does_not_counter_its_own_spell_to_answer_an_enemy_ability() {
    let mut v = view(0, &[20, 20], vec![]);
    v.hand = vec![hand_card(3, "Counterspell")];
    let mut friendly = carded(permanent(obj(1), v.seat, 0), "Brainstorm", TypeSet::INSTANT);
    friendly.stack_item = Some(baylee_view::StackItem::Spell);
    let mut enemy = permanent(obj(2), PlayerId::new(1), 0);
    enemy.stack_item = Some(baylee_view::StackItem::Ability {
        token: None,
        source: obj(99),
        ability: None,
        text: None,
        rules: None,
    });
    v.stack = vec![friendly, enemy];
    baylee_client_core::test_support::project_current_targets(&mut v);
    let pending = Pending::Priority {
        player: v.seat,
        legal: Box::new(baylee_engine::choice::LegalActions {
            castable: vec![obj(3)],
            ..Default::default()
        }),
    };
    assert_eq!(
        HeuristicAgent::new(AIProfile::EXPERT).act(&v, &pending),
        PlayerAction::PassPriority
    );
}

#[test]
fn expert_keeps_a_blocker_against_lethal_commander_retaliation() {
    let mut enemy = permanent(obj(2), PlayerId::new(1), 2);
    enemy.commander = true;
    enemy.status = ObjectStatus::TAPPED;
    let mut v = view(
        0,
        &[40, 40],
        vec![permanent(obj(1), PlayerId::new(0), 6), enemy],
    );
    v.seats[0].commander_damage = vec![baylee_view::CommanderDamage {
        source: obj(2),
        amount: 19,
    }];
    let pending = Pending::ChooseAttackers {
        player: v.seat,
        attackers: vec![obj(1)],
        defenders: vec![Defender::Player(PlayerId::new(1))],
        required: Vec::new(),
        limits: Vec::new(),
    };
    assert_eq!(
        HeuristicAgent::new(AIProfile::EXPERT).act(&v, &pending),
        PlayerAction::DeclareAttackers { attackers: vec![] }
    );
}

#[test]
fn skilled_mulligans_refuse_a_landless_seven_but_stop_at_four() {
    let mut v = view(0, &[20, 20], vec![]);
    v.hand = (0..7).map(|i| hand_card(i, "Brainstorm")).collect();
    let decision = |taken| Pending::Mulligan {
        player: v.seat,
        taken,
        next_is_free: taken == 0,
        can_take: true,
    };
    assert_eq!(
        HeuristicAgent::new(AIProfile::SHARP).act(&v, &decision(0)),
        PlayerAction::MulliganTake
    );
    assert_eq!(
        HeuristicAgent::new(AIProfile::SHARP).act(&v, &decision(4)),
        PlayerAction::MulliganKeep
    );
}

#[test]
fn a_mana_choice_completes_a_cast_instead_of_counting_unaffordable_pips() {
    let mut swamp = carded(
        permanent(obj(1), PlayerId::new(0), 0),
        "Swamp",
        TypeSet::LAND,
    );
    swamp
        .subtypes
        .insert(baylee_core::generated::subtypes::land::SWAMP);
    let mut v = view(0, &[20, 20], vec![swamp]);
    v.phase = baylee_view::Phase::FirstMain;
    v.hand = vec![hand_card(2, "Baleful Strix")];
    v.hand
        .extend((3..7).map(|id| hand_card(id, "Loran of the Third Path")));
    let pending = Pending::ChooseColor {
        player: v.seat,
        options: vec![
            baylee_core::mana::ManaColor::White,
            baylee_core::mana::ManaColor::Blue,
            baylee_core::mana::ManaColor::Black,
        ],
    };
    for profile in [AIProfile::STEADY, AIProfile::SHARP, AIProfile::EXPERT] {
        assert_eq!(
            HeuristicAgent::new(profile).act(&v, &pending),
            PlayerAction::ChooseColor(baylee_core::mana::ManaColor::Blue)
        );
    }
}

#[test]
fn mana_color_follows_the_spell_in_hand() {
    let mut v = view(0, &[20, 20], vec![]);
    v.hand = vec![hand_card(1, "Brainstorm")];
    let pending = Pending::ChooseColor {
        player: v.seat,
        options: vec![
            baylee_core::mana::ManaColor::White,
            baylee_core::mana::ManaColor::Blue,
        ],
    };
    assert_eq!(
        HeuristicAgent::new(AIProfile::SHARP).act(&v, &pending),
        PlayerAction::ChooseColor(baylee_core::mana::ManaColor::Blue)
    );
}
