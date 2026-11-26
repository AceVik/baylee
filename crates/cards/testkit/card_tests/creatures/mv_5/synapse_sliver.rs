//! `cards/creatures/mv_5/synapse_sliver.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

fn synapse_sliver() -> CardIndex {
    card_index("b76f123d-7b78-42a1-a66e-0dee6f773687")
}

/// Whenever a Sliver deals combat damage to a player, Synapse Sliver lets that
/// Sliver's controller optionally draw a card (non-Slivers do not trigger).
#[test]
fn synapse_sliver_draws_on_sliver_combat_damage_to_player() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[synapse_sliver(), index::METALLIC_SLIVER, grizzly_bears()],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
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

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1,
        "only the Sliver triggered Synapse Sliver"
    );
}

/// Symmetry: when an opponent's Sliver deals combat damage to a player,
/// that Sliver's controller (the opponent) gets the optional card draw.
#[test]
fn synapse_sliver_lets_opponent_draw_when_opponent_sliver_deals_combat_damage() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[synapse_sliver()])
        .battlefield(1, &[index::METALLIC_SLIVER])
        .start();
    keep_mulligans(&mut engine);

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p1),
    );
    let p0_hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    let p1_hand_before = engine.state().zones.list(ZoneLocation::Hand(p1)).len();
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
        "opponent who controls the damaging Sliver gets the draw YesNo prompt"
    );
    engine.apply(p1, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p1)).len(),
        p1_hand_before + 1,
        "opponent drew 1 card from their Sliver hitting"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        p0_hand_before,
        "Synapse Sliver controller did not draw"
    );
}
