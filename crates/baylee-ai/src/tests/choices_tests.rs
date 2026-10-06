use super::*;

#[test]
fn a_turn_trade_is_declined_without_declining_free_optional_effects() {
    let mut vault = carded(
        permanent(obj(7), PlayerId::new(0), 0),
        "Time Vault",
        TypeSet::ARTIFACT,
    );
    vault.status = ObjectStatus::TAPPED;
    let v = view(0, &[20, 20], vec![vault]);
    for profile in [AIProfile::STEADY, AIProfile::SHARP, AIProfile::EXPERT] {
        let agent = HeuristicAgent::new(profile);
        let offer = |prompt| Pending::YesNo {
            player: v.seat,
            prompt,
            source: None,
        };
        assert_eq!(
            agent.act(&v, &offer(YesNoPrompt::SkipTurn { source: obj(7) })),
            PlayerAction::YesNo(false)
        );
        assert_eq!(
            agent.act(&v, &offer(YesNoPrompt::MayDo)),
            PlayerAction::YesNo(true)
        );
    }
}

#[test]
fn gloom_taxed_mana_is_not_counted_as_free_production() {
    let mut land = carded(
        permanent(obj(10), PlayerId::new(0), 0),
        "Plains",
        TypeSet::LAND,
    );
    land.subtypes
        .insert(baylee_core::generated::subtypes::land::PLAINS);
    let mut v = view(0, &[20, 20], vec![land]);
    v.seats[0].mana_pool.colorless = 3;
    let legal = baylee_engine::choice::LegalActions {
        mana_abilities: vec![obj(10)],
        activation_increases: vec![(obj(10), 3)],
        ..Default::default()
    };
    assert!(mana_sources(&v, &legal).is_empty());
}

#[test]
fn gloom_mana_planning_continues_past_the_printed_cost() {
    let mut lands = Vec::new();
    for id in 10..13 {
        let mut land = carded(
            permanent(obj(id), PlayerId::new(0), 0),
            "Plains",
            TypeSet::LAND,
        );
        land.subtypes
            .insert(baylee_core::generated::subtypes::land::PLAINS);
        lands.push(land);
    }
    let mut v = view(0, &[20, 20], lands);
    v.phase = baylee_view::Phase::FirstMain;
    v.hand = vec![hand_card(1, "Savannah Lions")];
    v.seats[0].mana_pool.white = 1;
    let pending = Pending::Priority {
        player: v.seat,
        legal: Box::new(baylee_engine::choice::LegalActions {
            can_pass: true,
            mana_abilities: (10..13).map(obj).collect(),
            spell_increases: vec![(obj(1), baylee_engine::choice::CastModeKind::Normal, 3)],
            ..Default::default()
        }),
    };
    assert!(matches!(
        agent().act(&v, &pending),
        PlayerAction::ActivateManaAbility { .. }
    ));
}

#[test]
fn removal_waits_when_only_our_permanent_matches_its_colour_restriction() {
    let mut friendly = permanent(obj(2), PlayerId::new(0), 2);
    friendly.colors = ColorSet::of(baylee_core::color::Color::White);
    let enemy = permanent(obj(3), PlayerId::new(1), 5);
    let mut v = view(0, &[20, 20], vec![friendly, enemy]);
    v.hand = vec![hand_card(1, "Vanishing Verse")];
    let pending = Pending::Priority {
        player: v.seat,
        legal: Box::new(baylee_engine::choice::LegalActions {
            castable: vec![obj(1)],
            ..Default::default()
        }),
    };
    let agent = HeuristicAgent::new(AIProfile::EXPERT);
    assert_eq!(agent.act(&v, &pending), PlayerAction::PassPriority);
    v.battlefield[1].colors = ColorSet::of(baylee_core::color::Color::Blue);
    assert_eq!(
        agent.act(&v, &pending),
        PlayerAction::CastSpell { card: obj(1) }
    );
}

