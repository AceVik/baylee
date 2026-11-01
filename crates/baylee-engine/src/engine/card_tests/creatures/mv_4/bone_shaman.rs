//! `cards/creatures/mv_4/bone_shaman.rs`, played.

#[allow(clippy::wildcard_imports)] // The shared canonical-card harness.
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn bone_shaman() -> CardIndex {
    card_index("690ba846-964e-44ba-8637-9a1ce62f5931")
}

fn prepared(extra_hand: &[CardIndex]) -> Engine<RegistryLookup> {
    let mut hand = vec![index::SHATTER];
    hand.extend_from_slice(extra_hand);
    lk::game(SEED, &[bone_shaman()], &[index::LIVING_WALL], &hand, &[])
}

#[track_caller]
fn priority_to(e: &mut Engine<RegistryLookup>, seat: PlayerId) {
    pass_until(
        e,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == seat),
    );
}

#[track_caller]
fn grant_ability(e: &mut Engine<RegistryLookup>) {
    priority_to(e, P0);
    lk::float(e, P0, &[(ManaColor::Black, 1)]);
    lk::use_ability(e, P0, bone_shaman(), 0, &[], &[]);
    assert_eq!(
        lk::pool_of(e, P0, ManaColor::Black),
        0,
        "the {{B}} was paid"
    );
}

#[track_caller]
fn damage_wall(e: &mut Engine<RegistryLookup>) -> ObjectId {
    let shaman = lk::obj(e, P0, bone_shaman());
    let wall = lk::obj(e, P1, index::LIVING_WALL);
    let blocks = lk::attack(e, &[shaman]);
    assert!(lk::may_block(&blocks, wall, shaman));
    lk::block(e, &[(wall, shaman)]);
    lk::through_combat(e);
    assert_eq!(
        lk::damage_on(e, wall),
        3,
        "the Shaman dealt nonlethal combat damage"
    );
    wall
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
        "regeneration can still be activated and creates its shield"
    );
    priority_to(e, P0);
    lk::float(e, P0, &[(ManaColor::Red, 1), (ManaColor::Colorless, 1)]);
    lk::cast_at(e, P0, index::SHATTER, &[wall], &[]);
}

#[test]
fn an_activation_before_damage_prevents_regeneration_from_a_later_destruction() {
    for activate_before_damage in [false, true] {
        let mut e = prepared(&[]);
        if activate_before_damage {
            grant_ability(&mut e);
        }
        let wall = damage_wall(&mut e);
        shield_then_shatter(&mut e, wall);
        assert_eq!(
            on_battlefield(&e, P1, index::LIVING_WALL).is_none(),
            activate_before_damage,
            "only the activated Shaman stops a later Shatter being regenerated"
        );
        if !activate_before_damage {
            assert_eq!(lk::damage_on(&e, wall), 0, "the control really regenerated");
        }
    }
}

#[test]
fn an_activation_after_damage_also_prevents_regeneration() {
    let mut e = prepared(&[]);
    let wall = damage_wall(&mut e);
    grant_ability(&mut e);
    shield_then_shatter(&mut e, wall);
    assert!(
        in_graveyard(&e, P1, index::LIVING_WALL).is_some(),
        "the gained static ability includes damage dealt earlier this turn"
    );
}

#[test]
fn losing_the_gained_ability_or_leaving_the_battlefield_ends_the_prohibition() {
    for remover in [index::HUMBLE, index::UNSUMMON] {
        let mut e = prepared(&[remover]);
        let wall = damage_wall(&mut e);
        grant_ability(&mut e);
        let shaman = lk::obj(&e, P0, bone_shaman());
        priority_to(&mut e, P0);
        lk::float(
            &mut e,
            P0,
            &[
                (ManaColor::White, 1),
                (ManaColor::Blue, 1),
                (ManaColor::Colorless, 1),
            ],
        );
        lk::cast_at(&mut e, P0, remover, &[shaman], &[]);
        if remover == index::HUMBLE {
            assert_eq!(pt(&e, shaman), (0, 1), "Humble resolved");
        } else {
            assert!(
                in_hand(&e, P0, bone_shaman()).is_some(),
                "Unsummon resolved"
            );
        }
        shield_then_shatter(&mut e, wall);
        assert!(
            on_battlefield(&e, P1, index::LIVING_WALL).is_some(),
            "the Wall can regenerate once the Shaman no longer has the static ability"
        );
        assert_eq!(lk::damage_on(&e, wall), 0, "the shield was applied");
    }
}

#[test]
fn the_gained_ability_and_its_regeneration_prohibition_end_with_the_turn() {
    let mut e = prepared(&[]);
    grant_ability(&mut e);
    let wall = damage_wall(&mut e);
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
        "the Shaman's previous-turn activation cannot stop this regeneration"
    );
    assert_eq!(
        e.state()
            .object(wall)
            .expect("the Wall survived")
            .regeneration_shields,
        0,
        "the shield was actually consumed"
    );
}
