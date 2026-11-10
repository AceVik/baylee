//! Oracle: {T}: Target player draws a card, then discards a card.
#[allow(clippy::wildcard_imports)] // Shared behavioral harness.
use super::*;

fn subject() -> CardIndex {
    card_index("412294bf-a25f-47c2-8043-12eb3e142b9e")
}

// Checklist: tap and summoning restriction; both target players, draw before
// discard (newly drawn card may be discarded); target chooses, caster unchanged;
// source cannot repeat while tapped.
#[test]
fn target_player_draws_then_can_discard_the_drawn_card() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    for recipient in [p0, p1] {
        let mut e = Duel::new(SEED, forest())
            .battlefield(0, &[subject()])
            .start();
        keep_mulligans(&mut e);
        assert!(walk_to_own_main(&mut e, p0));
        let source = on_battlefield(&e, p0, subject()).unwrap();
        let hand = e.state().zones.list(ZoneLocation::Hand(recipient)).clone();
        let library = library_size(&e, recipient);
        let other = if recipient == p0 { p1 } else { p0 };
        let other_hand = e.state().zones.list(ZoneLocation::Hand(other)).clone();
        activate(&mut e, p0, subject(), 0);
        let Pending::ChooseTargets {
            options,
            player_options,
            ..
        } = e.pending()
        else {
            panic!("target player")
        };
        assert!(options.is_empty());
        assert_eq!(player_options.len(), 2);
        e.apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![recipient],
            },
        )
        .unwrap();
        assert!(is_tapped(&e, source));
        pass_until(&mut e, |e| {
            matches!(
                e.pending(),
                Pending::ChooseCards { .. } | Pending::DiscardChoice { .. }
            )
        });
        let now = e.state().zones.list(ZoneLocation::Hand(recipient)).clone();
        assert_eq!(now.len(), hand.len() + 1);
        assert_eq!(library_size(&e, recipient), library - 1);
        let drawn = *now.iter().find(|id| !hand.contains(id)).unwrap();
        let choosing = match e.pending() {
            Pending::ChooseCards { player, .. } | Pending::DiscardChoice { player, .. } => *player,
            _ => unreachable!(),
        };
        assert_eq!(choosing, recipient);
        e.apply(
            recipient,
            PlayerAction::ChooseObjects {
                objects: vec![drawn],
            },
        )
        .unwrap();
        pass_until(&mut e, stack_is_empty);
        assert_eq!(e.state().zones.list(ZoneLocation::Hand(recipient)), &hand);
        assert!(
            e.state()
                .zones
                .list(ZoneLocation::Graveyard(recipient))
                .contains(&drawn)
        );
        assert_eq!(e.state().zones.list(ZoneLocation::Hand(other)), &other_hand);
        let Pending::Priority { legal, .. } = e.pending() else {
            panic!("priority")
        };
        assert!(!legal.abilities.iter().any(|(id, _)| *id == source));
    }
}
#[test]
fn freshly_cast_looter_cannot_tap_yet() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[island(), island(), island()])
        .hand(0, &[subject()])
        .start();
    keep_mulligans(&mut e);
    assert!(walk_to_own_main(&mut e, p0));
    cast_from_hand(&mut e, p0, subject());
    pass_until(&mut e, stack_is_empty);
    let source = on_battlefield(&e, p0, subject()).unwrap();
    let Pending::Priority { legal, .. } = e.pending() else {
        panic!("priority")
    };
    assert!(!legal.abilities.iter().any(|(id, _)| *id == source));
}
