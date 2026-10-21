//! `cards/enchantments/auras/mv_4/snowblind.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn snowblind() -> CardIndex {
    card_index("3f8bfa3d-7490-4259-a403-d13585ed2f66")
}

/// Snowblind (3f8bfa3d-7490-4259-a403-d13585ed2f66) — {3}{G} Enchantment — Aura:
/// "Enchant creature"
/// (Coverage::Partial — dynamic -X/-Y based on snow lands with Y capped at toughness minus 1 not supported.)
#[test]
fn snowblind_attaches_to_target_creature() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest()])
        .battlefield(1, &[grizzly_bears()])
        .hand(0, &[snowblind()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let bears = on_battlefield(&engine, P1, grizzly_bears()).unwrap();
    cast_from_hand(&mut engine, P0, snowblind());
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![bears],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let aura = on_battlefield(&engine, P0, snowblind()).unwrap();
    assert_eq!(
        engine.state().object(aura).unwrap().attached_to,
        Some(bears),
        "Snowblind is attached to the targeted creature"
    );
    assert_eq!(pt(&engine, bears), (2, 2));
}
