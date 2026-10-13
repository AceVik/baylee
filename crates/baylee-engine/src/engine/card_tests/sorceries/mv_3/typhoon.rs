//! `cards/sorceries/mv_3/typhoon.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::engine::card_tests::partials_w1b_kit as kit;

// oracle_id = "ef295a34-0325-49dd-87a6-546dde395082"
fn typhoon() -> CardIndex {
    card_index("ef295a34-0325-49dd-87a6-546dde395082")
}

// oracle_id = "74b7fe23-5d3a-4092-8d78-7c0eba8f6f73"
fn tropical_island() -> CardIndex {
    card_index("74b7fe23-5d3a-4092-8d78-7c0eba8f6f73")
}

// oracle_id = "c718911c-c955-4eb9-9e16-be4bd49a4e4e"
fn volcanic_island() -> CardIndex {
    card_index("c718911c-c955-4eb9-9e16-be4bd49a4e4e")
}

/// Typhoon: "Typhoon deals damage to each opponent equal to the number of
/// Islands that player controls."
///
/// Three players, so that "that player" has two different answers on one
/// resolution: player 1 controls two Islands and player 2 controls four. Each
/// opponent is dealt their own count, and the caster — who controls three
/// Islands themselves, tapped for the mana the spell cost — is dealt nothing,
/// because the caster is nobody's opponent.
#[test]
fn typhoon_deals_each_opponent_damage_equal_to_the_islands_that_player_controls() {
    let (p0, p1, p2) = (kit::P0, kit::P1, kit::P2);
    let mut engine = Duel::table(SEED, forest(), 3)
        .battlefield(
            0,
            &[forest(), forest(), forest(), island(), island(), island()],
        )
        .hand(0, &[typhoon()])
        .battlefield(1, &[island(), island(), forest()])
        .battlefield(2, &[island(), island(), island(), island(), swamp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, typhoon());
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        kit::life(&engine, p1),
        18,
        "two Islands, two damage: the Forest is no Island"
    );
    assert_eq!(
        kit::life(&engine, p2),
        16,
        "four Islands, four damage: the Swamp is no Island"
    );
    assert_eq!(
        kit::life(&engine, p0),
        20,
        "the caster controls three Islands and is not an opponent"
    );
    assert!(
        in_graveyard(&engine, p0, typhoon()).is_some(),
        "the sorcery is spent"
    );
}

/// "…the number of Islands that player controls": a land is an Island by its
/// subtype, not by its name, so the dual lands that print Island count; and a
/// player who controls none is dealt nothing.
///
/// Player 1 controls Tropical Island (Forest Island) and Volcanic Island
/// (Island Mountain) beside a Forest and a Mountain: two Islands, two damage.
/// Player 2 controls three Swamps and no Island at all.
#[test]
fn typhoon_counts_every_land_with_the_island_subtype_and_nothing_else() {
    let (p0, p1, p2) = (kit::P0, kit::P1, kit::P2);
    let mut engine = Duel::table(SEED, forest(), 3)
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[typhoon()])
        .battlefield(
            1,
            &[tropical_island(), volcanic_island(), forest(), mountain()],
        )
        .battlefield(2, &[swamp(), swamp(), swamp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, typhoon());
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        kit::life(&engine, p1),
        18,
        "the two dual lands are Islands, the Forest and the Mountain are not"
    );
    assert_eq!(kit::life(&engine, p2), 20, "no Island, no damage");
}

/// Ruling 2004-10-04: "Number of Islands is counted on resolution and not on
/// announcement."
///
/// Player 1 controls three Islands when Typhoon is cast and answers it by
/// returning one of them to their hand with Boomerang. On resolution they
/// control two, so they are dealt two, not three.
#[test]
fn typhoon_counts_the_islands_when_it_resolves_not_when_it_is_cast() {
    let (p0, p1, p2) = (kit::P0, kit::P1, kit::P2);
    let mut engine = Duel::table(SEED, forest(), 3)
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[typhoon()])
        .battlefield(1, &[island(), island(), island(), forest()])
        .hand(1, &[boomerang()])
        .battlefield(2, &[island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let theirs = all_on_battlefield(&engine, p1, island());
    assert_eq!(theirs.len(), 3, "three Islands when the spell is cast");

    cast_from_hand(&mut engine, p0, typhoon());
    engine
        .apply(p0, PlayerAction::PassPriority)
        .expect("the caster passes with Typhoon on the stack");
    let Pending::Priority { player, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert_eq!(player, p1, "the next player in turn order may respond");

    cast_from_hand(&mut engine, p1, boomerang());
    aim_at(&mut engine, p1, theirs[0]);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        all_on_battlefield(&engine, p1, island()).len(),
        2,
        "Boomerang took one Island home"
    );
    assert_eq!(
        kit::life(&engine, p1),
        18,
        "two damage for the two Islands left on resolution, not three for the \
         three there were on announcement"
    );
    assert_eq!(
        kit::life(&engine, p2),
        19,
        "player 2's one Island, one damage"
    );
    assert_eq!(kit::life(&engine, p0), 20, "the caster is untouched");
}

/// "…damage to each opponent": a teammate is not an opponent (CR 102.3), so
/// at a table with sides Typhoon passes over the caster's partner however
/// many Islands they control, and still counts the real opponent's own.
#[test]
fn typhoon_does_not_deal_damage_to_a_teammate() {
    let (p0, p1, p2) = (kit::P0, kit::P1, kit::P2);
    let mut engine = Duel::table(SEED, forest(), 3)
        .team(0, 1)
        .team(1, 1)
        .team(2, 2)
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[typhoon()])
        .battlefield(1, &[island(), island(), island()])
        .battlefield(2, &[island(), island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, typhoon());
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        kit::life(&engine, p1),
        20,
        "player 1 plays on player 0's side and is no opponent"
    );
    assert_eq!(
        kit::life(&engine, p2),
        18,
        "player 2 is the opponent: two Islands, two damage"
    );
}
