//! Oracle checklist: pay {1}, sacrifice Amulet; search only for a basic land, reveal it, put it in hand, shuffle; nonbasic/nonland cards are excluded and a land enters hand.
#[allow(clippy::wildcard_imports)]
use super::*;
fn travelers_amulet() -> CardIndex {
    card_index("48b561dd-f23d-409a-87c8-e49902b2477b")
}
#[test]
fn amulet_searches_and_puts_a_basic_land_into_hand() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[travelers_amulet(), forest(), forest()])
        .hand(0, &[irrigated_farmland(), sol_ring()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let nonbasic = hand_to_library_top(&mut e, p0, irrigated_farmland());
    let nonland = hand_to_library_top(&mut e, p0, sol_ring());
    let hand_before = e.state().zones.list(ZoneLocation::Hand(p0)).len();
    let since = e.journal().entries().len();
    let source = on_battlefield(&e, p0, travelers_amulet()).unwrap();
    tap_all_mana(&mut e, p0);
    activate(&mut e, p0, travelers_amulet(), 0);
    assert!(in_graveyard(&e, p0, travelers_amulet()).is_some());
    assert_eq!(
        e.state().players[0]
            .mana_pool
            .available(baylee_core::mana::ManaColor::Green),
        1,
        "the {{1}} activation cost spends exactly one of two mana"
    );
    pass_until(&mut e, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards {
                prompt: ChoicePrompt::SearchLibrary,
                ..
            }
        )
    });
    let Pending::ChooseCards { options, .. } = e.pending().clone() else {
        unreachable!()
    };
    assert!(
        !options.contains(&nonbasic),
        "a nonbasic land cannot be found"
    );
    assert!(
        !options.contains(&nonland),
        "a nonland card cannot be found"
    );
    let basic = options
        .iter()
        .copied()
        .find(|id| {
            e.state()
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == forest()))
        })
        .expect("basic Forest is searchable");
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![basic],
        },
    )
    .expect("choose basic land");
    assert!(
        e.journal().entries()[since..].iter().any(|entry| {
            matches!(entry.event, crate::event::GameEvent::Revealed { cards: ref revealed, .. } if revealed.len() == 1 && revealed[0] == basic)
        }),
        "the chosen basic land itself is revealed"
    );
    assert!(
        e.journal().entries()[since..].iter().any(
            |entry| matches!(entry.event,crate::event::GameEvent::Shuffled{player,..} if player==p0)
        ),
        "the library is shuffled"
    );
    assert!(in_hand(&e, p0, forest()).is_some());
    assert_eq!(e.state().object(basic).unwrap().zone, Zone::Hand);
    assert_eq!(
        e.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1
    );
    assert!(
        e.state()
            .object(source)
            .is_some_and(|o| o.zone == Zone::Graveyard)
    );
}

#[test]
fn amulet_is_not_offered_without_its_one_mana_cost() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[travelers_amulet()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let source = on_battlefield(&e, p0, travelers_amulet()).unwrap();
    let Pending::Priority { legal, .. } = e.pending() else {
        panic!("the player has priority")
    };
    assert!(!legal.abilities.iter().any(|(id, _)| *id == source));
}
