use super::*;

#[test]
fn mana_planning_taps_the_right_color_and_does_not_tap_for_an_unaffordable_spell() {
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
    let mut v = view(0, &[20, 20], vec![forest, island]);
    v.phase = baylee_view::Phase::FirstMain;
    v.hand = vec![hand_card(3, "Brainstorm")];
    let pending = Pending::Priority {
        player: v.seat,
        legal: Box::new(baylee_engine::choice::LegalActions {
            can_pass: true,
            mana_abilities: vec![obj(1), obj(2)],
            ..Default::default()
        }),
    };
    assert_eq!(
        agent().act(&v, &pending),
        PlayerAction::ActivateManaAbility { source: obj(2) }
    );
    v.hand = vec![hand_card(3, "Darksteel Forge")];
    assert_eq!(agent().act(&v, &pending), PlayerAction::PassPriority);
}

#[test]
fn a_free_counter_with_an_unpayable_future_cost_is_declined() {
    let spell = carded(
        permanent(obj(1), PlayerId::new(1), 0),
        "Brainstorm",
        TypeSet::INSTANT,
    );
    let mut v = view(0, &[20, 20], vec![]);
    v.stack.push(spell);
    v.hand = vec![hand_card(2, "Pact of Negation")];
    let pending = Pending::Priority {
        player: v.seat,
        legal: Box::new(baylee_engine::choice::LegalActions {
            can_pass: true,
            castable: vec![obj(2)],
            ..Default::default()
        }),
    };
    assert_eq!(agent().act(&v, &pending), PlayerAction::PassPriority);
}

#[test]
fn commander_mana_planning_reads_the_command_zone_and_its_tax() {
    use baylee_core::generated::subtypes::land;
    let mut lands = Vec::new();
    for (i, subtype) in [land::PLAINS, land::ISLAND, land::SWAMP]
        .into_iter()
        .enumerate()
    {
        let mut source = permanent(obj(u32::try_from(i).unwrap() + 1), PlayerId::new(0), 0);
        source.types = TypeSet::LAND;
        source.subtypes.insert(subtype);
        lands.push(source);
    }
    let mut v = view(0, &[20, 20], lands);
    v.phase = baylee_view::Phase::FirstMain;
    let mut commander = carded(
        walker(obj(10), v.seat, 3),
        "Aminatou, the Fateshifter",
        TypeSet::PLANESWALKER,
    );
    commander.commander = true;
    v.seats[0].commanders.push(baylee_view::CommanderView {
        object: commander.id,
        card: commander.card,
        name: commander.name.clone(),
        casts: 0,
    });
    v.command[0].push(commander);
    let pending = Pending::Priority {
        player: v.seat,
        legal: Box::new(baylee_engine::choice::LegalActions {
            can_pass: true,
            mana_abilities: (1..=3).map(obj).collect(),
            ..Default::default()
        }),
    };
    assert!(
        matches!(
            agent().act(&v, &pending),
            PlayerAction::ActivateManaAbility { .. }
        ),
        "the command zone must participate in affordable spell plans"
    );
    v.seats[0].commanders[0].casts = 1;
    assert_eq!(
        agent().act(&v, &pending),
        PlayerAction::PassPriority,
        "three sources cannot pay the recast tax"
    );
    v.seats[0].commanders[0].casts = 0;
    v.command[0].clear();
    assert_eq!(
        agent().act(&v, &pending),
        PlayerAction::PassPriority,
        "a listed commander in another zone is not a cast candidate"
    );
}

#[test]
fn convoke_pays_with_the_offered_permanents_instead_of_targeting_one() {
    let v = view(
        0,
        &[20, 20],
        (1..=6)
            .map(|i| permanent(obj(i), PlayerId::new(0), 2))
            .collect(),
    );
    let offered: Vec<_> = (1..=6).map(obj).collect();
    let pending = Pending::ChooseTargets {
        player: v.seat,
        options: offered.clone(),
        player_options: vec![],
        min: 0,
        max: 6,
        reason: baylee_engine::choice::TargetPrompt::Convoke,
    };
    for (_, profile) in AIProfile::NAMED {
        assert_eq!(
            HeuristicAgent::new(profile).act(&v, &pending),
            PlayerAction::ChooseTargets {
                objects: offered.clone(),
                players: vec![]
            },
        );
    }
}

