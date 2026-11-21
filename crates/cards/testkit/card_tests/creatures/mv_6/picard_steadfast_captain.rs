//! `cards/creatures/mv_6/picard_steadfast_captain.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn picard_steadfast_captain() -> CardIndex {
    card_index("cda786aa-1259-45e8-a834-21c7ad0259a6")
}

/// Picard puts two +1/+1 counters on another target creature you control both
/// when Picard enters and when Picard attacks, excluding Picard himself and
/// opposing creatures, and ignoring attacks by other creatures alone.
#[test]
fn picard_puts_two_counters_on_another_controlled_creature_on_enter_and_attack() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                grizzly_bears(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[gray_ogre()])
        .hand(0, &[picard_steadfast_captain()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    let bears = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let elves = on_battlefield(&engine, p0, llanowar_elves()).unwrap();
    let opp_ogre = on_battlefield(&engine, p1, gray_ogre()).unwrap();

    let lands = all_on_battlefield(&engine, p0, plains());
    tap_mana_where(&mut engine, p0, |id| lands.contains(&id));
    let spell = in_hand(&engine, p0, picard_steadfast_captain()).unwrap();
    engine
        .apply(p0, PlayerAction::CastSpell { card: spell })
        .unwrap();

    let options = pass_until_targets(&mut engine, p0);
    let picard = on_battlefield(&engine, p0, picard_steadfast_captain()).expect("resolved");
    assert!(options.contains(&bears) && options.contains(&elves));
    assert!(!options.contains(&picard), "\"another\" excludes Picard");
    assert!(
        !options.contains(&opp_ogre),
        "\"you control\" excludes opponent"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bears],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, bears), (4, 4), "two +1/+1 counters on ETB");
    assert_eq!(pt(&engine, picard), (4, 4));

    // Attacking with Llanowar Elves alone this turn does NOT trigger Picard.
    let _ = attack_and_collect_blocks(&mut engine, elves, p1);
    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert_eq!(pt(&engine, bears), (4, 4));

    // Next turn, Picard attacks -> puts two more +1/+1 counters on Grizzly Bears (6/6).
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(picard, Defender::Player(p1))],
            },
        )
        .unwrap();
    let attack_options = pass_until_targets(&mut engine, p0);
    assert!(!attack_options.contains(&picard));
    assert!(!attack_options.contains(&opp_ogre));
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bears],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    assert_eq!(pt(&engine, bears), (6, 6));
}
