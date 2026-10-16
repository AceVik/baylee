//! `cards/creatures/mv_7/kalonian_behemoth.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn kalonian_behemoth() -> CardIndex {
    card_index("68d0b5c0-df6a-4eea-8dea-20358270f916")
}

/// Kalonian Behemoth — a 9/9 with shroud for {5}{G}{G}.
#[test]
fn kalonian_behemoth_costs_seven_and_is_a_nine_nine() {
    let mut engine = table7(&lots(forest(), 7), &[kalonian_behemoth()], &[]);
    let behemoth = cast_and_resolve(&mut engine, kalonian_behemoth());
    assert_body(&engine, behemoth, (9, 9), KeywordSet::SHROUD);
}

/// Shroud: no Lightning Bolt may aim at it, the opponent's or its own
/// controller's; the Ogre beside it is on both menus.
#[test]
fn kalonian_behemoth_cannot_be_targeted_by_anyone() {
    for caster in [0, 1] {
        let (menu, behemoth, ogre) = bolt_menu(kalonian_behemoth(), caster);
        assert!(
            menu.contains(&ogre),
            "seat {caster}: the Ogre is on the menu"
        );
        assert!(
            !menu.contains(&behemoth),
            "seat {caster}: shroud keeps everyone off it"
        );
    }
}
