//! `cards/creatures/mv_5/brood_sliver.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

fn brood_sliver() -> CardIndex {
    card_index("6b1d178f-9713-4dcf-920b-ddf2c94cc427")
}

/// Whenever any Sliver deals combat damage to a player, Brood Sliver lets its
/// controller create a 1/1 Sliver token; non-Slivers do not trigger it.
#[test]
fn brood_sliver_creates_sliver_token_only_on_sliver_combat_damage() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[brood_sliver(), index::METALLIC_SLIVER, grizzly_bears()],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let metallic = on_battlefield(&engine, p0, index::METALLIC_SLIVER).unwrap();
    let bears = on_battlefield(&engine, p0, grizzly_bears()).unwrap();

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![
                    (metallic, Defender::Player(p1)),
                    (bears, Defender::Player(p1)),
                ],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });

    let slivers: Vec<_> = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|&id| {
            engine.state().object(id).is_some_and(|o| {
                o.controller == p0 && o.characteristics().types.contains(TypeSet::CREATURE)
            })
        })
        .collect();
    assert_eq!(
        slivers.len(),
        4,
        "Brood + Metallic + Bears + 1 Sliver token"
    );
}

/// Symmetry: when an opponent's Sliver deals combat damage to a player,
/// that Sliver's controller (the opponent) gets the optional 1/1 Sliver token.
#[test]
fn brood_sliver_prompts_opponent_when_opponent_sliver_deals_combat_damage() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[brood_sliver()])
        .battlefield(1, &[index::METALLIC_SLIVER])
        .start();
    keep_mulligans(&mut engine);

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p1),
    );
    let opp_metallic = on_battlefield(&engine, p1, index::METALLIC_SLIVER).unwrap();
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(opp_metallic, Defender::Player(p0))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(p0, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    let Pending::YesNo { player, .. } = engine.pending().clone() else {
        unreachable!();
    };
    assert_eq!(
        player, p1,
        "opponent who controls the damaging Sliver gets the YesNo prompt"
    );
    engine.apply(p1, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });

    let count_creatures = |owner: PlayerId| {
        engine
            .state()
            .zones
            .list(ZoneLocation::Battlefield)
            .iter()
            .filter(|&&id| {
                engine.state().object(id).is_some_and(|o| {
                    o.controller == owner && o.characteristics().types.contains(TypeSet::CREATURE)
                })
            })
            .count()
    };
    assert_eq!(
        count_creatures(p1),
        2,
        "opponent created a 1/1 Sliver token"
    );
    assert_eq!(
        count_creatures(p0),
        1,
        "Brood Sliver controller got no token"
    );
}
