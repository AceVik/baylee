use super::*;

/// Arid Mesa is `{T}, Pay 1 life, Sacrifice this: search`. An agent that
/// leaves it alone has played a land that makes no mana whatsoever, which
/// is what every fetchland in the acceptance decks did until now.
#[test]
fn a_fetchland_is_cracked() {
    let mesa = carded(
        permanent(obj(1), PlayerId::new(0), 0),
        "Arid Mesa",
        TypeSet::LAND,
    );
    let v = view(0, &[20, 20], vec![mesa]);

    assert_eq!(
        agent().act(&v, &offering(vec![(obj(1), 0)])),
        PlayerAction::ActivateAbility {
            source: obj(1),
            ability_index: 0,
        },
        "the fetchland was left on the battlefield doing nothing"
    );
}

/// The same land at six life stays put. One life is not what stops it —
/// the margin after it is, and it is the same margin the pay-life-or-
/// enter-tapped answer keeps.
#[test]
fn a_fetchland_is_not_cracked_on_a_low_life_total() {
    let mesa = carded(
        permanent(obj(1), PlayerId::new(0), 0),
        "Arid Mesa",
        TypeSet::LAND,
    );
    let v = view(0, &[6, 20], vec![mesa]);

    assert_eq!(
        agent().act(&v, &offering(vec![(obj(1), 0)])),
        PlayerAction::PassPriority
    );
}

/// #214. An offered index counts into the list the object's abilities
/// are printed on, and for a copy that is the card it copied (CR 707.2).
/// A Glasspool Mimic that is a Werefox Bodyguard prints one ability of
/// its own and is offered the Fox's second, so reading the Mimic finds
/// nothing there and the agent never uses anything a copy has.
///
/// The Mimic is under an opponent's Lightning Bolt on the opponent's
/// turn, because that is when a 2/2 is worth two life: it is going
/// anyway (`worth::doomed`), and a body sacrificed for two life on an
/// empty stack is a card thrown away.
#[test]
fn a_copy_is_offered_what_it_copied_and_the_agent_reads_that() {
    let mimic = copying(
        carded(
            permanent(obj(1), PlayerId::new(0), 2),
            "Glasspool Mimic",
            TypeSet::CREATURE,
        ),
        "Werefox Bodyguard",
    );
    let mut v = view(0, &[20, 20], vec![mimic]);
    v.active = PlayerId::new(1);
    let mut bolt = carded(
        permanent(obj(9), PlayerId::new(1), 0),
        "Lightning Bolt",
        TypeSet::INSTANT,
    );
    bolt.targets = vec![baylee_client_core::test_support::target(obj(1))];
    v.stack.push(bolt);

    assert_eq!(
        agent().act(&v, &offering(vec![(obj(1), 1)])),
        PlayerAction::ActivateAbility {
            source: obj(1),
            ability_index: 1,
        },
        "the Fox's `{{1}}{{W}}, Sacrifice: gain 2 life` was looked up on \
         the Mimic, which has no second ability"
    );
}

/// The same misreading the other way round, and the worse one: the
/// physical card has something at that index too, so the agent weighs
/// one ability and presses another. A Sakura-Tribe Elder that has become
/// an Electric Eel reads as a sacrifice that finds a land, and the button
/// it presses is `{R}{R}: +2/+0 and 1 damage to you`.
///
/// The Elder is blocking a 3/3, which is when its sacrifice is right:
/// it dies in the fight either way, and the land is what it leaves.
#[test]
fn a_copy_is_not_weighed_by_the_ability_its_own_card_prints_there() {
    let elder = carded(
        permanent(obj(1), PlayerId::new(0), 1),
        "Sakura-Tribe Elder",
        TypeSet::CREATURE,
    );
    let blocked = |elder: PublicObject| {
        let mut v = view(
            0,
            &[20, 20],
            vec![elder, permanent(obj(2), PlayerId::new(1), 3)],
        );
        v.active = PlayerId::new(1);
        v.step = baylee_view::Step::DeclareBlockers;
        v.combat.attackers.push(baylee_view::AttackerView {
            creature: obj(2),
            defending: Defender::Player(PlayerId::new(0)),
            blocked: true,
        });
        v.combat.blockers.push(baylee_view::BlockerView {
            blocker: obj(1),
            attacker: obj(2),
        });
        v
    };
    let v = blocked(elder.clone());
    assert_eq!(
        agent().act(&v, &offering(vec![(obj(1), 0)])),
        PlayerAction::ActivateAbility {
            source: obj(1),
            ability_index: 0,
        },
        "the control: an Elder that is itself is sacrificed for its land, \
         so the pass below is the copy and not a refusal of the Elder"
    );

    let v = blocked(copying(elder, "Electric Eel"));
    assert_eq!(
        agent().act(&v, &offering(vec![(obj(1), 0)])),
        PlayerAction::PassPriority,
        "the Eel's pump was pressed on the strength of the Elder's search"
    );
}

