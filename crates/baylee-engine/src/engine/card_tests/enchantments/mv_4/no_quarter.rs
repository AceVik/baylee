//! `cards/enchantments/mv_4/no_quarter.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn no_quarter() -> CardIndex {
    card_index("d238a5c1-f992-4f89-851a-de75b59f6793")
}

/// No Quarter (`d238a5c1-f992-4f89-851a-de75b59f6793`) — {3}{R} Enchantment (Partial):
/// "Whenever a creature becomes blocked by a creature with lesser power, destroy the blocking creature."
#[test]
fn no_quarter_destroys_blocker_with_lesser_power() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                index::HILL_GIANT,
            ],
        )
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .hand(0, &[no_quarter()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, no_quarter());
    pass_until(&mut engine, stack_is_empty);

    let giant = on_battlefield(&engine, P0, index::HILL_GIANT).unwrap();
    let bears = on_battlefield(&engine, P1, index::GRIZZLY_BEARS).unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(giant, Defender::Player(P1))],
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
                blockers: vec![(bears, giant)],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, P1, index::GRIZZLY_BEARS).is_some());
}
