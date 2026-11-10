//! `cards/creatures/mv_3/cadaverous_knight.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{BURNING_SHIELD_ASKARI, HILL_GIANT};

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn cadaverous_knight() -> CardIndex {
    card_index("250a027f-2b3e-4eaf-899a-a9da8be20070")
}

/// Cadaverous Knight (250a027f-2b3e-4eaf-899a-a9da8be20070) — {2}{B} 2/2:
/// "Flanking; {1}{B}{B}: Regenerate this creature."
#[test]
fn cadaverous_knight_flanking_and_regenerate_win_combat_against_hill_giant() {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[cadaverous_knight(), swamp(), swamp(), swamp()])
        .battlefield(1, &[HILL_GIANT])
        .start();
    keep_mulligans(&mut engine);

    let knight = on_battlefield(&engine, P0, cadaverous_knight()).unwrap();
    let giant = on_battlefield(&engine, P1, HILL_GIANT).unwrap();

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
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, giant), (2, 2), "one -1/-1 on the 3/3, not two");

    tap_all_mana(&mut engine, P0);
    activate(&mut engine, P0, cadaverous_knight(), 0);
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);

    assert!(on_battlefield(&engine, P0, cadaverous_knight()).is_some());
    assert!(on_battlefield(&engine, P1, HILL_GIANT).is_none());
}

/// "Whenever a creature without flanking blocks": a flanking blocker
/// (Burning Shield Askari) is not shrunk.
#[test]
fn cadaverous_knight_does_not_shrink_a_blocker_with_flanking() {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[cadaverous_knight()])
        .battlefield(1, &[BURNING_SHIELD_ASKARI])
        .start();
    keep_mulligans(&mut engine);

    let knight = on_battlefield(&engine, P0, cadaverous_knight()).unwrap();
    let askari = on_battlefield(&engine, P1, BURNING_SHIELD_ASKARI).unwrap();

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
                blockers: vec![(askari, knight)],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, askari), (2, 2));
}
