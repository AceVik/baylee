//! `cards/creatures/mv_7/simic_sky_swallower.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn simic_sky_swallower() -> CardIndex {
    card_index("e5c48c6d-a7bf-45b3-b48c-dc884d49c5f7")
}

/// Simic Sky Swallower — a 6/6 with flying, trample and shroud for
/// {5}{G}{U}.
#[test]
fn simic_sky_swallower_costs_seven_and_is_a_six_six() {
    let board = [lots(forest(), 6), vec![island()]].concat();
    let mut engine = table7(&board, &[simic_sky_swallower()], &[]);
    let swallower = cast_and_resolve(&mut engine, simic_sky_swallower());
    assert_body(
        &engine,
        swallower,
        (6, 6),
        KeywordSet::FLYING
            .union(KeywordSet::TRAMPLE)
            .union(KeywordSet::SHROUD),
    );
}

/// Shroud: no Lightning Bolt, from either seat, may aim at it.
#[test]
fn simic_sky_swallower_cannot_be_targeted_by_anyone() {
    for caster in [0, 1] {
        let (menu, swallower, ogre) = bolt_menu(simic_sky_swallower(), caster);
        assert!(menu.contains(&ogre), "seat {caster}: the Ogre is a target");
        assert!(!menu.contains(&swallower), "seat {caster}: shroud");
    }
}

/// Flying: a ground Ogre may not block it.
#[test]
fn simic_sky_swallower_flies_over_ground_blockers() {
    let (may_block_it, may_block_ogre) = ogre_may_block(simic_sky_swallower());
    assert!(may_block_ogre);
    assert!(!may_block_it);
}
