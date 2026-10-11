//! `cards/lands/utility/edgewall_inn.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Edgewall Inn prints `This land enters tapped.`, `As this land enters, choose a color.`,
/// `{{T}}: Add one mana of the chosen color.`, and `{{3}}, {{T}}, Sacrifice this land: Return
/// target card that has an Adventure from your graveyard to your hand.`
///
/// Playing this land prompts for a color choice via `Pending::ChooseColor` and enters tapped.
/// In the next turn, it untaps, withholds ability index 1 despite floating `{{3}}` from three
/// `forest()` lands, and taps for one mana of the chosen color (`ManaColor::Black`).
#[test]
fn edgewall_inn_enters_tapped_chooses_color_and_taps_for_it() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[edgewall_inn()])
        .battlefield(0, &[forest(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let card = in_hand(&engine, p0, edgewall_inn()).expect("edgewall inn in hand");
    engine.apply(p0, PlayerAction::PlayLand { card }).unwrap();

    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!("expected ChooseColor, got {:?}", engine.pending());
    };
    assert_eq!(player, p0);
    assert_eq!(options, baylee_cards_dsl::ALL_MANA_COLORS.to_vec());
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .unwrap();

    let inn = on_battlefield(&engine, p0, edgewall_inn()).expect("inn on battlefield");
    assert!(entered_tapped(&engine, inn));

    reach_their_main_phase(&mut engine, PlayerId::new(1));
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, inn));

    // Float {3} green mana from the three Forests while keeping Edgewall Inn untapped.
    tap_mana_except(&mut engine, p0, inn);
    assert_eq!(engine.state().players[0].mana_pool.total(), 3);
    assert!(!is_tapped(&engine, inn));

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        legal.abilities.contains(&(inn, 0)),
        "ability 0 ({{T}}: Add chosen color) is offered"
    );
    assert!(
        !legal.abilities.contains(&(inn, 1)),
        "the graveyard contains no Adventure card to return"
    );

    activate(&mut engine, p0, edgewall_inn(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Black), 1);
    assert_eq!(pool.available(ManaColor::Green), 3);
    assert_eq!(pool.total(), 4);
    assert!(is_tapped(&engine, inn));
}

#[test]
fn edgewall_inn_returns_only_an_adventure_card_from_its_own_graveyard() {
    use baylee_core::generated::index;
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[edgewall_inn(), forest(), forest(), forest()])
        .hand(0, &[index::BRAZEN_BORROWER, quiet_creature()])
        .hand(1, &[index::BRAZEN_BORROWER])
        .start();
    keep_mulligans(&mut e);
    if matches!(e.pending(), Pending::ChooseColor { .. }) {
        e.apply(p0, PlayerAction::ChooseColor(ManaColor::Green))
            .unwrap();
    }
    reach_main_phase(&mut e, p0);
    let adventure = in_hand(&e, p0, index::BRAZEN_BORROWER).unwrap();
    let plain = in_hand(&e, p0, quiet_creature()).unwrap();
    let other = in_hand(&e, p1, index::BRAZEN_BORROWER).unwrap();
    for (card, owner) in [(adventure, p0), (plain, p0), (other, p1)] {
        e.dev_state_mut(p0)
            .unwrap()
            .move_object(
                card,
                ZoneLocation::Graveyard(owner),
                crate::zone::ZonePosition::Top,
                crate::event::Cause::Effect,
            )
            .unwrap();
    }
    let inn = on_battlefield(&e, p0, edgewall_inn()).unwrap();
    e.dev_state_mut(p0).unwrap().set_tapped(inn, false);
    e.refresh_offer();
    tap_mana_except(&mut e, p0, inn);
    activate(&mut e, p0, edgewall_inn(), 1);
    let Pending::ChooseTargets { options, .. } = e.pending() else {
        unreachable!()
    };
    assert_eq!(options, &vec![adventure]);
    aim_at(&mut e, p0, adventure);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().object(adventure).unwrap().zone, Zone::Hand);
    assert_eq!(e.state().object(inn).unwrap().zone, Zone::Graveyard);
}
