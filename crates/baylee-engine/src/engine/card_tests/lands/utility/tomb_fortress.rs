//! `cards/lands/utility/tomb_fortress.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Tomb Fortress` prints `This land enters tapped.`, `{{T}}: Add {{B}}.`, and `{{2}}{{B}}{{B}}{{B}}, {{T}}, Exile this land: Mill four cards, then return a creature card from your graveyard to the battlefield. Activate only as a sorcery.`
///
/// Playing the land puts it onto the battlefield tapped. After untapping,
/// both its mana ability and its sorcery-speed reanimation are offered with
/// sufficient mana, even with an empty graveyard: the choice follows milling.
#[test]
fn tomb_fortress_enters_tapped_and_taps_for_black() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .hand(0, &[tomb_fortress()])
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp(), swamp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let fortress = play_land(&mut engine, p0, tomb_fortress());
    assert!(
        entered_tapped(&engine, fortress),
        "tomb fortress enters tapped"
    );

    reach_their_main_phase(&mut engine, PlayerId::new(1));
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, fortress));

    // Float {{2}}{{B}}{{B}}{{B}} (5 black mana) from basic swamps while keeping Tomb Fortress untapped.
    tap_mana_except(&mut engine, p0, fortress);
    assert_eq!(engine.state().players[0].mana_pool.total(), 5);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        5
    );
    assert!(!is_tapped(&engine, fortress));

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        legal.abilities.contains(&(fortress, 0)),
        "ability 0 ({{T}}: Add {{B}}) is offered"
    );
    assert!(
        legal.abilities.contains(&(fortress, 1)),
        "reanimation is available even before a creature reaches the graveyard"
    );

    activate(&mut engine, p0, tomb_fortress(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Black), 6);
    assert_eq!(pool.total(), 6);
    assert!(is_tapped(&engine, fortress));
}

#[test]
fn tomb_fortress_finds_a_newly_milled_creature_untapped() {
    use baylee_core::generated::index;
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[index::TOMB_FORTRESS])
        .hand(0, &[index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let land = on_battlefield(&e, p0, index::TOMB_FORTRESS).unwrap();
    let bear = hand_to_library_top(&mut e, p0, index::GRIZZLY_BEARS);
    let state = e.dev_state_mut(p0).unwrap();
    state.set_tapped(land, false);
    state.players[0].mana_pool.add(ManaColor::Black, 5);
    e.refresh_offer();
    activate(&mut e, p0, index::TOMB_FORTRESS, 1);
    assert_eq!(e.state().object(land).unwrap().zone, Zone::Exile);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![bear],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().object(bear).unwrap().zone, Zone::Battlefield);
    assert!(!is_tapped(&e, bear));
}

/// "{2}{B}{B}{B}, {T}, Exile this land: Mill four cards, then return a
/// creature card from your graveyard to the battlefield. Activate only as a
/// sorcery." With the whole price floating, the reanimation is on offer in
/// the own main phase and in neither the stack-holding window nor the
/// opponent's turn.
#[test]
fn tomb_fortress_reanimation_is_sorcery_timing_only() {
    use crate::engine::card_tests::legends_kit as kit;
    let mut e = kit::game(953, &[tomb_fortress()], &[], &[llanowar_elves()], &[]);
    let fortress = kit::obj(&e, kit::P0, tomb_fortress());
    assert_sorcery_timing_only(&mut e, fortress, 1, &[(ManaColor::Black, 5)]);
}