/// Jace's ultimate is his −12, and at three loyalty the engine offers
/// only the first three abilities. "The most negative thing available"
/// would take the −1 every turn until he was gone; reading the ultimate
/// off the card instead sees it is not on offer, and pluses.
#[test]
fn a_walker_pluses_when_its_ultimate_is_out_of_reach() {
    let jace = carded(
        walker(obj(1), PlayerId::new(0), 3),
        "Jace, the Mind Sculptor",
        TypeSet::PLANESWALKER,
    );
    let v = view(0, &[20, 20], vec![jace]);

    assert_eq!(
        agent().act(&v, &offering(vec![(obj(1), 0), (obj(1), 1), (obj(1), 2)])),
        PlayerAction::ActivateAbility {
            source: obj(1),
            ability_index: 0,
        },
        "the walker spent loyalty it should have gained"
    );
}

/// Offered the ultimate, it takes the ultimate — the engine only lists a
/// negative ability the walker can actually pay for.
#[test]
fn a_walker_takes_its_ultimate_when_it_is_offered() {
    let jace = carded(
        walker(obj(1), PlayerId::new(0), 12),
        "Jace, the Mind Sculptor",
        TypeSet::PLANESWALKER,
    );
    let v = view(0, &[20, 20], vec![jace]);

    assert_eq!(
        agent().act(
            &v,
            &offering(vec![(obj(1), 0), (obj(1), 1), (obj(1), 2), (obj(1), 3)])
        ),
        PlayerAction::ActivateAbility {
            source: obj(1),
            ability_index: 3,
        }
    );
}

