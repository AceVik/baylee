//! `cards/creatures/mv_3/tolarian_emissary.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

fn tolarian_emissary() -> CardIndex {
    card_index("8042ade2-f589-4c3e-a461-a4c284a67ee4")
}

/// Tolarian Emissary ({2}{U} 1/2 Human Wizard):
/// "Kicker {1}{W}; Flying; When this creature enters, if it was kicked, destroy
/// target enchantment."
#[test]
fn tolarian_emissary_destroys_enchantment_only_when_kicked_and_has_flying() {
    let mut e_base = lk::game(
        SEED,
        &[],
        &[their_enchantment()],
        &[tolarian_emissary()],
        &[],
    );
    lk::float(
        &mut e_base,
        lk::P0,
        &[(ManaColor::Blue, 3), (ManaColor::White, 2)],
    );
    cast_with_floating(&mut e_base, lk::P0, tolarian_emissary());
    lk::yes_no(&mut e_base, false);
    lk::settle(&mut e_base);
    assert!(on_battlefield(&e_base, lk::P1, their_enchantment()).is_some());

    let mut e = lk::game(
        SEED,
        &[],
        &[their_enchantment(), index::GRIZZLY_BEARS],
        &[tolarian_emissary()],
        &[],
    );
    let ench = lk::obj(&e, lk::P1, their_enchantment());
    let bear = lk::obj(&e, lk::P1, index::GRIZZLY_BEARS);
    lk::float(
        &mut e,
        lk::P0,
        &[(ManaColor::Blue, 3), (ManaColor::White, 2)],
    );
    cast_with_floating(&mut e, lk::P0, tolarian_emissary());
    lk::yes_no(&mut e, true);
    pass_until(&mut e, |eng| {
        matches!(eng.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { options, .. } = e.pending().clone() else {
        unreachable!()
    };
    assert_eq!(options, vec![ench]);
    lk::aim(&mut e, &[ench], &[]);
    lk::settle(&mut e);

    assert!(in_graveyard(&e, lk::P1, their_enchantment()).is_some());
    let emissary = lk::obj(&e, lk::P0, tolarian_emissary());

    reach_their_main_phase(&mut e, lk::P1);
    reach_their_main_phase(&mut e, lk::P0);
    let offer = lk::attack(&mut e, &[emissary]);
    assert!(!lk::may_block(&offer, bear, emissary));
    lk::block(&mut e, &[]);
    lk::through_combat(&mut e);
    assert_eq!(lk::life(&e, lk::P1), 19);
}
