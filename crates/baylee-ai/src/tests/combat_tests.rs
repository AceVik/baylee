use super::*;

#[test]
fn lethal_power_is_not_lethal_through_a_larger_blocker() {
    let v = view(
        0,
        &[20, 5],
        vec![
            permanent(obj(1), PlayerId::new(0), 6),
            permanent(obj(2), PlayerId::new(1), 7),
        ],
    );
    let pending = Pending::ChooseAttackers {
        player: v.seat,
        attackers: vec![obj(1)],
        defenders: vec![Defender::Player(PlayerId::new(1))],
        required: Vec::new(),
        limits: Vec::new(),
    };
    assert_eq!(
        HeuristicAgent::new(AIProfile::SHARP).act(&v, &pending),
        PlayerAction::DeclareAttackers { attackers: vec![] }
    );
}

#[test]
fn combat_lifelink_can_save_a_seat_from_an_unblockable_attacker() {
    use baylee_cards_dsl::KeywordSet;
    let mut flyer = permanent(obj(2), PlayerId::new(1), 8);
    flyer.keywords = KeywordSet::FLYING.bits();
    let mut lifelinker = permanent(obj(3), PlayerId::new(0), 2);
    lifelinker.keywords = KeywordSet::LIFELINK.bits();
    let mut v = view(
        0,
        &[8, 20],
        vec![permanent(obj(1), PlayerId::new(1), 3), flyer, lifelinker],
    );
    v.combat.attackers = (1..=2)
        .map(|id| baylee_view::AttackerView {
            creature: obj(id),
            defending: Defender::Player(v.seat),
            blocked: false,
        })
        .collect();
    let pending = Pending::ChooseBlockers {
        demands: Vec::new(),
        player: v.seat,
        attacker: PlayerId::new(1),
        blockers: vec![baylee_engine::choice::BlockOption {
            blocker: obj(3),
            attackers: vec![obj(1)],
        }],
        capacity: Vec::new(),
        obeying: Vec::new(),
        bounds: Vec::new(),
    };
    for profile in [AIProfile::SHARP, AIProfile::EXPERT] {
        assert_eq!(
            HeuristicAgent::new(profile).act(&v, &pending),
            PlayerAction::DeclareBlockers {
                blockers: vec![(obj(3), obj(1))]
            }
        );
    }
}

#[test]
fn combat_lifelink_after_lethal_first_strike_is_too_late() {
    use baylee_cards_dsl::KeywordSet;
    let mut first = permanent(obj(1), PlayerId::new(1), 3);
    first.keywords = KeywordSet::FIRST_STRIKE.bits();
    let mut other = permanent(obj(2), PlayerId::new(1), 1);
    other.toughness = Some(20);
    let mut lifelinker = permanent(obj(3), PlayerId::new(0), 3);
    lifelinker.toughness = Some(2);
    lifelinker.keywords = KeywordSet::LIFELINK.bits();
    let mut v = view(0, &[3, 20], vec![first, other, lifelinker]);
    v.combat.attackers = (1..=2)
        .map(|id| baylee_view::AttackerView {
            creature: obj(id),
            defending: Defender::Player(v.seat),
            blocked: false,
        })
        .collect();
    let pending = Pending::ChooseBlockers {
        demands: Vec::new(),
        player: v.seat,
        attacker: PlayerId::new(1),
        blockers: vec![baylee_engine::choice::BlockOption {
            blocker: obj(3),
            attackers: vec![obj(1), obj(2)],
        }],
        capacity: Vec::new(),
        obeying: Vec::new(),
        bounds: Vec::new(),
    };
    for profile in [AIProfile::SHARP, AIProfile::EXPERT] {
        assert_eq!(
            HeuristicAgent::new(profile).act(&v, &pending),
            PlayerAction::DeclareBlockers {
                blockers: vec![(obj(3), obj(1))]
            }
        );
    }
}

