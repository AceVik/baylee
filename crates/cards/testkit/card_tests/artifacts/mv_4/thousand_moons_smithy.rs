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
/// Entry creates its own artifact-creature Gnome, which counts itself and
/// Smithy once each. Plains do not contribute to the count or optional tap-five.
/// The front has no activated ability.
#[test]
fn thousand_moons_smithy_creates_self_counting_gnome_on_entry() {
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

    let tokens = tokens_of(&engine, p0);
    assert_eq!(tokens.len(), 1);
    assert_eq!(
        pt(&engine, tokens[0]),
        (2, 2),
        "Smithy and artifact-creature token count once each"
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
