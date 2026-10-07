//! `cards/lands/battle/radiant_summit.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Radiant Summit is a `Mountain Plains` — a two-colour land printing "This
/// land enters tapped unless you control two or more basic lands" and
/// `{T}: Add {R} or {W}`. Two boards read both halves. With one Forest plus
/// an `irrigated_farmland`, which has the basic land *types* and no BASIC
/// supertype, the Summit arrives tapped and offers nothing for the rest of
/// the turn — the word the clause turns on is "basic", not "a land with a
/// basic land type". With two Forests beside it, it arrives untapped and its
/// own tap is a question with exactly two answers.
#[test]
fn radiant_summit_enters_tapped_for_one_basic_land_and_taps_for_one_of_its_colours() {
    let p0 = PlayerId::new(0);

    // One basic land, and a bystander that only looks like one.
    let mut short = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), irrigated_farmland()])
        .hand(0, &[radiant_summit()])
        .start();
    keep_mulligans(&mut short);
    assert!(walk_to_own_main(&mut short, p0), "p0 reaches its own main");
    let early = play_land(&mut short, p0, radiant_summit());
    assert!(
        entered_tapped(&short, early),
        "one Forest is not two basic lands, and the Farmland's Plains and \
         Island types are no BASIC supertype"
    );

    let Pending::Priority { legal, .. } = short.pending().clone() else {
        panic!("expected priority, got {:?}", short.pending())
    };
    assert!(
        !legal.mana_abilities.contains(&early)
            && !legal.abilities.iter().any(|(source, _)| *source == early),
        "and a land that entered tapped gives nothing this turn — its {{T}} \
         is not even on offer until the untap step: {legal:?}"
    );

    // Two basic lands, and nothing else.
    let mut ready = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest()])
        .hand(0, &[radiant_summit()])
        .start();
    keep_mulligans(&mut ready);
    assert!(walk_to_own_main(&mut ready, p0), "p0 reaches its own main");
    let summit = play_land(&mut ready, p0, radiant_summit());
    assert!(
        !entered_tapped(&ready, summit),
        "two basic lands is the number the card prints, so it enters untapped"
    );

    // Its own `{T}`, taken out of whichever list the offer put it on: a
    // Mountain Plains is a CR 305.6 source and it also prints the ability,
    // and both roads lead to the same question.
    let Pending::Priority { legal, .. } = ready.pending().clone() else {
        panic!("expected priority, got {:?}", ready.pending())
    };
    let route = legal
        .mana_abilities
        .iter()
        .copied()
        .find(|source| *source == summit)
        .map(|source| PlayerAction::ActivateManaAbility { source })
        .or_else(|| {
            legal
                .abilities
                .iter()
                .copied()
                .find(|(source, _)| *source == summit)
                .map(|(source, ability_index)| PlayerAction::ActivateAbility {
                    source,
                    ability_index,
                })
        })
        .expect("an untapped Summit is a mana source on one of the two lists");
    ready.apply(p0, route).expect("the offer listed it");

    let Pending::ChooseColor { player, options } = ready.pending().clone() else {
        panic!(
            "`Add {{R}} or {{W}}` is a choice, got {:?}",
            ready.pending()
        )
    };
    assert_eq!(player, p0, "the seat that tapped names the colour");
    assert_eq!(
        options.len(),
        2,
        "\"Add {{R}} or {{W}}\" is two colours and not any colour: {options:?}"
    );
    assert!(
        options.contains(&ManaColor::Red) && options.contains(&ManaColor::White),
        "the two colours the card prints: {options:?}"
    );
    ready
        .apply(p0, PlayerAction::ChooseColor(ManaColor::White))
        .expect("white was one of the colours it offered");

    let pool = &ready.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::White),
        1,
        "the colour that was named, off the Summit's own tap"
    );
    assert_eq!(
        pool.total(),
        1,
        "one mana and no more: the two Forests beside it are still standing"
    );
    assert!(
        stack_is_empty(&ready),
        "CR 605.3b: a mana ability uses no stack"
    );
    assert!(
        is_tapped(&ready, summit),
        "and its {{T}} was the whole price"
    );
}
