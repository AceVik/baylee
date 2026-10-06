use super::*;

/// A share of damage this seat divides (CR 601.2d): what finishes an
/// opponent's creature, its toughness less the damage already marked,
/// or a planeswalker's loyalty, within the question's bounds; the least
/// to one of its own, which leaves the most for the targets to come.
#[test]
fn a_divided_share_is_what_finishes_the_target() {
    use baylee_engine::choice::NumberPrompt;
    let (me, them) = (PlayerId::new(0), PlayerId::new(1));
    let mut hurt = permanent(obj(1), them, 3);
    hurt.damage = 1;
    let v = view(
        0,
        &[20, 20],
        vec![
            hurt,
            permanent(obj(2), them, 5),
            walker(obj(3), them, 2),
            permanent(obj(4), me, 1),
        ],
    );
    for (target, expected, why) in [
        (obj(1), 2, "a 3/3 with 1 marked"),
        (obj(2), 3, "a 5/5 takes all the question allows"),
        (obj(3), 2, "a planeswalker with 2 loyalty"),
        (obj(4), 1, "the seat's own creature"),
    ] {
        let action = HeuristicAgent::new(AIProfile::EXPERT).act(
            &v,
            &Pending::ChooseNumber {
                player: v.seat,
                min: 1,
                max: 3,
                reason: NumberPrompt::DivideDamage {
                    target,
                    index: 0,
                    of: 2,
                    left: 4,
                },
            },
        );
        assert_eq!(action, PlayerAction::ChooseNumber(expected), "{why}");
    }
}

/// An attacker's combat damage divided among its blockers (CR 510.1c):
/// what finishes each blocker in turn, the engine giving the last the
/// rest; from a deathtouch source one is lethal (CR 702.2c).
#[test]
fn a_combat_share_finishes_each_blocker_in_turn() {
    use baylee_engine::choice::NumberPrompt;
    let (me, them) = (PlayerId::new(0), PlayerId::new(1));
    let mut toucher = permanent(obj(8), me, 5);
    toucher.keywords |= baylee_cards_dsl::KeywordSet::DEATHTOUCH.bits();
    let v = view(
        0,
        &[20, 20],
        vec![
            permanent(obj(1), them, 2),
            permanent(obj(2), them, 3),
            permanent(obj(9), me, 5),
            toucher,
        ],
    );
    for (source, expected, why) in [
        (obj(9), 2, "a 2/2 takes two, and three are left for the 3/3"),
        (obj(8), 1, "deathtouch: one is lethal"),
    ] {
        let action = HeuristicAgent::new(AIProfile::EXPERT).act(
            &v,
            &Pending::ChooseNumber {
                player: v.seat,
                min: 0,
                max: 5,
                reason: NumberPrompt::CombatDamage {
                    source,
                    recipient: obj(1),
                    index: 0,
                    of: 2,
                    left: 5,
                },
            },
        );
        assert_eq!(action, PlayerAction::ChooseNumber(expected), "{why}");
    }
}

/// Fury's targets: the opponent's creatures its 4 damage can finish,
/// the most valuable first, and not the 5/5 it cannot, which would
/// only take damage from one it can. With nothing it can finish, the
/// best of the opponent's takes it all; its own are never named.
#[test]
fn divided_damage_is_aimed_at_what_it_can_finish() {
    use baylee_cards_dsl::Effect;
    use baylee_engine::engine::DecisionContext;
    let (me, them) = (PlayerId::new(0), PlayerId::new(1));
    let effects = [Effect::DealDamageDivided { amount: 4 }];
    let context = DecisionContext {
        effects: &effects,
        ..Default::default()
    };
    let aim = |battlefield: Vec<PublicObject>| {
        let v = view(0, &[20, 20], battlefield);
        let options = v.battlefield.iter().map(|o| o.id).collect();
        HeuristicAgent::new(AIProfile::EXPERT).act_with_context(
            &v,
            &Pending::ChooseTargets {
                player: v.seat,
                options,
                player_options: vec![],
                min: 0,
                max: 4,
                reason: baylee_engine::choice::TargetPrompt::Targets,
            },
            &context,
        )
    };
    let chosen = |objects| PlayerAction::ChooseTargets {
        objects,
        players: vec![],
    };
    assert_eq!(
        aim(vec![
            permanent(obj(4), me, 1),
            permanent(obj(1), them, 5),
            permanent(obj(2), them, 3),
            permanent(obj(3), them, 1),
        ]),
        chosen(vec![obj(2), obj(3)])
    );
    assert_eq!(
        aim(vec![permanent(obj(4), me, 1), permanent(obj(1), them, 5)]),
        chosen(vec![obj(1)])
    );
}