/// #224: the tap question was answered with every permanent offered,
/// so a convoke cast tapped the whole team with its price already
/// floating. It taps only what the pool leaves unpaid, and the ones the
/// seat misses least.
#[test]
fn a_convoke_answer_taps_only_what_the_pool_leaves_unpaid() {
    use baylee_engine::engine::DecisionContext;
    let seat = PlayerId::new(0);
    let asked_x = |v: &PlayerView, cost: &str, x: u32, max: u32| {
        let context = DecisionContext {
            source: Some(obj(1)),
            cost: Some(cost.parse().unwrap()),
            x,
            ..Default::default()
        };
        let pending = Pending::ChooseTargets {
            player: seat,
            options: v.battlefield.iter().map(|o| o.id).collect(),
            player_options: vec![],
            min: 0,
            max,
            reason: baylee_engine::choice::TargetPrompt::Convoke,
        };
        match agent().act_with_context(v, &pending, &context) {
            PlayerAction::ChooseTargets { objects, .. } => objects,
            other => panic!("the tap question was answered with {other:?}"),
        }
    };
    let asked = |v: &PlayerView, cost: &str, max: u32| asked_x(v, cost, 0, max);

    // Clever Concealment, `{2}{W}{W}`: a 3/3 and, listed second, a 1/1.
    let mut v = view(
        0,
        &[20, 20],
        vec![permanent(obj(10), seat, 3), permanent(obj(11), seat, 1)],
    );
    v.hand = vec![hand_card(1, "Clever Concealment")];
    v.seats[0].mana_pool.white = 4;
    assert_eq!(asked(&v, "{2}{W}{W}", 2), vec![], "the pool pays it all");
    v.seats[0].mana_pool.white = 3;
    assert_eq!(
        asked(&v, "{2}{W}{W}", 2),
        vec![obj(11)],
        "one short: one tap, and the smaller body"
    );
    v.seats[0].mana_pool.white = 2;
    assert_eq!(
        asked(&v, "{2}{W}{W}", 2),
        vec![obj(11), obj(10)],
        "two short: both"
    );

    // Which one goes first: an artifact, which neither attacks nor
    // blocks, even an uncrewed Vehicle's 4/4; then, on this seat's turn,
    // a creature that cannot attack yet; the 1/1 that could is last
    // despite its size.
    let mut vehicle = permanent(obj(12), seat, 4);
    vehicle.types = TypeSet::ARTIFACT;
    let mut sick = permanent(obj(13), seat, 3);
    sick.summoning_sick = true;
    v.battlefield = vec![permanent(obj(11), seat, 1), sick, vehicle];
    v.seats[0].mana_pool.white = 2;
    assert_eq!(
        asked(&v, "{2}{W}{W}", 2),
        vec![obj(12), obj(13)],
        "the artifact, then the creature that cannot attack this turn"
    );
    v.active = PlayerId::new(1);
    v.seats[0].mana_pool.white = 3;
    assert_eq!(
        asked(&v, "{2}{W}{W}", 2),
        vec![obj(12)],
        "on the opponent's turn the artifact still goes first"
    );
    v.seats[0].mana_pool.white = 1;
    assert_eq!(
        asked(&v, "{3}{W}", 3),
        vec![obj(12), obj(11), obj(13)],
        "and every creature is a blocker, so the smaller one goes next"
    );

    // X is part of the price the pool is measured against: Chord of
    // Calling for X = 2 is `{2}{G}{G}{G}`, one more than four Forests.
    let mut v = view(
        0,
        &[20, 20],
        vec![permanent(obj(10), seat, 1), permanent(obj(11), seat, 1)],
    );
    v.hand = vec![hand_card(1, "Chord of Calling")];
    v.seats[0].mana_pool.green = 4;
    assert_eq!(
        asked_x(&v, "{X}{G}{G}{G}", 2, 2).len(),
        1,
        "X = 2 is one short"
    );

    // A waterbend's question comes once its `{6}` was paid, so the `{6}`
    // is part of what the pool is measured against: Spirit Water Revival
    // with nine floating taps nothing, with three it taps six.
    let mut v = view(
        0,
        &[20, 20],
        (10..17).map(|i| permanent(obj(i), seat, 1)).collect(),
    );
    v.hand = vec![hand_card(1, "Spirit Water Revival")];
    v.seats[0].mana_pool.blue = 9;
    assert_eq!(
        asked(&v, "{1}{U}{U}", 6),
        vec![],
        "nine pay {{7}}{{U}}{{U}}"
    );
    v.seats[0].mana_pool.blue = 3;
    assert_eq!(
        asked(&v, "{1}{U}{U}", 6).len(),
        6,
        "three pay only {{1}}{{U}}{{U}}"
    );
}

#[test]
fn a_historical_target_does_not_threaten_a_returned_permanent() {
    let mut v = view(0, &[20, 20], vec![permanent(obj(9), PlayerId::new(0), 4)]);
    let spell = stack_spell(2, "Lightning Bolt", PlayerId::new(1), obj(9));
    assert!(agent().aimed_at_this_seat(&v, &spell) > 0);
    v.target_objects
        .iter_mut()
        .find(|target| target.source.object == obj(9))
        .unwrap()
        .is_current = false;
    assert_eq!(agent().aimed_at_this_seat(&v, &spell), 0);
}
