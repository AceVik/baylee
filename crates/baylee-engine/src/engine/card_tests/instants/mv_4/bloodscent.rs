//! `cards/instants/mv_4/bloodscent.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::HILL_GIANT;

fn bloodscent() -> CardIndex {
    card_index("b2aabfaf-5996-4b41-987a-6bd941f6ae88")
}

/// Bloodscent — {3}{G} Instant
/// "All creatures able to block target creature this turn do so."
#[test]
fn bloodscent_forces_all_able_creatures_to_block_target_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[bloodscent()])
        .battlefield(0, &[forest(), forest(), forest(), forest(), HILL_GIANT])
        .battlefield(1, &[llanowar_elves(), grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let giant = on_battlefield(&engine, p0, HILL_GIANT).unwrap();
    let elf = on_battlefield(&engine, p1, llanowar_elves()).unwrap();
    let bear = on_battlefield(&engine, p1, grizzly_bears()).unwrap();

    cast_from_hand(&mut engine, p0, bloodscent());
    aim_at(&mut engine, p0, giant);
    pass_until(&mut engine, stack_is_empty);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(giant, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });

    // Declaring no blockers is rejected because both Llanowar Elves and Grizzly Bears must block Hill Giant.
    assert!(
        engine
            .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
            .is_err()
    );
    // Declaring both creatures blocking Hill Giant is legal.
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(elf, giant), (bear, giant)],
            },
        )
        .unwrap();
}
