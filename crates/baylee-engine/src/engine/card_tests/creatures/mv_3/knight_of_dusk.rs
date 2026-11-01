//! `cards/creatures/mv_3/knight_of_dusk.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn knight_of_dusk() -> CardIndex {
    card_index("cd1ca3c8-1f03-4c03-a93a-0419ac205c18")
}

/// Knight of Dusk (`cd1ca3c8-1f03-4c03-a93a-0419ac205c18`) — {1}{B}{B} Creature — Human Knight:
/// "{B}{B}: Destroy target creature blocking this creature."
#[test]
fn knight_of_dusk_destroys_target_creature_blocking_it() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[knight_of_dusk(), swamp(), swamp()])
        .battlefield(1, &[index::HILL_GIANT, index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let knight = on_battlefield(&engine, P0, knight_of_dusk()).unwrap();
    let giant = on_battlefield(&engine, P1, index::HILL_GIANT).unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(knight, Defender::Player(P1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            P1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(giant, knight)],
            },
        )
        .unwrap();

    tap_all_mana(&mut engine, P0);
    activate(&mut engine, P0, knight_of_dusk(), 0);
    let idle = on_battlefield(&engine, P1, index::GRIZZLY_BEARS).unwrap();
    let menu = aim_at(&mut engine, P0, giant);
    assert!(!menu.contains(&idle));
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P1, index::HILL_GIANT).is_none());
    assert!(in_graveyard(&engine, P1, index::HILL_GIANT).is_some());
}