#[test]
fn pulse_does_not_kill_its_own_bird_when_the_enemy_board_has_hexproof() {
    let mut friendly = permanent(obj(2), PlayerId::new(0), 1);
    friendly.name = "Birds of Paradise".into();
    let mut enemy = permanent(obj(3), PlayerId::new(1), 5);
    enemy.name = "Padeem, Consul of Innovation".into();
    enemy.keywords = baylee_cards_dsl::KeywordSet::HEXPROOF.bits();
    let mut v = view(0, &[40, 40], vec![friendly, enemy]);
    v.hand = vec![hand_card(1, "Maelstrom Pulse")];
    let agent = HeuristicAgent::new(AIProfile::EXPERT);
    assert!(agent.spell_score(&v, v.hand[0].card) < 0);
    v.battlefield[1].keywords = 0;
    v.battlefield[1].name = "Opponent's creature".into();
    assert!(agent.spell_score(&v, v.hand[0].card) > 0);
}

#[test]
fn deluge_waits_for_a_profitable_exchange_before_spending_the_card() {
    let mut v = view(
        0,
        &[20, 20],
        vec![
            permanent(obj(2), PlayerId::new(0), 5),
            permanent(obj(3), PlayerId::new(1), 6),
        ],
    );
    v.hand = vec![hand_card(1, "Toxic Deluge")];
    let pending = Pending::Priority {
        player: v.seat,
        legal: Box::new(baylee_engine::choice::LegalActions {
            castable: vec![obj(1)],
            ..Default::default()
        }),
    };
    let agent = HeuristicAgent::new(AIProfile::EXPERT);
    assert_eq!(agent.act(&v, &pending), PlayerAction::PassPriority);
    v.battlefield[1].toughness = Some(4);
    assert_eq!(
        agent.act(&v, &pending),
        PlayerAction::CastSpell { card: obj(1) }
    );
    let pending = Pending::ChooseNumber {
        player: v.seat,
        min: 0,
        max: 20,
        reason: baylee_engine::choice::NumberPrompt::X,
    };
    assert_eq!(
        agent.act_with_context(
            &v,
            &pending,
            &baylee_engine::engine::DecisionContext {
                life_x: true,
                ..Default::default()
            }
        ),
        PlayerAction::ChooseNumber(4)
    );
}

#[test]
fn pulse_counts_our_same_named_permanents_before_casting_and_targeting() {
    let mut v = view(
        0,
        &[20, 20],
        vec![
            permanent(obj(2), PlayerId::new(0), 5),
            permanent(obj(3), PlayerId::new(0), 5),
            permanent(obj(4), PlayerId::new(1), 5),
        ],
    );
    v.hand = vec![hand_card(1, "Maelstrom Pulse")];
    let agent = HeuristicAgent::new(AIProfile::EXPERT);
    assert!(agent.spell_score(&v, v.hand[0].card) < 0);
    let mut other = permanent(obj(5), PlayerId::new(1), 3);
    other.name = "Other creature".into();
    v.battlefield.push(other);
    assert!(agent.spell_score(&v, v.hand[0].card) > 0);
    let def = baylee_cards::by_index(v.hand[0].card.index).unwrap();
    let effects = def
        .abilities
        .iter()
        .find_map(|a| match a {
            baylee_cards_dsl::AbilityDef::Spell { effects, .. } => Some(*effects),
            _ => None,
        })
        .unwrap();
    let pending = Pending::ChooseTargets {
        player: v.seat,
        options: v.battlefield.iter().map(|o| o.id).collect(),
        player_options: vec![],
        min: 1,
        max: 1,
        reason: baylee_engine::choice::TargetPrompt::Targets,
    };
    assert_eq!(
        agent.act_with_context(
            &v,
            &pending,
            &baylee_engine::engine::DecisionContext {
                source: Some(obj(1)),
                effects,
                ..Default::default()
            }
        ),
        PlayerAction::ChooseTargets {
            objects: vec![obj(5)],
            players: vec![]
        }
    );
}

#[test]
fn third_iteration_subtype_follows_the_cards_being_played() {
    use baylee_core::generated::subtypes::creature::{ALLY, BIRD};
    let mut v = view(0, &[20, 20], vec![]);
    v.hand = vec![hand_card(1, "Baleful Strix")];
    assert_eq!(
        HeuristicAgent::new(AIProfile::EXPERT).act(
            &v,
            &Pending::ChooseSubtype {
                player: v.seat,
                options: vec![ALLY, BIRD]
            }
        ),
        PlayerAction::ChooseSubtype(BIRD)
    );
}

