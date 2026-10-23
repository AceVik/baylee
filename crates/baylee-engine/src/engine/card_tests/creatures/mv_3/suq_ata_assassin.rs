//! `cards/creatures/mv_3/suq_ata_assassin.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index::{BLACK_KNIGHT, GRIZZLY_BEARS};

fn suq_ata_assassin() -> CardIndex {
    card_index("db43b866-13e0-4336-a580-b64fe1ee5954")
}

/// `Suq'Ata Assassin` ({1}{B}{B}, 1/1 Creature — Human Assassin,
/// `Coverage::Implemented`) has fear and gives the defending player a poison
/// counter whenever it attacks and isn't blocked.
#[test]
fn suq_ata_assassin_has_fear_and_gives_defending_player_a_poison_counter_when_unblocked() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp()])
        .hand(0, &[suq_ata_assassin()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, suq_ata_assassin());
    pass_until(&mut engine, stack_is_empty);
    let assassin = on_battlefield(&engine, p0, suq_ata_assassin()).unwrap();
    assert!(keywords(&engine, assassin).contains(KeywordSet::FEAR));

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(assassin, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[1].poison, 1);
    assert_eq!(engine.state().players[0].poison, 0);
}

/// Fear: only artifact and/or black creatures may block it. A black Knight
/// may, a green Bears may not; and once it is blocked there is no poison
/// ("attacks and isn't blocked").
#[test]
fn suq_ata_assassin_gives_no_poison_when_blocked_and_fear_limits_the_blockers() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = lk::game(
        SEED,
        &[suq_ata_assassin()],
        &[BLACK_KNIGHT, GRIZZLY_BEARS],
        &[],
        &[],
    );
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    let assassin = lk::obj(&engine, p0, suq_ata_assassin());
    let knight = lk::obj(&engine, p1, BLACK_KNIGHT);
    let bears = lk::obj(&engine, p1, GRIZZLY_BEARS);

    let offer = lk::attack(&mut engine, &[assassin]);
    assert!(
        lk::may_block(&offer, knight, assassin),
        "a black creature may block fear"
    );
    assert!(
        !lk::may_block(&offer, bears, assassin),
        "a green non-artifact creature may not"
    );
    lk::block(&mut engine, &[(knight, assassin)]);
    lk::settle(&mut engine);
    lk::through_combat(&mut engine);

    assert_eq!(
        engine.state().players[1].poison,
        0,
        "blocked: no poison counter"
    );
    assert_eq!(engine.state().players[0].poison, 0);
}
