//! `cards/lands/creatures/dryad_arbor.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Dryad Arbor prints no rules text at all — its whole card is reminder
/// text saying it is affected by summoning sickness and has "{T}: Add {G}".
///
/// So nothing in the card file can say whether the engine applies CR 302.6
/// to it: the sentence is true only if `summoning_sick` reads the
/// *projected* types, finds CREATURE on something that is also a LAND, and
/// both offer paths consult it. Two Arbors on one board answer that in one
/// priority — one seated before the game began, one played from hand this
/// turn, the same printing, both untapped, differing in nothing but when
/// they arrived.
///
/// Both lists are read, because this card is offered its {G} twice and the
/// two offers are gated in different functions: the intrinsic Forest tap
/// through `casting::can_activate_mana` into `LegalActions::mana_abilities`
/// (CR 305.6), and the printed `{T}: Add {G}` through `can_afford` into
/// `LegalActions::abilities`. A test that read one list would not see the
/// other going wrong. The pool-wide land-mana sweep skips this card by
/// name, calling it a question for a sickness test rather than a mana one;
/// this is that test.
#[test]
fn a_land_creature_played_this_turn_makes_no_mana_and_a_seated_one_does() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(59, forest())
        .battlefield(0, &[the_land_creature()])
        .hand(0, &[the_land_creature()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let preset = on_battlefield(&engine, p0, the_land_creature()).expect("the seated Arbor");
    let fresh = play_land(&mut engine, p0, the_land_creature());
    assert_ne!(preset, fresh, "two Arbors, not one object read twice");
    assert_eq!(
        all_on_battlefield(&engine, p0, the_land_creature()).len(),
        2,
        "the land drop put the second Arbor on the battlefield"
    );

    // The control, and the reason the two halves are one board: whatever
    // the engine withholds from the Arbor played this turn, it cannot be
    // withholding it for being tapped or for not being a creature.
    for id in [preset, fresh] {
        assert!(
            types(&engine, id).contains(TypeSet::CREATURE),
            "an Arbor is a creature"
        );
        assert!(
            types(&engine, id).contains(TypeSet::LAND),
            "and a land at the same time"
        );
        assert!(!is_tapped(&engine, id), "both Arbors stand untapped");
    }

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!(
            "a land drop hands priority back, got {:?}",
            engine.pending()
        )
    };
    assert!(
        legal.mana_abilities.contains(&preset),
        "the seated Arbor was refused the Forest tap CR 305.6 gives it"
    );
    assert!(
        legal.abilities.contains(&(preset, 0)),
        "and the {{T}}: Add {{G}} it actually prints"
    );
    assert!(
        !legal.mana_abilities.contains(&fresh),
        "an Arbor played this turn was offered the Forest tap anyway"
    );
    assert!(
        !legal.abilities.iter().any(|(id, _)| *id == fresh),
        "an Arbor played this turn was offered its printed {{T}} anyway"
    );

    // The enumeration and the validation read one predicate, so naming the
    // action the offer withheld has to be refused as well — otherwise a
    // client could reach past the list it was given.
    assert!(
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source: fresh })
            .is_err(),
        "the engine tapped a summoning-sick Arbor for its intrinsic mana"
    );
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: fresh,
                    ability_index: 0,
                },
            )
            .is_err(),
        "the engine tapped a summoning-sick Arbor for its printed mana"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "two refusals still made mana"
    );

    // The other half of the sentence: the same printing, seated since
    // before the turn began, pays its own {T} and adds the green.
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: preset,
                ability_index: 0,
            },
        )
        .expect("the seated Arbor taps for {G}");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 1, "one activation, one mana");
    assert_eq!(
        pool.available(ManaColor::Green),
        1,
        "and it is the colour the card prints"
    );
    assert!(is_tapped(&engine, preset), "paying {{T}} left it tapped");
    assert!(
        !is_tapped(&engine, fresh),
        "while the Arbor that could not be activated is still untapped"
    );
}
