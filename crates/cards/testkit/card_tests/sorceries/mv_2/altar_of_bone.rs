//! `cards/sorceries/mv_2/altar_of_bone.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

const P0: PlayerId = PlayerId::new(0);

fn altar_of_bone() -> CardIndex {
    card_index("607a3f5f-4a36-4077-b5cf-40ffd52fa580")
}

/// Altar of Bone (607a3f5f-4a36-4077-b5cf-40ffd52fa580) — {G}{W} Sorcery:
/// - "As an additional cost to cast this spell, sacrifice a creature."
/// - "Search your library for a creature card, reveal it, put it into your hand,
///   then shuffle."
#[test]
fn altar_of_bone_requires_sacrificing_a_creature_and_tutors_a_creature_card_to_hand() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), plains(), llanowar_elves()])
        .hand(0, &[altar_of_bone(), gray_ogre(), unsummon()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let ogre_in_lib = hand_to_library_top(&mut engine, P0, gray_ogre());
    let spell_in_lib = hand_to_library_top(&mut engine, P0, unsummon());
    let elf = on_battlefield(&engine, P0, llanowar_elves()).expect("Elf on battlefield");

    tap_all_mana_but(&mut engine, P0, Some(llanowar_elves()));
    cast_with_floating(&mut engine, P0, altar_of_bone());

    // Additional cost: sacrifice a creature.
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected sacrifice choice, got {:?}", engine.pending());
    };
    assert_eq!(player, P0);
    assert_eq!((min, max), (1, 1));
    assert_eq!(options, vec![elf]);
    engine
        .apply(P0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .expect("sacrifice Llanowar Elves");
    assert!(on_battlefield(&engine, P0, llanowar_elves()).is_none());
    assert!(in_graveyard(&engine, P0, llanowar_elves()).is_some());

    // Resolve Altar of Bone -> search library for a creature card.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!("expected library search, got {:?}", engine.pending());
    };
    assert_eq!(player, P0);
    assert!(
        options.contains(&ogre_in_lib),
        "creature card in library is offered"
    );
    assert!(
        !options.contains(&spell_in_lib),
        "non-creature card in library is excluded"
    );

    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![ogre_in_lib],
            },
        )
        .expect("choose Gray Ogre");
    pass_until(&mut engine, stack_is_empty);

    assert!(in_hand(&engine, P0, gray_ogre()).is_some());
    assert!(in_graveyard(&engine, P0, altar_of_bone()).is_some());
}
