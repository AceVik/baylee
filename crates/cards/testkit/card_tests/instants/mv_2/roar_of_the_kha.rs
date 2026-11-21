//! `cards/instants/mv_2/roar_of_the_kha.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "9dddb69a-00a3-448b-8f8b-8e50cdd70d57"
fn roar_of_the_kha() -> CardIndex {
    card_index("9dddb69a-00a3-448b-8f8b-8e50cdd70d57")
}

/// Roar of the Kha ({1}{W}, Entwine {1}{W}): Mode 1 pumps only your creatures
/// +1/+1 EOT without untapping; Mode 2 untaps only your creatures without
/// pumping; Entwine does both and spends {2}{W}{W}.
#[test]
fn roar_of_the_kha_plays_mode_one_mode_two_and_entwine() {
    let mut engine = lk::game(
        SEED,
        &[index::LLANOWAR_ELVES, index::GRIZZLY_BEARS],
        &[index::GRIZZLY_BEARS],
        &[roar_of_the_kha(), roar_of_the_kha(), roar_of_the_kha()],
        &[],
    );
    let my_elf = lk::obj(&engine, P0, index::LLANOWAR_ELVES);
    let my_bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);
    let their_bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);

    // Tap P0's Elf so untapping is observable.
    engine
        .apply(
            P0,
            PlayerAction::ActivateAbility {
                source: my_elf,
                ability_index: 0,
            },
        )
        .unwrap();
    assert!(is_tapped(&engine, my_elf));

    // Mode 0: +1/+1 EOT only (Elf stays tapped).
    lk::float(&mut engine, P0, &[(ManaColor::White, 2)]);
    cast_with_floating(&mut engine, P0, roar_of_the_kha());
    let m0 = choose_cast_kind(&engine, CastModeKind::Mode(0));
    engine.apply(P0, PlayerAction::ChooseMode(m0)).unwrap();
    lk::settle(&mut engine);
    assert_eq!(pt(&engine, my_bears), (3, 3));
    assert_eq!(pt(&engine, their_bears), (2, 2));
    assert!(is_tapped(&engine, my_elf), "Mode 0 does not untap");

    // Mode 1: untap all your creatures only (no further pump).
    lk::float(&mut engine, P0, &[(ManaColor::White, 2)]);
    cast_with_floating(&mut engine, P0, roar_of_the_kha());
    let m1 = choose_cast_kind(&engine, CastModeKind::Mode(1));
    engine.apply(P0, PlayerAction::ChooseMode(m1)).unwrap();
    lk::settle(&mut engine);
    assert!(!is_tapped(&engine, my_elf), "Mode 1 untapped Elf");
    assert_eq!(pt(&engine, my_bears), (3, 3), "Mode 1 did not pump");

    // Tap Elf again, then Entwine ({2}{W}{W}) for both untap AND +1/+1.
    engine
        .apply(
            P0,
            PlayerAction::ActivateAbility {
                source: my_elf,
                ability_index: 0,
            },
        )
        .unwrap();
    lk::float(&mut engine, P0, &[(ManaColor::White, 4)]);
    cast_with_floating(&mut engine, P0, roar_of_the_kha());
    let both = choose_cast_kind(&engine, CastModeKind::Modes(3));
    engine.apply(P0, PlayerAction::ChooseMode(both)).unwrap();
    lk::settle(&mut engine);
    assert!(!is_tapped(&engine, my_elf), "Entwine untapped Elf");
    assert_eq!(pt(&engine, my_bears), (4, 4), "Entwine added another +1/+1");

    reach_their_main_phase(&mut engine, P1);
    assert_eq!(pt(&engine, my_bears), (2, 2), "all +1/+1 bonuses wore off");
}
