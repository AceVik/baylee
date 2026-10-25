//! `cards/instants/mv_2/incinerate.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn incinerate() -> CardIndex {
    card_index("d8fd7a34-8418-4e98-b79b-119c4348c667")
}

/// Incinerate (d8fd7a34-8418-4e98-b79b-119c4348c667) — {1}{R} Instant:
/// - "Incinerate deals 3 damage to any target. A creature dealt damage this way can't be regenerated this turn."
#[test]
fn incinerate_kills_a_creature_through_a_regeneration_shield_and_can_target_a_player() {
    let mut engine = Duel::new(SEED, index::MOUNTAIN)
        .battlefield(0, &[index::MOUNTAIN, index::MOUNTAIN])
        .hand(0, &[incinerate()])
        .battlefield(1, &[index::DRUDGE_SKELETONS, index::SWAMP])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let skel = on_battlefield(&engine, P1, index::DRUDGE_SKELETONS).expect("Skeletons seated");

    // P0 casts Incinerate targeting P1's Drudge Skeletons.
    tap_all_mana_but(&mut engine, P0, None);
    cast_from_hand(&mut engine, P0, incinerate());
    aim_at(&mut engine, P0, skel);

    // In response, P1 activates Drudge Skeletons' "{B}: Regenerate this creature."
    engine.apply(P0, PlayerAction::PassPriority).unwrap();
    tap_all_mana_but(&mut engine, P1, None);
    activate(&mut engine, P1, index::DRUDGE_SKELETONS, 0);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, P1, index::DRUDGE_SKELETONS).is_some(),
        "Incinerate's 'can't be regenerated this turn' prevents the regeneration shield from saving Drudge Skeletons"
    );
}

#[test]
fn incinerate_deals_three_damage_to_target_player() {
    let mut engine = lk::game(
        SEED,
        &[index::MOUNTAIN, index::MOUNTAIN],
        &[],
        &[incinerate()],
        &[],
    );
    tap_all_mana_but(&mut engine, P0, None);
    cast_from_hand(&mut engine, P0, incinerate());
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![P1],
            },
        )
        .unwrap();
    lk::settle(&mut engine);

    assert_eq!(engine.state().players[1].life, 17);
}

#[track_caller]
fn priority_to(e: &mut Engine<RegistryLookup>, seat: PlayerId) {
    pass_until(
        e,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == seat),
    );
}

#[track_caller]
fn burn(e: &mut Engine<RegistryLookup>, objects: &[ObjectId], players: &[PlayerId]) {
    priority_to(e, P0);
    lk::float(e, P0, &[(ManaColor::Red, 1), (ManaColor::Colorless, 1)]);
    lk::cast_at(e, P0, incinerate(), objects, players);
}

#[track_caller]
fn shield_then_shatter(e: &mut Engine<RegistryLookup>, wall: ObjectId) {
    priority_to(e, P1);
    lk::float(e, P1, &[(ManaColor::Colorless, 1)]);
    lk::use_ability(e, P1, index::LIVING_WALL, 0, &[], &[]);
    assert_eq!(
        e.state()
            .object(wall)
            .expect("the Wall is still there")
            .regeneration_shields,
        1,
        "the prohibition does not prevent activating regeneration"
    );
    priority_to(e, P0);
    lk::float(e, P0, &[(ManaColor::Red, 1), (ManaColor::Colorless, 1)]);
    lk::cast_at(e, P0, index::SHATTER, &[wall], &[]);
}

#[test]
fn a_creature_receiving_redirected_incinerate_damage_cannot_regenerate() {
    let mut e = lk::game(
        SEED,
        &[],
        &[index::LIVING_WALL],
        &[incinerate(), index::PARIAH, index::SHATTER],
        &[],
    );
    let wall = lk::obj(&e, P1, index::LIVING_WALL);
    lk::float(
        &mut e,
        P0,
        &[(ManaColor::White, 1), (ManaColor::Colorless, 2)],
    );
    lk::cast_at(&mut e, P0, index::PARIAH, &[wall], &[]);
    burn(&mut e, &[], &[P0]);
    assert_eq!(lk::life(&e, P0), 20, "Pariah redirected all three damage");
    assert_eq!(
        lk::damage_on(&e, wall),
        3,
        "the Wall actually received the damage"
    );
    shield_then_shatter(&mut e, wall);
    assert!(
        in_graveyard(&e, P1, index::LIVING_WALL).is_some(),
        "the actual recipient cannot regenerate although Incinerate targeted a player"
    );
}

#[test]
fn preventing_all_incinerate_damage_leaves_the_creature_able_to_regenerate() {
    let mut e = lk::game(
        SEED,
        &[],
        &[index::LIVING_WALL],
        &[incinerate(), index::HEALING_SALVE, index::SHATTER],
        &[],
    );
    let wall = lk::obj(&e, P1, index::LIVING_WALL);
    lk::float(&mut e, P0, &[(ManaColor::White, 1)]);
    lk::cast_modal(&mut e, P0, index::HEALING_SALVE, 1, 0, &[wall], &[]);
    burn(&mut e, &[wall], &[]);
    assert_eq!(
        lk::damage_on(&e, wall),
        0,
        "all of Incinerate's damage was prevented"
    );
    shield_then_shatter(&mut e, wall);
    assert!(
        on_battlefield(&e, P1, index::LIVING_WALL).is_some(),
        "a creature dealt no damage this way may regenerate"
    );
    assert_eq!(
        e.state()
            .object(wall)
            .expect("the Wall survived")
            .regeneration_shields,
        0,
        "Shatter really consumed the regeneration shield"
    );
}

#[test]
fn incinerates_prohibition_ends_with_the_turn() {
    let mut e = lk::game(
        SEED,
        &[],
        &[index::LIVING_WALL],
        &[incinerate(), index::SHATTER],
        &[],
    );
    let wall = lk::obj(&e, P1, index::LIVING_WALL);
    burn(&mut e, &[wall], &[]);
    assert_eq!(lk::damage_on(&e, wall), 3);
    let damaged_turn = e.state().turn.number;
    pass_until(&mut e, |e| {
        e.state().turn.number > damaged_turn
            && e.state().turn.active == P1
            && e.state().turn.phase == Phase::FirstMain
            && matches!(e.pending(), Pending::Priority { .. })
    });
    shield_then_shatter(&mut e, wall);
    assert!(
        on_battlefield(&e, P1, index::LIVING_WALL).is_some(),
        "Incinerate cannot stop regeneration on the next turn"
    );
}
