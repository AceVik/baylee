//! `cards/creatures/mv_3/suq_ata_assassin.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

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
