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
