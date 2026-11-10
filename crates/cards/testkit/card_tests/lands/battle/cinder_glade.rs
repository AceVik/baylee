//! `cards/lands/battle/cinder_glade.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Cinder Glade is a `Mountain Forest` whose whole printed text is "({T}: Add
/// {R} or {G}. This land enters tapped unless you control two or more basic
/// lands.)", and the word the first sentence turns on is *two*. Two Forests
/// beside it are enough and one Forest is not — and the sharper of the two
/// boards is the one with a single Forest, because the Glade is itself a land
/// with basic land types, so a count that took the entering land along would
/// flip both readings at once: untapped next to one Forest, tapped next to
/// two. Both sides are played out for exactly that reason, and the untapped
/// Glade is then tapped for the mana its second sentence prints.
#[test]
fn cinder_glade_counts_the_basic_lands_already_there_and_not_itself() {
    let p0 = PlayerId::new(0);

    // One basic land on the table: the count is one, and the land asking is
    // not one of the two.
    let mut lonely = Duel::new(SEED, forest())
        .battlefield(0, &[forest()])
        .hand(0, &[cinder_glade()])
        .start();
    keep_mulligans(&mut lonely);
    assert!(walk_to_own_main(&mut lonely, p0), "p0 reaches its own main");
    let played = play_land(&mut lonely, p0, cinder_glade());
    assert!(
        entered_tapped(&lonely, played),
        "one Forest plus the Glade is one basic land and the land asking, so \
         the land it counted was short of two"
    );

    // Two of them, which is the printed threshold — and the reading a filter
    // that matched nothing would fail, since that would leave every Glade
    // tapped however many basics stood beside it.
    let mut pair = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest()])
        .hand(0, &[cinder_glade()])
        .start();
    keep_mulligans(&mut pair);
    assert!(walk_to_own_main(&mut pair, p0), "p0 reaches its own main");
    let glade = play_land(&mut pair, p0, cinder_glade());
    assert!(
        !entered_tapped(&pair, glade),
        "two basic lands are the two the card asks for, and the Glade came in \
         ready to be tapped"
    );

    // The two Forests pay into the pool first, so the only thing left untapped
    // on this board is the land whose own `{T}` is under test.
    tap_mana_except(&mut pair, p0, glade);
    assert_eq!(
        pair.state().players[0].mana_pool.total(),
        2,
        "the two Forests, and the Glade was kept back"
    );

    // Either list will do: a `Mountain Forest` has its mana by virtue of two
    // basic land types (CR 305.6) and the printing registers the ability in
    // its own right as well, so the route is taken out of whichever carries it.
    let Pending::Priority { legal, .. } = pair.pending().clone() else {
        panic!("expected priority, got {:?}", pair.pending())
    };
    let route = legal
        .mana_abilities
        .iter()
        .copied()
        .find(|id| *id == glade)
        .map(|source| PlayerAction::ActivateManaAbility { source })
        .or_else(|| {
            legal
                .abilities
                .iter()
                .copied()
                .find(|(source, _)| *source == glade)
                .map(|(source, ability_index)| PlayerAction::ActivateAbility {
                    source,
                    ability_index,
                })
        })
        .expect("an untapped Mountain Forest offers the tap its type line gives it");
    pair.apply(p0, route).unwrap();

    let Pending::ChooseColor { player, options } = pair.pending().clone() else {
        panic!(
            "\"Add {{R}} or {{G}}\" is a question with two answers, got {:?}",
            pair.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat names the colour");
    assert!(
        options.contains(&ManaColor::Red) && options.contains(&ManaColor::Green),
        "exactly the two colours the type line prints: {options:?}"
    );
    assert_eq!(options.len(), 2, "and no third one: {options:?}");
    pair.apply(p0, PlayerAction::ChooseColor(ManaColor::Green))
        .unwrap();

    let pool = &pair.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Green),
        3,
        "the two Forests and the colour that was named off the Glade"
    );
    assert_eq!(
        pool.available(ManaColor::Red),
        0,
        "and not a default: the colour that was named is the one that landed"
    );
    assert!(is_tapped(&pair, glade), "{{T}} was the price of it");
}
