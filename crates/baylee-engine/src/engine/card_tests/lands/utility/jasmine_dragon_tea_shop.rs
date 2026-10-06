//! `cards/lands/utility/jasmine_dragon_tea_shop.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Jasmine Dragon Tea Shop` is a utility land under `Coverage::Implemented`.
/// It prints "{T}: Add {C}." and "{T}: Add one mana of any color. Spend this mana only to cast an Ally spell or activate an ability of an Ally source."
/// Activating its second ability prompts with `Pending::ChooseColor` across all five colors,
/// adding restricted mana that lives in `mana_pool.restricted()` rather than the unrestricted pool.
#[test]
fn jasmine_dragon_tea_shop_produces_restricted_ally_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[jasmine_dragon_tea_shop()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let shop = on_battlefield(&engine, p0, jasmine_dragon_tea_shop())
        .expect("Jasmine Dragon Tea Shop is on the battlefield");

    activate(&mut engine, p0, jasmine_dragon_tea_shop(), 1);

    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!(
            "expected ChooseColor prompt for Jasmine Dragon Tea Shop, got {:?}",
            engine.pending()
        );
    };
    assert_eq!(options.len(), 5, "all five colors are offered");

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::White))
        .unwrap();

    let restricted = engine.state().players[0].mana_pool.restricted().to_vec();
    assert_eq!(
        restricted
            .iter()
            .filter(|m| m.color == ManaColor::White)
            .map(|m| u32::from(m.amount))
            .sum::<u32>(),
        1,
        "produced one restricted white mana"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::White),
        0,
        "restricted mana is not in the unrestricted pool"
    );
    assert!(is_tapped(&engine, shop), "tea shop is tapped");
}
