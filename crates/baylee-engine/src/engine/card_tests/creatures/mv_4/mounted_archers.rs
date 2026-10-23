//! `cards/creatures/mv_4/mounted_archers.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn mounted_archers() -> CardIndex {
    card_index("22517690-3ec1-49c9-951f-93eb3114423a")
}

/// Mounted Archers (`22517690-3ec1-49c9-951f-93eb3114423a`) — {3}{W} Creature — Human Soldier Archer:
/// "Reach. {W}: This creature can block an additional creature this turn."
#[test]
fn mounted_archers_can_block_an_additional_creature_after_activation() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mounted_archers(), plains()])
        .battlefield(1, &[index::AIR_ELEMENTAL, index::HILL_GIANT])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, P1);

    let archers = on_battlefield(&engine, P0, mounted_archers()).unwrap();
    let bears = on_battlefield(&engine, P1, index::AIR_ELEMENTAL).unwrap();
    let giant = on_battlefield(&engine, P1, index::HILL_GIANT).unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            P1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(bears, Defender::Player(P0)), (giant, Defender::Player(P0))],
            },
        )
        .unwrap();

    // Activate Mounted Archers during DeclareAttackers step before blockers are declared.
    engine.apply(P1, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, P0);
    activate(&mut engine, P0, mounted_archers(), 0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });

    engine
        .apply(
            P0,
            PlayerAction::DeclareBlockers {
                blockers: vec![(archers, bears), (archers, giant)],
            },
        )
        .unwrap();
}

/// Reach permits one flying blocker pairing without buying extra capacity.
#[test]
fn reach_does_not_allow_a_second_block_without_activation() {
    use crate::engine::card_tests::legends_kit as lk;
    let mut e = lk::game(
        SEED,
        &[mounted_archers()],
        &[index::AIR_ELEMENTAL, index::HILL_GIANT],
        &[],
        &[],
    );
    reach_their_main_phase(&mut e, P1);
    let archer = on_battlefield(&e, P0, mounted_archers()).unwrap();
    let air = on_battlefield(&e, P1, index::AIR_ELEMENTAL).unwrap();
    let giant = on_battlefield(&e, P1, index::HILL_GIANT).unwrap();
    let offer = lk::attack(&mut e, &[air, giant]);
    assert!(lk::may_block(&offer, archer, air));
    assert!(
        e.apply(
            P0,
            PlayerAction::DeclareBlockers {
                blockers: vec![(archer, air), (archer, giant)]
            }
        )
        .is_err()
    );
    lk::block(&mut e, &[(archer, air)]);
}
