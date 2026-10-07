//! `cards/lands/utility/edgewall_inn.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Edgewall Inn prints `This land enters tapped.`, `As this land enters, choose a color.`,
/// `{{T}}: Add one mana of the chosen color.`, and `{{3}}, {{T}}, Sacrifice this land: Return
/// target card that has an Adventure from your graveyard to your hand.`
///
/// Under `Coverage::Partial`, `EnterModifier::Tapped`, `EnterModifier::ChooseColor`, and the chosen
/// color mana ability are implemented, while the adventure return ability is omitted.
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
        "under `Coverage::Partial`, adventure return ability is omitted despite floating {{3}}"
    );

    activate(&mut engine, p0, edgewall_inn(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Black), 1);
    assert_eq!(pool.available(ManaColor::Green), 3);
    assert_eq!(pool.total(), 4);
    assert!(is_tapped(&engine, inn));
}