/// "Up to four targets" with nothing in the context to rank them by,
/// over eleven enemy creatures and one of this seat's. The fallback
/// takes every enemy it was offered, and it named all eleven where the
/// question allows four: the engine refused the answer as too many
/// (the trained AI's fuzzer on main 50050ff3, seeds 486 and 1931, with
/// Flying Men, Meloku and its Illusions across the table). Four
/// enemies, and never the seat's own creature.
#[test]
fn an_open_count_of_targets_is_held_to_its_maximum() {
    let (me, them) = (PlayerId::new(0), PlayerId::new(1));
    let mut battlefield: Vec<PublicObject> =
        (1..=11).map(|slot| permanent(obj(slot), them, 1)).collect();
    battlefield.push(permanent(obj(12), me, 2));
    let v = view(0, &[20, 20], battlefield);
    let pending = Pending::ChooseTargets {
        player: v.seat,
        options: v.battlefield.iter().map(|o| o.id).collect(),
        player_options: vec![],
        min: 0,
        max: 4,
        reason: baylee_engine::choice::TargetPrompt::Targets,
    };
    let agent = HeuristicAgent::new(AIProfile::EXPERT);
    let action = agent.act(&v, &pending);
    assert_eq!(pending.answer_fault(&action), None, "{action:?}");
    assert_eq!(
        agent.fallbacks(),
        0,
        "the picker's own answer, and not one refitted after it"
    );
    let PlayerAction::ChooseTargets { objects, players } = action else {
        panic!("a target question answered with targets")
    };
    assert_eq!(objects.len(), 4, "as many as the question allows");
    assert!(players.is_empty(), "no player was offered");
    assert!(!objects.contains(&obj(12)), "and none of the seat's own");
}

/// An answer that breaks its question is refitted to the nearest one
/// inside it, and counted. Each proposal below is one a picker could
/// build and the engine refuses (`Pending::answer_fault`); the refit
/// keeps the proposal's own choices in its own order and makes up a
/// shortfall from what the question offers, an opponent's first. The
/// count is shared with a clone, which is how a host that answers
/// through a copy of its seat's agent reads it.
#[test]
fn an_answer_that_breaks_its_question_is_refitted_and_counted() {
    let (me, them) = (PlayerId::new(0), PlayerId::new(1));
    let v = view(
        0,
        &[20, 20],
        vec![
            permanent(obj(1), them, 1),
            permanent(obj(2), me, 1),
            permanent(obj(3), them, 1),
            permanent(obj(4), them, 1),
        ],
    );
    let targets = |min, max| Pending::ChooseTargets {
        player: v.seat,
        options: vec![obj(1), obj(2), obj(3), obj(4)],
        player_options: vec![me, them],
        min,
        max,
        reason: baylee_engine::choice::TargetPrompt::Targets,
    };
    let chosen = |objects: Vec<ObjectId>, players: Vec<PlayerId>| PlayerAction::ChooseTargets {
        objects,
        players,
    };
    let agent = HeuristicAgent::new(AIProfile::EXPERT);
    let seated = agent.clone();
    let cases = [
        (
            targets(0, 2),
            chosen(vec![obj(4), obj(1), obj(3)], vec![them]),
            chosen(vec![obj(4), obj(1)], vec![]),
            "too many: the first two it ranked",
        ),
        (
            targets(3, 3),
            chosen(vec![obj(9), obj(3), obj(3)], vec![]),
            chosen(vec![obj(3), obj(1), obj(4)], vec![]),
            "one not offered, one named twice, and made up with the opponent's",
        ),
        (
            targets(4, 4),
            chosen(vec![], vec![]),
            chosen(vec![obj(1), obj(3), obj(4), obj(2)], vec![]),
            "the seat's own only once the opponent's are all named",
        ),
        (
            Pending::ChooseNumber {
                player: v.seat,
                min: 1,
                max: 3,
                reason: baylee_engine::choice::NumberPrompt::X,
            },
            PlayerAction::ChooseNumber(7),
            PlayerAction::ChooseNumber(3),
            "a number held to its range",
        ),
        (
            targets(1, 1),
            PlayerAction::YesNo(true),
            chosen(vec![obj(1)], vec![]),
            "an answer of the wrong kind: the least the question offers",
        ),
    ];
    for (at, (pending, proposal, expected, why)) in cases.into_iter().enumerate() {
        assert!(pending.answer_fault(&proposal).is_some(), "{why}: a fault");
        let answer = seated.held_to(&v, &pending, proposal);
        assert_eq!(answer, expected, "{why}");
        assert_eq!(pending.answer_fault(&answer), None, "{why}: taken");
        assert_eq!(agent.fallbacks(), at + 1, "{why}: counted, and shared");
    }
    let fine = chosen(vec![obj(3)], vec![]);
    assert_eq!(seated.held_to(&v, &targets(1, 1), fine.clone()), fine);
    assert_eq!(agent.fallbacks(), 5, "an answer inside its question is not");
}

