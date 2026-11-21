//! `cards/instants/mv_3/inspirit.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "90fd82e8-f638-4ba4-a03c-a30d46f93a39"
fn inspirit() -> CardIndex {
    card_index("90fd82e8-f638-4ba4-a03c-a30d46f93a39")
}

/// Inspirit ({2}{W}): untaps target creature and gives it +2/+4 until end of
/// turn, then the bonus wears off next turn.
#[test]
fn inspirit_untaps_target_creature_and_gives_plus_two_plus_four_until_eot() {
    let mut engine = lk::game(
        SEED,
        &[index::LLANOWAR_ELVES, index::GRIZZLY_BEARS],
        &[],
        &[inspirit()],
        &[],
    );
    let elf = lk::obj(&engine, P0, index::LLANOWAR_ELVES);
    let bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);

    // Tap the Elf for mana so we observe Inspirit untapping it.
    engine
        .apply(
            P0,
            PlayerAction::ActivateAbility {
                source: elf,
                ability_index: 0,
            },
        )
        .unwrap();
    assert!(is_tapped(&engine, elf));

    lk::float(&mut engine, P0, &[(ManaColor::White, 3)]);
    lk::cast_at(&mut engine, P0, inspirit(), &[elf], &[]);

    assert!(!is_tapped(&engine, elf), "Inspirit untapped the Elf");
    assert_eq!(pt(&engine, elf), (3, 5), "+2/+4 until end of turn");
    assert_eq!(pt(&engine, bears), (2, 2), "bystander untouched");

    reach_their_main_phase(&mut engine, P1);
    assert_eq!(pt(&engine, elf), (1, 1), "+2/+4 wears off at end of turn");
}
