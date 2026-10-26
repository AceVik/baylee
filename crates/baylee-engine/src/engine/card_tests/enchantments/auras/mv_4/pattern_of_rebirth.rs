//! `cards/enchantments/auras/mv_4/pattern_of_rebirth.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

fn pattern_of_rebirth() -> CardIndex {
    card_index("4b8856f8-3cd1-450c-a71b-eb0026e8e179")
}

/// Pattern of Rebirth (4b8856f8-3cd1-450c-a71b-eb0026e8e179) — "Enchant
/// creature / When enchanted creature dies, that creature's controller may
/// search their library for a creature card and put that card onto the
/// battlefield. If that player does, they shuffle."
#[test]
fn pattern_of_rebirth_lets_dying_creatures_controller_search_library_for_creature_to_battlefield() {
    let mut e = lk::game(
        SEED,
        &[llanowar_elves()],
        &[],
        &[
            pattern_of_rebirth(),
            lightning_bolt(),
            rootbreaker_wurm(),
            sol_ring(),
        ],
        &[],
    );
    let elf = lk::obj(&e, lk::P0, llanowar_elves());
    let wurm = hand_to_library_top(&mut e, lk::P0, rootbreaker_wurm());
    let ring = hand_to_library_top(&mut e, lk::P0, sol_ring());

    lk::float(&mut e, lk::P0, &[(ManaColor::Green, 4)]);
    cast_with_floating(&mut e, lk::P0, pattern_of_rebirth());
    e.apply(lk::P0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .unwrap();
    lk::settle(&mut e);

    // Kill the enchanted Elf; P0 gets MayDo and searches P0's library for a creature card.
    lk::float(&mut e, lk::P0, &[(ManaColor::Red, 1)]);
    cast_with_floating(&mut e, lk::P0, lightning_bolt());
    lk::aim(&mut e, &[elf], &[]);
    lk::pass_once(&mut e);
    lk::pass_once(&mut e);
    // Pattern of Rebirth trigger is now on the stack; resolve it to reach MayDo.
    lk::pass_once(&mut e);
    lk::pass_once(&mut e);

    let Pending::YesNo { player, .. } = e.pending().clone() else {
        panic!("expected MayDo question, got {:?}", e.pending());
    };
    assert_eq!(player, lk::P0);
    lk::yes_no(&mut e, true);

    let Pending::ChooseCards {
        player,
        options,
        prompt: ChoicePrompt::SearchLibrary,
        ..
    } = e.pending().clone()
    else {
        panic!("expected library search, got {:?}", e.pending());
    };
    assert_eq!(player, lk::P0);
    assert!(
        options.contains(&wurm),
        "creature card in library is offered"
    );
    assert!(
        !options.contains(&ring),
        "noncreature card in library is excluded"
    );
    e.apply(
        lk::P0,
        PlayerAction::ChooseObjects {
            objects: vec![wurm],
        },
    )
    .unwrap();
    lk::settle(&mut e);

    assert!(
        on_battlefield(&e, lk::P0, rootbreaker_wurm()).is_some(),
        "searched creature enters the battlefield under P0's control"
    );
}

/// Pattern of Rebirth — when attached to an opponent's creature that dies,
/// THAT creature's controller (P1), not the Aura's controller (P0), gets the
/// optional library search and puts the creature onto the battlefield under
/// their own control.
#[test]
fn pattern_of_rebirth_on_opponents_creature_lets_opponent_search_their_own_library() {
    let mut e = lk::game(
        SEED,
        &[],
        &[llanowar_elves()],
        &[pattern_of_rebirth(), lightning_bolt()],
        &[gray_ogre()],
    );
    let their_elf = lk::obj(&e, lk::P1, llanowar_elves());
    let their_ogre = hand_to_library_top(&mut e, lk::P1, gray_ogre());

    lk::float(&mut e, lk::P0, &[(ManaColor::Green, 4)]);
    cast_with_floating(&mut e, lk::P0, pattern_of_rebirth());
    e.apply(
        lk::P0,
        PlayerAction::ChooseObjects {
            objects: vec![their_elf],
        },
    )
    .unwrap();
    lk::settle(&mut e);

    lk::float(&mut e, lk::P0, &[(ManaColor::Red, 1)]);
    cast_with_floating(&mut e, lk::P0, lightning_bolt());
    lk::aim(&mut e, &[their_elf], &[]);
    lk::pass_once(&mut e);
    lk::pass_once(&mut e);
    lk::pass_once(&mut e);
    lk::pass_once(&mut e);

    let Pending::YesNo { player, .. } = e.pending().clone() else {
        panic!("expected MayDo question, got {:?}", e.pending());
    };
    assert_eq!(
        player,
        lk::P1,
        "enchanted creature's controller (P1) decides whether to search"
    );
    lk::yes_no(&mut e, true);

    let Pending::ChooseCards {
        player, options, ..
    } = e.pending().clone()
    else {
        panic!("expected library search for P1, got {:?}", e.pending());
    };
    assert_eq!(player, lk::P1);
    assert!(options.contains(&their_ogre));
    e.apply(
        lk::P1,
        PlayerAction::ChooseObjects {
            objects: vec![their_ogre],
        },
    )
    .unwrap();
    lk::settle(&mut e);

    assert!(
        on_battlefield(&e, lk::P1, gray_ogre()).is_some(),
        "searched creature enters under P1's control"
    );
    assert!(on_battlefield(&e, lk::P0, gray_ogre()).is_none());
}
