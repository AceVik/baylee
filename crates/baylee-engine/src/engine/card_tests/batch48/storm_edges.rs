//! Additional acceptance edges for this card.
#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn storm_separates_first_strike_and_regular_damage_batches() {
    let mut e = prepared(
        &[],
        &[
            index::STORM_THE_VAULT,
            index::WHITE_KNIGHT,
            index::GRIZZLY_BEARS,
        ],
        &[],
    );
    let knight = source(&e, P0, index::WHITE_KNIGHT);
    let bear = source(&e, P0, index::GRIZZLY_BEARS);
    attackers(&mut e, &[knight, bear]);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    e.apply(P1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut e, |e| {
        e.state.turn.step == crate::turn::Step::CombatEnd && stack_is_empty(e)
    });
    assert_eq!(token_count(&e, subtypes::artifact::TREASURE), 2);
}

#[test]
fn storm_prevented_combat_damage_creates_no_treasure() {
    let mut e = prepared(
        &[index::FOG],
        &[index::STORM_THE_VAULT, index::GRIZZLY_BEARS],
        &[],
    );
    let bear = source(&e, P0, index::GRIZZLY_BEARS);
    let before = e.state.players[1].life;
    attackers(&mut e, &[bear]);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    e.apply(P1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    cast(&mut e, index::FOG);
    pass_until(&mut e, stack_is_empty);
    pass_until(&mut e, |e| {
        e.state.turn.step == crate::turn::Step::CombatEnd && stack_is_empty(e)
    });
    assert_eq!(e.state.players[1].life, before);
    assert_eq!(token_count(&e, subtypes::artifact::TREASURE), 0);
}

#[test]
fn storm_rechecks_five_artifacts_when_transform_trigger_resolves() {
    let mut e = prepared(
        &[index::SHATTER],
        &[
            index::STORM_THE_VAULT,
            index::SOL_RING,
            index::SOL_RING,
            index::SOL_RING,
            index::SOL_RING,
            index::SOL_RING,
        ],
        &[],
    );
    let vault = source(&e, P0, index::STORM_THE_VAULT);
    let ring = source(&e, P0, index::SOL_RING);
    pass_until(&mut e, |e| {
        e.state.turn.step == crate::turn::Step::End
            && !stack_is_empty(e)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == P0)
    });
    cast(&mut e, index::SHATTER);
    targets(&mut e, vec![ring], vec![]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(face(&e, vault), 0);
}

#[test]
fn storm_simultaneous_combat_damage_groups_separately_for_each_recipient() {
    let p2 = PlayerId::new(2);
    let mut e = Duel::table(SEED, index::FOREST, 3)
        .battlefield(
            0,
            &[
                index::STORM_THE_VAULT,
                index::GRIZZLY_BEARS,
                index::GRIZZLY_BEARS,
                index::GRIZZLY_BEARS,
            ],
        )
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    let bears: Vec<_> = e
        .state
        .battlefield_seen()
        .filter(|id| {
            e.state
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == index::GRIZZLY_BEARS))
        })
        .collect();
    let before = [e.state.players[1].life, e.state.players[2].life];
    pass_until(
        &mut e,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == P0),
    );
    e.apply(
        P0,
        PlayerAction::DeclareAttackers {
            attackers: vec![
                (bears[0], Defender::Player(P1)),
                (bears[1], Defender::Player(p2)),
                (bears[2], Defender::Player(p2)),
            ],
        },
    )
    .unwrap();
    // The generic helper declines every defender's blockers in turn.
    pass_until(&mut e, |e| {
        e.state.turn.step == crate::turn::Step::CombatEnd && stack_is_empty(e)
    });
    assert_eq!(e.state.players[1].life, before[0] - 2);
    assert_eq!(e.state.players[2].life, before[1] - 4);
    assert_eq!(token_count(&e, subtypes::artifact::TREASURE), 2);
}
