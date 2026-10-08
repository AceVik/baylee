use super::*;

/// The two threat policies read the same table differently: one goes for
/// the player who is winning the race, the other for the biggest board.
#[test]
fn politics_decides_who_gets_attacked() {
    // Seat 1 is ahead on life with nothing out; seat 2 is on 5 life with
    // three creatures.
    let board = vec![
        permanent(obj(10), PlayerId::new(2), 2),
        permanent(obj(11), PlayerId::new(2), 2),
        permanent(obj(12), PlayerId::new(2), 2),
    ];
    let v = view(0, &[40, 40, 5], board);
    let defenders = [PlayerId::new(1), PlayerId::new(2)];

    let leader = HeuristicAgent::new(AIProfile {
        politics: Politics::AttackLeader,
        ..AIProfile::default()
    });
    assert_eq!(
        leader.pick_defender(&v, &defenders),
        PlayerId::new(1),
        "attack-leader goes for the player on 40 life"
    );

    let archenemy = HeuristicAgent::new(AIProfile {
        politics: Politics::Archenemy,
        ..AIProfile::default()
    });
    assert_eq!(
        archenemy.pick_defender(&v, &defenders),
        PlayerId::new(2),
        "archenemy goes for the board, not the life total"
    );
}

/// The monarch is the one to hit, under every profile: combat damage takes
/// the crown (CR 724.2). Seat 1 is neither the leader nor the biggest board,
/// and every policy used to pass it by.
#[test]
fn every_politics_goes_for_the_monarch() {
    let board = vec![
        permanent(obj(10), PlayerId::new(2), 2),
        permanent(obj(11), PlayerId::new(2), 2),
    ];
    let mut v = view(0, &[40, 5, 40], board);
    v.monarch = Some(PlayerId::new(1));
    let defenders = [PlayerId::new(1), PlayerId::new(2)];
    for politics in [
        Politics::AttackLeader,
        Politics::Archenemy,
        Politics::Random,
    ] {
        let agent = HeuristicAgent::new(AIProfile {
            politics,
            ..AIProfile::default()
        });
        assert_eq!(
            agent.pick_defender(&v, &defenders),
            PlayerId::new(1),
            "{politics:?}"
        );
    }
    v.monarch = Some(PlayerId::new(0));
    let leader = HeuristicAgent::new(AIProfile {
        politics: Politics::AttackLeader,
        ..AIProfile::default()
    });
    assert_eq!(
        leader.pick_defender(&v, &defenders),
        PlayerId::new(2),
        "this seat holds the crown itself: back to the policy"
    );
}

/// "Random" must still be a function of the game state — a real RNG here
/// would make replays and the soak diverge.
#[test]
fn random_politics_stays_deterministic() {
    let v = view(0, &[40, 40, 40], vec![]);
    let defenders = [PlayerId::new(1), PlayerId::new(2)];
    let agent = HeuristicAgent::new(AIProfile {
        politics: Politics::Random,
        ..AIProfile::default()
    });
    let first = agent.pick_defender(&v, &defenders);
    for _ in 0..10 {
        assert_eq!(agent.pick_defender(&v, &defenders), first);
    }
    assert!(defenders.contains(&first));
}

/// Where `squad` is aimed by the walker rule, on a board that has
/// already been judged not to win.
fn aimed_at(
    v: &PlayerView,
    victim: PlayerId,
    squad: &[ObjectId],
    defenders: &[Defender],
) -> Vec<(ObjectId, Defender)> {
    combat::aim(&board::Board::new(v), victim, squad, defenders)
}

/// A planeswalker is worth attacking only when the attack kills it:
/// three 1/1s finish a 3-loyalty walker, so they go for the walker.
#[test]
fn a_squad_that_can_finish_a_planeswalker_goes_for_it() {
    let victim = PlayerId::new(1);
    let squad = vec![obj(1), obj(2), obj(3)];
    let mut board: Vec<PublicObject> = squad
        .iter()
        .map(|id| permanent(*id, PlayerId::new(0), 1))
        .collect();
    board.push(walker(obj(20), victim, 3));
    let v = view(0, &[20, 20], board);
    let defenders = [Defender::Player(victim), Defender::Planeswalker(obj(20))];

    assert_eq!(
        aimed_at(&v, victim, &squad, &defenders),
        squad
            .iter()
            .map(|id| (*id, Defender::Planeswalker(obj(20))))
            .collect::<Vec<_>>(),
        "three power went to the player instead of killing the walker"
    );
}

