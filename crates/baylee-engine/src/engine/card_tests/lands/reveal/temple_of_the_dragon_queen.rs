//! `cards/lands/reveal/temple_of_the_dragon_queen.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Temple of the Dragon Queen prints `As this land enters, you may reveal a Dragon
/// card from your hand. This land enters tapped unless you revealed a Dragon card
/// this way or you control a Dragon.`, `As this land enters, choose a color.`, and
/// `{{T}}: Add one mana of the chosen color.`
///
/// Under `Coverage::Partial`, revealing a Dragon from hand is omitted because no
/// `EnterModifier` reveals cards from hand. Without a Dragon controlled, the land
/// enters tapped and prompts for a color choice via `Pending::ChooseColor`.
/// After advancing to the next turn, the land untaps and taps for one mana of
/// the chosen color.
#[test]
fn temple_of_the_dragon_queen_enters_tapped_chooses_color_and_taps_for_it() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[temple_of_the_dragon_queen()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let card = in_hand(&engine, p0, temple_of_the_dragon_queen()).expect("temple in hand");
    engine.apply(p0, PlayerAction::PlayLand { card }).unwrap();

    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!("expected ChooseColor, got {:?}", engine.pending());
    };
    assert_eq!(player, p0);
    assert_eq!(options, baylee_cards_dsl::ALL_MANA_COLORS.to_vec());
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Red))
        .unwrap();

    let land =
        on_battlefield(&engine, p0, temple_of_the_dragon_queen()).expect("temple on battlefield");
    assert!(entered_tapped(&engine, land));

    reach_their_main_phase(&mut engine, PlayerId::new(1));
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, land));

    activate(&mut engine, p0, temple_of_the_dragon_queen(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Red), 1);
    assert_eq!(pool.total(), 1);
    assert!(is_tapped(&engine, land));
}
