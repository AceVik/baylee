//! `cards/creatures/mv_3/shivan_emissary.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

fn shivan_emissary() -> CardIndex {
    card_index("1b885600-def1-4b2c-a6cb-02df34d58a1b")
}

/// Shivan Emissary ({2}{R} 1/1 Human Wizard):
/// "Kicker {1}{B}; When this creature enters, if it was kicked, destroy target
/// nonblack creature. It can't be regenerated."
#[test]
fn shivan_emissary_kicked_destroys_nonblack_creature_without_regeneration() {
    let mut e_base = lk::game(SEED, &[], &[pearled_unicorn()], &[shivan_emissary()], &[]);
    lk::float(
        &mut e_base,
        lk::P0,
        &[(ManaColor::Red, 3), (ManaColor::Black, 2)],
    );
    cast_with_floating(&mut e_base, lk::P0, shivan_emissary());
    lk::yes_no(&mut e_base, false);
    lk::settle(&mut e_base);
    assert!(on_battlefield(&e_base, lk::P1, pearled_unicorn()).is_some());

    let mut e = lk::game(
        SEED,
        &[],
        &[index::UTHDEN_TROLL, index::DRUDGE_SKELETONS],
        &[shivan_emissary()],
        &[],
    );
    let troll = lk::obj(&e, lk::P1, index::UTHDEN_TROLL);
    let skel = lk::obj(&e, lk::P1, index::DRUDGE_SKELETONS);

    // Put a regeneration shield on Uthden Troll first, then cast Shivan Emissary kicked.
    e.apply(lk::P0, PlayerAction::PassPriority).unwrap();
    lk::float(&mut e, lk::P1, &[(ManaColor::Red, 1)]);
    lk::use_ability(&mut e, lk::P1, index::UTHDEN_TROLL, 0, &[], &[]);

    lk::float(
        &mut e,
        lk::P0,
        &[(ManaColor::Red, 3), (ManaColor::Black, 2)],
    );
    cast_with_floating(&mut e, lk::P0, shivan_emissary());
    lk::yes_no(&mut e, true);
    pass_until(&mut e, |eng| {
        matches!(eng.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { options, .. } = e.pending().clone() else {
        unreachable!()
    };
    assert!(options.contains(&troll));
    assert!(!options.contains(&skel));
    lk::aim(&mut e, &[troll], &[]);
    lk::settle(&mut e);

    assert!(in_graveyard(&e, lk::P1, index::UTHDEN_TROLL).is_some());
    assert!(on_battlefield(&e, lk::P1, index::DRUDGE_SKELETONS).is_some());
}
