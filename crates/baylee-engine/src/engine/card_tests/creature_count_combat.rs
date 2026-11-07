//! Shared played scenarios for the two "control more creatures" combat
//! restrictions. Each card keeps its own tests and literal card id.

use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

/// Reach player 0's second attack declaration, when a preset creature is no
/// longer summoning-sick. Other players declare no attackers on the way.
#[track_caller]
fn at_player_zeros_second_attack(engine: &mut Engine<RegistryLookup>) {
    pass_until(
        engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == P0),
    );
    engine
        .apply(P0, PlayerAction::DeclareAttackers { attackers: vec![] })
        .expect("the first, summoning-sick attack may be empty");
    pass_until(
        engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == P0),
    );
}

/// Two against two is not "more" against player 1, but is more than player
/// 2's zero. The offer must name only the latter, and `apply` must agree.
#[track_caller]
pub(crate) fn attack_compares_each_defender(card: CardIndex) {
    let p2 = PlayerId::new(2);
    let mut engine = Duel::table(SEED, forest(), 3)
        .battlefield(0, &[card, gray_ogre()])
        .battlefield(
            1,
            &[
                baylee_core::generated::index::GRIZZLY_BEARS,
                baylee_core::generated::index::GRIZZLY_BEARS,
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    at_player_zeros_second_attack(&mut engine);

    let creature = on_battlefield(&engine, P0, card).expect("the tested creature is out");
    let Pending::ChooseAttackers {
        attackers,
        defenders,
        limits,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("the helper stopped at the attack declaration")
    };
    assert!(attackers.contains(&creature));
    let allowed = limits
        .iter()
        .find(|limit| limit.creature == creature)
        .map_or(defenders, |limit| limit.defenders.clone());
    assert_eq!(
        allowed,
        vec![Defender::Player(p2)],
        "an equal creature count is not a legal defender"
    );
    assert!(
        engine
            .apply(
                P0,
                PlayerAction::DeclareAttackers {
                    attackers: vec![(creature, Defender::Player(P1))],
                },
            )
            .is_err(),
        "a direct declaration may not bypass the per-defender restriction"
    );
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(creature, Defender::Player(p2))],
            },
        )
        .expect("two creatures outnumber an empty board");
    assert!(engine.state().combat.is_attacking(creature));
}

fn controlled_creatures(engine: &Engine<RegistryLookup>, player: PlayerId) -> usize {
    engine
        .state()
        .battlefield_seen()
        .filter(|id| {
            engine.state().object(*id).is_some_and(|object| {
                object.controller == player
                    && object.characteristics().types.contains(TypeSet::CREATURE)
            })
        })
        .count()
}

/// One board and what it must answer. `stolen` names the creature player 2
/// takes with Ray of Command after attackers are declared, and whose it was.
struct BlockCase<'a> {
    attacking: &'a [CardIndex],
    defending: &'a [CardIndex],
    third: &'a [CardIndex],
    stolen: Option<(PlayerId, CardIndex)>,
    counts: [usize; 3],
    legal: bool,
}

