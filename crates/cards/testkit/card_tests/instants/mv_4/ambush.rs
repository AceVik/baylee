//! Ambush — `{3}{R}` Instant (Homelands)
//!
//! Blocking creatures gain first strike until end of turn.

#[allow(clippy::wildcard_imports)]
use super::*;

fn ambush() -> CardIndex {
    card_index("05828c1e-9185-4b70-94fe-753eacdf45a1")
}

#[test]
fn ambush_grants_first_strike_only_to_blocking_creatures_until_end_of_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .hand(1, &[ambush()])
        .battlefield(0, &[grizzly_bears()])
        .battlefield(
            1,
            &[
                grizzly_bears(),
                gray_ogre(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let attacker = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let blocker = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    let bystander = on_battlefield(&engine, p1, gray_ogre()).unwrap();

    // p0 attacks with 2/2 Grizzly Bears; p1 blocks with 2/2 Grizzly Bears.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(attacker, Defender::Player(p1))],
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
                blockers: vec![(blocker, attacker)],
            },
        )
        .unwrap();

    // In DeclareBlockers step, p0 passes priority to p1, who casts Ambush ({3}{R}).
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    let ambush_hand = in_hand(&engine, p1, ambush()).unwrap();
    tap_all_mana(&mut engine, p1);
    engine
        .apply(p1, PlayerAction::CastSpell { card: ambush_hand })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        keywords(&engine, blocker).contains(KeywordSet::FIRST_STRIKE),
        "blocking creature gains first strike"
    );
    assert!(
        !keywords(&engine, bystander).contains(KeywordSet::FIRST_STRIKE),
        "nonblocking creature does not gain first strike"
    );
    assert!(
        !keywords(&engine, attacker).contains(KeywordSet::FIRST_STRIKE),
        "attacking creature does not gain first strike"
    );

    // Combat damage resolves: first-striking 2/2 blocker kills the 2/2 attacker without taking damage.
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert_eq!(
        engine.state().object(attacker).unwrap().zone,
        Zone::Graveyard,
        "2/2 attacker dies to first-strike combat damage"
    );
    assert_eq!(
        engine.state().object(blocker).unwrap().zone,
        Zone::Battlefield,
        "2/2 blocker with first strike survives unscathed"
    );
    assert_eq!(engine.state().object(blocker).unwrap().damage, 0);
}
