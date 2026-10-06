//! `cards/lands/utility/mount_doom.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Mount Doom prints `{{T}}, Pay 1 life: Add {{B}} or {{R}}.`, `{{1}}{{B}}{{R}}, {{T}}: Mount Doom deals 1 damage to each opponent.`, and `{{5}}{{B}}{{R}}, {{T}}, Sacrifice Mount Doom and a legendary artifact: Choose up to two creatures, then destroy the rest. Activate only as a sorcery.`
///
/// Under `Coverage::Partial`, the board-wipe ability is omitted because no effect chooses a subset of creatures and destroys the remainder.
/// With Mount Doom, five `forest()`, one `swamp()`, and one `mountain()` under `PlayerId::new(0)`,
/// floating `{{5}}{{B}}{{R}}` with Mount Doom untapped confirms that abilities 0 and 1 are offered while ability 2 is absent from `legal.abilities`.
/// Activating ability 1 deals 1 damage to the opponent, reducing their life total to 19 while leaving controller life at 20.
#[test]
fn mount_doom_deals_damage_to_opponent_and_omits_board_wipe() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                mount_doom(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                swamp(),
                mountain(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let doom = on_battlefield(&engine, p0, mount_doom()).expect("mount doom on battlefield");

    // Float {{5}}{{B}}{{R}} from basics while keeping Mount Doom untapped.
    tap_mana_except(&mut engine, p0, doom);
    assert_eq!(engine.state().players[0].mana_pool.total(), 7);

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(legal.abilities.contains(&(doom, 0)), "ability 0 is offered");
    assert!(
        legal.abilities.contains(&(doom, 1)),
        "ability 1 ({{1}}{{B}}{{R}}: Deal 1 damage) is offered"
    );
    assert!(
        !legal.abilities.contains(&(doom, 2)),
        "ability 2 is omitted under `Coverage::Partial` even with {{5}}{{B}}{{R}} floating"
    );

    activate(&mut engine, p0, mount_doom(), 1);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[1].life,
        19,
        "opponent takes 1 damage"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "controller takes no damage"
    );
    assert!(is_tapped(&engine, doom));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "{{1}}{{B}}{{R}} spent from floating pool"
    );
}
