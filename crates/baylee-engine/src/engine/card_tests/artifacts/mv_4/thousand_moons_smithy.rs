//! `cards/artifacts/mv_4/thousand_moons_smithy.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Thousand Moons Smithy` // `Barracks of the Thousand` (`Coverage::Partial`):
/// "When `Thousand Moons Smithy` enters, create a white Gnome Soldier artifact creature token
/// with 'This token's power and toughness are each equal to the number of artifacts and/or
/// creatures you control.' At the beginning of your first main phase, you may tap five untapped
/// artifacts and/or creatures you control. If you do, transform `Thousand Moons Smithy`.
/// // `{{T}}`: Add `{{W}}`. Whenever you cast an artifact or creature spell using mana produced
/// by `Barracks of the Thousand`, create a white Gnome Soldier artifact creature token …"
///
/// Under `Coverage::Partial`, the enter-trigger Gnome Soldier token, the first-main-phase
/// transform, and the mana-produced cast trigger are omitted, leaving the front face as a
/// `{2}{W}{W}` legendary artifact with no abilities. The test casts `Thousand Moons Smithy`
/// from hand, confirms that it enters as a legendary artifact on face 0 without creating tokens,
/// confirms that with mana floating `LegalActions::abilities` offers no abilities on it,
/// and confirms that it remains on face 0 in the subsequent turn's main phase.
#[test]
fn thousand_moons_smithy_casts_and_enters_as_legendary_artifact_without_token() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(305, plains())
        .battlefield(
            0,
            &[plains(), plains(), plains(), plains(), plains(), plains()],
        )
        .hand(0, &[thousand_moons_smithy()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, thousand_moons_smithy());
    pass_until(&mut engine, stack_is_empty);

    let smithy = on_battlefield(&engine, p0, thousand_moons_smithy())
        .expect("Thousand Moons Smithy on battlefield");
    assert_eq!(
        engine.state().object(smithy).map(|o| o.face_index),
        Some(0),
        "Smithy is on face 0"
    );

    let t = types(&engine, smithy);
    assert!(t.contains(TypeSet::ARTIFACT), "Smithy is an artifact");
    assert!(!t.contains(TypeSet::LAND), "Smithy is not a land");
    assert!(
        engine
            .state()
            .object(smithy)
            .expect("Smithy exists")
            .characteristics()
            .supertypes
            .contains(SupertypeSet::LEGENDARY),
        "Smithy is legendary"
    );

    // Under Coverage::Partial, the enter trigger token creation is omitted.
    assert!(
        tokens_of(&engine, p0).is_empty(),
        "under `Coverage::Partial` no Gnome Soldier token is created on entry"
    );

    // Two Plains remain untapped; float mana and verify no abilities on the front face.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "two Plains floated mana"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        !legal.abilities.iter().any(|(src, _)| *src == smithy),
        "front face offers no activated abilities with floating mana"
    );

    // Advance to the next turn's first main phase and verify no transform occurs.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    assert_eq!(
        engine.state().object(smithy).map(|o| o.face_index),
        Some(0),
        "Smithy remains on face 0 in the following turn"
    );
}
