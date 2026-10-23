//! `cards/creatures/mv_4/tainted_specter.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn tainted_specter() -> CardIndex {
    card_index("fc638800-6aec-4a48-ad2c-c9a5ce93c236")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

/// Tainted Specter (`{3}{B}`, `Coverage::Partial`): flying is the part that is
/// played. The Specter attacks past an untapped Hill Giant that cannot block
/// it, and 2 damage goes through. The activated ability is not played, so
/// nothing is asked of the opponent's hand.
#[test]
fn tainted_specter_attacks_in_the_air_past_a_ground_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[tainted_specter()])
        .battlefield(1, &[hill_giant()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    let specter = on_battlefield(&engine, p0, tainted_specter()).unwrap();
    assert!(on_battlefield(&engine, p1, hill_giant()).is_some());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(specter, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
            || e.state().turn.step == crate::turn::Step::CombatEnd
    });
    // Either nobody was asked, because nobody could block, or the question
    // pairs no creature with the Specter.
    let offered = match engine.pending() {
        Pending::ChooseBlockers { blockers, .. } => blockers.clone(),
        _ => Vec::new(),
    };
    assert!(
        offered
            .iter()
            .all(|option| !option.attackers.contains(&specter)),
        "the Hill Giant has no reach: nothing may block the Specter"
    );
    pass_until(&mut engine, |e| {
        e.state().turn.step == crate::turn::Step::CombatEnd
    });
    assert_eq!(engine.state().players[1].life, 18, "two damage in the air");
}
