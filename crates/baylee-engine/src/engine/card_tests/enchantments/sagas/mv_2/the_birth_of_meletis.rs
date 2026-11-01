//! `cards/enchantments/sagas/mv_2/the_birth_of_meletis.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn the_birth_of_meletis() -> CardIndex {
    card_index("1ae54fe7-b1d3-4c13-a8ef-f502cf3eb1a0")
}

/// All three chapters resolve in natural turn order, including the final
/// sacrifice and the complete type line of chapter II's Wall token.
#[test]
fn birth_of_meletis_plays_all_chapters_then_sacrifices_itself() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains()])
        .hand(0, &[the_birth_of_meletis()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    cast_from_hand(&mut e, p0, the_birth_of_meletis());
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        options,
        prompt,
        min,
        max,
        ..
    } = e.pending().clone()
    else {
        unreachable!("chapter I searches the library")
    };
    assert_eq!(prompt, ChoicePrompt::SearchLibrary);
    assert_eq!(
        (min, max),
        (0, 1),
        "a hidden-library search for a named quality may fail to find"
    );
    assert!(!options.is_empty());
    assert!(options.iter().all(|id| {
        e.state()
            .object(*id)
            .and_then(|o| o.card)
            .is_some_and(|card| card.index == plains())
    }));
    let found = options[0];
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![found],
        },
    )
    .expect("a basic Plains is the printed search");
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        e.state().object(found).expect("found Plains").zone,
        Zone::Hand
    );
    let saga = on_battlefield(&e, p0, the_birth_of_meletis()).expect("the Saga");
    assert_eq!(counters_on(&e, saga, CounterKind::Lore), 1);

    reach_their_main_phase(&mut e, p1);
    reach_their_main_phase(&mut e, p0);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(counters_on(&e, saga, CounterKind::Lore), 2);
    let tokens = tokens_of(&e, p0);
    assert_eq!(tokens.len(), 1, "chapter II creates one token");
    let wall = tokens[0];
    let projected = types(&e, wall);
    assert!(projected.contains(TypeSet::ARTIFACT));
    assert!(projected.contains(TypeSet::CREATURE));
    assert!(
        e.state()
            .object(wall)
            .expect("Wall")
            .characteristics()
            .colors
            .is_empty()
    );
    assert!(
        e.state()
            .object(wall)
            .expect("Wall")
            .characteristics()
            .subtypes
            .contains(baylee_core::generated::subtypes::creature::WALL,)
    );
    assert_eq!(pt(&e, wall), (0, 4));
    assert!(keywords(&e, wall).contains(KeywordSet::DEFENDER));

    reach_their_main_phase(&mut e, p1);
    reach_their_main_phase(&mut e, p0);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().players[0].life, 22, "chapter III gains two life");
    assert!(in_graveyard(&e, p0, the_birth_of_meletis()).is_some());
    assert!(on_battlefield(&e, p0, the_birth_of_meletis()).is_none());
    assert_eq!(
        e.state().object(wall).expect("Wall remains").zone,
        Zone::Battlefield
    );
}
