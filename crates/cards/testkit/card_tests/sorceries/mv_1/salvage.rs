//! `cards/sorceries/mv_1/salvage.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn salvage() -> CardIndex {
    card_index("c01a090a-11e4-469e-aded-74203c06fef4")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Salvage (c01a090a-11e4-469e-aded-74203c06fef4) — {G} Sorcery:
/// "Put target card from your graveyard on top of your library."
#[test]
fn salvage_puts_target_card_from_your_graveyard_on_top_of_your_library() {
    let mut engine = lk::game(
        SEED,
        &[forest()],
        &[],
        &[salvage(), llanowar_elves(), counterspell()],
        &[grizzly_bears()],
    );
    let my_elf = hand_to_graveyard(&mut engine, P0, llanowar_elves());
    let my_spell = hand_to_graveyard(&mut engine, P0, counterspell());
    let their_bears = hand_to_graveyard(&mut engine, P1, grizzly_bears());

    cast_from_hand(&mut engine, P0, salvage());
    let menu = aim_at(&mut engine, P0, my_spell);
    assert!(menu.contains(&my_elf) && menu.contains(&my_spell));
    assert!(
        !menu.contains(&their_bears),
        "cards in an opponent's graveyard are not legal targets"
    );
    lk::settle(&mut engine);

    assert_eq!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Library(P0))
            .last()
            .copied(),
        Some(my_spell),
        "target card moved from graveyard to top of library"
    );
    assert_eq!(lk::zone_of(&engine, my_elf), Zone::Graveyard);
    assert!(in_graveyard(&engine, P0, salvage()).is_some());
}

/// Salvage — requires a target in your graveyard, so it cannot be cast when
/// your graveyard is empty even if the opponent's graveyard has cards.
#[test]
fn salvage_cannot_be_cast_when_your_graveyard_is_empty() {
    let mut engine = lk::game(SEED, &[forest()], &[], &[salvage()], &[grizzly_bears()]);
    hand_to_graveyard(&mut engine, P1, grizzly_bears());
    tap_all_mana(&mut engine, P0);

    assert!(!lk::castable(&engine, P0, salvage()));
}
