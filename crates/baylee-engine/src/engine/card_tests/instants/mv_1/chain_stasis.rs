//! Chain Stasis — `{U}` Instant (Homelands)
//!
//! You may tap or untap target creature. Then that creature's controller may
//! pay `{2}{U}`. If the player does, they may copy this spell and may choose a
//! new target for that copy.
//!
//! (`Coverage::Partial`: target controller paying `{2}{U}` to copy the spell on
//! resolution is not in `Effect`; Chain Stasis may tap or untap target
//! creature.)

#[allow(clippy::wildcard_imports)]
use super::*;

fn chain_stasis() -> CardIndex {
    card_index("1ec365f8-bfb8-416f-b4db-868a8f5d042a")
}

#[test]
fn chain_stasis_may_tap_or_untap_target_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .hand(0, &[chain_stasis(), chain_stasis()])
        .battlefield(0, &[island(), island(), grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bears = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    assert!(!is_tapped(&engine, bears));

    // Cast first Chain Stasis to tap Grizzly Bears (YesNo(true) -> ChooseMode(0)).
    let first = in_hand(&engine, p0, chain_stasis()).unwrap();
    tap_all_mana(&mut engine, p0);
    engine
        .apply(p0, PlayerAction::CastSpell { card: first })
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bears],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCastMode { .. })
    });
    engine.apply(p0, PlayerAction::ChooseMode(0)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(is_tapped(&engine, bears));

    // Cast second Chain Stasis to untap Grizzly Bears (YesNo(true) -> ChooseMode(1)).
    let second = in_hand(&engine, p0, chain_stasis()).unwrap();
    engine
        .apply(p0, PlayerAction::CastSpell { card: second })
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bears],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCastMode { .. })
    });
    engine.apply(p0, PlayerAction::ChooseMode(1)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(!is_tapped(&engine, bears));
}
