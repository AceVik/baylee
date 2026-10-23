//! Reviewed knight of dawn behavior.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

use baylee_core::generated::index;
fn knight_of_dawn() -> CardIndex {
    card_index("51262155-f1be-45a8-a21f-94e920a93e55")
}

#[test]
fn knight_of_dawn_chooses_a_protection_color_on_resolution_without_targeting_itself() {
    for (mode, spell, color, protected) in [
        (0, index::HOLY_STRENGTH, ManaColor::White, true),
        (1, index::UNSUMMON, ManaColor::Blue, true),
        (4, index::GIANT_GROWTH, ManaColor::Green, true),
        (3, index::LIGHTNING_BOLT, ManaColor::Red, true),
        (2, index::LIGHTNING_BOLT, ManaColor::Red, false),
        (2, index::TERROR, ManaColor::Black, true),
    ] {
        let mut e = lk::game(
            SEED,
            &[knight_of_dawn(), index::GRIZZLY_BEARS],
            &[],
            &[spell],
            &[],
        );
        let knight = lk::obj(&e, lk::P0, knight_of_dawn());
        assert!(keywords(&e, knight).contains(KeywordSet::FIRST_STRIKE));
        lk::float(&mut e, lk::P0, &[(ManaColor::White, 2)]);
        activate(&mut e, lk::P0, knight_of_dawn(), 0);
        assert!(matches!(e.pending(), Pending::Priority { .. }));
        pass_until(&mut e, |e| {
            matches!(e.pending(), Pending::ChooseCastMode { .. })
        });
        assert!(e.apply(lk::P0, PlayerAction::ChooseMode(5)).is_err());
        e.apply(lk::P0, PlayerAction::ChooseMode(mode)).unwrap();
        lk::settle(&mut e);
        assert_eq!(lk::pool_total(&e, lk::P0), 0);
        lk::float(&mut e, lk::P0, &[(color, 2)]);
        cast_with_floating(&mut e, lk::P0, spell);
        let Pending::ChooseTargets { options, .. } = e.pending() else {
            panic!("target question")
        };
        assert_eq!(options.contains(&knight), !protected);
    }
}

#[test]
fn knight_of_dawn_protection_ends_with_the_turn() {
    let mut e = lk::game(
        SEED,
        &[knight_of_dawn()],
        &[],
        &[index::LIGHTNING_BOLT],
        &[],
    );
    let knight = lk::obj(&e, lk::P0, knight_of_dawn());
    lk::float(&mut e, lk::P0, &[(ManaColor::White, 2)]);
    activate(&mut e, lk::P0, knight_of_dawn(), 0);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCastMode { .. })
    });
    e.apply(lk::P0, PlayerAction::ChooseMode(3)).unwrap();
    lk::settle(&mut e);
    reach_their_main_phase(&mut e, lk::P1);
    reach_their_main_phase(&mut e, lk::P0);
    lk::float(&mut e, lk::P0, &[(ManaColor::Red, 1)]);
    lk::cast_at(&mut e, lk::P0, index::LIGHTNING_BOLT, &[knight], &[]);
    assert!(in_graveyard(&e, lk::P0, knight_of_dawn()).is_some());
}
