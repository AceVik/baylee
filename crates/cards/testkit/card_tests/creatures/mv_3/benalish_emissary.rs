//! `cards/creatures/mv_3/benalish_emissary.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

fn benalish_emissary() -> CardIndex {
    card_index("99f2e195-5da5-4239-87e0-ed552643e7b5")
}

/// Benalish Emissary ({2}{W} 1/4 Human Wizard):
/// "Kicker {1}{G}; When this creature enters, if it was kicked, destroy target land."
#[test]
fn benalish_emissary_destroys_target_land_only_when_kicked() {
    let mut e_base = lk::game(SEED, &[], &[plains()], &[benalish_emissary()], &[]);
    lk::float(
        &mut e_base,
        lk::P0,
        &[(ManaColor::White, 3), (ManaColor::Green, 2)],
    );
    cast_with_floating(&mut e_base, lk::P0, benalish_emissary());
    lk::yes_no(&mut e_base, false);
    lk::settle(&mut e_base);
    assert!(on_battlefield(&e_base, lk::P0, benalish_emissary()).is_some());
    assert!(on_battlefield(&e_base, lk::P1, plains()).is_some());

    let mut e = lk::game(
        SEED,
        &[],
        &[plains(), index::GRIZZLY_BEARS],
        &[benalish_emissary()],
        &[],
    );
    let their_land = lk::obj(&e, lk::P1, plains());
    let their_bear = lk::obj(&e, lk::P1, index::GRIZZLY_BEARS);
    lk::float(
        &mut e,
        lk::P0,
        &[(ManaColor::White, 3), (ManaColor::Green, 2)],
    );
    cast_with_floating(&mut e, lk::P0, benalish_emissary());
    lk::yes_no(&mut e, true);
    pass_until(&mut e, |eng| {
        matches!(eng.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { options, .. } = e.pending().clone() else {
        unreachable!()
    };
    assert!(options.contains(&their_land));
    assert!(!options.contains(&their_bear));
    lk::aim(&mut e, &[their_land], &[]);
    lk::settle(&mut e);

    assert!(on_battlefield(&e, lk::P1, plains()).is_none());
    assert!(in_graveyard(&e, lk::P1, plains()).is_some());
}
