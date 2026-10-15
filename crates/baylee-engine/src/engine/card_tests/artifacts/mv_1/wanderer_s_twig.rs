//! Oracle checklist: pay {1} and sacrifice immediately; search only for a basic land card, reveal it, put it in the controller's hand, and shuffle.
#[allow(clippy::wildcard_imports)]
use super::*;

fn wanderers_twig() -> CardIndex {
    card_index("fd15a4b0-abd7-4bae-8331-81b85c94fce8")
}

#[test]
fn twig_searches_for_a_basic_land_and_sacrifices_before_searching() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[wanderers_twig()])
        .hand(0, &[sapseep_forest(), island(), llanowar_elves()])
        .start();
    keep_mulligans(&mut e);
    assert!(walk_to_own_main(&mut e, p0));
    let nonbasic = hand_to_library_top(&mut e, p0, sapseep_forest());
    let basic_island = hand_to_library_top(&mut e, p0, island());
    let nonland = hand_to_library_top(&mut e, p0, llanowar_elves());
    e.dev_state_mut(p0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::Colorless, 1);
    e.refresh_offer();
    let baseline = e.state().journal.entries().len();
    activate(&mut e, p0, wanderers_twig(), 0);
    assert!(
        on_battlefield(&e, p0, wanderers_twig()).is_none(),
        "the Twig is sacrificed as a cost"
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

    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
        ..
    } = e.pending().clone()
    else {
        panic!("the search asks for a land card: {:?}", e.pending());
    };
    assert_eq!(player, p0);
    assert_eq!(prompt, ChoicePrompt::SearchLibrary);
    assert_eq!((min, max), (0, 1));
    assert!(
        !options.contains(&nonbasic),
        "a nonbasic land is ineligible"
    );
    assert!(
        options.contains(&basic_island),
        "a basic Island is eligible"
    );
    assert!(!options.contains(&nonland), "a nonland card is ineligible");
    let land = options
        .into_iter()
        .find(|id| {
            e.state()
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == forest()))
        })
        .expect("the library contains basic Forest cards");
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![land],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);

    assert_eq!(e.state().object(land).unwrap().zone, Zone::Hand);
    assert!(
        in_hand(&e, p0, forest()).is_some(),
        "the selected basic land is in the searcher's hand"
    );
    assert!(e.state().journal.entries()[baseline..].iter().any(|entry| matches!(
        &entry.event,
        crate::event::GameEvent::Revealed { player, cards } if *player == p0 && cards.contains(&land)
    )), "the chosen land is revealed");
    assert!(
        e.state().journal.entries()[baseline..]
            .iter()
            .any(|entry| matches!(
                &entry.event,
                crate::event::GameEvent::Shuffled { player, zone: Zone::Library } if *player == p0
            )),
        "the library is shuffled after searching"
    );
    assert!(in_graveyard(&e, p0, wanderers_twig()).is_some());
}