/// Delve is a cost, so the agent pays it; every other pile is declined.
///
/// The `ChooseCards` rule answers `min` whenever the list is longer than
/// two, which is right for a search, a scry and a wish — declining any of
/// them loses nothing. Delve is the one prompt in that family the engine
/// asks *during a cast*, and `casting::can_cast` counted the graveyard
/// when it offered the spell: answering zero leaves a cast that cannot
/// pay, which is reversed whole and hands priority back with the same
/// `LegalActions`. A deterministic agent then casts it again, forever.
///
/// Both halves are here on purpose. The first alone would pass just as
/// well if the arm had been widened for every prompt at once, and an
/// agent that bottomed its whole hand to a scry would be a worse player
/// than one that never delved.
/// #74. Delve is a price before it is a question, and the price is what
/// decides whether the agent reaches for the spell at all.
///
/// `the_delve_question_is_a_cost_and_the_agent_pays_it` proves the
/// *answer*: asked to exile, the agent exiles the whole reduction. It
/// says nothing about `policy::spell_cost`, which subtracts the
/// graveyard before any question is asked — and that subtraction was
/// unasserted. Removed, the whole suite stayed green: Dig Through Time
/// was priced at its printed eight, `manaplan::plan` found no way to
/// make eight out of two Islands, and the agent passed a turn it could
/// have dug on. It is the same subtraction `casting::can_cast` makes
/// (CR 702.66a), so a seat that did not make it disagrees with the offer
/// it is being shown.
///
/// Written at the *tap*, which is where it is visible: the spell is not
/// castable yet — the mana is still in the lands — so the agent has to
/// price it to decide the first Island is worth turning sideways.
#[test]
fn a_delve_spell_is_priced_with_the_graveyard_before_the_first_tap() {
    let island = |slot: u32| {
        let mut o = permanent(obj(slot), PlayerId::new(0), 0);
        o.types = TypeSet::LAND;
        o.subtypes = {
            let mut set = SubtypeSet::EMPTY;
            set.insert(baylee_core::generated::subtypes::land::ISLAND);
            set
        };
        o.power = None;
        o.toughness = None;
        o
    };
    let mut v = view(0, &[20, 20], vec![island(20), island(21)]);
    v.hand = vec![hand_card(1, "Dig Through Time")];
    // Six cards in the graveyard, which is what turns {6}{U}{U} into
    // {U}{U} — exactly the two Islands beside it.
    v.graveyards[0] = (30..36)
        .map(|i| permanent(obj(i), PlayerId::new(0), 1))
        .collect();
    let pending = Pending::Priority {
        player: v.seat,
        legal: Box::new(baylee_engine::choice::LegalActions {
            can_pass: true,
            mana_abilities: vec![obj(20), obj(21)],
            ..Default::default()
        }),
    };
    for (name, profile) in PROFILES {
        let action = HeuristicAgent::new(profile).act(&v, &pending);
        assert!(
            matches!(action, PlayerAction::ActivateManaAbility { .. }),
            "{name} would not tap toward a delve spell it can afford: {action:?}",
        );
    }
    // The counter-test: empty the graveyard and the same spell is out of
    // reach, so the agent must *not* tap for it. Without this the
    // assertion above would pass against an agent that taps for anything.
    v.graveyards[0].clear();
    for (name, profile) in PROFILES {
        let action = HeuristicAgent::new(profile).act(&v, &pending);
        assert!(
            !matches!(action, PlayerAction::ActivateManaAbility { .. }),
            "{name} tapped toward an eight-drop with two lands: {action:?}",
        );
    }
}

/// Kicker (CR 702.33a) and "you may waterbend" (CR 701.67a) are one
/// question to the engine, `YesNoPrompt::Kicker`, and it pays the answer
/// out of the floating pool alone: a yes the pool cannot cover loses the
/// whole cast. The answer used to be no, always — Rite of Replication made
/// one copy with nine mana floating.
#[test]
fn an_optional_additional_cost_is_paid_when_the_pool_covers_it() {
    use baylee_engine::engine::DecisionContext;
    let pending = Pending::YesNo {
        player: PlayerId::new(0),
        prompt: YesNoPrompt::Kicker,
        source: None,
    };
    let asked = |v: &PlayerView, name: &str, cost: &str| {
        let def = baylee_cards::by_index(baylee_cards::decks::by_name(name).unwrap()).unwrap();
        let effects = def
            .abilities_for_face(0)
            .iter()
            .find_map(|a| match a {
                AbilityDef::Spell { effects, .. } => Some(*effects),
                _ => None,
            })
            .unwrap();
        let context = DecisionContext {
            source: Some(obj(1)),
            cost: Some(cost.parse().unwrap()),
            effects,
            ..Default::default()
        };
        agent().act_with_context(v, &pending, &context)
    };

    let mut v = view(0, &[20, 20], vec![]);
    v.hand = vec![hand_card(1, "Rite of Replication")];
    v.seats[0].mana_pool.blue = 9;
    assert_eq!(
        asked(&v, "Rite of Replication", "{2}{U}{U}"),
        PlayerAction::YesNo(true),
        "nine floating pays {{2}}{{U}}{{U}} and its kicker {{5}}"
    );
    v.seats[0].mana_pool.blue = 8;
    assert_eq!(
        asked(&v, "Rite of Replication", "{2}{U}{U}"),
        PlayerAction::YesNo(false),
        "one short, a yes loses the cast and not only the kicker"
    );

    // Waterbend {6}: every untapped creature pays {1} of it.
    let mut v = view(
        0,
        &[20, 20],
        (10..16)
            .map(|i| permanent(obj(i), PlayerId::new(0), 1))
            .collect(),
    );
    v.hand = vec![hand_card(1, "Spirit Water Revival")];
    v.seats[0].mana_pool.blue = 3;
    assert_eq!(
        asked(&v, "Spirit Water Revival", "{1}{U}{U}"),
        PlayerAction::YesNo(true),
        "six creatures pay the {{6}}, the pool pays the rest"
    );
    v.battlefield[0].status = ObjectStatus::TAPPED;
    assert_eq!(
        asked(&v, "Spirit Water Revival", "{1}{U}{U}"),
        PlayerAction::YesNo(false),
        "a tapped creature pays nothing"
    );
    v.battlefield[0].status = ObjectStatus::default();
    // Eight bodies pay the `{6}` and not the `{1}` beside it (CR 701.67b):
    // the engine asks for six taps, so `{U}{U}` floating is one short.
    v.battlefield
        .extend((16..18).map(|i| permanent(obj(i), PlayerId::new(0), 1)));
    v.seats[0].mana_pool.blue = 2;
    assert_eq!(
        asked(&v, "Spirit Water Revival", "{1}{U}{U}"),
        PlayerAction::YesNo(false),
        "a tap past the six was counted toward the printed {{1}}"
    );
    v.battlefield.truncate(6);
    v.seats[0].mana_pool.blue = 3;
    v.seats[0].library_count = 7;
    assert_eq!(
        asked(&v, "Spirit Water Revival", "{1}{U}{U}"),
        PlayerAction::YesNo(false),
        "the kicked half draws seven, and seven is the whole library"
    );
    v.seats[0].library_count = 8;
    assert_eq!(
        asked(&v, "Spirit Water Revival", "{1}{U}{U}"),
        PlayerAction::YesNo(true),
        "eight leaves one behind"
    );
}

