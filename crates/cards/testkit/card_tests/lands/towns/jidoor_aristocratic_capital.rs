//! `cards/lands/towns/jidoor_aristocratic_capital.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

#[test]
fn jidoor_is_played_as_the_town_and_taps_for_blue() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[jidoor_aristocratic_capital()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let land = play_land(&mut engine, p0, jidoor_aristocratic_capital());
    assert!(
        entered_tapped(&engine, land),
        "\"This land enters tapped\" is unconditional"
    );
    assert!(
        types(&engine, land).contains(TypeSet::LAND),
        "what arrived is the Town and not the sorcery on its back"
    );
    assert!(
        !types(&engine, land).contains(TypeSet::SORCERY),
        "and the Adventure face stayed on the other side of the card"
    );

    cross_into_the_next_own_main(&mut engine, p0);
    activate(&mut engine, p0, jidoor_aristocratic_capital(), 0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Blue),
        1,
        "{{T}}: Add {{U}}"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "one blue and nothing else"
    );
}

#[test]
fn overture_mills_half_the_target_library_rounded_down() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for mill_first in [0, 1] {
        let mut e = Duel::new(SEED, forest())
            .hand(0, &[jidoor_aristocratic_capital()])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p0);
        seed_graveyard(&mut e, p1, mill_first);
        let before = library_size(&e, p1);
        let own = library_size(&e, p0);
        let card = begin_town_adventure(
            &mut e,
            p0,
            jidoor_aristocratic_capital(),
            ManaColor::Blue,
            6,
        );
        let Pending::ChoosePlayer { options, .. } = e.pending() else {
            panic!("opponent target")
        };
        assert_eq!(options.as_slice(), &[p1]);
        e.apply(p0, PlayerAction::ChoosePlayer(p1)).unwrap();
        pass_until(&mut e, stack_is_empty);
        assert_eq!(library_size(&e, p1), before - before / 2);
        assert_eq!(library_size(&e, p0), own);
        assert_adventure_exiled(&e, p0, card);
    }
}
