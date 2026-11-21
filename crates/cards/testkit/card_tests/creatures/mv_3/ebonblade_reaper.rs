//! `cards/creatures/mv_3/ebonblade_reaper.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

fn ebonblade_reaper() -> CardIndex {
    card_index("86fd4652-a629-429a-bf95-0b8724708a71")
}

/// Ebonblade Reaper ({2}{B} 1/1 Human Cleric):
/// "Whenever this creature attacks, you lose half your life, rounded up.
/// Whenever this creature deals combat damage to a player, that player loses
/// half their life, rounded up.
/// Morph {3}{B}{B}"
#[test]
fn ebonblade_reaper_halves_your_life_rounded_up_on_attack_and_defender_on_hit() {
    let mut e = lk::game(
        SEED,
        &[ebonblade_reaper(), prodigal_sorcerer()],
        &[],
        &[],
        &[],
    );
    let reaper = lk::obj(&e, lk::P0, ebonblade_reaper());

    // Ping P0 to 19 life so ceil(19 / 2) = 10 proves rounding up (9 remaining, not 10).
    lk::use_ability(&mut e, lk::P0, prodigal_sorcerer(), 0, &[], &[lk::P0]);
    assert_eq!(lk::life(&e, lk::P0), 19);

    let _ = lk::attack(&mut e, &[reaper]);
    assert_eq!(
        lk::life(&e, lk::P0),
        9,
        "attack trigger cost ceil(19/2) = 10 life before blockers"
    );
    lk::block(&mut e, &[]);
    lk::through_combat(&mut e);

    // P1 took 1 combat damage (20 -> 19), then lost ceil(19/2) = 10 -> 9 life.
    assert_eq!(lk::life(&e, lk::P1), 9);
}

#[test]
fn ebonblade_reaper_morph_attacks_without_life_loss_and_turns_face_up_before_damage() {
    let mut e = lk::game(SEED, &[swamp(); 8], &[], &[ebonblade_reaper()], &[]);
    cast_from_hand(&mut e, lk::P0, ebonblade_reaper());
    let slot = choose_cast_kind(&e, CastModeKind::Disguise);
    e.apply(lk::P0, PlayerAction::ChooseMode(slot)).unwrap();
    lk::settle(&mut e);

    let id = lk::obj(&e, lk::P0, ebonblade_reaper());
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
    .expect("remaining {3}{B}{B} pays Morph {3}{B}{B}");
    assert!(
        !e.state()
            .object(id)
            .unwrap()
            .status
            .contains(Status::FACE_DOWN)
    );
    assert_eq!(pt(&e, id), (1, 1));
    assert_eq!(lk::pool_total(&e, lk::P0), 0);
}
