//! Root Spider — `{3}{G}` 2/2 Creature — Spider (Homelands)
//!
//! Whenever this creature blocks, it gets +1/+0 and gains first strike until end of turn.

#[allow(clippy::wildcard_imports)]
use super::*;

fn root_spider() -> CardIndex {
    card_index("d8fe0115-5d98-4251-a48c-5e849430a445")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

#[test]
fn root_spider_gets_plus_one_plus_zero_and_first_strike_when_blocking() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[root_spider()])
        .battlefield(1, &[hill_giant()])
        .start();
    keep_mulligans(&mut engine);

    let spider = on_battlefield(&engine, p0, root_spider()).unwrap();
    let giant = on_battlefield(&engine, p1, hill_giant()).unwrap();
    assert_eq!(pt(&engine, spider), (2, 2));

    // Advance to p1's declare-attackers step; p1 attacks with 3/3 Hill Giant.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(giant, Defender::Player(p0))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });

    // p0 blocks with 2/2 Root Spider -> becomes 3/2 with first strike and kills the 3/3 Hill Giant without dying!
    engine
        .apply(
            p0,
            PlayerAction::DeclareBlockers {
                blockers: vec![(spider, giant)],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });

    assert_eq!(pt(&engine, spider), (3, 2));
    assert_eq!(
        engine.state().object(spider).unwrap().zone,
        Zone::Battlefield,
        "3/2 first-striking Root Spider survives combat against 3/3 Hill Giant"
    );
    assert_eq!(
        engine.state().object(giant).unwrap().zone,
        Zone::Graveyard,
        "3/2 first-striking Root Spider kills the 3/3 Hill Giant before normal combat damage"
    );
}
