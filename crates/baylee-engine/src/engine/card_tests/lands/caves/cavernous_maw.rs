//! `cards/lands/caves/cavernous_maw.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Cavernous Maw prints `{{T}}: Add {{C}}` and `{{2}}: This land becomes a 3/3 Elemental creature until end of turn. It's still a Cave land.`
/// The card is marked `Coverage::Partial` because the activation condition requiring three or more Caves is unsupported and offered unconditionally.
/// Paying `{{2}}` to activate the animation turns the land into an untapped 3/3 Elemental creature while retaining its land type.
#[test]
fn cavernous_maw_animates_into_a_three_three_elemental_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest()])
        .hand(0, &[cavernous_maw()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let maw = play_land(&mut engine, p0, cavernous_maw());
    assert!(!entered_tapped(&engine, maw));
    assert!(!types(&engine, maw).contains(TypeSet::CREATURE));

    tap_all_mana_but(&mut engine, p0, Some(cavernous_maw()));
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        2
    );

    activate(&mut engine, p0, cavernous_maw(), 1);
    pass_until(&mut engine, stack_is_empty);

    let t = types(&engine, maw);
    assert!(t.contains(TypeSet::CREATURE));
    assert!(t.contains(TypeSet::LAND));
    assert_eq!(pt(&engine, maw), (3, 3));
    assert!(!is_tapped(&engine, maw));
}
