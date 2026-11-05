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

/// Move the declared attacker to player 2 after player 0's declaration. In
/// one case player 1 then outnumbers player 2 but not active player 0; in the
/// other the reverse is true. A rule reading the active player fails one arm.
#[track_caller]
pub(crate) fn block_compares_actual_attackers_controller(card: CardIndex) {
    type BlockCase<'a> = (
        &'a [CardIndex],
        &'a [CardIndex],
        &'a [CardIndex],
        [usize; 3],
        bool,
    );
    let p2 = PlayerId::new(2);
    let cases: &[BlockCase<'_>] = &[
        (
            &[
                gray_ogre(),
                baylee_core::generated::index::GRIZZLY_BEARS,
                baylee_core::generated::index::GRIZZLY_BEARS,
                baylee_core::generated::index::GRIZZLY_BEARS,
            ],
            &[card, gray_ogre()],
            &[],
            [3, 2, 1],
            true,
        ),
        (&[gray_ogre()], &[card], &[gray_ogre()], [0, 1, 2], false),
    ];
    for &(active, defending, third, expected_counts, legal) in cases {
        let mut engine = Duel::table(SEED, forest(), 3)
            .battlefield(0, active)
            .battlefield(1, defending)
            .battlefield(2, third)
            .start();
        keep_mulligans(&mut engine);
        at_player_zeros_second_attack(&mut engine);
        let attacker = on_battlefield(&engine, P0, gray_ogre()).expect("the attacker is out");
        let blocker = on_battlefield(&engine, P1, card).expect("the tested blocker is out");
        engine
            .apply(
                P0,
                PlayerAction::DeclareAttackers {
                    attackers: vec![(attacker, Defender::Player(P1))],
                },
            )
            .expect("a vanilla creature attacks player 1");
        engine
            .state
            .object_mut(attacker)
            .expect("the attacker remains on the battlefield")
            .set_controller(p2);
        engine.state.invalidate_projections();
        engine.sync_static_effects();
        engine.state.refresh_characteristics();
        pass_until(
            &mut engine,
            |e| matches!(e.pending(), Pending::ChooseBlockers { player, .. } if *player == P1),
        );

        assert_eq!(
            engine.state().turn.active,
            P0,
            "the active player did not change"
        );
        let counts = [
            controlled_creatures(&engine, P0),
            controlled_creatures(&engine, P1),
            controlled_creatures(&engine, p2),
        ];
        assert_eq!(
            counts, expected_counts,
            "the distinguishing board was built"
        );

        let Pending::ChooseBlockers { blockers, .. } = engine.pending().clone() else {
            unreachable!("the refreshed declaration still asks player 1 for blocks")
        };
        let offered = blockers
            .iter()
            .find(|option| option.blocker == blocker)
            .is_some_and(|option| option.attackers.contains(&attacker));
        assert_eq!(
            offered, legal,
            "the comparison reads the attacker's controller"
        );
        let result = engine.apply(
            P1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(blocker, attacker)],
            },
        );
        assert_eq!(result.is_ok(), legal, "offer and declaration disagree");
        assert_eq!(engine.state().combat.is_blocking(blocker, attacker), legal);
    }
}