/// A number below its range is raised to the minimum, and a zero-width
/// range answers its one value.
#[test]
fn a_number_under_its_range_is_raised_to_the_minimum() {
    let v = view(0, &[20, 20], vec![]);
    let agent = HeuristicAgent::new(AIProfile::EXPERT);
    for (min, max, said, expect) in [(2, 5, 0, 2), (3, 3, 9, 3), (0, 4, 4, 4)] {
        let pending = Pending::ChooseNumber {
            player: v.seat,
            min,
            max,
            reason: baylee_engine::choice::NumberPrompt::X,
        };
        let answer = agent.held_to(&v, &pending, PlayerAction::ChooseNumber(said));
        assert_eq!(answer, PlayerAction::ChooseNumber(expect), "{min}..{max}");
    }
}

/// A wrong-kind answer to "choose a player" falls to an opponent, never
/// the answering seat itself.
#[test]
fn a_wrong_kind_answer_to_choose_player_falls_to_an_opponent() {
    let (me, them) = (PlayerId::new(0), PlayerId::new(1));
    let v = view(0, &[20, 20], vec![]);
    let pending = Pending::ChoosePlayer {
        player: me,
        options: vec![me, them],
    };
    let agent = HeuristicAgent::new(AIProfile::EXPERT);
    let answer = agent.held_to(&v, &pending, PlayerAction::YesNo(true));
    assert_eq!(pending.answer_fault(&answer), None);
    assert_eq!(answer, PlayerAction::ChoosePlayer(them));
    assert_eq!(agent.fallbacks(), 1);
}

/// Every refit is counted, including one that lands on a no-op.
#[test]
fn each_refit_is_counted_once_and_an_answer_inside_its_question_is_not() {
    let v = view(0, &[20, 20], vec![]);
    let pending = Pending::ChooseNumber {
        player: v.seat,
        min: 1,
        max: 2,
        reason: baylee_engine::choice::NumberPrompt::X,
    };
    let agent = HeuristicAgent::new(AIProfile::EXPERT);
    agent.held_to(&v, &pending, PlayerAction::ChooseNumber(1));
    assert_eq!(agent.fallbacks(), 0);
    agent.held_to(&v, &pending, PlayerAction::ChooseNumber(9));
    agent.held_to(&v, &pending, PlayerAction::ChooseNumber(0));
    assert_eq!(agent.fallbacks(), 2);
}