/// Two 1/1s only chip it, which is the worst of both — so they hit the
/// player instead.
#[test]
fn a_squad_that_would_only_chip_a_planeswalker_hits_the_player() {
    let victim = PlayerId::new(1);
    let squad = vec![obj(1), obj(2)];
    let mut board: Vec<PublicObject> = squad
        .iter()
        .map(|id| permanent(*id, PlayerId::new(0), 1))
        .collect();
    board.push(walker(obj(20), victim, 3));
    let v = view(0, &[20, 20], board);
    let defenders = [Defender::Player(victim), Defender::Planeswalker(obj(20))];

    assert_eq!(
        aimed_at(&v, victim, &squad, &defenders),
        squad
            .iter()
            .map(|id| (*id, Defender::Player(victim)))
            .collect::<Vec<_>>(),
        "the squad chipped a walker it could not kill"
    );
}

/// How many attackers `profile` sends at each defender, in the order the
/// answer first names them.
fn attacks(profile: AIProfile, v: &PlayerView, pending: &Pending) -> Vec<(Defender, usize)> {
    let PlayerAction::DeclareAttackers { attackers } = HeuristicAgent::new(profile).act(v, pending)
    else {
        panic!("not an attack answer")
    };
    let mut split: Vec<(Defender, usize)> = Vec::new();
    for (_, at) in attackers {
        match split.iter_mut().find(|(d, _)| *d == at) {
            Some((_, n)) => *n += 1,
            None => split.push((at, 1)),
        }
    }
    split
}

/// `count` creatures of `power` on seat 0; on seat 1 a player on `life`
/// with `blockers` untapped 1/1s and a 3-loyalty walker.
fn walker_board(count: u32, power: i16, life: i32, blockers: u32) -> (PlayerView, Pending) {
    let victim = PlayerId::new(1);
    let mut board: Vec<PublicObject> = (1..=count)
        .map(|i| permanent(obj(i), PlayerId::new(0), power))
        .collect();
    board.extend((0..blockers).map(|i| permanent(obj(100 + i), victim, 1)));
    board.push(walker(obj(200), victim, 3));
    let v = view(0, &[20, life], board);
    let pending = Pending::ChooseAttackers {
        player: v.seat,
        attackers: (1..=count).map(obj).collect(),
        defenders: vec![Defender::Player(victim), Defender::Planeswalker(obj(200))],
        required: Vec::new(),
        limits: Vec::new(),
    };
    (v, pending)
}

/// Self-play r001 #431: a token board that doubled every turn attacked
/// the walker its opponent recast from the command zone, every turn,
/// and the opponent's life never moved. An attack that wins goes at the
/// player, for every profile.
///
/// Two boards, because two things were blind to the kill. Twenty-four
/// 1/1s past two chump blockers are twenty-two into twenty: more than
/// the sixteen creatures the search takes, so no profile had a proof,
/// and every one of them sent all twenty-four at a 3-loyalty walker.
/// Three 5/5s into ten life with nothing to block is a board the search
/// does prove, which is why SHARP and EXPERT already took it and the
/// three shallow profiles, which never search, did not.
#[test]
fn an_attack_that_wins_goes_at_the_player_and_not_at_a_walker() {
    let player = Defender::Player(PlayerId::new(1));
    for (count, power, life, blockers) in [(24, 1, 20, 2), (3, 5, 10, 0)] {
        let (v, pending) = walker_board(count, power, life, blockers);
        for (name, profile) in EVERY_PROFILE {
            assert_eq!(
                attacks(profile, &v, &pending),
                vec![(player, count as usize)],
                "{name}: {count} {power}/{power}s past {blockers} blockers into {life} life"
            );
        }
    }
}

