//! CR 605.1a: an ability that moves a card to or from a library is no mana
//! ability, however the sentence that moves it is written.

use super::*;
use crate::{
    Amount, ChooseMove, ChooseZone, Filter, Find, ManaColor, PlayerRel, RevealUntilFound,
    RevealUntilRest, VoteOption, VoteOutcome, ZoneSel,
};

const ADD: Effect = Effect::mana(ManaColor::Blue, 1);
const DRAW: &[Effect] = &[Effect::draw(1)];
const ANY: &Filter = &Filter::CREATURE;

/// Whether an ability that adds mana and does `effect` is a mana ability.
fn stays_a_mana_ability(effect: Effect) -> bool {
    activated_mana_ability(&Cost::FREE, &[ADD, effect], false)
}

#[test]
fn revealing_until_a_card_is_a_move_unless_the_card_goes_back_on_the_library() {
    let until = |found| stays_a_mana_ability(Effect::RevealUntil { filter: ANY, found });
    assert!(until(SearchDest::TopOfLibrary));
    for found in [
        SearchDest::Hand,
        SearchDest::Battlefield,
        SearchDest::Graveyard,
        SearchDest::Exile,
    ] {
        assert!(!until(found), "{found:?}");
    }
}

#[test]
fn every_card_revealed_until_then_has_to_stay_in_the_library() {
    let until = |exile_while_revealing, found, rest, then| {
        stays_a_mana_ability(Effect::RevealUntilThen {
            library: PlayerRel::You,
            filter: ANY,
            count: Amount::Fixed(1),
            max_revealed: None,
            exile_while_revealing,
            found,
            rest,
            then,
        })
    };
    let stay = RevealUntilFound::StayInLibrary;
    for found in [
        stay,
        RevealUntilFound::SameAsRest,
        RevealUntilFound::TopOfLibrary,
    ] {
        assert!(
            until(false, found, RevealUntilRest::BottomRandom, &[]),
            "{found:?}"
        );
    }
    for rest in [
        RevealUntilRest::StayInLibrary,
        RevealUntilRest::TopOfLibrary,
        RevealUntilRest::BottomRandom,
        RevealUntilRest::BottomChosenOrder,
        RevealUntilRest::ShuffleIntoLibrary,
    ] {
        assert!(until(false, stay, rest, &[]), "{rest:?}");
    }
    let rest = RevealUntilRest::BottomRandom;
    assert!(!until(true, stay, rest, &[]), "exiled while revealing");
    assert!(!until(false, RevealUntilFound::Hand, rest, &[]));
    assert!(!until(false, stay, RevealUntilRest::Hand, &[]));
    assert!(!until(false, stay, rest, DRAW), "then draws");
}

#[test]
fn both_halves_of_a_sorted_reveal_have_to_stay_in_the_library() {
    use SearchDest::{Hand, TopOfLibrary};
    for (matched, otherwise, stays) in [
        (TopOfLibrary, TopOfLibrary, true),
        (Hand, TopOfLibrary, false),
        (TopOfLibrary, Hand, false),
        (Hand, Hand, false),
    ] {
        let find = |dest| Find { dest, ..Find::HAND };
        for effect in [
            Effect::RevealUntilMatching {
                player: PlayerRel::You,
                filter: ANY,
                matched,
                otherwise,
            },
            Effect::RevealTopAndSort {
                filter: ANY,
                matched,
                otherwise,
            },
            Effect::RevealTopFind {
                found: find(matched),
                count: 2,
                filter: ANY,
                otherwise,
            },
            Effect::LookAtTopMayPut {
                filter: ANY,
                matched: find(matched),
                otherwise,
            },
        ] {
            assert_eq!(stays_a_mana_ability(effect), stays, "{effect:?}");
        }
    }
}

#[test]
fn a_search_is_a_move_when_any_card_it_finds_leaves_the_library() {
    const TOP: Find = Find::TOP_OF_LIBRARY;
    const HAND: Find = Find::HAND;
    let search = |finds| {
        stays_a_mana_ability(Effect::SearchLibrary {
            filter: ANY,
            finds,
            optional: false,
        })
    };
    assert!(search(&[TOP]));
    assert!(!search(&[HAND]));
    assert!(!search(&[TOP, HAND]));
    assert!(!search(&[HAND, TOP]));
    for (find, stays) in [(&TOP, true), (&HAND, false)] {
        assert_eq!(
            stays_a_mana_ability(Effect::SearchLibraryUpTo {
                filter: ANY,
                count: Amount::Fixed(2),
                find,
            }),
            stays
        );
        assert_eq!(
            stays_a_mana_ability(Effect::RevealTopAllMatching {
                count: 3,
                filter: ANY,
                found: *find,
            }),
            stays
        );
    }
}