#[test]
fn third_iteration_counter_sign_decides_which_team_to_target() {
    use baylee_cards_dsl::{Amount, CounterKind as Counter, Effect};
    use baylee_engine::engine::DecisionContext;
    let v = view(
        0,
        &[20, 20],
        vec![
            permanent(obj(1), PlayerId::new(1), 6),
            permanent(obj(2), PlayerId::new(0), 4),
        ],
    );
    let pending = Pending::ChooseTargets {
        player: v.seat,
        options: vec![obj(1), obj(2)],
        player_options: vec![],
        min: 1,
        max: 1,
        reason: baylee_engine::choice::TargetPrompt::Targets,
    };
    for (kind, expected) in [(Counter::P1P1, obj(2)), (Counter::M1M1, obj(1))] {
        let effects = [Effect::AddCounter {
            kind,
            amount: Amount::Fixed(1),
        }];
        let context = DecisionContext {
            effects: &effects,
            ..Default::default()
        };
        assert_eq!(
            HeuristicAgent::new(AIProfile::EXPERT).act_with_context(&v, &pending, &context),
            PlayerAction::ChooseTargets {
                objects: vec![expected],
                players: vec![]
            }
        );
    }
}

/// A Roaming Throne with ward {2} beside a plain 2/2, and one Plains.
///
/// The Throne is the better card to kill and the agent used to say so
/// and nothing else: material ranked it first, the ward countered the
/// removal, and the 2/2 that could have been exiled for free was still
/// there afterwards. Targets are chosen before mana is paid (CR 601.2),
/// so the seat has to cover the spell and the tax out of the same
/// untapped lands — with a third Plains it can, and then the bigger
/// threat is worth the toll again.
#[test]
fn removal_goes_around_a_ward_it_cannot_pay_and_through_one_it_can() {
    use baylee_cards_dsl::{Effect, Filter, TargetSpec};
    use baylee_engine::engine::DecisionContext;
    let throne = carded(
        permanent(obj(1), PlayerId::new(1), 4),
        "Roaming Throne",
        TypeSet::ARTIFACT.union(TypeSet::CREATURE),
    );
    let bear = permanent(obj(2), PlayerId::new(1), 2);
    let plains = |id| {
        let mut land = carded(permanent(id, PlayerId::new(0), 0), "Plains", TypeSet::LAND);
        land.subtypes
            .insert(baylee_core::generated::subtypes::land::PLAINS);
        land.power = None;
        land.toughness = None;
        land
    };
    let pending = Pending::ChooseTargets {
        player: PlayerId::new(0),
        options: vec![obj(1), obj(2)],
        player_options: vec![],
        min: 1,
        max: 1,
        reason: baylee_engine::choice::TargetPrompt::Targets,
    };
    let effects = [Effect::Exile {
        target: TargetSpec::Object(&Filter::CREATURE),
    }];
    let swords = DecisionContext {
        effects: &effects,
        cost: Some(baylee_core::mana::ManaCost::parse("{W}")),
        ..Default::default()
    };

    let one = view(
        0,
        &[20, 20],
        vec![throne.clone(), bear.clone(), plains(obj(3))],
    );
    assert_eq!(
        agent().act_with_context(&one, &pending, &swords),
        PlayerAction::ChooseTargets {
            objects: vec![obj(2)],
            players: vec![]
        },
        "one Plains pays for the spell and not for ward {{2}}, so the \
         Throne is a card thrown away and the bear is a creature exiled"
    );

    // #214: ward is an ability, so it is the copied card's (CR 707.2) —
    // an Elf that became the Throne has it, and the Throne's own card
    // that became an Elf does not.
    let elf = carded(
        permanent(obj(1), PlayerId::new(1), 4),
        "Llanowar Elves",
        TypeSet::CREATURE,
    );
    for (copy, bear_it_is) in [
        (copying(elf, "Roaming Throne"), true),
        (copying(throne.clone(), "Llanowar Elves"), false),
    ] {
        let v = view(0, &[20, 20], vec![copy, bear.clone(), plains(obj(3))]);
        assert_eq!(
            agent().act_with_context(&v, &pending, &swords),
            PlayerAction::ChooseTargets {
                objects: vec![if bear_it_is { obj(2) } else { obj(1) }],
                players: vec![]
            },
            "ward was read off the card underneath the copy"
        );
    }

    let three = view(
        0,
        &[20, 20],
        vec![throne, bear, plains(obj(3)), plains(obj(4)), plains(obj(5))],
    );
    assert_eq!(
        agent().act_with_context(&three, &pending, &swords),
        PlayerAction::ChooseTargets {
            objects: vec![obj(1)],
            players: vec![]
        },
        "with the tax covered the bigger threat is worth {{2}} again"
    );
}

