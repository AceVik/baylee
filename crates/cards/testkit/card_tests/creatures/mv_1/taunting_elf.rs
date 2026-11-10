//! `cards/creatures/mv_1/taunting_elf.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn taunting_elf() -> CardIndex {
    card_index("aef07e62-ac7f-4a5b-9732-136052d88a06")
}

/// Taunting Elf (aef07e62-ac7f-4a5b-9732-136052d88a06) — {G} 0/1 Creature — Elf:
/// "All creatures able to block this creature do so."
///
/// Attacking with Taunting Elf and a Grizzly Bears forces every untapped able
/// blocker controlled by the defending player to block Taunting Elf (leaving
/// the Grizzly Bears unblocked), while a tapped defender is not able to block
/// and is excluded from the requirement.
#[test]
fn taunting_elf_forces_all_able_creatures_to_block_it() {
    use baylee_core::generated::index;

    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[taunting_elf(), grizzly_bears()])
        .battlefield(1, &[grizzly_bears(), index::HILL_GIANT, llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p0, taunting_elf()).unwrap();
    let attacker_bears = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let blocker_bears = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    let blocker_giant = on_battlefield(&engine, p1, index::HILL_GIANT).unwrap();
    let tapped_elf = on_battlefield(&engine, p1, llanowar_elves()).unwrap();

    // Tap one defender so it is unable to block.
    engine
        .dev_state_mut(p1)
        .unwrap()
        .set_tapped(tapped_elf, true);
    engine.refresh_offer();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![
                    (elf, Defender::Player(p1)),
                    (attacker_bears, Defender::Player(p1)),
                ],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });

    // Leaving out an able blocker or diverting one to the Grizzly Bears is refused.
    assert!(
        engine
            .apply(
                p1,
                PlayerAction::DeclareBlockers {
                    blockers: vec![(blocker_bears, elf)],
                },
            )
            .is_err(),
        "leaving an able blocker unassigned is refused"
    );
    assert!(
        engine
            .apply(
                p1,
                PlayerAction::DeclareBlockers {
                    blockers: vec![(blocker_bears, attacker_bears), (blocker_giant, elf)],
                },
            )
            .is_err(),
        "assigning an able blocker to another attacker instead of Taunting Elf is refused"
    );

    // Assigning both able blockers to Taunting Elf succeeds.
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(blocker_bears, elf), (blocker_giant, elf)],
            },
        )
        .unwrap();

    let mut blockers_of_elf = engine.state().combat.blockers_of(elf);
    blockers_of_elf.sort_unstable();
    let mut expected = vec![blocker_bears, blocker_giant];
    expected.sort_unstable();
    assert_eq!(blockers_of_elf, expected);
    assert!(
        engine.state().combat.blockers_of(attacker_bears).is_empty(),
        "the second attacker goes unblocked"
    );
}
