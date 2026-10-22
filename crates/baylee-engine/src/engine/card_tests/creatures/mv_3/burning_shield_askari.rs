//! `cards/creatures/mv_3/burning_shield_askari.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{BURNING_SHIELD_ASKARI, HILL_GIANT};

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

/// Burning Shield Askari (042fd95a-5e85-4ce5-bb68-066a295c54fb) — {2}{R} 2/2:
/// "Flanking; {R}{R}: This creature gains first strike until end of turn."
#[test]
fn burning_shield_askari_flanking_and_first_strike_kill_three_three_blocker() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[BURNING_SHIELD_ASKARI, mountain(), mountain()])
        .battlefield(1, &[HILL_GIANT])
        .start();
    keep_mulligans(&mut engine);

    let askari = on_battlefield(&engine, P0, BURNING_SHIELD_ASKARI).unwrap();
    let giant = on_battlefield(&engine, P1, HILL_GIANT).unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(askari, Defender::Player(P1))],
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
                blockers: vec![(giant, askari)],
            },
        )
        .unwrap();
    // Resolve the flanking trigger (-1/-1 to Hill Giant -> 2/2).
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, giant), (2, 2));

    // Activate {R}{R} for first strike.
    tap_all_mana(&mut engine, P0);
    activate(&mut engine, P0, BURNING_SHIELD_ASKARI, 1);
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);

    assert!(on_battlefield(&engine, P0, BURNING_SHIELD_ASKARI).is_some());
    assert!(on_battlefield(&engine, P1, HILL_GIANT).is_none());
}
