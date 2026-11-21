//! `cards/sorceries/mv_6/plague_spores.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn plague_spores() -> CardIndex {
    card_index("f79f5cb7-d238-41ab-85f0-eb21cc6bf561")
}

fn uthden_troll() -> CardIndex {
    card_index("d6329dcc-b450-482a-8ee3-45449f7a4b3d")
}

/// Plague Spores (f79f5cb7-d238-41ab-85f0-eb21cc6bf561) — {4}{B}{R} Sorcery:
/// "Destroy target nonblack creature and target land. They can't be
/// regenerated."
#[test]
fn plague_spores_destroys_nonblack_creature_and_land_through_regeneration() {
    let mut engine = lk::game(
        SEED,
        &[
            swamp(),
            mountain(),
            mountain(),
            mountain(),
            mountain(),
            mountain(),
            festering_goblin(),
        ],
        &[mountain(), island(), uthden_troll(), lotleth_troll()],
        &[plague_spores()],
        &[],
    );
    let my_black = lk::obj(&engine, P0, festering_goblin());
    let their_mtn = lk::obj(&engine, P1, mountain());
    let their_island = lk::obj(&engine, P1, island());
    let their_red_troll = lk::obj(&engine, P1, uthden_troll());
    let their_black_green = lk::obj(&engine, P1, lotleth_troll());

    // Give Uthden Troll a live regeneration shield ({R}: Regenerate, index 0).
    cast_from_hand(&mut engine, P0, plague_spores());
    let creature_menu = aim_at(&mut engine, P0, their_red_troll);
    assert_eq!(creature_menu, vec![their_red_troll]);
    assert!(!creature_menu.contains(&my_black) && !creature_menu.contains(&their_black_green));

    let land_menu = aim_at(&mut engine, P0, their_island);
    assert!(land_menu.contains(&their_island) && land_menu.contains(&their_mtn));
    assert!(!land_menu.contains(&their_red_troll));

    lk::pass_once(&mut engine);
    tap_mana_where(&mut engine, P1, |id| id == their_mtn);
    engine
        .apply(
            P1,
            PlayerAction::ActivateAbility {
                source: their_red_troll,
                ability_index: 0,
            },
        )
        .unwrap();
    lk::settle(&mut engine);

    assert_eq!(
        lk::zone_of(&engine, their_red_troll),
        Zone::Graveyard,
        "destroyed through its regeneration shield"
    );
    assert_eq!(lk::zone_of(&engine, their_island), Zone::Graveyard);
    assert_eq!(lk::zone_of(&engine, their_mtn), Zone::Battlefield);
}