/// An attack that does not win still kills the walker it can, with what
/// it takes and not with everything: past one untapped 1/1, four 1/1s
/// are what kill a 3-loyalty walker, and the other four hit the player.
/// It used to send all eight at the walker.
#[test]
fn a_walker_is_sent_what_kills_it_and_the_rest_hits_the_player() {
    let victim = PlayerId::new(1);
    let at_walker = Defender::Planeswalker(obj(200));
    let (v, pending) = walker_board(8, 1, 20, 1);
    let squad: Vec<ObjectId> = (1..=8).map(obj).collect();
    let defenders = [Defender::Player(victim), at_walker];
    let aimed = aimed_at(&v, victim, &squad, &defenders);
    assert_eq!(
        aimed.iter().map(|(id, _)| *id).collect::<Vec<_>>(),
        squad,
        "the attack lost a creature or its order"
    );
    assert_eq!(
        aimed.iter().filter(|(_, d)| *d == at_walker).count(),
        4,
        "one blocker stops one attacker, so the walker needs four: {aimed:?}"
    );
    // Every profile that attacks at all splits it the same way: which
    // creatures go is the profile's, where they go is not.
    for (name, profile) in EVERY_PROFILE {
        let split = attacks(profile, &v, &pending);
        let sent: usize = split.iter().map(|(_, n)| n).sum();
        let walker = split
            .iter()
            .find(|(d, _)| *d == at_walker)
            .map_or(0, |(_, n)| *n);
        assert_eq!(
            walker,
            if sent > 4 { 4 } else { sent },
            "{name} sent {walker} of {sent} at the walker"
        );
    }
}

/// A walker every blocker could save is still sent the whole squad, as
/// it was before: nothing here is spare, and the defender has to block
/// to keep it.
#[test]
fn a_walker_the_blockers_could_save_is_still_sent_everything() {
    let victim = PlayerId::new(1);
    let v = view(
        0,
        &[20, 20],
        vec![
            permanent(obj(1), PlayerId::new(0), 5),
            permanent(obj(100), victim, 1),
            walker(obj(200), victim, 3),
        ],
    );
    let defenders = [Defender::Player(victim), Defender::Planeswalker(obj(200))];
    assert_eq!(
        aimed_at(&v, victim, &[obj(1)], &defenders),
        vec![(obj(1), Defender::Planeswalker(obj(200)))]
    );
}

/// A teammate's creature is a legal target and the wrong one. The engine
/// offers both (CR 115.4); picking is the agent's job.
#[test]
fn removal_goes_past_a_teammate_to_an_opponent() {
    let mine = permanent(obj(1), PlayerId::new(0), 2);
    let partner = permanent(obj(2), PlayerId::new(1), 2);
    let enemy = permanent(obj(3), PlayerId::new(2), 2);
    let v = view(0, &[20, 20, 20], vec![mine, partner, enemy]);
    let agent =
        HeuristicAgent::new(AIProfile::default()).with_teams(vec![Some(1), Some(1), Some(2)]);
    let pending = Pending::ChooseTargets {
        player: PlayerId::new(0),
        options: vec![obj(1), obj(2), obj(3)],
        player_options: vec![PlayerId::new(0), PlayerId::new(1), PlayerId::new(2)],
        min: 1,
        max: 1,
        reason: baylee_engine::choice::TargetPrompt::Targets,
    };

    let PlayerAction::ChooseTargets { objects, .. } = agent.act(&v, &pending) else {
        panic!("the agent answered a target choice with something else");
    };
    assert_eq!(objects, vec![obj(3)], "the agent shot its own side");
}

/// The same rule for the face: a burn spell goes at an opponent, never at
/// the partner whose life total is half the team's problem.
#[test]
fn burn_goes_at_an_opponent_and_not_at_a_teammate() {
    let v = view(0, &[20, 20, 20], vec![]);
    let agent =
        HeuristicAgent::new(AIProfile::default()).with_teams(vec![Some(1), Some(1), Some(2)]);
    let pending = Pending::ChooseTargets {
        player: PlayerId::new(0),
        options: vec![],
        player_options: vec![PlayerId::new(0), PlayerId::new(1), PlayerId::new(2)],
        min: 1,
        max: 1,
        reason: baylee_engine::choice::TargetPrompt::Targets,
    };

    let PlayerAction::ChooseTargets { players, .. } = agent.act(&v, &pending) else {
        panic!("the agent answered a target choice with something else");
    };
    assert_eq!(
        players,
        vec![PlayerId::new(2)],
        "the agent burned its partner"
    );
}

/// And "choose a player" is the same question asked without a target.
#[test]
fn choosing_a_player_skips_the_teammate() {
    let v = view(0, &[20, 20, 20], vec![]);
    let agent =
        HeuristicAgent::new(AIProfile::default()).with_teams(vec![Some(1), Some(1), Some(2)]);
    let pending = Pending::ChoosePlayer {
        player: PlayerId::new(0),
        options: vec![PlayerId::new(0), PlayerId::new(1), PlayerId::new(2)],
    };

    assert_eq!(
        agent.act(&v, &pending),
        PlayerAction::ChoosePlayer(PlayerId::new(2))
    );
}

