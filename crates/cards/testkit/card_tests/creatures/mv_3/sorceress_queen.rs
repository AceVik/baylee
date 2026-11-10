//! `cards/creatures/mv_3/sorceress_queen.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Sorceress Queen — `{1}{B}{B}` 1/1: "{T}: Target creature other than this
/// one has base power and toughness 0/2 until end of turn."
///
/// The set is layer 7b (CR 613.4b), so a 1/1 Elf becomes 0/2 and Giant
/// Growth's +3/+3 in layer 7c lands on top for 3/5. The menu is the other
/// half: "other than this one" keeps the Queen off it, and the effect ends
/// in the cleanup step (CR 514.2).
#[test]
fn sorceress_queen_sets_another_creatures_base_body_and_never_herself() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[sorceress_queen(), llanowar_elves(), forest()])
        .hand(0, &[giant_growth()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let queen = on_battlefield(&engine, p0, sorceress_queen()).expect("seated");
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the other creature");
    assert_eq!(pt(&engine, elf), (1, 1), "the printed body");

    activate(&mut engine, p0, sorceress_queen(), 0);
    let menu = aim_at(&mut engine, p0, elf);
    assert!(menu.contains(&elf), "the Elf is a legal target: {menu:?}");
    assert!(
        !menu.contains(&queen),
        "\"other than this one\": the Queen cannot point the ability at \
         herself: {menu:?}"
    );
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, elf),
        (0, 2),
        "\"base power and toughness 0/2\" (CR 613.4b)"
    );

    cast_from_hand(&mut engine, p0, giant_growth());
    aim_at(&mut engine, p0, elf);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, elf),
        (3, 5),
        "+3/+3 in layer 7c applies on top of the set body"
    );

    reach_their_main_phase(&mut engine, p1);
    assert_eq!(
        pt(&engine, elf),
        (1, 1),
        "both effects ended in the cleanup step (CR 514.2)"
    );
}