/// Pithing Needle names what it stops: the opponent's Karn, whose loyalty
/// abilities are locked by it, and not the Llanowar Elves beside him,
/// whose only activated ability is a mana ability the Needle spares; nor
/// the agent's own Jace, which has the most to lose.
#[test]
fn a_needle_names_the_opponents_permanent_it_would_stop() {
    let mut v = view(0, &[20, 20], vec![]);
    let karn = baylee_cards::decks::by_name("Karn, the Great Creator").unwrap();
    v.battlefield = vec![
        carded(
            permanent(obj(1), PlayerId::new(0), 0),
            "Jace, the Mind Sculptor",
            TypeSet::PLANESWALKER,
        ),
        carded(
            permanent(obj(2), PlayerId::new(1), 0),
            "Karn, the Great Creator",
            TypeSet::PLANESWALKER,
        ),
        carded(
            permanent(obj(3), PlayerId::new(1), 0),
            "Llanowar Elves",
            TypeSet::CREATURE,
        ),
    ];
    let pending = Pending::ChooseCardName { player: v.seat };
    assert_eq!(
        HeuristicAgent::new(AIProfile::EXPERT).act(&v, &pending),
        PlayerAction::ChooseCardName {
            card: karn,
            face: 0
        }
    );
    // With only the Elves across the table, nothing of theirs is
    // stopped, and the name is one of the pool's that is not the
    // agent's own Jace.
    v.battlefield.remove(1);
    let PlayerAction::ChooseCardName { card, .. } =
        HeuristicAgent::new(AIProfile::EXPERT).act(&v, &pending)
    else {
        panic!("a card name is answered with one");
    };
    assert!(baylee_cards::by_index(card).is_some(), "a card of the pool");
    assert_ne!(
        Some(card),
        baylee_cards::decks::by_name("Jace, the Mind Sculptor"),
        "and not the agent's own"
    );
    assert_ne!(
        Some(card),
        baylee_cards::decks::by_name("Llanowar Elves"),
        "nor the Elves, whose mana ability the Needle would spare"
    );
}

#[test]
fn third_iteration_jace_bounces_a_threat_instead_of_blindly_ticking_up() {
    let jace = carded(
        permanent(obj(1), PlayerId::new(0), 0),
        "Jace, the Mind Sculptor",
        TypeSet::PLANESWALKER,
    );
    let v = view(
        0,
        &[5, 20],
        vec![jace, permanent(obj(2), PlayerId::new(1), 6)],
    );
    let legal = baylee_engine::choice::LegalActions {
        abilities: vec![(obj(1), 0), (obj(1), 1), (obj(1), 2)],
        ..Default::default()
    };
    assert_eq!(
        HeuristicAgent::new(AIProfile::EXPERT).act(
            &v,
            &Pending::Priority {
                player: v.seat,
                legal: Box::new(legal)
            }
        ),
        PlayerAction::ActivateAbility {
            source: obj(1),
            ability_index: 2
        }
    );
}

