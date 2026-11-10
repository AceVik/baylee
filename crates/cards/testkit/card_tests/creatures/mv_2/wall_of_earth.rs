//! `cards/creatures/mv_2/wall_of_earth.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Wall of Earth is {1}{R} for a 0/6 Wall whose entire printed text is
/// "Defender". Defender restricts *attacking* and nothing else (CR 702.3b),
/// so the scenario reads both halves of it on one board: a turn after the Wall
/// resolved — so CR 302.6 cannot be what leaves it out — it is missing from
/// the declare-attackers offer while an untapped, non-sick Llanowar Elves
/// under the same seat is on it; and when the Elf across the table attacks,
/// that same Wall *is* offered as a blocker. A keyword read as "can't attack
/// or block" would pass the first assertion and fail the second, so the
/// control creature and the block together are what make the absence mean
/// "Defender" rather than "sick" or "nobody can attack here".
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn wall_of_earth_cannot_attack_but_still_blocks() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(), mountain(), llanowar_elves()])
        .hand(0, &[wall_of_earth()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // {1}{R} out of the two Mountains, with the Elf named as the printing kept
    // back: it is the untapped, non-sick creature the attack declaration is
    // read against, and a creature tapped for mana may not attack at all.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "two Mountains, and the Elf still standing"
    );
    cast_with_floating(&mut engine, p0, wall_of_earth());
    pass_until(&mut engine, |e| at_rest(e, p0));
    let wall = on_battlefield(&engine, p0, wall_of_earth()).expect("the Wall resolved");
    assert_eq!(pt(&engine, wall), (0, 6), "the body the card prints");
    assert!(
        keywords(&engine, wall).contains(KeywordSet::DEFENDER),
        "and the one keyword it prints"
    );

    // p1's turn: the Elf across the table attacks, and the Wall may block it.
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p1),
    );
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(theirs, Defender::Player(p0))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let Pending::ChooseBlockers {
        player, blockers, ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until only stops on the declare-blockers step")
    };
    assert_eq!(player, p0, "the seat being attacked declares the blockers");
    let pair = blockers
        .iter()
        .find(|option| option.blocker == wall)
        .expect("the Wall may block — Defender restricts attacking and nothing else");
    assert!(
        pair.attackers.contains(&theirs),
        "and it may block the attacking Elf: {:?}",
        pair.attackers
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareBlockers {
                blockers: vec![(wall, theirs)],
            },
        )
        .unwrap();

    // Onto p0's next turn. Walked by hand rather than with `pass_until`
    // because the cleanup step of the turn in between asks a seat to discard
    // down to seven cards, and that question has no arm in the shared walker.
    let mut arrival = false;
    for _ in 0..200 {
        match engine.pending().clone() {
            Pending::ChooseAttackers { player, .. } if player == p0 => {
                arrival = true;
                break;
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseAttackers { player, .. } => {
                engine
                    .apply(player, PlayerAction::DeclareAttackers { attackers: vec![] })
                    .unwrap();
            }
            Pending::ChooseBlockers { player, .. } => {
                engine
                    .apply(player, PlayerAction::DeclareBlockers { blockers: vec![] })
                    .unwrap();
            }
            Pending::DiscardChoice { player, count } => {
                let hand = engine
                    .state()
                    .zones
                    .list(ZoneLocation::Hand(player))
                    .clone();
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: hand.into_iter().take(usize::from(count)).collect(),
                        },
                    )
                    .unwrap();
            }
            other => panic!("unexpected on the way to p0's next attack step: {other:?}"),
        }
    }
    assert!(arrival, "p0 takes another turn");

    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elf is still out");
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("the loop only leaves on the attack declaration")
    };
    assert!(
        attackers.contains(&elves),
        "an untapped Elf past summoning sickness may attack: {attackers:?}"
    );
    assert!(
        !attackers.contains(&wall),
        "while the Wall, out since a turn ago, may not — that is \"Defender\" \
         (CR 702.3b) and not summoning sickness: {attackers:?}"
    );
    engine
        .apply(p0, PlayerAction::DeclareAttackers { attackers: vec![] })
        .unwrap();
    assert!(
        on_battlefield(&engine, p0, wall_of_earth()).is_some(),
        "and the 0/6 that blocked a 1/1 a turn earlier is still on the battlefield"
    );
}