#[test]
fn combat_retaliation_counts_a_defender_not_offered_as_an_attacker() {
    let mut wall = permanent(obj(3), PlayerId::new(0), 0);
    wall.toughness = Some(6);
    wall.keywords = baylee_cards_dsl::KeywordSet::DEFENDER.bits();
    let v = view(
        0,
        &[4, 20],
        vec![
            permanent(obj(1), PlayerId::new(0), 6),
            permanent(obj(2), PlayerId::new(1), 5),
            wall,
        ],
    );
    let pending = Pending::ChooseAttackers {
        player: v.seat,
        attackers: vec![obj(1)],
        defenders: vec![Defender::Player(PlayerId::new(1))],
        required: Vec::new(),
        limits: Vec::new(),
    };
    let agent = HeuristicAgent::new(AIProfile::EXPERT);
    let attack = PlayerAction::DeclareAttackers {
        attackers: vec![(obj(1), Defender::Player(PlayerId::new(1)))],
    };
    assert_eq!(agent.act(&v, &pending), attack);
    let mut v = v;
    v.battlefield[2].summoning_sick = true;
    assert_eq!(
        agent.act(&v, &pending),
        attack,
        "summoning sickness does not stop a block"
    );
    for status in [ObjectStatus::TAPPED, ObjectStatus::PHASED_OUT] {
        v.battlefield[2].status = status;
        assert_eq!(
            agent.act(&v, &pending),
            PlayerAction::DeclareAttackers { attackers: vec![] }
        );
    }
    v.battlefield[2].status = ObjectStatus::NONE;
    v.battlefield[1].keywords = baylee_cards_dsl::KeywordSet::FLYING.bits();
    v.battlefield[0].keywords = baylee_cards_dsl::KeywordSet::REACH.bits();
    assert_eq!(
        agent.act(&v, &pending),
        PlayerAction::DeclareAttackers { attackers: vec![] }
    );
}

#[test]
fn combat_a_winning_attack_does_not_get_redirected_to_a_planeswalker() {
    let v = view(
        0,
        &[20, 3],
        vec![
            permanent(obj(1), PlayerId::new(0), 5),
            walker(obj(2), PlayerId::new(1), 2),
        ],
    );
    let pending = Pending::ChooseAttackers {
        player: v.seat,
        attackers: vec![obj(1)],
        defenders: vec![
            Defender::Player(PlayerId::new(1)),
            Defender::Planeswalker(obj(2)),
        ],
        required: Vec::new(),
        limits: Vec::new(),
    };
    // Every profile, since the shallow ones ask the estimate too: until
    // they did, only the two that search took the kill.
    for (name, profile) in EVERY_PROFILE {
        assert_eq!(
            HeuristicAgent::new(profile).act(&v, &pending),
            PlayerAction::DeclareAttackers {
                attackers: vec![(obj(1), Defender::Player(PlayerId::new(1)))]
            },
            "{name}"
        );
    }
}

/// A combat where seat 1 attacks with `attackers` — each `(slot, size,
/// defending)`, an `n/n` — and seat 0 has one `blocker`-sized creature
/// in slot 3 that may block any of those aimed at seat 0 or its walker in
/// slot 4, which is on `loyalty` and costs four.
fn walker_combat(
    lives: &[i32],
    attackers: &[(u32, i16, Defender)],
    blocker: i16,
    loyalty: u16,
) -> (PlayerView, Pending) {
    let mut battlefield: Vec<PublicObject> = attackers
        .iter()
        .map(|&(slot, size, _)| permanent(obj(slot), PlayerId::new(1), size))
        .collect();
    battlefield.push(permanent(obj(3), PlayerId::new(0), blocker));
    let mut planeswalker = walker(obj(4), PlayerId::new(0), loyalty);
    planeswalker.mana_value = 4;
    battlefield.push(planeswalker);
    let mut v = view(0, lives, battlefield);
    v.active = PlayerId::new(1);
    v.step = baylee_view::Step::DeclareBlockers;
    v.combat.attackers = attackers
        .iter()
        .map(|&(slot, _, defending)| baylee_view::AttackerView {
            creature: obj(slot),
            defending,
            blocked: false,
        })
        .collect();
    let ours = |d: Defender| {
        d == Defender::Player(PlayerId::new(0)) || d == Defender::Planeswalker(obj(4))
    };
    let pending = Pending::ChooseBlockers {
        demands: Vec::new(),
        player: v.seat,
        attacker: PlayerId::new(1),
        blockers: vec![baylee_engine::choice::BlockOption {
            blocker: obj(3),
            attackers: attackers
                .iter()
                .filter(|a| ours(a.2))
                .map(|a| obj(a.0))
                .collect(),
        }],
        capacity: Vec::new(),
        obeying: Vec::new(),
        bounds: Vec::new(),
    };
    (v, pending)
}

