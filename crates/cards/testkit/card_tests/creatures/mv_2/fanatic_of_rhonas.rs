//! `cards/creatures/mv_2/fanatic_of_rhonas.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Fanatic of Rhonas`: "`{{T}}`: Add `{{G}}`. Ferocious — `{{T}}`: Add
/// `{{G}}{{G}}{{G}}{{G}}`. Activate only if you control a creature with power 4 or greater.
/// Eternalize `{{2}}{{G}}{{G}}`."
///
/// The first mana ability; ferocious and eternalize are the two tests after this one.
/// With no creature of power 4 or greater the ferocious ability is closed, so the test
/// verifies that `Fanatic of Rhonas` has 1/4
/// stats, that `tap_all_mana` taps it for exactly one green mana, and that it is tapped after
/// producing mana.
#[test]
fn fanatic_of_rhonas_taps_for_one_green_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(307, forest())
        .battlefield(0, &[fanatic_of_rhonas()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let snake =
        on_battlefield(&engine, p0, fanatic_of_rhonas()).expect("Fanatic of Rhonas on battlefield");
    assert_eq!(pt(&engine, snake), (1, 4), "Fanatic is a 1/4 creature");
    assert!(!is_tapped(&engine, snake), "Fanatic starts untapped");

    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "mana pool starts empty"
    );

    let taken = tap_all_mana(&mut engine, p0);
    assert_eq!(taken, 1, "tapped exactly one mana route");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "produced exactly one mana"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1,
        "produced green mana"
    );
    assert!(
        is_tapped(&engine, snake),
        "Fanatic is tapped after activating its mana ability"
    );
}

/// Fanatic of Rhonas's eternalize (CR 702.129a): "{2}{G}{G}, Exile this card
/// from your graveyard: Create a token that's a copy of it, except it's a
/// 4/4 black Zombie Snake Druid with no mana cost. Eternalize only as a
/// sorcery."
///
/// Offered from the graveyard in the owner's main phase, and not while a
/// spell is on the stack. Paid with four mana and the card, which goes to
/// exile. The token is a copy (the card's name and its three abilities)
/// that is a 4/4, black and no other colour, a Zombie Snake Druid, with
/// mana value 0.
#[test]
fn fanatic_of_rhonas_eternalizes_into_a_black_four_four_zombie() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(398, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest(), forest()])
        .hand(0, &[fanatic_of_rhonas(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let card = hand_to_graveyard(&mut engine, p0, fanatic_of_rhonas());
    // Ability 2: the two mana abilities come first, in printed order.
    let eternalize = (card, 2);
    assert!(
        !priority_offer(&engine).abilities.contains(&eternalize),
        "an empty pool pays no {{2}}{{G}}{{G}}"
    );

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, llanowar_elves());
    assert!(
        !priority_offer(&engine).abilities.contains(&eternalize),
        "\"only as a sorcery\": not with the Elves on the stack"
    );
    pass_until(&mut engine, stack_is_empty);
    assert!(
        priority_offer(&engine).abilities.contains(&eternalize),
        "from the graveyard, in a main phase with the stack empty"
    );

    let before: Vec<ObjectId> = engine.state().zones.list(ZoneLocation::Battlefield).clone();
    activate(&mut engine, p0, fanatic_of_rhonas(), 2);
    assert_eq!(
        engine.state().object(card).map(|o| o.zone),
        Some(Zone::Exile),
        "\"exile this card from your graveyard\" is the cost"
    );
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    pass_until(&mut engine, stack_is_empty);

    let token = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .find(|id| !before.contains(id))
        .expect("a token arrived");
    let obj = engine.state().object(token).expect("the token");
    let chars = obj.characteristics();
    assert!(obj.card.is_none(), "a token");
    assert_eq!(engine.state().names.get(chars.name), "Fanatic of Rhonas");
    assert_eq!(pt(&engine, token), (4, 4));
    assert_eq!(
        chars.colors,
        baylee_core::color::ColorSet::from_slice(&[baylee_core::color::Color::Black]),
        "black, and no longer green"
    );
    for subtype in [
        baylee_core::generated::subtypes::creature::ZOMBIE,
        baylee_core::generated::subtypes::creature::SNAKE,
        baylee_core::generated::subtypes::creature::DRUID,
    ] {
        assert!(chars.subtypes.contains(subtype), "a Zombie Snake Druid");
    }
    assert_eq!(chars.mana_cost.cmc(), 0, "no mana cost");
    assert_eq!(
        obj.abilities(&engine.lookup).len(),
        3,
        "the card's abilities come with the copy"
    );
}
