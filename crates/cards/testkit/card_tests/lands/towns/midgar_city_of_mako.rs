//! `cards/lands/towns/midgar_city_of_mako.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

#[test]
fn midgar_city_of_mako_enters_tapped_and_makes_the_one_colour_it_prints() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[midgar_city_of_mako()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let land = play_land(&mut engine, p0, midgar_city_of_mako());
    assert!(
        entered_tapped(&engine, land),
        "\"This land enters tapped\" is unconditional on this card"
    );
    assert!(
        types(&engine, land).contains(TypeSet::LAND),
        "the face played out of hand is the land face and not the Adventure"
    );

    // A tapped land makes no mana, so the turn has to come round before the
    // second sentence can be read at all.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
            && !is_tapped(e, land)
    });
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "nothing is floating before the ability is activated"
    );
    activate(&mut engine, p0, midgar_city_of_mako(), 0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        1,
        "{{T}}: Add {{B}} — one black and not a choice of five"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "and one mana in total: a Town is not a dual"
    );
    assert!(is_tapped(&engine, land), "its own tap symbol was the cost");
}

#[test]
fn reactor_raid_draws_only_for_a_completed_optional_sacrifice() {
    use baylee_core::generated::index;
    let p0 = PlayerId::new(0);
    for sacrifice in [None, Some(false), Some(true)] {
        let board = if sacrifice.is_some() {
            vec![index::ORNITHOPTER, forest()]
        } else {
            vec![forest()]
        };
        let mut e = Duel::new(SEED, forest())
            .battlefield(0, &board)
            .hand(0, &[midgar_city_of_mako()])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p0);
        let before = library_size(&e, p0);
        let card = begin_town_adventure(&mut e, p0, midgar_city_of_mako(), ManaColor::Black, 3);
        if let Some(pay) = sacrifice {
            pass_until(&mut e, |e| {
                matches!(e.pending(), Pending::ChooseCards { .. })
            });
            let Pending::ChooseCards {
                options, min, max, ..
            } = e.pending().clone()
            else {
                panic!("sacrifice")
            };
            let artifact = on_battlefield(&e, p0, index::ORNITHOPTER).unwrap();
            assert_eq!(options, vec![artifact]);
            assert_eq!((min, max), (0, 1));
            e.apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: if pay { vec![artifact] } else { vec![] },
                },
            )
            .unwrap();
            assert_eq!(
                e.state().object(artifact).unwrap().zone,
                if pay {
                    Zone::Graveyard
                } else {
                    Zone::Battlefield
                }
            );
        }
        pass_until(&mut e, stack_is_empty);
        assert_eq!(
            library_size(&e, p0),
            before - if sacrifice == Some(true) { 2 } else { 0 }
        );
        assert_adventure_exiled(&e, p0, card);
    }
}
