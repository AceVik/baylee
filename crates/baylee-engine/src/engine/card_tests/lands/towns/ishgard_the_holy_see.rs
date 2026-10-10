//! `cards/lands/towns/ishgard_the_holy_see.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

#[test]
fn ishgard_the_holy_see_enters_tapped_and_taps_for_white() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[ishgard_the_holy_see()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, ishgard_the_holy_see());
    assert!(entered_tapped(&engine, land));
    assert!(types(&engine, land).contains(TypeSet::LAND));
    assert!(!types(&engine, land).contains(TypeSet::SORCERY));

    reach_their_main_phase(&mut engine, PlayerId::new(1));
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, land));

    activate(&mut engine, p0, ishgard_the_holy_see(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::White), 1);
    assert_eq!(pool.total(), 1);
    assert!(is_tapped(&engine, land));
}

#[test]
fn ishgard_returns_two_qualified_cards_then_plays_from_exile_once() {
    use baylee_core::generated::index;
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .hand(
            0,
            &[
                ishgard_the_holy_see(),
                index::ORNITHOPTER,
                index::HOLY_STRENGTH,
                llanowar_elves(),
            ],
        )
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let artifact = bury_from_hand(&mut e, p0, index::ORNITHOPTER);
    let enchantment = bury_from_hand(&mut e, p0, index::HOLY_STRENGTH);
    let creature = bury_from_hand(&mut e, p0, llanowar_elves());
    let card = begin_town_adventure(&mut e, p0, ishgard_the_holy_see(), ManaColor::White, 5);
    let Pending::ChooseTargets {
        options, min, max, ..
    } = e.pending()
    else {
        panic!("targets")
    };
    assert_eq!((*min, *max), (0, 2));
    assert!(options.contains(&artifact) && options.contains(&enchantment));
    assert!(!options.contains(&creature));
    e.apply(
        p0,
        PlayerAction::ChooseTargets {
            objects: vec![artifact, enchantment],
            players: vec![],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().object(artifact).unwrap().zone, Zone::Hand);
    assert_eq!(e.state().object(enchantment).unwrap().zone, Zone::Hand);
    assert_adventure_exiled(&e, p0, card);
    let Pending::Priority { legal, .. } = e.pending() else {
        panic!("priority")
    };
    assert!(legal.lands.contains(&card));
    assert!(!legal.castable.contains(&card));
    e.apply(p0, PlayerAction::PlayLand { card }).unwrap();
    assert!(is_tapped(&e, card));
    assert_eq!(e.state().players[0].lands_played_this_turn, 1);
    e.dev_state_mut(p0)
        .unwrap()
        .move_object(
            card,
            ZoneLocation::Exile(p0),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .unwrap();
    assert!(
        !crate::casting::land_card_open(e.state(), p0, card),
        "a later exile does not inherit permission"
    );
}

#[test]
fn countered_land_adventure_goes_to_graveyard_without_a_play_permission() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .hand(0, &[ishgard_the_holy_see()])
        .hand(1, &[counterspell()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let card = begin_town_adventure(&mut e, p0, ishgard_the_holy_see(), ManaColor::White, 5);
    // With no eligible cards, the optional target question is skipped.
    pass_until(
        &mut e,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p1),
    );
    fund_adventure(&mut e, p1, ManaColor::Blue, 2);
    let counter = in_hand(&e, p1, counterspell()).unwrap();
    e.apply(p1, PlayerAction::CastSpell { card: counter })
        .unwrap();
    aim_at(&mut e, p1, card);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().object(card).unwrap().zone, Zone::Graveyard);
    assert!(!crate::casting::adventure_permission(e.state(), p0, card));
}
