//! `cards/lands/koskun_keep.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Koskun Keep prints three mana abilities — `{T}: Add {C}`, `{1}, {T}: Add
/// {R}`, and `{2}, {T}: Add {B} or {G}` — and only the first is the tap
/// symbol on its own. So `tap_all_mana` presses just that one (#159, CR
/// 605.1) and leaves the source the other two need tapped, which is why the
/// Keep is what `tap_all_mana_but` names as staying standing. Three Forests
/// are then the read of `{2}` — three green float, one comes back — and the
/// question the third line asks has to be exactly its two colors: not the
/// Red of the line above it, and not colorless.
#[test]
fn koskun_keep_asks_black_or_green_for_two_generic_and_pays_its_own_tap() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[koskun_keep(), forest(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let keep = on_battlefield(&engine, p0, koskun_keep()).expect("the Keep is out");
    tap_all_mana_but(&mut engine, p0, Some(koskun_keep()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "three Forests, and the Keep is one of the sources kept back"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.mana_abilities.contains(&keep),
        "the Keep holds no basic land type, so the CR 305.6 shortcut names \
         it nowhere: its three printed lines are ordinary `abilities`"
    );
    assert!(
        legal.abilities.contains(&(keep, 0))
            && legal.abilities.contains(&(keep, 1))
            && legal.abilities.contains(&(keep, 2)),
        "three green pays the {{1}} and the {{2}} the two upgraded lines \
         charge, so all three are offered: {:?}",
        legal.abilities
    );

    // Ability 2: {2}, {T}: Add {B} or {G}. Two of the three green pay the
    // generic and the third is the change.
    activate(&mut engine, p0, koskun_keep(), 2);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!("two colors is a question, got {:?}", engine.pending())
    };
    assert_eq!(
        player, p0,
        "the seat that activates is the seat that names it"
    );
    assert_eq!(
        options.len(),
        2,
        "the line prints two colors, so the question offers two: {options:?}"
    );
    assert!(
        options.contains(&ManaColor::Black) && options.contains(&ManaColor::Green),
        "Black and Green, the pair on this line: {options:?}"
    );
    assert!(
        !options.contains(&ManaColor::Red),
        "the {{R}} belongs to the {{1}}, {{T}} line above it: {options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .expect("black was one of the colors the question offered");

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Black),
        1,
        "the color that was named, and not a default"
    );
    assert_eq!(
        pool.available(ManaColor::Green),
        1,
        "three green less the {{2}} the line charges"
    );
    assert_eq!(
        pool.total(),
        2,
        "one of each, and nothing else came with it"
    );
    assert!(is_tapped(&engine, keep), "the Keep paid its own {{T}}");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability never uses the stack"
    );
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "and the seat holds priority again, got {:?}",
        engine.pending()
    );
}
