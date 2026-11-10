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

/// Casts the Piper and walks to the opponent's block declaration on the next
/// turn of its controller.
fn piper_attacks(p1_board: &[CardIndex]) -> (Engine<RegistryLookup>, ObjectId) {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, MOUNTAIN)
        .battlefield(0, &[MOUNTAIN, MOUNTAIN, MOUNTAIN, MOUNTAIN, MOUNTAIN])
        .battlefield(1, p1_board)
        .hand(0, &[talruum_piper()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

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
    (engine, piper)
}

/// Only creatures *with flying* are forced: with no flier to block, declaring no
/// blockers is legal.
#[test]
fn talruum_piper_forces_nothing_when_no_creature_has_flying() {
    let p1 = PlayerId::new(1);
    let (mut engine, _piper) = piper_attacks(&[GRIZZLY_BEARS]);

    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .expect("a ground creature is not forced to block");
}

/// The creatures forced are the fliers; a ground creature may still block as
/// well, it is just not compelled to.
#[test]
fn talruum_piper_lets_a_ground_creature_block_beside_the_flier() {
    let p1 = PlayerId::new(1);
    let (mut engine, piper) = piper_attacks(&[SERRA_ANGEL, GRIZZLY_BEARS]);
    let flyer = on_battlefield(&engine, p1, SERRA_ANGEL).unwrap();
    let ground = on_battlefield(&engine, p1, GRIZZLY_BEARS).unwrap();

    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(flyer, piper), (ground, piper)],
            },
        )
        .expect("the flier blocks as required and the ground creature joins it");
}
