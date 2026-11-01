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
    // A library of Forests holding one basic Plains and one nonbasic Plains
    // (Savannah): "basic Plains" offers the first alone.
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains()])
        .hand(0, &[the_birth_of_meletis(), plains(), savannah()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let basic = in_hand(&e, p0, plains()).expect("the basic Plains");
    let nonbasic = in_hand(&e, p0, savannah()).expect("Savannah");
    for card in [basic, nonbasic] {
        e.state
            .move_object(
                card,
                ZoneLocation::Library(p0),
                crate::zone::ZonePosition::Top,
                crate::event::Cause::Effect,
            )
            .unwrap();
    }
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
    assert_eq!(
        options,
        vec![basic],
        "neither a Forest nor a nonbasic Plains is offered"
    );
    let found = basic;
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

fn savannah() -> CardIndex {
    card_index("703243f0-8cb3-420f-958f-5fd4bde30293")
}
