//! `cards/creatures/mv_3/fyndhorn_brownie.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Fyndhorn Brownie — {2}{G} 1/1 Ouphe: "{2}{G}, {T}: Untap target creature."
///
/// Two Forests and two Llanowar Elves are four mana and nothing else on the
/// board makes any, so a pool that reads empty once the ability is announced is
/// the {2}{G} really paid — and CR 601.2c names the target before CR 601.2h
/// pays, which the four mana still floating and the untapped Brownie *while the
/// target question stands* are the other half of.
///
/// The untap is read off the two Elves that tapping for that mana left down:
/// the one that was named stands back up while its twin stays where the mana
/// left it, so the sentence is "target creature" and not "each creature you
/// control". The Elf across the table and the Brownie itself are the same
/// reading from the other two directions — the offer reaches both. The turn
/// cycle in the middle is the tap symbol (CR 302.6): the Brownie entered this
/// turn and could not have paid it yet.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn fyndhorn_brownie_taps_for_two_green_to_untap_the_creature_it_names() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(41, forest())
        .battlefield(0, &[forest(), forest(), llanowar_elves(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[fyndhorn_brownie()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // {2}{G} out of the four sources, which is every mana this board has.
    cast_from_hand(&mut engine, p0, fyndhorn_brownie());
    pass_until(&mut engine, stack_is_empty);
    let brownie = on_battlefield(&engine, p0, fyndhorn_brownie()).expect("the Brownie resolved");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "four tapped sources against a {{2}}{{G}} leaves one green floating, \
         and the turn cycle below is what empties it (CR 500.4)"
    );

    // A whole turn cycle, because the price includes the tap symbol (CR 302.6).
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(elves.len(), 2, "the two Elves untapped with the lands");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    let (aimed, bystander) = (elves[0], elves[1]);

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "two Forests and two Elves: four mana and no more"
    );
    assert!(
        is_tapped(&engine, aimed) && is_tapped(&engine, bystander),
        "both Elves tapped for the mana, which is what there now is to untap"
    );
    assert!(
        !is_tapped(&engine, theirs),
        "and the Elf across the table never moved"
    );

    activate(&mut engine, p0, fyndhorn_brownie(), 0);
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat is the one that chooses");
    assert!(
        options.contains(&aimed) && options.contains(&bystander),
        "both creatures you control are on the menu: {options:?}"
    );
    assert!(
        options.contains(&theirs),
        "\"target creature\" is no \"you control\": the Elf across the table is \
         a legal target too: {options:?}"
    );
    assert!(
        options.contains(&brownie),
        "the source is a creature as well, so it is on its own menu: {options:?}"
    );
    // CR 601.2c picked the target, so nothing of the price has been paid yet.
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "the {{2}}{{G}} is the last step of the activation, after the target"
    );
    assert!(!is_tapped(&engine, brownie), "and so is its {{T}}");

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![aimed],
            },
        )
        .expect("the Elf was one of the options it enumerated");
    assert!(
        is_tapped(&engine, brownie),
        "{{T}} was paid as the answer was applied"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "three of the four went on the {{2}}{{G}} — the fourth is the Elf's \
         own green, tapped because a tapped Elf is what there is to untap"
    );
    assert!(
        !stack_is_empty(&engine),
        "untapping is no mana ability, so the ability is on the stack"
    );

    pass_until(&mut engine, stack_is_empty);
    assert!(
        !is_tapped(&engine, aimed),
        "\"untap target creature\": the Elf that was named is standing again"
    );
    assert!(
        is_tapped(&engine, bystander),
        "the Elf it did not name is still down — the ability untaps its own \
         target and no other creature on the table"
    );
    assert!(
        !is_tapped(&engine, theirs),
        "and the opponent's board was never touched"
    );
}
