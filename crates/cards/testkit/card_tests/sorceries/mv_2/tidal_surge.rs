//! `cards/sorceries/mv_2/tidal_surge.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn tidal_surge() -> CardIndex {
    card_index("be738992-77fe-498d-b219-e5da4ce5bf07")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn serra_angel() -> CardIndex {
    card_index("4b7ac066-e5c7-43e6-9e7e-2739b24a905d")
}

/// Tidal Surge (be738992-77fe-498d-b219-e5da4ce5bf07) — {1}{U} Sorcery:
/// "Tap up to three target creatures without flying." Excludes creatures with
/// flying from the target menu and taps up to three chosen nonflying creatures.
#[test]
fn tidal_surge_taps_up_to_three_nonflying_creatures_and_excludes_flyers() {
    let mut engine = lk::game(
        SEED,
        &[island(), island(), llanowar_elves()],
        &[grizzly_bears(), aurochs(), fangren_hunter(), serra_angel()],
        &[tidal_surge()],
        &[],
    );
    let my_elf = lk::obj(&engine, P0, llanowar_elves());
    let bears = lk::obj(&engine, P1, grizzly_bears());
    let beast = lk::obj(&engine, P1, aurochs());
    let hunter = lk::obj(&engine, P1, fangren_hunter());
    let angel = lk::obj(&engine, P1, serra_angel());

    tap_mana_except(&mut engine, P0, my_elf);
    cast_with_floating(&mut engine, P0, tidal_surge());
    let Pending::ChooseTargets {
        options, min, max, ..
    } = engine.pending().clone()
    else {
        panic!("expected ChooseTargets, got {:?}", engine.pending());
    };
    assert_eq!((min, max), (0, 3), "up to three targets");
    assert!(options.contains(&my_elf));
    assert!(options.contains(&bears) && options.contains(&beast) && options.contains(&hunter));
    assert!(
        !options.contains(&angel),
        "creatures with flying are excluded"
    );

    lk::aim(&mut engine, &[bears, beast, hunter], &[]);
    lk::settle(&mut engine);

    assert!(lk::tapped(&engine, bears));
    assert!(lk::tapped(&engine, beast));
    assert!(lk::tapped(&engine, hunter));
    assert!(!lk::tapped(&engine, my_elf), "unchosen fourth creature");
    assert!(!lk::tapped(&engine, angel), "flying creature untouched");
}

/// Tidal Surge — "up to three": choosing zero targets is legal.
#[test]
fn tidal_surge_may_resolve_with_zero_targets_chosen() {
    let mut engine = lk::game(
        SEED,
        &[island(), island()],
        &[grizzly_bears()],
        &[tidal_surge()],
        &[],
    );
    let bears = lk::obj(&engine, P1, grizzly_bears());

    cast_from_hand(&mut engine, P0, tidal_surge());
    lk::aim(&mut engine, &[], &[]);
    lk::settle(&mut engine);

    assert!(!lk::tapped(&engine, bears));
    assert!(in_graveyard(&engine, P0, tidal_surge()).is_some());
}
