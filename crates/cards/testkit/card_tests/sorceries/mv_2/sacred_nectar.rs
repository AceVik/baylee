//! `cards/sorceries/mv_2/sacred_nectar.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

#[test]
fn sacred_nectar_gains_four_life_when_it_resolves_off_a_generic_and_a_white() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(311, forest())
        .battlefield(0, &[plains(), plains()])
        .hand(0, &[sacred_nectar()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let sources = all_on_battlefield(&engine, p0, plains());
    assert_eq!(sources.len(), 2, "two Plains are the whole of {{1}}{{W}}");
    assert_eq!(
        engine.state().players[0].life,
        20,
        "nothing has been gained yet"
    );

    // Mana first, then the claim: `castable` is read off the pool.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "two white floating, which is exactly the printed cost"
    );
    cast_with_floating(&mut engine, p0, sacred_nectar());

    assert!(
        !stack_is_empty(&engine),
        "a sorcery uses the stack, so the life is not gained on announcement"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{1}}{{W}} came out of the pool"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "still twenty while the spell is waiting to resolve"
    );

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[0].life,
        24,
        "\"You gain 4 life\" — four, and not one life per mana that paid it"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "and the life belongs to the caster, not to the opponent"
    );
    assert!(
        in_graveyard(&engine, p0, sacred_nectar()).is_some(),
        "a resolved sorcery goes to its owner's graveyard"
    );
}
