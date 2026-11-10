//! `cards/creatures/mv_1/raging_goblin.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Raging Goblin is a `{R}` 1/1 Goblin Berserker whose entire printed text is
/// haste, and haste is a keyword no projection reads by itself: the only place
/// it means anything is the attack declaration on the turn the creature
/// arrived. The scenario casts the Goblin and a Llanowar Elves in the same
/// first main phase off the same tapped pool, so both creatures entered under
/// the same seat in the same turn — and reads the offer: the Goblin is an
/// attacker, the Elves are not. Both are asserted untapped immediately before
/// the declaration, because a creature excluded for being *tapped* would
/// satisfy "the Elves are not offered" without saying anything about summoning
/// sickness (CR 302.6), and haste is exactly the permission that skips it.
#[test]
fn raging_goblin_attacks_the_turn_it_arrives_while_a_fresh_elf_cannot() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(777, forest())
        .battlefield(0, &[mountain(), mountain(), forest(), forest()])
        .hand(0, &[raging_goblin(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Every source on the board is a land whose whole price is its own {T},
    // so the pool is where the two spells come from — and nothing else on this
    // board makes mana, which is what `total` below claims.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "two Mountains and two Forests: {{R}}{{R}}{{G}}{{G}}"
    );
    cast_with_floating(&mut engine, p0, raging_goblin());
    pass_until(&mut engine, stack_is_empty);
    cast_with_floating(&mut engine, p0, llanowar_elves());
    pass_until(&mut engine, stack_is_empty);

    let goblin = on_battlefield(&engine, p0, raging_goblin()).expect("the Goblin resolved");
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves resolved");
    assert_eq!(pt(&engine, goblin), (1, 1), "the body the card prints");
    assert!(
        keywords(&engine, goblin).contains(KeywordSet::HASTE),
        "haste is on the permanent once the layers have run"
    );
    assert!(
        !is_tapped(&engine, goblin) && !is_tapped(&engine, elves),
        "neither creature paid for anything: its own mana ability was not \
         pressed, and both are standing when the declaration is offered"
    );

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("pass_until stops on nothing but the attack declaration")
    };
    assert!(
        attackers.contains(&goblin),
        "haste lets it attack and {{T}} the turn it comes under your control: {attackers:?}"
    );
    assert!(
        !attackers.contains(&elves),
        "the Elves arrived this turn too and print no haste, so CR 302.6 \
         keeps them out of the offer while they stand untapped: {attackers:?}"
    );
}