/// #223 (a). A Treasure is a registry token: no card, so no `rules`, and
/// its text is its definition's (CR 111.3), which the view names by
/// `token`. The agent read abilities off `rules` alone, so a Treasure was
/// no source at all and a one-mana spell beside it went uncast. With a
/// Mountain untapped too, the Mountain pays and the Treasure is kept.
#[test]
fn a_treasure_pays_for_a_spell_when_nothing_else_can() {
    let mut treasure = permanent(obj(20), PlayerId::new(0), 0);
    treasure.name = "Treasure".into();
    treasure.types = TypeSet::ARTIFACT;
    treasure.power = None;
    treasure.toughness = None;
    treasure.token = Some(baylee_cards::tokens::token_id(
        &baylee_cards::tokens::TREASURE,
    ));
    let mut mountain = permanent(obj(21), PlayerId::new(0), 0);
    mountain.types = TypeSet::LAND;
    mountain
        .subtypes
        .insert(baylee_core::generated::subtypes::land::MOUNTAIN);
    mountain.power = None;
    mountain.toughness = None;
    let pending = |mana_abilities: Vec<ObjectId>| Pending::Priority {
        player: PlayerId::new(0),
        legal: Box::new(baylee_engine::choice::LegalActions {
            can_pass: true,
            abilities: vec![(obj(20), 0)],
            mana_abilities,
            ..Default::default()
        }),
    };
    let mut v = view(0, &[20, 20], vec![treasure.clone()]);
    v.phase = baylee_view::Phase::FirstMain;
    v.hand = vec![hand_card(1, "Ragavan, Nimble Pilferer")];
    for (name, profile) in PROFILES {
        assert_eq!(
            HeuristicAgent::new(profile).act(&v, &pending(vec![])),
            PlayerAction::ActivateAbility {
                source: obj(20),
                ability_index: 0
            },
            "{name} left the Treasure unspent"
        );
    }
    v.battlefield.push(mountain);
    for (name, profile) in PROFILES {
        assert_eq!(
            HeuristicAgent::new(profile).act(&v, &pending(vec![obj(21)])),
            PlayerAction::ActivateManaAbility { source: obj(21) },
            "{name} sacrificed the Treasure with a Mountain untapped"
        );
    }
}

