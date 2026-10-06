//! `cards/lands/tapland/base_camp.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Base Camp prints `This land enters tapped.`, `{{T}}: Add {{C}}.`, and `{{T}}: Add
/// one mana of any color. Spend this mana only to cast a Cleric, Rogue, Warrior,
/// or Wizard spell or to activate an ability of a Cleric, Rogue, Warrior, or Wizard.`
///
/// Under `Coverage::Partial`, activating abilities of the party classes is omitted
/// from the spend restriction because a `ManaRestriction` names only spells on the
/// stack. Playing the land causes it to enter tapped. After advancing to the next turn,
/// activating ability 1 prompts for a color choice via `Pending::ChooseColor` and
/// places restricted mana into `pool.restricted()` rather than `pool.available()`.
#[test]
fn base_camp_enters_tapped_and_adds_restricted_party_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest()).hand(0, &[base_camp()]).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, base_camp());
    assert!(entered_tapped(&engine, land));

    reach_their_main_phase(&mut engine, PlayerId::new(1));
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, land));

    activate(&mut engine, p0, base_camp(), 1);
    let Pending::ChooseColor { .. } = engine.pending().clone() else {
        panic!("expected ChooseColor, got {:?}", engine.pending());
    };
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Red))
        .unwrap();

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.restricted().len(), 1);
    assert_eq!(pool.restricted()[0].amount, 1);
    assert_eq!(pool.restricted()[0].color, ManaColor::Red);
    assert_eq!(pool.available(ManaColor::Red), 0);
    assert_eq!(pool.total(), 1);
    assert!(is_tapped(&engine, land));
}
