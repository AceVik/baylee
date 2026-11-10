//! `cards/creatures/mv_5/telim_tor.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn telim_tor() -> CardIndex {
    card_index("b13a723d-80a9-4116-9857-baf3b373f87a")
}

fn searing_spear_askari() -> CardIndex {
    card_index("d3c68573-95b1-4db5-bbea-363b86f713c1")
}

fn hill_giant() -> CardIndex {
    baylee_core::generated::index::HILL_GIANT
}

/// Telim'Tor (`{4}{R}`, `Coverage::Implemented`): 2/2 Legendary Human Knight
/// with flanking that pumps all attacking creatures with flanking +1/+1 until
/// end of turn whenever it attacks.
#[test]
fn telim_tor_pumps_attacking_flanking_creatures_and_flanks_blocker() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[telim_tor(), searing_spear_askari(), grizzly_bears()])
        .battlefield(1, &[hill_giant()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    let telim = on_battlefield(&engine, p0, telim_tor()).unwrap();
    let askari = on_battlefield(&engine, p0, searing_spear_askari()).unwrap();
    let bear = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let giant = on_battlefield(&engine, p1, hill_giant()).unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![
                    (telim, Defender::Player(p1)),
                    (askari, Defender::Player(p1)),
                    (bear, Defender::Player(p1)),
                ],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });

    // Telim'Tor and Searing Spear Askari (both attacking with flanking) get +1/+1 (3/3);
    // Grizzly Bears (no flanking) stays 2/2.
    assert_eq!(pt(&engine, telim), (3, 3));
    assert_eq!(pt(&engine, askari), (3, 3));
    assert_eq!(pt(&engine, bear), (2, 2));

    // Hill Giant blocks Telim'Tor and gets -1/-1 from flanking (2/2).
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(giant, telim)],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, giant), (2, 2));
}

/// Telim'Tor — "Whenever Telim'Tor attacks": a flanking creature that attacks
/// without him gets nothing, and when he does attack the bonus lasts only until
/// end of turn.
#[test]
fn telim_tor_pumps_only_when_he_attacks_and_only_until_end_of_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[telim_tor(), searing_spear_askari()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    let telim = on_battlefield(&engine, p0, telim_tor()).unwrap();
    let askari = on_battlefield(&engine, p0, searing_spear_askari()).unwrap();

    // The Askari attacks alone: Telim'Tor's trigger is not his to fire.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(askari, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, askari), (2, 2), "Telim'Tor stayed home");

    // Both attack the next turn: both are pumped, and it wears off.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![
                    (telim, Defender::Player(p1)),
                    (askari, Defender::Player(p1)),
                ],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, telim), (3, 3));
    assert_eq!(pt(&engine, askari), (3, 3));
    reach_their_main_phase(&mut engine, p1);
    assert_eq!(pt(&engine, telim), (2, 2), "until end of turn");
    assert_eq!(pt(&engine, askari), (2, 2), "until end of turn");
}
