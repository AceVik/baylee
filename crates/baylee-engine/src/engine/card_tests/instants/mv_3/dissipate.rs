//! `cards/instants/mv_3/dissipate.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::HILL_GIANT;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn dissipate() -> CardIndex {
    card_index("4b2e9aa9-6f91-41de-84b6-e9a89be06a53")
}

/// Dissipate (4b2e9aa9-6f91-41de-84b6-e9a89be06a53) — {1}{U}{U} Instant:
/// "Counter target spell. If that spell is countered this way, exile it instead
/// of putting it into its owner's graveyard."
#[test]
fn dissipate_counters_target_spell_and_exiles_it() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[mountain(), mountain(), mountain(), mountain()])
        .battlefield(1, &[island(), island(), island()])
        .hand(0, &[HILL_GIANT])
        .hand(1, &[dissipate()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, HILL_GIANT);
    engine.apply(P0, PlayerAction::PassPriority).unwrap();

    let spell = engine.state().zones.list(ZoneLocation::Stack)[0];
    tap_all_mana(&mut engine, P1);
    cast_with_floating(&mut engine, P1, dissipate());
    aim_at(&mut engine, P1, spell);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P0, HILL_GIANT).is_none());
    assert!(in_graveyard(&engine, P0, HILL_GIANT).is_none());
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Exile(P0))
            .iter()
            .any(
                |&id| engine.state().object(id).unwrap().card.map(|c| c.index) == Some(HILL_GIANT)
            )
    );
}
