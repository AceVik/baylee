//! `cards/creatures/mv_3/verduran_emissary.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

fn verduran_emissary() -> CardIndex {
    card_index("915bd598-c3dd-4982-bdcc-5904a6f17436")
}

/// Verduran Emissary ({2}{G} 2/3 Human Wizard):
/// "Kicker {1}{R}; When this creature enters, if it was kicked, destroy target
/// artifact. It can't be regenerated."
#[test]
fn verduran_emissary_kicked_destroys_target_artifact_through_regeneration() {
    let mut e_base = lk::game(SEED, &[], &[sol_ring()], &[verduran_emissary()], &[]);
    lk::float(
        &mut e_base,
        lk::P0,
        &[(ManaColor::Green, 3), (ManaColor::Red, 2)],
    );
    cast_with_floating(&mut e_base, lk::P0, verduran_emissary());
    lk::yes_no(&mut e_base, false);
    lk::settle(&mut e_base);
    assert!(on_battlefield(&e_base, lk::P1, sol_ring()).is_some());

    let mut e = lk::game(
        SEED,
        &[],
        &[sol_ring(), index::WELDING_JAR, index::GRIZZLY_BEARS],
        &[verduran_emissary()],
        &[],
    );
    let ring = lk::obj(&e, lk::P1, sol_ring());
    let bear = lk::obj(&e, lk::P1, index::GRIZZLY_BEARS);

    // Put a regeneration shield on Sol Ring via Welding Jar, then cast Verduran Emissary kicked.
    e.apply(lk::P0, PlayerAction::PassPriority).unwrap();
    lk::use_ability(&mut e, lk::P1, index::WELDING_JAR, 0, &[ring], &[]);

    lk::float(
        &mut e,
        lk::P0,
        &[(ManaColor::Green, 3), (ManaColor::Red, 2)],
    );
    cast_with_floating(&mut e, lk::P0, verduran_emissary());
    lk::yes_no(&mut e, true);
    pass_until(&mut e, |eng| {
        matches!(eng.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { options, .. } = e.pending().clone() else {
        unreachable!()
    };
    assert!(options.contains(&ring));
    assert!(!options.contains(&bear));
    lk::aim(&mut e, &[ring], &[]);
    lk::settle(&mut e);

    assert!(in_graveyard(&e, lk::P1, sol_ring()).is_some());
}
