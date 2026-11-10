//! `cards/creatures/mv_2/talas_scout.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Talas Scout is `{1}{U}` for a 1/2 Human Pirate Scout whose entire text is
/// flying, and the set of things a test could get wrong is exactly the set an
/// empty-bodied card would hide behind: the body, the keyword, and the fact
/// that the keyword means something in a combat step. The Scout is cast off
/// two Islands, read off the battlefield (1/2, a creature, flying), left a
/// whole turn cycle so that summoning sickness (CR 302.6) is behind it, and
/// then sent at the opponent across a ground Elf. It arrives unblocked because
/// flying is a *pairing* rule (CR 702.9b) — the Elf is a legal blocker for
/// creatures in general and is offered nothing against a flier — and the one
/// life the 1/2 takes is the evidence that the attack happened at all.
#[test]
fn talas_scout_flies_over_the_ground_and_takes_a_life() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island()])
        .hand(0, &[talas_scout()])
        // A creature on the ground: "can it be blocked" is a question about a
        // board with something on the other side of the table to ask it with.
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    cast_from_hand(&mut engine, p0, talas_scout());
    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, talas_scout()).is_some()
    });
    let scout = on_battlefield(&engine, p0, talas_scout()).expect("the Scout resolved");
    let kinds = types(&engine, scout);
    assert!(
        kinds.contains(TypeSet::CREATURE),
        "a creature, and not the sorcery the rest of the pool is: {kinds:?}"
    );
    assert_eq!(pt(&engine, scout), (1, 2), "the body the card prints");
    assert!(
        keywords(&engine, scout).contains(KeywordSet::FLYING),
        "and the flying its whole text is"
    );

    // A turn cycle, because the Scout arrived this turn: CR 302.6 keeps a
    // creature that has not been under its controller's control since their
    // most recent turn began from attacking, and the attack below is the only
    // reading of flying this board can offer.
    reach_their_main_phase(&mut engine, p1);
    assert!(walk_to_own_main(&mut engine, p0), "back to p0's own main");

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("pass_until stopped on nothing but the attack declaration")
    };
    assert!(
        attackers.contains(&scout),
        "an unsick, untapped 1/2 may attack: {attackers:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(scout, Defender::Player(p1))],
            },
        )
        .expect("the Scout was offered as an attacker");

    // Flying is a *pairing* rule (CR 702.9b), so the ground Elf is never
    // offered against the Scout. The walk answers whatever priority is handed
    // out on the way rather than assuming the question comes immediately, and
    // whether the engine asks at all is not what is asserted.
    for _ in 0..20 {
        match engine.pending().clone() {
            Pending::ChooseBlockers {
                player, blockers, ..
            } => {
                assert_eq!(player, p1, "the attacked seat is the one asked");
                for option in blockers {
                    assert!(
                        !option.attackers.contains(&scout),
                        "a ground creature cannot be paired with a flier: {option:?}"
                    );
                }
                break;
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            _ => break,
        }
    }

    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });
    assert_eq!(
        engine.state().players[1].life,
        19,
        "the unblocked 1/2 took exactly one life — the Islands on p0's board \
         have no way of dealing damage, so the point can only be the Scout's"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "and the Elf that was offered nothing to block is still standing"
    );
}