/// #75. Only what attacks this seat threatens its life (CR 508.1b names
/// one player, planeswalker or battle per attacker). A 3/3 at a seat on
/// four is not lethal, whatever else is attacking: a 5/5 at its walker on
/// nine, or one at another seat. The shallow path summed every attacker in
/// combat, read eight as lethal and chumped the 5/5 with its only 2/2.
#[test]
fn only_what_attacks_the_seat_counts_toward_its_life() {
    let me = Defender::Player(PlayerId::new(0));
    let boards = [
        walker_combat(
            &[4, 20],
            &[(1, 3, me), (2, 5, Defender::Planeswalker(obj(4)))],
            2,
            9,
        ),
        walker_combat(
            &[4, 20, 20],
            &[(1, 3, me), (2, 5, Defender::Player(PlayerId::new(2)))],
            2,
            9,
        ),
    ];
    for (board, (v, pending)) in boards.iter().enumerate() {
        for (name, profile) in PROFILES {
            assert_eq!(
                HeuristicAgent::new(profile).act(v, pending),
                PlayerAction::DeclareBlockers { blockers: vec![] },
                "{name} spent its 2/2 on board {board}"
            );
        }
    }
}

/// #75, the walker half: a creature worth less than the walker chumps
/// when that block is what keeps it (CR 120.3c, CR 704.5i). The same
/// chump is wrong when the walker survives the hit, when it dies anyway,
/// and when the seat's own life needs the creature.
#[test]
fn a_walker_that_would_die_is_chumped_for_and_one_that_would_not_is_not() {
    let walker = Defender::Planeswalker(obj(4));
    let me = Defender::Player(PlayerId::new(0));
    let chump = PlayerAction::DeclareBlockers {
        blockers: vec![(obj(3), obj(1))],
    };
    let none = PlayerAction::DeclareBlockers { blockers: vec![] };
    let trample = |blocker: i16| {
        let (mut v, pending) = walker_combat(&[20, 20], &[(1, 4, walker)], blocker, 3);
        v.battlefield[0].keywords = baylee_cards_dsl::KeywordSet::TRAMPLE.bits();
        (v, pending)
    };
    let cases = [
        (
            "a 3/3 at a walker on three",
            walker_combat(&[20, 20], &[(1, 3, walker)], 2, 3),
            &chump,
        ),
        (
            "a 3/3 at a walker on nine",
            walker_combat(&[20, 20], &[(1, 3, walker)], 2, 9),
            &none,
        ),
        (
            "a 3/3 and a 4/4 at a walker on three",
            walker_combat(&[20, 20], &[(1, 3, walker), (2, 4, walker)], 2, 3),
            &none,
        ),
        (
            "a 3/3 at a seat on three beside a 5/5 at its walker",
            walker_combat(&[3, 20], &[(1, 3, me), (2, 5, walker)], 2, 3),
            &chump,
        ),
        // Trample puts the excess on the walker (CR 702.19b).
        ("a 4/4 trampler held to two by a 2/2", trample(2), &chump),
        ("a 4/4 trampler held to three by a 1/1", trample(1), &none),
        // Only the sum with a 2/2 nothing here can block reaches four.
        (
            "a 3/3 beside an unblockable 2/2 at a walker on four",
            {
                let (v, mut pending) =
                    walker_combat(&[20, 20], &[(1, 3, walker), (2, 2, walker)], 2, 4);
                if let Pending::ChooseBlockers { blockers, .. } = &mut pending {
                    blockers[0].attackers.retain(|a| *a == obj(1));
                }
                (v, pending)
            },
            &chump,
        ),
    ];
    for (case, (v, pending), expected) in &cases {
        for (name, profile) in PROFILES {
            assert_eq!(
                &HeuristicAgent::new(profile).act(v, pending),
                *expected,
                "{name}: {case}"
            );
        }
    }
}

