//! `cards/creatures/mv_5/talruum_piper.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{GRIZZLY_BEARS, MOUNTAIN, SERRA_ANGEL};

fn talruum_piper() -> CardIndex {
    card_index("d18e60ed-ab6e-4b73-88f6-caa3f8e9c78e")
}

/// `Talruum Piper` ({4}{R}, 3/3 Creature — Minotaur, `Coverage::Implemented`)
/// forces all creatures with flying able to block it to do so.
#[test]
fn talruum_piper_forces_creatures_with_flying_to_block_it() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, MOUNTAIN)
        .battlefield(0, &[MOUNTAIN, MOUNTAIN, MOUNTAIN, MOUNTAIN, MOUNTAIN])
        .battlefield(1, &[SERRA_ANGEL, GRIZZLY_BEARS])
        .hand(0, &[talruum_piper()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let flyer = on_battlefield(&engine, p1, SERRA_ANGEL).unwrap();
    let ground = on_battlefield(&engine, p1, GRIZZLY_BEARS).unwrap();
    cast_from_hand(&mut engine, p0, talruum_piper());
    pass_until(&mut engine, stack_is_empty);
    let piper = on_battlefield(&engine, p0, talruum_piper()).unwrap();

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
                attackers: vec![(piper, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });

    // Declaring no blockers or only ground creature fails because Serra Angel must block
    assert!(
        engine
            .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
            .is_err()
    );
    assert!(
        engine
            .apply(
                p1,
                PlayerAction::DeclareBlockers {
                    blockers: vec![(ground, piper)],
                },
            )
            .is_err()
    );
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(flyer, piper)],
            },
        )
        .expect("flyer blocking Talruum Piper satisfies requirement");
}