/// A price the player may decline is a price this agent pays.
///
/// "… unless you return a land you control to its owner's hand" asks
/// with `min: 0`, because naming nothing *is* the refusal — so an agent
/// answering it at `min` would sacrifice every Karoo land it played,
/// and silently, since declining is a legal answer and nothing logs it.
///
/// For `ChoicePrompt::CostReturn` and `CostTap` that is **not because
/// the prompt is handled**: they reach no arm of `policy::select_cards`
/// and fall out of its `_`, and the answer comes from the fallback's
/// `_ if max <= 2 => max`. That is a correct outcome resting on an
/// unrelated shortcut, which is exactly the kind of thing that is right
/// until somebody tidies it. Adding the two prompts to the policy was
/// tried and reverted: with every option a land of the same rank, the
/// ordering it would impose is the one already there, and a change no
/// test can see fall is not a change.
///
/// `CostSacrifice`, `CostDiscard` and `CostExile` *are* handled, by the
/// policy's own cost arm at `max(min, 1)` — and until that arm said
/// `max(min, 1)` it said `min`, and an expert seat declined every
/// "unless you sacrifice" price it was shown.
/// `a_price_that_may_be_declined_is_paid_with_the_least_valuable_card`
/// is that arm's test.
///
/// Both directions are checked, because "pay it" alone would pass on an
/// agent that pays every cost question at `max`: an activation cost
/// asks with `min: 1` and must still take one permanent, not two.
#[test]
fn a_price_that_may_be_declined_is_paid_and_an_activation_cost_is_not_overpaid() {
    use baylee_engine::choice::ChoicePrompt;
    let v = view(
        0,
        &[20, 20],
        vec![
            permanent(obj(1), PlayerId::new(0), 0),
            permanent(obj(2), PlayerId::new(0), 0),
        ],
    );
    for prompt in [ChoicePrompt::CostReturn, ChoicePrompt::CostTap] {
        let action = HeuristicAgent::new(AIProfile::EXPERT).act(
            &v,
            &Pending::ChooseCards {
                player: v.seat,
                options: vec![obj(1), obj(2)],
                min: 0,
                max: 1,
                prompt,
                total: None,
            },
        );
        let PlayerAction::ChooseObjects { objects } = action else {
            panic!("expected a card choice for {prompt:?}, got {action:?}")
        };
        assert_eq!(
            objects.len(),
            1,
            "{prompt:?} is a price worth paying, and naming nothing pays none of it"
        );
    }

    let action = HeuristicAgent::new(AIProfile::EXPERT).act(
        &v,
        &Pending::ChooseCards {
            player: v.seat,
            options: vec![obj(1), obj(2)],
            min: 1,
            max: 2,
            prompt: ChoicePrompt::CostSacrifice,
            total: None,
        },
    );
    let PlayerAction::ChooseObjects { objects } = action else {
        panic!("expected a card choice, got {action:?}")
    };
    assert_eq!(
        objects.len(),
        1,
        "an activation cost takes what it asks for and not one permanent more"
    );
}

#[test]
fn balance_keeps_the_valuable_permanent_or_castable_hand_card() {
    use baylee_engine::choice::ChoicePrompt;
    let me = PlayerId::new(0);
    let (elves, wurm) = (obj(1), obj(2));
    let board = view(
        0,
        &[20, 20],
        vec![
            carded(permanent(elves, me, 1), "Llanowar Elves", TypeSet::CREATURE),
            carded(permanent(wurm, me, 6), "Endless Wurm", TypeSet::CREATURE),
        ],
    );
    let mut hand = view(0, &[20, 20], vec![]);
    hand.hand = vec![hand_card(1, "Llanowar Elves"), hand_card(2, "Endless Wurm")];
    for (view, keep, prompt) in [
        (&board, wurm, ChoicePrompt::KeepCreatures),
        (&hand, elves, ChoicePrompt::Keep),
    ] {
        for (name, profile) in PROFILES {
            assert_eq!(
                HeuristicAgent::new(profile).act(
                    view,
                    &Pending::ChooseCards {
                        player: me,
                        options: vec![elves, wurm],
                        min: 1,
                        max: 1,
                        prompt,
                        total: None,
                    }
                ),
                PlayerAction::ChooseObjects {
                    objects: vec![keep]
                },
                "{name}"
            );
        }
    }
}

