//! `cards/sorceries/mv_4/reign_of_chaos.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn reign_of_chaos() -> CardIndex {
    card_index("436b5989-9b8c-4e03-a82e-046837c891ca")
}

fn savannah_lions() -> CardIndex {
    card_index("60ba93eb-39e6-4af2-9c66-cd38f72daff2")
}

fn mahamoti_djinn() -> CardIndex {
    card_index("c39ea5f9-6ec0-4697-897b-779e326754a7")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Reign of Chaos (436b5989-9b8c-4e03-a82e-046837c891ca) — {2}{R}{R} Sorcery:
/// Mode 1: "Destroy target Plains and target white creature."
#[test]
fn reign_of_chaos_first_mode_destroys_target_plains_and_target_white_creature() {
    let mut engine = lk::game(
        SEED,
        &[mountain(), mountain(), mountain(), mountain()],
        &[
            plains(),
            island(),
            savannah_lions(),
            mahamoti_djinn(),
            grizzly_bears(),
        ],
        &[reign_of_chaos()],
        &[],
    );
    let their_plains = lk::obj(&engine, P1, plains());
    let their_island = lk::obj(&engine, P1, island());
    let white_creature = lk::obj(&engine, P1, savannah_lions());
    let blue_creature = lk::obj(&engine, P1, mahamoti_djinn());
    let green_creature = lk::obj(&engine, P1, grizzly_bears());

    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, reign_of_chaos());
    let mode_zero = choose_cast_kind(&engine, CastModeKind::Mode(0));
    engine
        .apply(P0, PlayerAction::ChooseMode(mode_zero))
        .unwrap();

    let land_menu = aim_at(&mut engine, P0, their_plains);
    assert_eq!(land_menu, vec![their_plains]);
    assert!(!land_menu.contains(&their_island));

    let creature_menu = aim_at(&mut engine, P0, white_creature);
    assert_eq!(creature_menu, vec![white_creature]);
    assert!(!creature_menu.contains(&blue_creature) && !creature_menu.contains(&green_creature));

    lk::settle(&mut engine);
    assert_eq!(lk::zone_of(&engine, their_plains), Zone::Graveyard);
    assert_eq!(lk::zone_of(&engine, white_creature), Zone::Graveyard);
    assert_eq!(lk::zone_of(&engine, their_island), Zone::Battlefield);
    assert_eq!(lk::zone_of(&engine, blue_creature), Zone::Battlefield);
}

/// Reign of Chaos — Mode 2: "Destroy target Island and target blue creature."
#[test]
fn reign_of_chaos_second_mode_destroys_target_island_and_target_blue_creature() {
    let mut engine = lk::game(
        SEED,
        &[mountain(), mountain(), mountain(), mountain()],
        &[
            plains(),
            island(),
            savannah_lions(),
            mahamoti_djinn(),
            grizzly_bears(),
        ],
        &[reign_of_chaos()],
        &[],
    );
    let their_plains = lk::obj(&engine, P1, plains());
    let their_island = lk::obj(&engine, P1, island());
    let white_creature = lk::obj(&engine, P1, savannah_lions());
    let blue_creature = lk::obj(&engine, P1, mahamoti_djinn());

    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, reign_of_chaos());
    let mode_one = choose_cast_kind(&engine, CastModeKind::Mode(1));
    engine
        .apply(P0, PlayerAction::ChooseMode(mode_one))
        .unwrap();

    let land_menu = aim_at(&mut engine, P0, their_island);
    assert_eq!(land_menu, vec![their_island]);
    let creature_menu = aim_at(&mut engine, P0, blue_creature);
    assert_eq!(creature_menu, vec![blue_creature]);

    lk::settle(&mut engine);
    assert_eq!(lk::zone_of(&engine, their_island), Zone::Graveyard);
    assert_eq!(lk::zone_of(&engine, blue_creature), Zone::Graveyard);
    assert_eq!(lk::zone_of(&engine, their_plains), Zone::Battlefield);
    assert_eq!(lk::zone_of(&engine, white_creature), Zone::Battlefield);
}