#[test]
fn choosing_a_card_is_a_move_when_it_goes_to_a_library_or_was_found_in_one() {
    let library = [
        ChooseMove::TopOfLibrary,
        ChooseMove::BottomOfLibrary,
        ChooseMove::BottomOfLibraryChosenOrder,
        ChooseMove::BottomOfLibraryRandom,
    ];
    let choose = |zone, movement, then| {
        stays_a_mana_ability(Effect::ChooseCard {
            chooser: PlayerRel::You,
            zone,
            filter: ANY,
            min: 0,
            max: Amount::Fixed(1),
            movement,
            then,
        })
    };
    let split = |zone, movement, not_chosen_movement, then, for_unchosen| {
        stays_a_mana_ability(Effect::ChooseCardEx {
            chooser: PlayerRel::You,
            zone,
            filter: ANY,
            min: 0,
            max: Amount::Fixed(1),
            reveal_chosen: false,
            movement,
            not_chosen_movement,
            then,
            for_unchosen,
        })
    };
    let hand = ChooseZone::Hand(PlayerRel::You);
    let none = ChooseMove::None;
    assert!(choose(hand, none, &[]));
    assert!(choose(hand, ChooseMove::Battlefield { tapped: true }, &[]));
    assert!(choose(ChooseZone::Found, none, &[]));
    assert!(!choose(ChooseZone::Found, ChooseMove::Hand, &[]));
    assert!(!choose(hand, none, DRAW));
    assert!(split(hand, none, none, &[], &[]));
    assert!(!split(hand, none, none, DRAW, &[]));
    assert!(!split(hand, none, none, &[], DRAW));
    for movement in library {
        assert!(!choose(hand, movement, &[]), "{movement:?}");
        assert!(!split(hand, movement, none, &[], &[]), "{movement:?}");
        assert!(!split(hand, none, movement, &[], &[]), "{movement:?}");
    }
}

#[test]
fn piles_are_a_move_when_they_come_from_a_library_or_one_of_them_goes_there() {
    let piles = |zone, chosen_pile, unchosen_pile| {
        stays_a_mana_ability(Effect::SeparateIntoTwoPiles {
            separator: PlayerRel::You,
            pile_chooser: PlayerRel::You,
            zone,
            filter: ANY,
            only_found: false,
            chosen_pile,
            unchosen_pile,
        })
    };
    assert!(piles(ZoneSel::Battlefield, &[], &[]));
    assert!(!piles(ZoneSel::LibraryYou, &[], &[]));
    assert!(!piles(ZoneSel::LibraryOf(PlayerRel::You), &[], &[]));
    assert!(!piles(ZoneSel::Battlefield, DRAW, &[]));
    assert!(!piles(ZoneSel::Battlefield, &[], DRAW));
}

#[test]
fn a_vote_is_a_move_when_any_of_its_options_is() {
    const STILL: VoteOption = VoteOption {
        name: "still",
        effects: &[],
    };
    const DRAWS: VoteOption = VoteOption {
        name: "draw",
        effects: DRAW,
    };
    let vote = |options| {
        stays_a_mana_ability(Effect::Vote {
            options,
            outcome: VoteOutcome::AllWinners,
            secret: false,
        })
    };
    assert!(vote(&[STILL, STILL]));
    assert!(!vote(&[STILL, DRAWS]));
    assert!(!vote(&[DRAWS, STILL]));
}

#[test]
fn a_cost_paid_from_the_library_is_a_move_even_as_one_of_two_choices() {
    const LIFE: &Cost = &Cost {
        mana: baylee_core::mana::ManaCost::ZERO,
        parts: &[CostPart::PayLife(1)],
    };
    const MILL: &Cost = &Cost {
        mana: baylee_core::mana::ManaCost::ZERO,
        parts: &[CostPart::TapSelf, CostPart::ExileTopOfLibrary(1)],
    };
    const EITHER: &Cost = &Cost {
        mana: baylee_core::mana::ManaCost::ZERO,
        parts: &[CostPart::Either(MILL, LIFE)],
    };
    const OTHER: &Cost = &Cost {
        mana: baylee_core::mana::ManaCost::ZERO,
        parts: &[CostPart::Either(LIFE, MILL)],
    };
    const NEITHER: &Cost = &Cost {
        mana: baylee_core::mana::ManaCost::ZERO,
        parts: &[CostPart::Either(LIFE, LIFE)],
    };
    for (cost, is_one) in [
        (&Cost::FREE, true),
        (LIFE, true),
        (NEITHER, true),
        (MILL, false),
        (EITHER, false),
        (OTHER, false),
    ] {
        assert_eq!(activated_mana_ability(cost, &[ADD], false), is_one);
        assert!(!activated_mana_ability(cost, &[ADD], true), "it targets");
        assert!(!activated_mana_ability(cost, &[], false), "it adds none");
    }
}
