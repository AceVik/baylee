//! `cards/lands/manlands/restless_vents.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Restless Vents: "This land enters tapped." / "{T}: Add {B} or {R}." / "{1}{B}{R}: Until end of turn, this land becomes a 2/3 black and red Insect creature with menace. It's still a land."
/// Playing this land causes it to enter tapped, and paying `{1}{B}{R}` after untapping animates it into a 2/3 Insect creature with menace.
#[test]
fn restless_vents_animates_into_insect_with_menace() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(317, forest())
        .battlefield(0, &[swamp(), mountain(), forest()])
        .hand(0, &[restless_vents()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, restless_vents());
    assert!(entered_tapped(&engine, land));

    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(!is_tapped(&engine, land));

    tap_mana_except(&mut engine, p0, land);
    activate(&mut engine, p0, restless_vents(), 1);

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, land), (2, 3));
    let types = engine.state().object(land).unwrap().characteristics().types;
    assert!(types.contains(TypeSet::CREATURE));
    assert!(types.contains(TypeSet::LAND));
    assert!(keywords(&engine, land).contains(KeywordSet::MENACE));
    assert!(!is_tapped(&engine, land));
}

#[test]
fn restless_vents_draws_only_after_an_attack_discards_a_card() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for pay in [false, true] {
        let mut e = Duel::new(SEED, forest())
            .battlefield(0, &[restless_vents(), swamp(), mountain(), forest()])
            .hand(0, &[forest()])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p0);
        let land = on_battlefield(&e, p0, restless_vents()).unwrap();
        tap_mana_except(&mut e, p0, land);
        activate(&mut e, p0, restless_vents(), 1);
        pass_until(&mut e, stack_is_empty);
        pass_until(&mut e, |e| {
            matches!(e.pending(), Pending::ChooseAttackers { .. })
        });
        e.apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(land, Defender::Player(p1))],
            },
        )
        .unwrap();
        pass_until(&mut e, |e| {
            matches!(
                e.pending(),
                Pending::ChooseCards {
                    prompt: ChoicePrompt::CostDiscard,
                    ..
                }
            )
        });
        let card = in_hand(&e, p0, forest()).unwrap();
        let before = library_size(&e, p0);
        e.apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: if pay { vec![card] } else { vec![] },
            },
        )
        .unwrap();
        pass_until(&mut e, stack_is_empty);
        assert_eq!(library_size(&e, p0), before - usize::from(pay));
        assert_eq!(
            e.state().object(card).unwrap().zone,
            if pay { Zone::Graveyard } else { Zone::Hand }
        );
    }
}

/// "Whenever this land attacks, you may discard a card. If you do, draw a
/// card." With an empty hand there is nothing to discard, so nothing is drawn:
/// the hand stays empty and the library keeps every card, whatever the
/// optional question is answered with.
#[test]
fn restless_vents_with_an_empty_hand_attacks_without_drawing() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[restless_vents(), swamp(), mountain(), forest()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let land = on_battlefield(&e, p0, restless_vents()).unwrap();
    assert!(
        e.state().zones.list(ZoneLocation::Hand(p0)).is_empty(),
        "the hand is empty"
    );
    tap_mana_except(&mut e, p0, land);
    activate(&mut e, p0, restless_vents(), 1);
    pass_until(&mut e, stack_is_empty);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let library = library_size(&e, p0);
    e.apply(
        p0,
        PlayerAction::DeclareAttackers {
            attackers: vec![(land, Defender::Player(p1))],
        },
    )
    .unwrap();
    // Step through the trigger by hand: answer a discard prompt (if one is
    // asked at all) with nothing, and let nothing else answer for us.
    let mut triggered = false;
    for _ in 0..40 {
        match e.pending().clone() {
            Pending::ChooseCards {
                prompt: ChoicePrompt::CostDiscard,
                options,
                ..
            } => {
                assert!(options.is_empty(), "nothing to discard: {options:?}");
                e.apply(p0, PlayerAction::ChooseObjects { objects: vec![] })
                    .unwrap();
            }
            Pending::YesNo { player, .. } => {
                e.apply(player, PlayerAction::YesNo(true)).unwrap();
            }
            Pending::Priority { .. } if stack_is_empty(&e) => break,
            Pending::Priority { player, .. } => {
                triggered = true;
                e.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected question: {other:?}"),
        }
    }
    assert!(triggered, "the attack trigger was put on the stack");
    assert!(stack_is_empty(&e), "the trigger has finished");
    assert!(
        e.state().zones.list(ZoneLocation::Hand(p0)).is_empty(),
        "no card was discarded and none was drawn: the hand is still empty"
    );
    assert_eq!(
        library_size(&e, p0),
        library,
        "\"if you do\": no discard, so no draw, and the library is untouched"
    );
}