/// Plays `case`: player 0 attacks with everything, player 2 steals if the case
/// says so, and player 1 is offered and tries the block of player 0's Ogre.
#[track_caller]
fn play_block_case(card: CardIndex, case: &BlockCase<'_>) {
    let p2 = PlayerId::new(2);
    let ray_of_command = baylee_core::generated::index::RAY_OF_COMMAND;
    let third: Vec<CardIndex> = case.third.iter().copied().chain([island(); 4]).collect();
    let mut engine = Duel::table(SEED, forest(), 3)
        .battlefield(0, case.attacking)
        .battlefield(1, case.defending)
        .battlefield(2, &third)
        .hand(2, &[ray_of_command])
        .start();
    keep_mulligans(&mut engine);
    at_player_zeros_second_attack(&mut engine);
    let attacker = on_battlefield(&engine, P0, gray_ogre()).expect("the attacker is out");
    let blocker = on_battlefield(&engine, P1, card).expect("the tested blocker is out");
    let attackers: Vec<_> = engine
        .state()
        .battlefield_seen()
        .filter(|id| engine.state().object(*id).unwrap().controller == P0)
        .filter(|id| {
            let object = engine.state().object(*id).unwrap();
            object.characteristics().types.contains(TypeSet::CREATURE)
        })
        .map(|id| (id, Defender::Player(P1)))
        .collect();
    let stolen = case
        .stolen
        .map(|(seat, creature)| on_battlefield(&engine, seat, creature).unwrap());
    engine
        .apply(P0, PlayerAction::DeclareAttackers { attackers })
        .expect("vanilla creatures attack player 1");

    if let Some(stolen) = stolen {
        pass_until(
            &mut engine,
            |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p2),
        );
        tap_all_mana(&mut engine, p2);
        cast_with_floating(&mut engine, p2, ray_of_command);
        aim_at(&mut engine, p2, stolen);
        pass_until(&mut engine, stack_is_empty);
        assert_eq!(engine.state().object(stolen).unwrap().controller, p2);
        assert!(
            !engine.state().combat.is_attacking(stolen),
            "a creature that changes controllers is out of combat (CR 506.4)"
        );
        assert!(engine.state().combat.is_attacking(attacker));
    }
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseBlockers { player, .. } if *player == P1),
    );

    assert_eq!(engine.state().turn.active, P0, "the attacking player");
    let counts = [
        controlled_creatures(&engine, P0),
        controlled_creatures(&engine, P1),
        controlled_creatures(&engine, p2),
    ];
    assert_eq!(counts, case.counts, "the distinguishing board was built");

    let Pending::ChooseBlockers { blockers, .. } = engine.pending().clone() else {
        unreachable!("the pass stopped at player 1's blocks")
    };
    let offered = blockers
        .iter()
        .find(|option| option.blocker == blocker)
        .is_some_and(|option| option.attackers.contains(&attacker));
    assert_eq!(
        offered, case.legal,
        "the comparison reads the attacking player's creatures"
    );
    let result = engine.apply(
        P1,
        PlayerAction::DeclareBlockers {
            blockers: vec![(blocker, attacker)],
        },
    );
    assert_eq!(result.is_ok(), case.legal, "offer and declaration disagree");
    assert_eq!(
        engine.state().combat.is_blocking(blocker, attacker),
        case.legal
    );
}

/// Player 1's creatures against the attacking player's, as blocks are
/// declared. Player 0 is the active player and so the attacking player
/// (CR 506.2), and a creature that changes controllers leaves combat
/// (CR 506.4), so every attacker here is player 0's: "attacking player" and
/// "the attacker's controller" part only on a shared team turn (CR 805.10c),
/// which no game hosted here has. What can be read wrongly is the third seat,
/// which the first two cases put on the other side of the answer, or a count
/// from before the declaration, which the last two move with a real spell.
#[track_caller]
pub(crate) fn block_compares_the_attacking_player(card: CardIndex) {
    let bears = baylee_core::generated::index::GRIZZLY_BEARS;
    let cases = [
        // A tie with player 0 is not more, however few player 2 has.
        BlockCase {
            attacking: &[gray_ogre(), bears],
            defending: &[card, gray_ogre()],
            third: &[],
            stolen: None,
            counts: [2, 2, 0],
            legal: false,
        },
        // More than player 0 is enough, however many player 2 has.
        BlockCase {
            attacking: &[gray_ogre()],
            defending: &[card, gray_ogre()],
            third: &[gray_ogre(), bears, bears],
            stolen: None,
            counts: [1, 2, 3],
            legal: true,
        },
        // The tie is broken after the declaration: the attacking Bears leave
        // combat and player 0's count with them.
        BlockCase {
            attacking: &[gray_ogre(), bears],
            defending: &[card, gray_ogre()],
            third: &[],
            stolen: Some((P0, bears)),
            counts: [1, 2, 1],
            legal: true,
        },
        // The lead is lost after the declaration: player 1 had two to one.
        BlockCase {
            attacking: &[gray_ogre()],
            defending: &[card, gray_ogre()],
            third: &[],
            stolen: Some((P1, gray_ogre())),
            counts: [1, 1, 1],
            legal: false,
        },
    ];
    for case in &cases {
        play_block_case(card, case);
    }
}
