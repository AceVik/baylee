//! `cards/creatures/mv_4/toxin_sliver.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

/// Toxin Sliver (`96ccad44-7912-4fdb-bb2d-9d16cc3cd68a`) — {3}{B} Creature — Sliver 3/3:
/// "Whenever a Sliver deals combat damage to a creature, destroy that creature. It can't be regenerated."
#[test]
fn toxin_sliver_destroys_creature_dealt_combat_damage_by_any_sliver() {
    let toxin_card = card_index("96ccad44-7912-4fdb-bb2d-9d16cc3cd68a");
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);

    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[toxin_card, index::METALLIC_SLIVER])
        .battlefield(1, &[index::FORCE_OF_NATURE])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let sliver = on_battlefield(&engine, p0, index::METALLIC_SLIVER).unwrap();
    let force = on_battlefield(&engine, p1, index::FORCE_OF_NATURE).unwrap();
    engine
        .dev_state_mut(p1)
        .unwrap()
        .object_mut(force)
        .unwrap()
        .regeneration_shields = 1;

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(sliver, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(force, sliver)],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);

    assert!(in_graveyard(&engine, p1, index::FORCE_OF_NATURE).is_some());
}
