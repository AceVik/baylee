//! `cards/creatures/mv_5/balduvian_war_makers.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn balduvian_war_makers() -> CardIndex {
    card_index("1bda7f60-f986-455b-9d56-bbd494e27c17")
}

/// `Balduvian War-Makers` ({4}{R}, 3/3 Creature — Human Barbarian) has haste
/// and rampage 1 (+1/+1 until end of turn for each blocker beyond the first).
#[test]
fn balduvian_war_makers_attacks_with_haste_and_gets_rampage_bonus_when_double_blocked() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(); 5])
        .battlefield(1, &[mons_s_goblin_raiders(), grizzly_bears()])
        .hand(0, &[balduvian_war_makers()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, balduvian_war_makers());
    pass_until(&mut engine, stack_is_empty);
    let makers = on_battlefield(&engine, p0, balduvian_war_makers()).expect("War-Makers enters");
    assert_eq!(pt(&engine, makers), (3, 3));
    assert!(keywords(&engine, makers).contains(KeywordSet::HASTE));

    pass_until(&mut engine, |e| {
        e.state().turn.active == p0 && e.state().turn.step == Step::DeclareAttackers
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(makers, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.step == Step::DeclareBlockers
    });
    let b1 = on_battlefield(&engine, p1, mons_s_goblin_raiders()).unwrap();
    let b2 = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(b1, makers), (b2, makers)],
            },
        )
        .unwrap();

    // Rampage 1 triggers and resolves before combat damage.
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, makers), (4, 4));
}