#[test]
fn combat_blocks_lethal_player_damage_before_protecting_a_planeswalker() {
    let mut v = view(
        0,
        &[3, 20],
        vec![
            permanent(obj(1), PlayerId::new(1), 3),
            permanent(obj(2), PlayerId::new(1), 10),
            permanent(obj(3), PlayerId::new(0), 1),
            walker(obj(4), PlayerId::new(0), 5),
        ],
    );
    v.combat.attackers = vec![
        baylee_view::AttackerView {
            creature: obj(1),
            defending: Defender::Player(v.seat),
            blocked: false,
        },
        baylee_view::AttackerView {
            creature: obj(2),
            defending: Defender::Planeswalker(obj(4)),
            blocked: false,
        },
    ];
    let pending = Pending::ChooseBlockers {
        demands: Vec::new(),
        player: v.seat,
        attacker: PlayerId::new(1),
        blockers: vec![baylee_engine::choice::BlockOption {
            blocker: obj(3),
            attackers: vec![obj(1), obj(2)],
        }],
        capacity: Vec::new(),
        obeying: Vec::new(),
        bounds: Vec::new(),
    };
    for (name, profile) in PROFILES {
        assert_eq!(
            HeuristicAgent::new(profile).act(&v, &pending),
            PlayerAction::DeclareBlockers {
                blockers: vec![(obj(3), obj(1))]
            },
            "{name} kept the walker and lost the game"
        );
    }
}

#[test]
fn every_adjacent_difficulty_changes_a_real_decision() {
    assert_eq!(AIProfile::NAMED.len(), 5);
    let mut v = view(0, &[20, 20], vec![]);
    v.hand = (0..7).map(|i| hand_card(i, "Brainstorm")).collect();
    let pending = Pending::Mulligan {
        player: v.seat,
        taken: 0,
        next_is_free: true,
        can_take: true,
    };
    let answer = |profile, view: &PlayerView, pending: &Pending| {
        HeuristicAgent::new(profile).act(view, pending)
    };
    assert_ne!(
        answer(AIProfile::NOVICE, &v, &pending),
        answer(AIProfile::CASUAL, &v, &pending)
    );
    v.hand = vec![hand_card(0, "Island"), hand_card(1, "Island")];
    v.hand
        .extend((2..7).map(|i| hand_card(i, "Darksteel Forge")));
    assert_ne!(
        answer(AIProfile::CASUAL, &v, &pending),
        answer(AIProfile::STEADY, &v, &pending)
    );
    let v = view(
        0,
        &[20, 5],
        vec![
            permanent(obj(1), PlayerId::new(0), 6),
            permanent(obj(2), PlayerId::new(1), 7),
        ],
    );
    let pending = Pending::ChooseAttackers {
        player: v.seat,
        attackers: vec![obj(1)],
        defenders: vec![Defender::Player(PlayerId::new(1))],
        required: Vec::new(),
        limits: Vec::new(),
    };
    assert_ne!(
        answer(AIProfile::STEADY, &v, &pending),
        answer(AIProfile::SHARP, &v, &pending)
    );
    // SHARP and EXPERT used to part on a 6/6 at four life beside their
    // 5/5, which SHARP sent and died to the swing back of. Since #123 no
    // profile attacks into a swing back that kills it, so they now part
    // on one that does not: two 4/4s at six life against two 3/3s, where
    // SHARP sends a 4/4 and EXPERT keeps both home (measured 24.09.2026).
    let v = view(
        0,
        &[6, 20],
        vec![
            permanent(obj(1), PlayerId::new(0), 4),
            permanent(obj(3), PlayerId::new(0), 4),
            permanent(obj(2), PlayerId::new(1), 3),
            permanent(obj(4), PlayerId::new(1), 3),
        ],
    );
    let pending = Pending::ChooseAttackers {
        player: v.seat,
        attackers: vec![obj(1), obj(3)],
        defenders: vec![Defender::Player(PlayerId::new(1))],
        required: Vec::new(),
        limits: Vec::new(),
    };
    assert_ne!(
        answer(AIProfile::SHARP, &v, &pending),
        answer(AIProfile::EXPERT, &v, &pending)
    );
}
