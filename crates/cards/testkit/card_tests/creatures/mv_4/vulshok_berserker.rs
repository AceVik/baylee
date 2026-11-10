//! `cards/creatures/mv_4/vulshok_berserker.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Vulshok Berserker prints two lines and only one of them is a body: a
/// `{3}{R}` 3/2 with haste. What separates haste from an ordinary creature is
/// nothing the card file can show, so both are cast in the *same* main phase
/// and the attack declaration is read: the Elf arrives under the same control,
/// untapped and unsick in every way but one, and is not offered (CR 508.1a).
/// The offer therefore names the Berserker because of the keyword and not
/// because the board was listed whole — a sweep that offered everything would
/// put the Elf beside it.
#[test]
fn vulshok_berserker_attacks_the_turn_it_arrives_where_a_fresh_elf_may_not() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .hand(0, &[llanowar_elves(), vulshok_berserker()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Four Mountains and two Forests are six mana, enough for both spells with
    // room to spare, so neither cast can be refused for a payment the auto-payer
    // happened to choose badly. Everything happens in this one main phase, so
    // nothing on the board has had a turn in which to lose summoning sickness.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        6,
        "six lands tapped for six mana"
    );

    // The control first: a printed 1/1 with no text that could ever let it
    // attack early.
    cast_with_floating(&mut engine, p0, llanowar_elves());
    pass_until(&mut engine, stack_is_empty);
    cast_with_floating(&mut engine, p0, vulshok_berserker());
    pass_until(&mut engine, stack_is_empty);

    let berserker =
        on_battlefield(&engine, p0, vulshok_berserker()).expect("the Berserker resolved");
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elf resolved");
    assert_eq!(pt(&engine, berserker), (3, 2), "the body the card prints");
    assert!(
        keywords(&engine, berserker).contains(KeywordSet::HASTE),
        "haste, read after the layer system has run"
    );
    assert!(
        !is_tapped(&engine, berserker) && !is_tapped(&engine, elf),
        "neither creature was tapped for anything, so the difference between \
         them below is sickness and not status"
    );

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("pass_until stopped on nothing but the attack declaration")
    };
    assert!(
        attackers.contains(&berserker),
        "\"This creature can attack … as soon as it comes under your control\": \
         it arrived this turn and is offered: {attackers:?}"
    );
    assert!(
        !attackers.contains(&elf),
        "and the Elf that arrived in the same main phase is not — so the offer \
         is not simply every untapped creature under this seat: {attackers:?}"
    );
}