/// A price is paid, and paid with the card this seat misses least.
///
/// "Sacrifice Endless Wurm unless you sacrifice an enchantment" asks
/// with `min: 0`, because naming nothing is the refusal, and the policy
/// used to answer these prompts with `min` — so an expert seat gave up
/// every permanent that carried such a price, twelve implemented cards
/// of them, and nothing logged it. An activation asks the same question
/// with `min: 1`, so both shapes are asked here.
///
/// A card in hand or a graveyard is worth what it would be to cast:
/// with no lands on the table the Elves are worth 800 - 150 and the
/// five-drop Wurm 800 - 750, so the Wurm is what goes. A permanent on
/// the table is worth what it is there (`worth::given_up`, the number
/// the activation was priced with), and a 1/1 is worth less than a 6/6:
/// the Elves go. The menu lists the one that must *not* go first on the
/// table and the one that must go second elsewhere, so an agent that
/// paid `options[0]` without ranking fails one or the other.
#[test]
fn a_price_that_may_be_declined_is_paid_with_the_least_valuable_card() {
    use baylee_engine::choice::ChoicePrompt;
    let me = PlayerId::new(0);
    let (elves, wurm) = (obj(1), obj(2));
    let on_the_table = view(
        0,
        &[20, 20],
        vec![
            carded(permanent(elves, me, 1), "Llanowar Elves", TypeSet::CREATURE),
            carded(permanent(wurm, me, 6), "Endless Wurm", TypeSet::CREATURE),
        ],
    );
    let mut in_hand = view(0, &[20, 20], vec![]);
    in_hand.hand = vec![hand_card(1, "Llanowar Elves"), hand_card(2, "Endless Wurm")];
    let mut in_the_graveyard = view(0, &[20, 20], vec![]);
    in_the_graveyard.graveyards[0] = vec![
        carded(permanent(elves, me, 1), "Llanowar Elves", TypeSet::CREATURE),
        carded(permanent(wurm, me, 6), "Endless Wurm", TypeSet::CREATURE),
    ];

    for (prompt, view, options, goes) in [
        (
            ChoicePrompt::CostSacrifice,
            &on_the_table,
            vec![wurm, elves],
            elves,
        ),
        (ChoicePrompt::CostDiscard, &in_hand, vec![elves, wurm], wurm),
        (
            ChoicePrompt::CostExile,
            &in_the_graveyard,
            vec![elves, wurm],
            wurm,
        ),
    ] {
        for min in [0, 1] {
            let action = HeuristicAgent::new(AIProfile::EXPERT).act(
                view,
                &Pending::ChooseCards {
                    player: view.seat,
                    options: options.clone(),
                    min,
                    max: 1,
                    prompt,
                    total: None,
                },
            );
            assert_eq!(
                action,
                PlayerAction::ChooseObjects {
                    objects: vec![goes]
                },
                "{prompt:?} at min {min}: the price is paid, with the card \
                 worth least"
            );
        }
    }
}

/// Escape's "exile five other cards from your graveyard" (CR 702.138a)
/// is the same price asked with `min == max`: exactly that many go, and
/// they are the least valuable, so the Elves listed first stay.
#[test]
fn an_escape_exiles_exactly_its_count_and_the_least_valuable() {
    use baylee_engine::choice::ChoicePrompt;
    let me = PlayerId::new(0);
    let (elves, wurm, other_wurm) = (obj(1), obj(2), obj(3));
    let mut v = view(0, &[20, 20], vec![]);
    v.graveyards[0] = vec![
        carded(permanent(elves, me, 1), "Llanowar Elves", TypeSet::CREATURE),
        carded(permanent(wurm, me, 6), "Endless Wurm", TypeSet::CREATURE),
        carded(
            permanent(other_wurm, me, 6),
            "Endless Wurm",
            TypeSet::CREATURE,
        ),
    ];
    let action = HeuristicAgent::new(AIProfile::EXPERT).act(
        &v,
        &Pending::ChooseCards {
            player: v.seat,
            options: vec![elves, wurm, other_wurm],
            min: 2,
            max: 2,
            prompt: ChoicePrompt::CostExile,
            total: None,
        },
    );
    let PlayerAction::ChooseObjects { mut objects } = action else {
        panic!("expected cards, got {action:?}")
    };
    objects.sort();
    assert_eq!(objects, vec![wurm, other_wurm], "two, and not the Elves");
}

/// Atraxa's "for each card type, you may put a card of that type … into
/// your hand": one of the type's cards is taken, and the best of them.
/// The Wurm is listed first on purpose, because the fallback answers
/// `options[..max]`; with no lands on the table the Elves are worth
/// 800 - 150 and the Wurm 800 - 750.
#[test]
fn a_card_of_a_revealed_type_is_taken_and_the_best_one() {
    use baylee_engine::choice::ChoicePrompt;
    let me = PlayerId::new(0);
    let (elves, wurm) = (obj(1), obj(2));
    let mut v = view(0, &[20, 20], vec![]);
    v.graveyards[0] = vec![
        carded(permanent(elves, me, 1), "Llanowar Elves", TypeSet::CREATURE),
        carded(permanent(wurm, me, 6), "Endless Wurm", TypeSet::CREATURE),
    ];
    let action = HeuristicAgent::new(AIProfile::EXPERT).act(
        &v,
        &Pending::ChooseCards {
            player: v.seat,
            options: vec![wurm, elves],
            min: 0,
            max: 1,
            prompt: ChoicePrompt::OneOfType {
                card_type: TypeSet::CREATURE,
            },
            total: None,
        },
    );
    assert_eq!(
        action,
        PlayerAction::ChooseObjects {
            objects: vec![elves]
        }
    );
}
