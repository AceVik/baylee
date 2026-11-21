//! `cards/artifacts/mv_2/dream_chisel.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn dream_chisel() -> CardIndex {
    card_index("7776c114-c02a-4694-9b9d-00fbfad2d2a2")
}

fn nantuko_vigilante() -> CardIndex {
    card_index("28ea78dc-7ad6-4006-aaf8-4b25057ec019")
}

/// Dream Chisel (7776c114-c02a-4694-9b9d-00fbfad2d2a2) — {2} Artifact:
/// "Face-down creature spells you cast cost {1} less to cast."
///
/// Reduces your face-down Morph cast from {3} to {2}, does not reduce a
/// face-up cast ({3}{G} still unaffordable with 3 Forests), and does not
/// discount the opponent's face-down Morph cast.
#[test]
fn dream_chisel_discounts_your_face_down_casts_only() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[dream_chisel(), forest(), forest()])
        .hand(0, &[nantuko_vigilante()])
        .battlefield(1, &[forest(), forest()])
        .hand(1, &[nantuko_vigilante()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // 2 Forests suffice for p0 to cast Nantuko Vigilante face down ({3} -> {2}).
    let my_morph = in_hand(&engine, p0, nantuko_vigilante()).unwrap();
    tap_all_mana(&mut engine, p0);
    assert!(
        priority_offer(&engine).castable.contains(&my_morph),
        "Dream Chisel lowers face-down cast from {{3}} to {{2}}"
    );
    engine
        .apply(p0, PlayerAction::CastSpell { card: my_morph })
        .expect("cast Nantuko Vigilante face down for {2}");
    pass_until(&mut engine, stack_is_empty);

    let landed = on_battlefield(&engine, p0, nantuko_vigilante()).expect("face-down 2/2 landed");
    assert!(
        engine
            .state()
            .object(landed)
            .unwrap()
            .status
            .contains(baylee_engine::object::Status::FACE_DOWN),
        "it entered face down"
    );
    assert_eq!(pt(&engine, landed), (2, 2));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "exactly {{2}} was spent"
    );

    // Opponent with only 2 Forests does NOT get Dream Chisel's discount.
    reach_their_main_phase(&mut engine, p1);
    let their_morph = in_hand(&engine, p1, nantuko_vigilante()).unwrap();
    tap_all_mana(&mut engine, p1);
    assert!(
        !priority_offer(&engine).castable.contains(&their_morph),
        "Dream Chisel says 'you cast': opponent's face-down spell still costs {{3}}"
    );
}

/// Face-up creature spells are NOT discounted by Dream Chisel: with 3 Forests
/// and a {3}{G} creature (Nantuko Vigilante), only the face-down {2} cast is
/// affordable, leaving {1} in the pool and entering face down.
#[test]
fn dream_chisel_does_not_discount_face_up_creature_spells() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[dream_chisel(), forest(), forest(), forest()])
        .hand(0, &[nantuko_vigilante()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let vigilante = in_hand(&engine, p0, nantuko_vigilante()).unwrap();
    let offer = priority_offer(&engine);
    assert_eq!(
        offer.spell_reduction(vigilante, baylee_engine::choice::CastModeKind::Normal),
        0,
        "face-up cast gets zero reduction"
    );
    assert_eq!(
        offer.spell_reduction(vigilante, baylee_engine::choice::CastModeKind::Disguise),
        1,
        "face-down cast gets {{1}} reduction"
    );

    // Tap 3 Forests: {3}{G} face-up is still unaffordable, so CastSpell takes
    // the only legal mode (face down for {2}) and leaves 1 mana floating.
    tap_all_mana(&mut engine, p0);
    engine
        .apply(p0, PlayerAction::CastSpell { card: vigilante })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let landed = on_battlefield(&engine, p0, nantuko_vigilante()).unwrap();
    assert!(
        engine
            .state()
            .object(landed)
            .unwrap()
            .status
            .contains(baylee_engine::object::Status::FACE_DOWN),
        "only the face-down cast was legal with 3 mana"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "paid {{2}} out of 3 floating"
    );
}