/// #223. Restricted mana is spent first, so it is never counted as held
/// up.
///
/// A Bolt in hand beside a creature on board holds one mana back. The
/// Mountain is that mana, and the Elves are paid off Ancient Ziggurat,
/// which could hold up nothing: its mana may not pay for the Bolt.
/// Counting the Ziggurat's mana against the reserve as though the Elves
/// were paid from the Mountain left the Elves in hand.
#[test]
fn restricted_mana_is_not_counted_as_held_up() {
    let index = baylee_cards::decks::by_name("Ancient Ziggurat").unwrap();
    let mut ziggurat = permanent(obj(20), PlayerId::new(0), 0);
    ziggurat.types = TypeSet::LAND;
    ziggurat.power = None;
    ziggurat.toughness = None;
    ziggurat.card = Some(baylee_view::CardIdentity {
        index,
        print: baylee_core::ids::PrintRef::new(0),
        face: 0,
    });
    ziggurat.rules = Some(baylee_view::RulesFace {
        card: index,
        face: 0,
    });
    let mut mountain = permanent(obj(21), PlayerId::new(0), 0);
    mountain.types = TypeSet::LAND;
    mountain
        .subtypes
        .insert(baylee_core::generated::subtypes::land::MOUNTAIN);
    mountain.power = None;
    mountain.toughness = None;
    let bear = permanent(obj(30), PlayerId::new(0), 2);
    let mut v = view(0, &[20, 20], vec![ziggurat, mountain, bear]);
    v.phase = baylee_view::Phase::FirstMain;
    v.hand = vec![
        hand_card(1, "Lightning Bolt"),
        hand_card(2, "Llanowar Elves"),
    ];
    let pending = Pending::Priority {
        player: PlayerId::new(0),
        legal: Box::new(baylee_engine::choice::LegalActions {
            can_pass: true,
            abilities: vec![(obj(20), 0)],
            mana_abilities: vec![obj(21)],
            ..Default::default()
        }),
    };
    // NOVICE and CASUAL hold nothing up, and their noise is wide enough
    // to pick the Bolt.
    for (name, profile) in &PROFILES[2..] {
        assert_eq!(
            HeuristicAgent::new(*profile).act(&v, &pending),
            PlayerAction::ActivateAbility {
                source: obj(20),
                ability_index: 0
            },
            "{name} did not pay for the Elves off the Ziggurat"
        );
    }
}

/// #223. Restricted mana is named for a spell it may pay for (CR 106.6).
///
/// Ancient Ziggurat asks its colour after the tap, and the agent named
/// the colour of the best spell in hand — here Swords to Plowshares,
/// which a creature-only mana cannot pay for, so the Birds it was tapped
/// for stayed in hand as well. The premise is asserted first: without the
/// restriction in the context, the Swords is the spell the colour goes to.
#[test]
fn restricted_mana_is_named_for_a_spell_it_may_pay_for() {
    use baylee_core::mana::ManaColor;
    use baylee_engine::engine::DecisionContext;
    let ziggurat =
        baylee_cards::by_index(baylee_cards::decks::by_name("Ancient Ziggurat").unwrap()).unwrap();
    let AbilityDef::Activated { effects, .. } = &ziggurat.abilities_for_face(0)[0] else {
        panic!("the Ziggurat's mana ability moved");
    };
    let mut v = view(0, &[20, 20], vec![permanent(obj(30), PlayerId::new(1), 2)]);
    v.phase = baylee_view::Phase::FirstMain;
    v.hand = vec![
        hand_card(1, "Swords to Plowshares"),
        hand_card(2, "Birds of Paradise"),
    ];
    let pending = Pending::ChooseColor {
        player: PlayerId::new(0),
        options: vec![
            ManaColor::White,
            ManaColor::Blue,
            ManaColor::Black,
            ManaColor::Red,
            ManaColor::Green,
        ],
    };
    let agent = HeuristicAgent::new(AIProfile {
        temperature_milli: 0,
        ..AIProfile::STEADY
    });
    assert_eq!(
        agent.act_with_context(&v, &pending, &DecisionContext::default()),
        PlayerAction::ChooseColor(ManaColor::White),
        "the premise: unrestricted mana goes to the Swords"
    );
    let context = DecisionContext {
        source: Some(obj(20)),
        effects,
        ..Default::default()
    };
    assert_eq!(
        agent.act_with_context(&v, &pending, &context),
        PlayerAction::ChooseColor(ManaColor::Green),
        "the Ziggurat's mana may pay only for the Birds"
    );
}

