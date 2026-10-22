//! `cards/artifacts/mv_2/cursed_totem.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{CURSED_TOTEM, PRODIGAL_SORCERER};

const P0: PlayerId = PlayerId::new(0);

/// Cursed Totem (6225a704-430a-4f56-ad87-0e8d87f285f5) — {2} Artifact:
/// "Activated abilities of creatures can't be activated."
#[test]
fn cursed_totem_prevents_activating_creature_abilities() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), PRODIGAL_SORCERER])
        .hand(0, &[CURSED_TOTEM])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let tim = on_battlefield(&engine, P0, PRODIGAL_SORCERER).unwrap();

    // Before Cursed Totem is on the battlefield, Prodigal Sorcerer can activate.
    let Pending::Priority { legal, .. } = engine.pending() else {
        unreachable!()
    };
    assert!(legal.abilities.contains(&(tim, 0)));

    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, CURSED_TOTEM);
    pass_until(&mut engine, stack_is_empty);

    // With Cursed Totem on the battlefield, Prodigal Sorcerer's ability cannot be activated.
    let Pending::Priority { legal, .. } = engine.pending() else {
        unreachable!()
    };
    assert!(!legal.abilities.contains(&(tim, 0)));
}
