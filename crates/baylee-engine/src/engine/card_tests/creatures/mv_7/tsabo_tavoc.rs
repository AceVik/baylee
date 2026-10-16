//! `cards/creatures/mv_7/tsabo_tavoc.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn tsabo_tavoc() -> CardIndex {
    card_index("f98f1f71-99ec-4ffb-9d8e-acbab173f972")
}

/// Tsabo Tavoc — a legendary 7/4 with first strike for {5}{B}{R}.
#[test]
fn tsabo_tavoc_costs_seven_and_is_a_seven_four() {
    let board = [vec![swamp(), mountain()], lots(forest(), 5)].concat();
    let mut engine = table7(&board, &[tsabo_tavoc()], &[]);
    let tsabo = cast_and_resolve(&mut engine, tsabo_tavoc());
    assert_body(&engine, tsabo, (7, 4), KeywordSet::FIRST_STRIKE);
}

/// "{B}{B}, {T}: Destroy target legendary creature. It can't be regenerated."
/// The menu is legendary creatures only (Isamaru yes, the Ogre no); Isamaru
/// dies.
#[test]
fn tsabo_tavoc_destroys_a_legendary_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = table7(
        &[tsabo_tavoc(), swamp(), swamp()],
        &[],
        &[isamaru(), gray_ogre()],
    );
    let legend = on_battlefield(&engine, p1, isamaru()).expect("out");
    let ogre = on_battlefield(&engine, p1, gray_ogre()).expect("out");
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, tsabo_tavoc(), 1);
    let (options, _) = target_menu(&engine);
    assert!(options.contains(&legend));
    assert!(!options.contains(&ogre), "not legendary");
    point_at(&mut engine, p0, &[legend], &[]);
    pass_until(&mut engine, stack_is_empty);
    assert!(in_graveyard(&engine, p1, isamaru()).is_some());
    assert!(on_battlefield(&engine, p1, gray_ogre()).is_some());
}

/// Protection from legendary creatures: a legendary creature may not block
/// Tsabo, an ordinary Ogre may.
#[test]
fn tsabo_tavoc_cannot_be_blocked_by_a_legendary_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = table7(&[tsabo_tavoc()], &[], &[isamaru(), gray_ogre()]);
    let tsabo = on_battlefield(&engine, p0, tsabo_tavoc()).expect("out");
    let legend = on_battlefield(&engine, p1, isamaru()).expect("out");
    let ogre = on_battlefield(&engine, p1, gray_ogre()).expect("out");
    let blocks = attack_and_collect_blocks(&mut engine, tsabo, p1);
    let can = |who: ObjectId| {
        blocks
            .iter()
            .any(|b| b.blocker == who && b.attackers.contains(&tsabo))
    };
    assert!(can(ogre), "the control: an ordinary creature may block");
    assert!(!can(legend), "protection from legendary creatures");
}