/// The planner's half of the kicker: the engine asks with no window to
/// tap anything more, so the kicked price is floated before the cast.
/// The spell is castable with its base cost floating, which is where the
/// agent used to cast it.
#[test]
fn the_kicker_is_floated_before_the_cast() {
    let island = |slot: u32| {
        let mut o = permanent(obj(slot), PlayerId::new(0), 0);
        o.types = TypeSet::LAND;
        o.subtypes = {
            let mut set = SubtypeSet::EMPTY;
            set.insert(baylee_core::generated::subtypes::land::ISLAND);
            set
        };
        o.power = None;
        o.toughness = None;
        o
    };
    let mut battlefield: Vec<PublicObject> = (20..25).map(island).collect();
    battlefield.push(permanent(obj(30), PlayerId::new(1), 3));
    let mut v = view(0, &[20, 20], battlefield);
    v.phase = baylee_view::Phase::FirstMain;
    v.hand = vec![hand_card(1, "Rite of Replication")];
    v.seats[0].mana_pool.blue = 4;
    let pending = |lands: std::ops::Range<u32>| Pending::Priority {
        player: PlayerId::new(0),
        legal: Box::new(baylee_engine::choice::LegalActions {
            can_pass: true,
            castable: vec![obj(1)],
            mana_abilities: lands.map(obj).collect(),
            ..Default::default()
        }),
    };
    for (name, profile) in PROFILES {
        let action = HeuristicAgent::new(profile).act(&v, &pending(20..25));
        assert!(
            matches!(action, PlayerAction::ActivateManaAbility { .. }),
            "{name} cast with the base cost floating and five Islands untapped: {action:?}",
        );
    }
    // One Island fewer and the kicked nine is out of reach, so the spell
    // is cast as it is rather than waited on.
    v.battlefield.remove(0);
    for (name, profile) in PROFILES {
        let action = HeuristicAgent::new(profile).act(&v, &pending(21..25));
        assert_eq!(
            action,
            PlayerAction::CastSpell { card: obj(1) },
            "{name} waited on a kicker it cannot reach",
        );
    }
}

/// The planner's half of replicate: the engine offers as many payments
/// as the floating pool covers, so the copies are floated before the
/// cast. Lose Focus is castable with {1}{U} floating and an opponent's
/// Path aimed at this seat's creature, and two Islands still untapped pay
/// for two copies, so the agent taps one first. With none untapped it
/// casts what it has.
#[test]
fn replicate_is_floated_before_the_cast() {
    let island = |slot: u32| {
        let mut o = permanent(obj(slot), PlayerId::new(0), 0);
        o.types = TypeSet::LAND;
        o.subtypes = {
            let mut set = SubtypeSet::EMPTY;
            set.insert(baylee_core::generated::subtypes::land::ISLAND);
            set
        };
        o.power = None;
        o.toughness = None;
        o
    };
    let (me, them) = (PlayerId::new(0), PlayerId::new(1));
    let mut v = view(
        0,
        &[20, 20],
        vec![island(20), island(21), permanent(obj(9), me, 5)],
    );
    v.stack = vec![stack_spell(2, "Path to Exile", them, obj(9))];
    baylee_client_core::test_support::project_current_targets(&mut v);
    v.hand = vec![hand_card(1, "Lose Focus")];
    v.seats[0].mana_pool.blue = 2;
    let pending = |lands: Vec<ObjectId>| Pending::Priority {
        player: me,
        legal: Box::new(baylee_engine::choice::LegalActions {
            can_pass: true,
            castable: vec![obj(1)],
            mana_abilities: lands,
            ..Default::default()
        }),
    };
    let action = agent().act(&v, &pending(vec![obj(20), obj(21)]));
    assert!(
        matches!(action, PlayerAction::ActivateManaAbility { .. }),
        "cast with two copies' mana still in the lands: {action:?}",
    );
    v.battlefield.retain(|o| o.id == obj(9));
    assert_eq!(
        agent().act(&v, &pending(vec![])),
        PlayerAction::CastSpell { card: obj(1) },
        "waited on copies it cannot pay for"
    );
}

