//! `cards/creatures/mv_5/acidic_slime.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// "When this creature enters, destroy target artifact, enchantment, or
/// land." The menu is every land on the table and nothing else here — not
/// the opponent's creature — and the named land is destroyed. Deathtouch
/// is printed.
#[test]
fn acidic_slime_destroys_the_land_it_points_at() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest(), forest()])
        .battlefield(1, &[rogue_s_passage(), steadfast_guard()])
        .hand(0, &[acidic_slime()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, acidic_slime());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let passage = on_battlefield(&engine, p1, rogue_s_passage()).unwrap();
    let guard = on_battlefield(&engine, p1, steadfast_guard()).unwrap();
    let menu = aim_at(&mut engine, p0, passage);
    assert_eq!(menu.len(), 6, "five Forests and the Passage");
    assert!(!menu.contains(&guard), "a creature is none of the three");
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p1, rogue_s_passage()).is_none());
    assert!(in_graveyard(&engine, p1, rogue_s_passage()).is_some());
    let slime = on_battlefield(&engine, p0, acidic_slime()).unwrap();
    assert!(keywords(&engine, slime).contains(KeywordSet::DEATHTOUCH));
}
