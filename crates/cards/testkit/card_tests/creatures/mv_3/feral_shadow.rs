//! `cards/creatures/mv_3/feral_shadow.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Feral Shadow prints one line — flying — on a {2}{B} 2/1 Nightstalker, so
/// the whole card is whether that body arrives off the printed cost and
/// whether the keyword is what the combat step reads. Three Swamps pay
/// {2}{B} exactly and leave nothing floating, the layer projection hands back
/// a 2/1 with flying, and the attack is declared past a Llanowar Elves: a
/// ground creature with neither flying nor reach may never be offered as its
/// blocker (CR 509.1b), and the two damage it deals are the only thing on
/// this board that can move a life total.
#[test]
fn feral_shadow_arrives_as_a_two_one_flier_that_a_ground_creature_cannot_block() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(41, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[feral_shadow()])
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // The printed {2}{B} off three Swamps. The pool is read afterwards so
    // that the body below is the card's and not a leftover of the payment.
    cast_from_hand(&mut engine, p0, feral_shadow());
    pass_until(&mut engine, stack_is_empty);
    let shadow = on_battlefield(&engine, p0, feral_shadow()).expect("the Shadow resolved");
    assert_eq!(pt(&engine, shadow), (2, 1), "the printed body");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "three Swamps paid {{2}}{{B}} and nothing came back"
    );
    assert!(
        keywords(&engine, shadow).contains(KeywordSet::FLYING),
        "which is the whole of its rules text"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "an Elf across the table is the control for the keyword"
    );

    // A whole turn cycle: the Shadow was cast this turn and is summoning
    // sick (CR 302.6), so the attack only becomes available on p0's next one.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers {
        player, attackers, ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stops on nothing but the attack declaration")
    };
    assert_eq!(player, p0, "the active player declares attackers");
    assert!(
        attackers.contains(&shadow),
        "an untapped 2/1 with no other duty is offered as an attacker: {attackers:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(shadow, Defender::Player(p1))],
            },
        )
        .unwrap();

    // The block step, walked by hand so that the menu itself can be read.
    // The Elf is p1's whole board, so one entry in this list would be the
    // keyword missing rather than a board that had nothing to block with.
    for _ in 0..20 {
        if matches!(engine.state().turn.phase, Phase::Ending) {
            break;
        }
        match engine.pending().clone() {
            Pending::ChooseBlockers {
                player, blockers, ..
            } => {
                assert_eq!(player, p1, "the defending player is asked to block");
                assert!(
                    blockers.is_empty(),
                    "a Llanowar Elves has neither flying nor reach, so it is \
                     no legal blocker for a flier (CR 509.1b): {blockers:?}"
                );
                engine
                    .apply(player, PlayerAction::DeclareBlockers { blockers: vec![] })
                    .unwrap();
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected between attackers and damage: {other:?}"),
        }
    }

    assert_eq!(
        engine.state().players[1].life,
        18,
        "two damage to the player it flew over"
    );
    assert!(
        on_battlefield(&engine, p0, feral_shadow()).is_some(),
        "and the Shadow is a live creature afterwards: nothing blocked it"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "while the Elf across the table never moved"
    );
}
