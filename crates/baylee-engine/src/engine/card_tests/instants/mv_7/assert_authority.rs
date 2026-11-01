//! `cards/instants/mv_7/assert_authority.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{HILL_GIANT, ORNITHOPTER};

fn assert_authority() -> CardIndex {
    card_index("085bc7be-bd44-40af-8e8f-a1a8006fe22c")
}

/// Assert Authority — {5}{U}{U} Instant
/// "Affinity for artifacts"
/// "Counter target spell. If that spell is countered this way, exile it instead of putting it into its owner's graveyard."
#[test]
fn assert_authority_has_affinity_and_counters_spell_to_exile() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .hand(0, &[assert_authority()])
        .battlefield(
            0,
            &[
                island(),
                island(),
                ORNITHOPTER,
                ORNITHOPTER,
                ORNITHOPTER,
                ORNITHOPTER,
                ORNITHOPTER,
            ],
        )
        .hand(1, &[HILL_GIANT])
        .battlefield(1, &[mountain(), mountain(), mountain(), mountain()])
        .start();
    keep_mulligans(&mut engine);

    // Advance to P1's main phase so P1 casts Hill Giant.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1
            && e.state().turn.phase == Phase::FirstMain
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p1)
    });
    cast_from_hand(&mut engine, p1, HILL_GIANT);
    let spell = on_stack(&engine, HILL_GIANT).unwrap();

    // P1 passes priority to P0; P0 casts Assert Authority for {U}{U} thanks to 5 artifacts.
    engine.apply(p1, PlayerAction::PassPriority).unwrap();
    cast_from_hand(&mut engine, p0, assert_authority());
    aim_at(&mut engine, p0, spell);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p1, HILL_GIANT).is_none());
    assert!(in_graveyard(&engine, p1, HILL_GIANT).is_none());
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Exile(p1))
            .iter()
            .any(|&id| engine.state().object(id).unwrap().card.unwrap().index == HILL_GIANT),
        "countered Hill Giant is exiled instead of going to graveyard"
    );
}
