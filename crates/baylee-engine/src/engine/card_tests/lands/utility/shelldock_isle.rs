//! `cards/lands/utility/shelldock_isle.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Shelldock Isle: "Hideaway 4" / "This land enters tapped." / "{T}: Add {U}." / "{U}, {T}: You may play the exiled card without paying its mana cost if a library has twenty or fewer cards in it."
/// Playing this land causes it to enter tapped, and after untapping on a subsequent turn it taps for blue mana.
#[test]
fn shelldock_isle_enters_tapped_and_taps_for_blue() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(230, forest())
        .hand(0, &[shelldock_isle()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, shelldock_isle());
    crate::engine::card_tests::hidden_exile::finish_hideaway(&mut engine);
    assert!(entered_tapped(&engine, land));

    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(!is_tapped(&engine, land));

    activate(&mut engine, p0, shelldock_isle(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Blue), 1);
    assert!(is_tapped(&engine, land));
}

/// Trims `seat`'s library to exactly `n` cards, the rest going to their
/// graveyard.
#[track_caller]
fn trim_library(e: &mut Engine<RegistryLookup>, seat: PlayerId, n: usize) {
    let len = e.state().zones.list(ZoneLocation::Library(seat)).len();
    assert!(len >= n, "the library is already shorter than {n}");
    seed_graveyard(e, seat, len - n);
}

/// "{U}, {T}: You may play the exiled card without paying its mana cost if a
/// library has twenty or fewer cards in it." With twenty-one in each library
/// the exiled card stays, and with twenty in either one it is offered.
#[test]
fn shelldock_isle_plays_the_exiled_card_only_if_a_library_has_twenty_or_fewer() {
    use crate::engine::card_tests::legends_kit as kit;
    let p0 = PlayerId::new(0);
    for (mine, theirs, offered) in [(21, 21, false), (21, 20, true), (20, 21, true)] {
        let mut e = Duel::new(SEED, forest())
            .hand(0, &[shelldock_isle()])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p0);
        let land = play_land(&mut e, p0, shelldock_isle());
        pass_until(&mut e, |e| !e.inspected_cards().is_empty());
        let hidden = e.inspected_cards()[0];
        e.apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![hidden],
            },
        )
        .unwrap();
        pass_until(&mut e, stack_is_empty);
        e.dev_state_mut(p0).unwrap().set_tapped(land, false);
        trim_library(&mut e, p0, mine);
        trim_library(&mut e, PlayerId::new(1), theirs);
        kit::float(&mut e, p0, &[(ManaColor::Blue, 1)]);

        activate(&mut e, p0, shelldock_isle(), 2);
        pass_until(&mut e, |e| {
            stack_is_empty(e)
                || matches!(
                    e.pending(),
                    Pending::ChooseCards {
                        prompt: ChoicePrompt::PlayFromExile,
                        ..
                    }
                )
        });
        let asked = matches!(
            e.pending(),
            Pending::ChooseCards {
                prompt: ChoicePrompt::PlayFromExile,
                ..
            }
        );
        assert_eq!(
            asked, offered,
            "libraries of {mine} and {theirs}: the play is offered only at twenty or fewer"
        );
        if !asked {
            assert_eq!(
                e.state().object(hidden).unwrap().zone,
                Zone::Exile,
                "still hidden in exile"
            );
        }
    }
}