/// Facing lethal, the seat blocks with whatever it has — even a 1/1
/// under a 5/5, which dies and stops the game being over.
///
/// Asked through `act` and not through `choose_blocks`, because the
/// bug this replaces was not in the maths: `Pending::ChooseBlockers`
/// was answered `vec![]` and no combat function was ever called.
#[test]
fn a_seat_facing_lethal_chump_blocks() {
    let attacker = permanent(obj(1), PlayerId::new(1), 5);
    let chump = permanent(obj(2), PlayerId::new(0), 1);
    let mut v = view(0, &[4, 20], vec![attacker, chump]);
    v.combat = CombatView {
        attackers: vec![baylee_view::AttackerView {
            creature: obj(1),
            defending: Defender::Player(PlayerId::new(0)),
            blocked: false,
        }],
        blockers: vec![],
        bands: vec![],
    };
    let pending = Pending::ChooseBlockers {
        demands: Vec::new(),
        player: PlayerId::new(0),
        attacker: PlayerId::new(1),
        blockers: vec![baylee_engine::choice::BlockOption {
            blocker: obj(2),
            attackers: vec![obj(1)],
        }],
        capacity: Vec::new(),
        obeying: Vec::new(),
        bounds: Vec::new(),
    };

    assert_eq!(
        agent().act(&v, &pending),
        PlayerAction::DeclareBlockers {
            blockers: vec![(obj(2), obj(1))],
        },
        "the seat took five to the face on four life"
    );
}

/// The same 1/1 does not block the same 5/5 at a comfortable life
/// total: the creature is worth more than three points of life.
#[test]
fn the_same_block_is_declined_when_it_is_not_lethal() {
    let attacker = permanent(obj(1), PlayerId::new(1), 5);
    let chump = permanent(obj(2), PlayerId::new(0), 1);
    let mut v = view(0, &[20, 20], vec![attacker, chump]);
    v.combat = CombatView {
        attackers: vec![baylee_view::AttackerView {
            creature: obj(1),
            defending: Defender::Player(PlayerId::new(0)),
            blocked: false,
        }],
        blockers: vec![],
        bands: vec![],
    };
    let pending = Pending::ChooseBlockers {
        demands: Vec::new(),
        player: PlayerId::new(0),
        attacker: PlayerId::new(1),
        blockers: vec![baylee_engine::choice::BlockOption {
            blocker: obj(2),
            attackers: vec![obj(1)],
        }],
        capacity: Vec::new(),
        obeying: Vec::new(),
        bounds: Vec::new(),
    };

    assert_eq!(
        agent().act(&v, &pending),
        PlayerAction::DeclareBlockers { blockers: vec![] },
        "a 1/1 was thrown under a 5/5 for nothing"
    );
}

/// A 1/1 does not run into an untapped 4/4; the 4/4 on the same board
/// does attack, because nothing over there kills it.
#[test]
fn only_the_creature_that_survives_the_block_attacks() {
    let small = permanent(obj(1), PlayerId::new(0), 1);
    let big = permanent(obj(2), PlayerId::new(0), 4);
    let wall = permanent(obj(3), PlayerId::new(1), 3);
    let v = view(0, &[20, 20], vec![small, big, wall]);
    let pending = Pending::ChooseAttackers {
        player: PlayerId::new(0),
        attackers: vec![obj(1), obj(2)],
        defenders: vec![Defender::Player(PlayerId::new(1))],
        required: Vec::new(),
        limits: Vec::new(),
    };

    let PlayerAction::DeclareAttackers { attackers } = agent().act(&v, &pending) else {
        panic!("the agent answered an attack declaration with something else");
    };
    assert_eq!(
        attackers,
        vec![(obj(2), Defender::Player(PlayerId::new(1)))],
        "the 1/1 charged a 3/3, or the 4/4 stayed home"
    );
}

/// With no teams at the table nothing changes: every other seat is an
/// opponent and the first one still gets it.
#[test]
fn a_table_with_no_teams_chooses_as_it_did_before() {
    let v = view(0, &[20, 20, 20], vec![]);
    let agent = HeuristicAgent::new(AIProfile::default());
    let pending = Pending::ChoosePlayer {
        player: PlayerId::new(0),
        options: vec![PlayerId::new(0), PlayerId::new(1), PlayerId::new(2)],
    };

    assert_eq!(
        agent.act(&v, &pending),
        PlayerAction::ChoosePlayer(PlayerId::new(1))
    );
}
