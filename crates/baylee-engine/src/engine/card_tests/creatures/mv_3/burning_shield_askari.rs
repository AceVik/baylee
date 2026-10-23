//! `cards/creatures/mv_3/burning_shield_askari.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{CADAVEROUS_KNIGHT, HILL_GIANT};

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn burning_shield_askari() -> CardIndex {
    card_index("042fd95a-5e85-4ce5-bb68-066a295c54fb")
}

/// Burning Shield Askari (042fd95a-5e85-4ce5-bb68-066a295c54fb) — {2}{R} 2/2:
/// "Flanking; {R}{R}: This creature gains first strike until end of turn."
#[test]
fn burning_shield_askari_flanking_and_first_strike_kill_three_three_blocker() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[burning_shield_askari(), mountain(), mountain()])
        .battlefield(1, &[HILL_GIANT])
        .start();
    keep_mulligans(&mut engine);

    let askari = on_battlefield(&engine, P0, burning_shield_askari()).unwrap();
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
    assert_eq!(pt(&engine, giant), (2, 2), "one -1/-1 on the 3/3, not two");

    // Activate {R}{R} for first strike.
    tap_all_mana(&mut engine, P0);
    activate(&mut engine, P0, burning_shield_askari(), 0);
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);

    assert!(on_battlefield(&engine, P0, burning_shield_askari()).is_some());
    assert!(on_battlefield(&engine, P1, HILL_GIANT).is_none());
}

/// "Whenever a creature without flanking blocks": a flanking blocker
/// (Cadaverous Knight) is not shrunk.
#[test]
fn burning_shield_askari_does_not_shrink_a_blocker_with_flanking() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[burning_shield_askari()])
        .battlefield(1, &[CADAVEROUS_KNIGHT])
        .start();
    keep_mulligans(&mut engine);

    let askari = on_battlefield(&engine, P0, burning_shield_askari()).unwrap();
    let knight = on_battlefield(&engine, P1, CADAVEROUS_KNIGHT).unwrap();

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
                blockers: vec![(knight, askari)],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, knight), (2, 2));
}

/// "until end of turn": the first strike is gone on the opponent's turn.
#[test]
fn burning_shield_askari_first_strike_ends_with_the_turn() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[burning_shield_askari(), mountain(), mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let askari = on_battlefield(&engine, P0, burning_shield_askari()).unwrap();
    assert!(!keywords(&engine, askari).contains(KeywordSet::FIRST_STRIKE));
    tap_all_mana(&mut engine, P0);
    activate(&mut engine, P0, burning_shield_askari(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert!(keywords(&engine, askari).contains(KeywordSet::FIRST_STRIKE));

    reach_their_main_phase(&mut engine, P1);
    assert!(!keywords(&engine, askari).contains(KeywordSet::FIRST_STRIKE));
}
