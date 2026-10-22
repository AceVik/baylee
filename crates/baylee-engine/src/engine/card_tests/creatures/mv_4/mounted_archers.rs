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
        .battlefield(1, &[index::GRIZZLY_BEARS, index::HILL_GIANT])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, P1);

    let archers = on_battlefield(&engine, P0, mounted_archers()).unwrap();
    let bears = on_battlefield(&engine, P1, index::GRIZZLY_BEARS).unwrap();
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
