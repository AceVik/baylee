//! `cards/instants/mv_3/prismatic_strands.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "8e414ad6-ba19-44a1-a291-2e420734a6dd"
fn prismatic_strands() -> CardIndex {
    card_index("8e414ad6-ba19-44a1-a291-2e420734a6dd")
}

/// Prismatic Strands ({2}{W}): prevents all damage (combat and noncombat) from
/// sources of the chosen color this turn while letting other colors deal
/// damage normally.
#[test]
fn prismatic_strands_prevents_all_damage_from_chosen_color_only() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), plains()])
        .hand(0, &[prismatic_strands()])
        .battlefield(1, &[mountain(), index::HILL_GIANT, index::GRIZZLY_BEARS])
        .hand(1, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, P1);

    let giant = lk::obj(&engine, P1, index::HILL_GIANT);
    let bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);

    // P0 casts Prismatic Strands choosing Red.
    engine.apply(P1, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, prismatic_strands());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseColor { .. })
    });
    engine
        .apply(P0, PlayerAction::ChooseColor(ManaColor::Red))
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    // Red Lightning Bolt deals 0 damage to P0.
    tap_all_mana(&mut engine, P1);
    lk::cast_at(&mut engine, P1, lightning_bolt(), &[], &[P0]);
    assert_eq!(lk::life(&engine, P0), 20, "red spell damage prevented");

    // Red Hill Giant deals 0 combat damage; green Grizzly Bears still deals 2.
    let _ = lk::attack(&mut engine, &[giant, bears]);
    lk::block(&mut engine, &[]);
    lk::through_combat(&mut engine);
    assert_eq!(
        lk::life(&engine, P0),
        18,
        "red Giant prevented; green Bears still dealt 2"
    );
}

/// Flashback—Tap an untapped white creature you control: non-white creatures
/// cannot pay it; tapping an untapped white creature casts it from the
/// graveyard for 0 mana, prevents chosen-color damage, and exiles it.
#[test]
fn prismatic_strands_flashback_taps_untapped_white_creature_and_exiles() {
    let mut engine = lk::game(
        SEED,
        &[index::SAVANNAH_LIONS, index::GRIZZLY_BEARS],
        &[],
        &[prismatic_strands(), lightning_bolt()],
        &[],
    );
    let lions = lk::obj(&engine, P0, index::SAVANNAH_LIONS);
    let bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);

    // Move Prismatic Strands directly into P0's graveyard after a normal cast
    // on turn 1, then advance to turn 3 so Lions is untapped and ready.
    lk::float(&mut engine, P0, &[(ManaColor::White, 3)]);
    cast_with_floating(&mut engine, P0, prismatic_strands());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseColor { .. })
    });
    engine
        .apply(P0, PlayerAction::ChooseColor(ManaColor::Blue))
        .unwrap();
    lk::settle(&mut engine);
    let gy_strands = in_graveyard(&engine, P0, prismatic_strands()).unwrap();

    reach_their_main_phase(&mut engine, P1);
    reach_their_main_phase(&mut engine, P0);

    // Cast via flashback with 0 mana floating by tapping white Savannah Lions.
    engine
        .apply(P0, PlayerAction::CastSpell { card: gy_strands })
        .expect("untapped white creature pays flashback cost");
    if matches!(engine.pending(), Pending::ChooseCastMode { .. }) {
        let fb = choose_cast_kind(&engine, CastModeKind::Flashback);
        engine.apply(P0, PlayerAction::ChooseMode(fb)).unwrap();
    }
    if let Pending::ChooseCards { options, .. } = engine.pending().clone() {
        assert_eq!(
            options,
            vec![lions],
            "only white creature offered (not {bears:?})"
        );
        engine
            .apply(
                P0,
                PlayerAction::ChooseObjects {
                    objects: vec![lions],
                },
            )
            .unwrap();
    }
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseColor { .. })
    });
    engine
        .apply(P0, PlayerAction::ChooseColor(ManaColor::Red))
        .unwrap();
    lk::settle(&mut engine);

    assert!(
        is_tapped(&engine, lions),
        "Savannah Lions tapped for flashback"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Exile(P0))
            .contains(&gy_strands),
        "flashed-back Prismatic Strands exiled"
    );

    lk::float(&mut engine, P0, &[(ManaColor::Red, 1)]);
    lk::cast_at(&mut engine, P0, lightning_bolt(), &[], &[P0]);
    assert_eq!(
        lk::life(&engine, P0),
        20,
        "flashback prevention blocked red Bolt"
    );
}