#[test]
fn the_delve_question_is_a_cost_and_the_agent_pays_it() {
    let graveyard: Vec<ObjectId> = (10..17).map(obj).collect();
    let v = view(0, &[20, 20], vec![]);
    let pile = |prompt| Pending::ChooseCards {
        player: PlayerId::new(0),
        options: graveyard.clone(),
        min: 0,
        max: 6,
        prompt,
        total: None,
    };

    let PlayerAction::ChooseObjects { objects } = agent().act(&v, &pile(ChoicePrompt::Delve))
    else {
        panic!("expected a card choice")
    };
    assert_eq!(
        objects.len(),
        6,
        "the whole reduction the spell was offered on"
    );

    // A scry and a surveil are arrangements, and an agent with no
    // opinion about the cards moves none of them: every card stays on
    // top as it lay. For the surveil that is the point — its second pile
    // is a graveyard and does not come back, and a surveil is 1 or 2 on
    // every card in the pool, which is exactly where a "take the max of
    // a small menu" shortcut would have milled this agent every time.
    for (n, prompt, away) in [
        (6_u32, ArrangePrompt::Scry, ArrangePlace::LibraryBottom),
        (2, ArrangePrompt::Surveil, ArrangePlace::Graveyard),
    ] {
        let cards = graveyard[..n as usize].to_vec();
        let look = Pending::Arrange {
            player: PlayerId::new(0),
            cards: cards.clone(),
            piles: vec![
                ArrangePile::up_to(ArrangePlace::LibraryTop, n),
                ArrangePile::up_to(away, n),
            ],
            prompt,
        };
        assert_eq!(
            agent().act(&v, &look),
            PlayerAction::Arrange {
                piles: vec![cards, vec![]]
            },
            "{prompt:?}: the agent keeps what it cannot read"
        );
    }
}

/// The untap step's determination, where naming a permanent is what
/// costs something.
///
/// A one-permanent menu is the shape the storage lands ask with, and it
/// is the shape the `max <= 2` shortcut answers with `max`: the agent
/// would have named the land every turn and never untapped it again.
/// The second half is the counter-test — the same tiny menu under a
/// different prompt is still answered with `max`, so this is about the
/// prompt and not about the size.
#[test]
fn the_untap_determination_is_answered_by_untapping() {
    let v = view(0, &[20, 20], vec![]);
    let menu = |prompt| Pending::ChooseCards {
        player: PlayerId::new(0),
        options: vec![obj(10)],
        min: 0,
        max: 1,
        prompt,
        total: None,
    };

    let PlayerAction::ChooseObjects { objects } = agent().act(&v, &menu(ChoicePrompt::LeaveTapped))
    else {
        panic!("expected a card choice")
    };
    assert!(
        objects.is_empty(),
        "naming it would leave it tapped for the rest of the game"
    );

    let PlayerAction::ChooseObjects { objects } =
        agent().act(&v, &menu(ChoicePrompt::SearchLibrary))
    else {
        panic!("expected a card choice")
    };
    assert_eq!(objects.len(), 1, "a short menu is otherwise taken whole");
}

/// Under an untap limit (Static Orb) the menu is what untaps, so the
/// agent untaps as many as the limit lets through rather than the `min`
/// a long menu otherwise gets.
#[test]
fn an_untap_limit_is_answered_with_as_many_as_it_allows() {
    let v = view(0, &[20, 20], vec![]);
    let menu = Pending::ChooseCards {
        player: PlayerId::new(0),
        options: (10..15).map(obj).collect(),
        min: 1,
        max: 3,
        prompt: ChoicePrompt::Untap,
        total: None,
    };
    let PlayerAction::ChooseObjects { objects } = agent().act(&v, &menu) else {
        panic!("expected a card choice")
    };
    assert_eq!(objects.len(), 3, "everything the limit lets untap");
}
