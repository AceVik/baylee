//! `cards/creatures/mv_3/searing_spear_askari.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn searing_spear_askari() -> CardIndex {
    card_index("d3c68573-95b1-4db5-bbea-363b86f713c1")
}

/// Searing Spear Askari (`{2}{R}`, `Coverage::Implemented`): 2/2 Human Knight
/// with flanking and `{1}{R}` to gain menace until end of turn.
#[test]
fn searing_spear_askari_flanking_and_menace_activation() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[searing_spear_askari(), mountain(), mountain()])
        .battlefield(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let askari = on_battlefield(&engine, p0, searing_spear_askari()).unwrap();
    let bear = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    assert!(keywords(&engine, askari).contains(KeywordSet::FLANKING));

    // Activate {1}{R}: gains menace until end of turn.
    tap_all_mana(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: askari,
                ability_index: 1,
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(keywords(&engine, askari).contains(KeywordSet::MENACE));

    // With menace, a single blocker (Grizzly Bears) cannot block Searing Spear Askari;
    // on the next turn without activating menace, Grizzly Bears blocks and gets -1/-1 from flanking.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(!keywords(&engine, askari).contains(KeywordSet::MENACE));

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
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(bear, askari)],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, bear), (1, 1));
}
