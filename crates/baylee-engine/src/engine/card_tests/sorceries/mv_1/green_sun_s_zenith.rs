//! Search, fail-to-find, X payment, and the resolving spell's self-shuffle.
#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn green_suns_zenith_searches_within_x_and_shuffles_itself_back() {
    let p0 = PlayerId::new(0);
    for find in [false, true] {
        let mut e = Duel::new(SEED, llanowar_elves())
            .battlefield(0, &[forest(), forest(), forest()])
            .hand(0, &[green_suns_zenith()])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p0);
        let before = library_size(&e, p0);
        let zenith = in_hand(&e, p0, green_suns_zenith()).unwrap();
        cast_from_hand(&mut e, p0, green_suns_zenith());
        let Pending::ChooseNumber { min, max, .. } = e.pending() else {
            panic!("announce X")
        };
        assert!(*min <= 2 && *max >= 2);
        e.apply(p0, PlayerAction::ChooseNumber(2)).unwrap();
        pass_until(&mut e, |e| {
            matches!(e.pending(), Pending::ChooseCards { .. })
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
            panic!("library search")
        };
        assert_eq!(player, p0);
        assert_eq!(
            (min, max),
            (0, 1),
            "a quality-constrained hidden search may fail to find"
        );
        assert_eq!(prompt, crate::choice::ChoicePrompt::SearchLibrary);
        assert!(!options.is_empty(), "mana value one is less than X = two");
        assert!(
            options
                .iter()
                .all(|id| e.state().zones.list(ZoneLocation::Library(p0)).contains(id))
        );
        e.apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: if find { vec![options[0]] } else { vec![] },
            },
        )
        .unwrap();
        pass_until(&mut e, stack_is_empty);
        assert_eq!(on_battlefield(&e, p0, llanowar_elves()).is_some(), find);
        assert_eq!(library_size(&e, p0), before + usize::from(!find));
        assert_eq!(e.state().players[0].mana_pool.total(), 0);
        assert!(
            e.state()
                .zones
                .list(ZoneLocation::Library(p0))
                .contains(&zenith)
        );
        assert!(in_graveyard(&e, p0, green_suns_zenith()).is_none());
    }
}

/// Casts Green Sun's Zenith for `x` over a library of Forests, a green
/// creature of mana value 2 (Grizzly Bears), a non-green creature of mana
/// value 5 (Serra Angel), a green creature of mana value 4 (Wood Elemental)
/// and a green non-creature (Giant Growth), and returns what the search
/// offers, sorted, with the four cards in that order.
fn zenith_search_menu(x: u32) -> (Vec<ObjectId>, [ObjectId; 4]) {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[forest(), forest(), forest(), forest(), forest(), forest()],
        )
        .hand(
            0,
            &[
                green_suns_zenith(),
                baylee_core::generated::index::GRIZZLY_BEARS,
                serra_angel(),
                baylee_core::generated::index::WOOD_ELEMENTAL,
                giant_growth(),
            ],
        )
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let cards = [
        baylee_core::generated::index::GRIZZLY_BEARS,
        serra_angel(),
        baylee_core::generated::index::WOOD_ELEMENTAL,
        giant_growth(),
    ]
    .map(|card| hand_to(&mut e, p0, card, ZoneLocation::Library(p0)));
    e.refresh_offer();
    cast_from_hand(&mut e, p0, green_suns_zenith());
    e.apply(p0, PlayerAction::ChooseNumber(x)).unwrap();
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        mut options,
        prompt,
        ..
    } = e.pending().clone()
    else {
        panic!("library search")
    };
    assert_eq!(prompt, crate::choice::ChoicePrompt::SearchLibrary);
    options.sort();
    (options, cards)
}

/// "a green creature card with mana value X or less": with X = 2 the Bears are
/// the one card that is green, a creature and of mana value 2 or less. Serra
/// Angel is a creature within no X here and not green; Wood Elemental is green
/// but above X; Giant Growth is green and cheap but no creature.
#[test]
fn green_suns_zenith_x_two_offers_only_the_green_creature_of_mana_value_two_or_less() {
    let (options, [bears, _angel, _wood, _growth]) = zenith_search_menu(2);
    assert_eq!(
        options,
        vec![bears],
        "X = 2: green AND creature AND mana value 2 or less"
    );
}

/// The same library with X = 4: Wood Elemental (green, mana value 4) joins the
/// Bears; the white Serra Angel (mana value 5) and the green non-creature
/// Giant Growth still do not.
#[test]
fn green_suns_zenith_x_four_adds_the_green_creature_of_mana_value_four() {
    let (options, [bears, angel, wood, growth]) = zenith_search_menu(4);
    let mut expected = vec![bears, wood];
    expected.sort();
    assert_eq!(
        options, expected,
        "X = 4: Bears and Wood Elemental; not the non-green Angel ({angel:?}) or the non-creature ({growth:?})"
    );
}

/// X = 5 is enough for Serra Angel's mana value, so only her colour keeps her
/// off the menu: "a **green** creature card". The Bears and Wood Elemental are
/// offered, the white Angel and the green Giant Growth are not.
#[test]
fn green_suns_zenith_x_five_still_leaves_out_the_creature_that_is_not_green() {
    let (options, [bears, angel, wood, growth]) = zenith_search_menu(5);
    let mut expected = vec![bears, wood];
    expected.sort();
    assert_eq!(
        options, expected,
        "X = 5 reaches the Angel's mana value ({angel:?}) but she is not green; Giant Growth ({growth:?}) is no creature"
    );
}
