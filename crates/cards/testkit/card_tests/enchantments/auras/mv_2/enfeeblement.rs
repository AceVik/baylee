//! `cards/enchantments/auras/mv_2/enfeeblement.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Enfeeblement prints two sentences — "Enchant creature" and "Enchanted
/// creature gets -2/-2" — and neither can be read off the card file: one is a
/// target the engine offers, the other a static whose filter is
/// `Filter::AttachedToBySource`. So the board holds a body a -2/-2 leaves
/// alive (a 4/4, or there would be no projected numbers to read at all) and a
/// bystander across the table, which is what separates this card's filter from
/// a modifier that shrank everything in play. The pump is asserted as the
/// *delta* off the body the board printed rather than as a number assumed from
/// the printing, and the target prompt is read for what it declines as well as
/// for what it offers.
#[test]
fn enfeeblement_shrinks_the_creature_it_enchants_and_no_other() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(211, swamp())
        .battlefield(0, &[swamp(), swamp(), thrun_the_last_troll()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[enfeeblement()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let host = on_battlefield(&engine, p0, thrun_the_last_troll()).expect("the Troll is out");
    let bystander = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    let land = on_battlefield(&engine, p0, swamp()).expect("a Swamp is out");
    let (power, toughness) = pt(&engine, host);
    assert!(
        toughness >= 3,
        "the host has to live through -2/-2, or there is no projection left to \
         read: {power}/{toughness}"
    );
    assert_eq!(pt(&engine, bystander), (1, 1), "the Elf is a printed 1/1");

    let spell = in_hand(&engine, p0, enfeeblement()).expect("the Aura is in hand");
    cast_from_hand(&mut engine, p0, enfeeblement());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "an Aura picks its host as it is cast, got {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&host) && options.contains(&bystander),
        "\"Enchant creature\" is any creature, on either side of the table: {options:?}"
    );
    assert!(
        !options.contains(&land),
        "a Swamp is a land and no creature: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .expect("the host was one of the options it offered");

    pass_until(&mut engine, |e| {
        stack_is_empty(e)
            && e.state()
                .object(spell)
                .is_some_and(|o| o.attached_to == Some(host))
    });

    let aura =
        on_battlefield(&engine, p0, enfeeblement()).expect("the Aura resolved onto the table");
    assert_eq!(
        engine.state().object(aura).and_then(|o| o.attached_to),
        Some(host),
        "and it entered attached to the creature it was cast at"
    );
    assert_eq!(
        pt(&engine, host),
        (power - 2, toughness - 2),
        "the enchanted creature gets -2/-2 off the body the card prints"
    );
    assert_eq!(
        pt(&engine, bystander),
        (1, 1),
        "the static reaches the enchanted creature and never across the table"
    );
}
