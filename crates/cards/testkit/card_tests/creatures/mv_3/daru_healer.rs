//! `cards/creatures/mv_3/daru_healer.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

fn daru_healer() -> CardIndex {
    card_index("630b1685-d0c9-4d94-97dc-70058fb52e5d")
}

/// Daru Healer ({2}{W} 1/2 Human Cleric):
/// "{T}: Prevent the next 1 damage that would be dealt to any target this turn.
/// Morph {W}"
#[test]
fn daru_healer_prevents_next_one_damage_to_target_creature_or_player() {
    let mut e = lk::game(
        SEED,
        &[daru_healer(), benalish_hero()],
        &[],
        &[lightning_bolt()],
        &[],
    );
    let hero = lk::obj(&e, lk::P0, benalish_hero());
    lk::use_ability(&mut e, lk::P0, daru_healer(), 0, &[hero], &[]);

    lk::float(&mut e, lk::P0, &[(ManaColor::Red, 1)]);
    lk::cast_at(&mut e, lk::P0, lightning_bolt(), &[hero], &[]);
    assert!(
        on_battlefield(&e, lk::P0, benalish_hero()).is_none(),
        "preventing 1 of 3 still leaves 2 lethal damage on a 1/1"
    );

    let mut e2 = lk::game(
        SEED,
        &[daru_healer(), grizzly_bears()],
        &[],
        &[lightning_bolt(), lightning_bolt()],
        &[],
    );
    let bear = lk::obj(&e2, lk::P0, grizzly_bears());
    lk::use_ability(&mut e2, lk::P0, daru_healer(), 0, &[bear], &[]);
    lk::float(&mut e2, lk::P0, &[(ManaColor::Red, 1)]);
    lk::cast_at(&mut e2, lk::P0, lightning_bolt(), &[bear], &[]);
    assert!(
        on_battlefield(&e2, lk::P0, grizzly_bears()).is_none(),
        "3 - 1 = 2 still kills 2/2, so test player target directly too"
    );

    let mut e3 = lk::game(SEED, &[daru_healer()], &[], &[lightning_bolt()], &[]);
    lk::use_ability(&mut e3, lk::P0, daru_healer(), 0, &[], &[lk::P1]);
    lk::float(&mut e3, lk::P0, &[(ManaColor::Red, 1)]);
    lk::cast_at(&mut e3, lk::P0, lightning_bolt(), &[], &[lk::P1]);
    assert_eq!(
        lk::life(&e3, lk::P1),
        18,
        "prevented 1 of Lightning Bolt's 3 damage"
    );
}

#[test]
fn daru_healer_saves_1_1_from_1_damage_and_turns_face_up_for_white() {
    let mut e = lk::game(
        SEED,
        &[plains(); 4],
        &[prodigal_sorcerer()],
        &[daru_healer(), benalish_hero()],
        &[],
    );
    cast_from_hand(&mut e, lk::P0, daru_healer());
    let slot = choose_cast_kind(&e, CastModeKind::Disguise);
    e.apply(lk::P0, PlayerAction::ChooseMode(slot)).unwrap();
    lk::settle(&mut e);

    let id = lk::obj(&e, lk::P0, daru_healer());
    assert!(
        e.state()
            .object(id)
            .unwrap()
            .status
            .contains(Status::FACE_DOWN)
    );
    assert_eq!(pt(&e, id), (2, 2));

    e.apply(
        lk::P0,
        PlayerAction::ActivateAbility {
            source: id,
            ability_index: baylee_engine::choice::TURN_FACE_UP,
        },
    )
    .expect("remaining {W} pays Morph {W}");
    assert!(
        !e.state()
            .object(id)
            .unwrap()
            .status
            .contains(Status::FACE_DOWN)
    );
    assert_eq!(pt(&e, id), (1, 2));

    lk::float(&mut e, lk::P0, &[(ManaColor::White, 1)]);
    cast_with_floating(&mut e, lk::P0, benalish_hero());
    lk::settle(&mut e);
    let hero = lk::obj(&e, lk::P0, benalish_hero());

    // Clear Daru Healer's summoning sickness at the start of P0's next turn, then respond on P1's turn.
    reach_their_main_phase(&mut e, lk::P1);
    reach_their_main_phase(&mut e, lk::P0);
    reach_their_main_phase(&mut e, lk::P1);
    activate(&mut e, lk::P1, prodigal_sorcerer(), 0);
    lk::aim(&mut e, &[hero], &[]);
    lk::pass_once(&mut e);
    lk::use_ability(&mut e, lk::P0, daru_healer(), 0, &[hero], &[]);
    assert!(on_battlefield(&e, lk::P0, benalish_hero()).is_some());
    assert_eq!(lk::damage_on(&e, hero), 0);
}
