//! `cards/creatures/mv_2/plague_fiend.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

fn plague_fiend() -> CardIndex {
    card_index("d287c11e-0684-42ed-abdb-758852ed0502")
}

/// Plague Fiend ({1}{B} 1/1 Creature — Insect):
/// "Whenever this creature deals combat damage to a creature, destroy that creature unless its controller pays {2}."
#[test]
fn plague_fiend_destroys_damaged_creature_when_controller_declines_to_pay_two() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let subject = plague_fiend();
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[subject])
        .battlefield(1, &[index::HILL_GIANT])
        .start();
    keep_mulligans(&mut engine);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );

    let fiend = on_battlefield(&engine, p0, subject).unwrap();
    let giant = on_battlefield(&engine, p1, index::HILL_GIANT).unwrap();
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(fiend, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseBlockers { player, .. } if *player == p1),
    );
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(giant, fiend)],
            },
        )
        .unwrap();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::YesNo { player, .. } if *player == p1),
    );
    engine.apply(p1, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p1, index::HILL_GIANT).is_none());
    assert!(in_graveyard(&engine, p1, index::HILL_GIANT).is_some());
}

/// When the damaged creature's controller pays {2}, Plague Fiend's trigger does not destroy it.
#[test]
fn plague_fiend_spares_damaged_creature_when_controller_pays_two() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let subject = plague_fiend();
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[subject])
        .battlefield(1, &[index::HILL_GIANT, mountain(), mountain()])
        .start();
    keep_mulligans(&mut engine);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );

    let fiend = on_battlefield(&engine, p0, subject).unwrap();
    let giant = on_battlefield(&engine, p1, index::HILL_GIANT).unwrap();
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(fiend, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseBlockers { player, .. } if *player == p1),
    );
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(giant, fiend)],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        !stack_is_empty(e)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p1)
    });
    tap_all_mana(&mut engine, p1);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::YesNo { player, .. } if *player == p1),
    );
    engine.apply(p1, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p1, index::HILL_GIANT).is_some());
}