#[test]
fn life_ward_preserves_the_house_agents_life_reserve() {
    let pending = Pending::YesNo {
        player: PlayerId::new(0),
        prompt: YesNoPrompt::PayLife { amount: 7 },
        source: None,
    };
    for (life, pay) in [(20, true), (12, false), (7, false), (6, false)] {
        assert_eq!(
            agent().act(&view(0, &[life, 20], vec![]), &pending),
            PlayerAction::YesNo(pay)
        );
    }
}

/// Ward's question arrives at the caster as `YesNoPrompt::PayTax`, and
/// the agent used to answer it in the same arm as a kicker and an
/// offered draw: no, always. Declining a kicker costs nothing and
/// declining this counters the agent's own spell — with two untapped
/// lands sitting on the table.
///
/// What separates the two is not the prompt but what refusing it does,
/// which the resolving effect says out loud: ward's alternative counters
/// the spell, a Rhystic tax's gives an opponent a card. The second is
/// still declined here, because spare mana and needed mana look alike to
/// a stateless policy and a card is the cheaper of the two to give up.
#[test]
fn a_warded_spell_is_paid_for_instead_of_being_countered() {
    use baylee_cards_dsl::{Amount, Effect, PlayerRel};
    use baylee_engine::engine::DecisionContext;
    let mut forest = carded(
        permanent(obj(1), PlayerId::new(0), 0),
        "Forest",
        TypeSet::LAND,
    );
    forest
        .subtypes
        .insert(baylee_core::generated::subtypes::land::FOREST);
    let mut island = carded(
        permanent(obj(2), PlayerId::new(0), 0),
        "Island",
        TypeSet::LAND,
    );
    island
        .subtypes
        .insert(baylee_core::generated::subtypes::land::ISLAND);
    let pending = Pending::YesNo {
        player: PlayerId::new(0),
        prompt: YesNoPrompt::PayTax { mana: 2 },
        source: None,
    };
    let ward = [Effect::PlayerMayPayOr {
        player: PlayerRel::ControllerOfTarget,
        mana: Amount::Fixed(2),
        effect: &Effect::CounterTargetSpellOrAbility,
    }];
    let ward = DecisionContext {
        effects: &ward,
        ..Default::default()
    };

    let v = view(0, &[20, 20], vec![forest, island]);
    assert_eq!(
        agent().act_with_context(&v, &pending, &ward),
        PlayerAction::YesNo(true),
        "two untapped lands and the spell dies if the tax goes unpaid"
    );

    let bare = view(0, &[20, 20], vec![]);
    assert_eq!(
        agent().act_with_context(&bare, &pending, &ward),
        PlayerAction::YesNo(false),
        "nothing to tap, so promising the mana only spends the window"
    );

    let rhystic = [Effect::PlayerMayPayOr {
        player: PlayerRel::ControllerOfTarget,
        mana: Amount::Fixed(2),
        effect: &Effect::DrawCards {
            amount: Amount::Fixed(1),
        },
    }];
    let rhystic = DecisionContext {
        effects: &rhystic,
        ..Default::default()
    };
    assert_eq!(
        agent().act_with_context(&v, &pending, &rhystic),
        PlayerAction::YesNo(false),
        "a tax that only draws them a card is not worth mana this policy \
         cannot tell it has to spare"
    );

    // Mystic Remora's cumulative upkeep: unpaid, the seat's own
    // enchantment is sacrificed, which is ward's trade from the other
    // side of the table.
    let upkeep = [Effect::PlayerMayPayOr {
        player: PlayerRel::You,
        mana: Amount::Fixed(2),
        effect: &Effect::SacrificeSelf,
    }];
    let upkeep = DecisionContext {
        effects: &upkeep,
        ..Default::default()
    };
    assert_eq!(
        agent().act_with_context(&v, &pending, &upkeep),
        PlayerAction::YesNo(true),
        "the upkeep is paid while the lands cover it"
    );
    assert_eq!(
        agent().act_with_context(&bare, &pending, &upkeep),
        PlayerAction::YesNo(false),
        "and let go when they do not"
    );
}
