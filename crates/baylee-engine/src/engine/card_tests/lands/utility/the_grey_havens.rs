//! `cards/lands/utility/the_grey_havens.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

#[test]
fn the_grey_havens_triggers_scry_and_taps_for_colorless() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(137, forest())
        .hand(0, &[the_grey_havens()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, the_grey_havens());
    assert!(!entered_tapped(&engine, land));

    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::Arrange {
                prompt: ArrangePrompt::Scry,
                ..
            }
        )
    });
    let Pending::Arrange {
        player,
        cards,
        piles,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected a scry arrangement, got {:?}", engine.pending());
    };
    assert_eq!(player, p0);
    assert_eq!(piles, scry_piles(1));

    engine.apply(p0, look_answer(&cards, &[])).unwrap();
    pass_until(&mut engine, stack_is_empty);

    activate(&mut engine, p0, the_grey_havens(), 1);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, land));
}

#[test]
fn grey_havens_reads_only_legendary_creatures_in_your_graveyard_and_pool_sees_their_colors() {
    use baylee_core::generated::index;
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                index::THE_GREY_HAVENS,
                index::HAPATRA_VIZIER_OF_POISONS,
                index::ARCHANGEL_AVACYN,
                index::RAGING_GOBLIN,
                index::REFLECTING_POOL,
            ],
        )
        .battlefield(1, &[index::BIRGI_GOD_OF_STORYTELLING])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let moon = on_battlefield(&e, p0, index::RAGING_GOBLIN).unwrap();
    let hapatra = on_battlefield(&e, p0, index::HAPATRA_VIZIER_OF_POISONS).unwrap();
    let birgi = on_battlefield(&e, p1, index::BIRGI_GOD_OF_STORYTELLING).unwrap();
    bury(&mut e, &[moon, hapatra, birgi]);
    e.apply(p0, PlayerAction::PassPriority).unwrap();
    pass_until(
        &mut e,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );
    pass_until(&mut e, stack_is_empty);
    let havens = on_battlefield(&e, p0, index::THE_GREY_HAVENS).unwrap();
    let pool = on_battlefield(&e, p0, index::REFLECTING_POOL).unwrap();
    let options = crate::resolve::colors_of(
        e.state(),
        p0,
        baylee_cards_dsl::ManaSource::LandColor {
            mine: true,
            any_type: true,
        },
        pool,
    );
    assert!(options.contains(&ManaColor::Black) && options.contains(&ManaColor::Green));
    assert!(!options.contains(&ManaColor::White) && !options.contains(&ManaColor::Red));
    e.apply(
        p0,
        PlayerAction::ActivateAbility {
            source: havens,
            ability_index: 2,
        },
    )
    .unwrap();
    assert!(
        matches!(e.pending(), Pending::ChooseColor { options, .. } if options == &[ManaColor::Black, ManaColor::Green])
    );
    e.apply(p0, PlayerAction::ChooseColor(ManaColor::Green))
        .unwrap();
    assert_eq!(
        e.state().players[0].mana_pool.available(ManaColor::Green),
        1
    );
    assert!(stack_is_empty(&e));
}

#[test]
fn grey_havens_adds_no_mana_when_the_graveyard_has_no_matching_color() {
    use baylee_core::generated::index;
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[index::THE_GREY_HAVENS])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    activate(&mut e, p0, index::THE_GREY_HAVENS, 2);
    assert_eq!(e.state().players[0].mana_pool.total(), 0);
    assert!(is_tapped(
        &e,
        on_battlefield(&e, p0, index::THE_GREY_HAVENS).unwrap()
    ));
}
