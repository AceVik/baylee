//! `cards/lands/eclipsed_realms.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Eclipsed Realms: "As this land enters, choose Elemental, Elf, Faerie, Giant, Goblin, Kithkin, Merfolk, or Treefolk." / "{T}: Add {C}." / "{T}: Add one mana of any color..."
/// Under `Coverage::Partial`, the printed spend restriction and restricted list of eight creature types are omitted.
/// Playing this land asks for a subtype choice as it enters, and activating ability 1 adds one mana of any chosen color.
#[test]
fn eclipsed_realms_chooses_subtype_on_entry_and_produces_colored_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(308, forest())
        .hand(0, &[eclipsed_realms()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let card = in_hand(&engine, p0, eclipsed_realms()).expect("land in hand");
    engine.apply(p0, PlayerAction::PlayLand { card }).unwrap();

    let Pending::ChooseSubtype { player, options } = engine.pending().clone() else {
        panic!("expected subtype choice, got {:?}", engine.pending());
    };
    assert_eq!(player, p0);
    engine
        .apply(p0, PlayerAction::ChooseSubtype(options[0]))
        .unwrap();

    let land = on_battlefield(&engine, p0, eclipsed_realms()).expect("land on battlefield");
    assert!(!entered_tapped(&engine, land));

    activate(&mut engine, p0, eclipsed_realms(), 1);
    let Pending::ChooseColor { .. } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending());
    };
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .unwrap();

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.restricted().len(),
        1,
        "the any-colour mana is restricted to the chosen type, which is where \
         it has to land — an unrestricted blue here is the land this card was \
         until the restriction was written"
    );
    assert_eq!(pool.restricted()[0].color, ManaColor::Blue);
    assert_eq!(
        pool.available(ManaColor::Blue),
        0,
        "and not in the plain pool"
    );
    assert!(is_tapped(&engine, land));
}
