//! Sorceries of the Arabian Nights / Antiquities / Legends pass (45c5bbd7),
//! each played from its Oracle text.

#[allow(clippy::wildcard_imports)]
use super::super::legends_kit::*;
#[allow(clippy::wildcard_imports)]
use super::*;

/// Energy Tap: "Tap target untapped creature you control. If you do, add an
/// amount of {C} equal to that creature's mana value."
#[test]
fn energy_tap_taps_a_creature_for_its_mana_value() {
    let mut e = game(5100, &[ids::moss_monster()], &[], &[ids::energy_tap()], &[]);
    let moss = obj(&e, P0, ids::moss_monster());
    float(&mut e, P0, &[(ManaColor::Blue, 1)]);
    cast_modal(&mut e, P0, ids::energy_tap(), 0, 0, &[moss], &[]);
    assert!(tapped(&e, moss));
    assert_eq!(pool_of(&e, P0, ManaColor::Colorless), 5, "mana value 5");
}

/// Part Water: "X target creatures gain islandwalk until end of turn."
#[test]
fn part_water_gives_the_target_islandwalk() {
    let mut e = game(
        5200,
        &[ids::barbary_apes()],
        &[ids::moss_monster(), island()],
        &[ids::part_water()],
        &[],
    );
    let apes = obj(&e, P0, ids::barbary_apes());
    float(
        &mut e,
        P0,
        &[(ManaColor::Blue, 1), (ManaColor::Colorless, 2)],
    );
    cast_modal(&mut e, P0, ids::part_water(), 0, 1, &[apes], &[]);
    assert!(keywords(&e, apes).contains(KeywordSet::ISLANDWALK));
    let offer = attack(&mut e, &[apes]);
    assert!(
        !may_block(&offer, obj(&e, P1, ids::moss_monster()), apes),
        "the defender controls an Island"
    );
}

/// Acid Rain: "Destroy all Forests."
#[test]
fn acid_rain_destroys_every_forest() {
    let mut e = game(
        5300,
        &[forest(), island()],
        &[forest(), plains()],
        &[ids::acid_rain()],
        &[],
    );
    float(
        &mut e,
        P0,
        &[(ManaColor::Blue, 1), (ManaColor::Colorless, 3)],
    );
    cast_modal(&mut e, P0, ids::acid_rain(), 0, 0, &[], &[]);
    assert!(on_battlefield(&e, P0, forest()).is_none());
    assert!(on_battlefield(&e, P1, forest()).is_none());
    assert!(on_battlefield(&e, P0, island()).is_some());
    assert!(on_battlefield(&e, P1, plains()).is_some());
}

/// Cleanse: "Destroy all black creatures."
#[test]
fn cleanse_destroys_every_black_creature() {
    let mut e = game(
        5400,
        &[ids::vampire_bats(), ids::barbary_apes()],
        &[ids::headless_horseman()],
        &[ids::cleanse()],
        &[],
    );
    float(
        &mut e,
        P0,
        &[(ManaColor::White, 2), (ManaColor::Colorless, 2)],
    );
    cast_modal(&mut e, P0, ids::cleanse(), 0, 0, &[], &[]);
    assert!(on_battlefield(&e, P0, ids::vampire_bats()).is_none());
    assert!(on_battlefield(&e, P1, ids::headless_horseman()).is_none());
    assert!(on_battlefield(&e, P0, ids::barbary_apes()).is_some());
}
