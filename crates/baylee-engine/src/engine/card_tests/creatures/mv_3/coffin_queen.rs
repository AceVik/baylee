//! `cards/creatures/mv_3/coffin_queen.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn coffin_queen() -> CardIndex {
    card_index("c19ec6aa-aadd-4767-b695-384fd07b4ded")
}

/// Coffin Queen (c19ec6aa-aadd-4767-b695-384fd07b4ded) — {2}{B} Creature — Zombie Wizard (Partial):
/// "You may choose not to untap this creature during your untap step. {2}{B}, {T}: Put target creature card from a graveyard onto the battlefield under your control."
#[test]
fn coffin_queen_reanimates_creature_from_any_graveyard_under_your_control() {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[coffin_queen(), swamp(), swamp(), swamp()])
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let bears_on_bf = on_battlefield(&engine, P1, index::GRIZZLY_BEARS).unwrap();
    bury(&mut engine, &[bears_on_bf]);
    engine.refresh_offer();
    let bears = in_graveyard(&engine, P1, index::GRIZZLY_BEARS).unwrap();

    tap_all_mana(&mut engine, P0);
    activate(&mut engine, P0, coffin_queen(), 1);
    aim_at(&mut engine, P0, bears);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P0, index::GRIZZLY_BEARS).is_some());
}
