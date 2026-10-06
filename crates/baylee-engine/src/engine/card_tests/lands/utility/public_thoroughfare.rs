//! `cards/lands/utility/public_thoroughfare.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Public Thoroughfare prints three sentences: it enters tapped, it asks its
/// controller to tap an untapped artifact or land as it arrives and is
/// sacrificed if they will not, and it taps for one mana of any color. The
/// price is paid here with the Forest standing beside it, which leaves the
/// land on the table with the Forest — and not the land — tapped; the same
/// play on a board with nothing to tap must end in the graveyard without a
/// question, because an "unless" whose price is unpayable is no choice at
/// all. Once the untap step has stood the land back up, the printed mana
/// ability is played out, since "any color" is a question with exactly five
/// answers.
#[test]
fn public_thoroughfare_taps_a_land_to_stay_or_dies_alone_and_then_taps_for_any_color() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);

    // ---- a land to tap: the price is payable and the Thoroughfare stays ----
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest()])
        .hand(0, &[public_thoroughfare()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    let forest_land = on_battlefield(&engine, p0, forest()).expect("the Forest is out");

    let land = play_land(&mut engine, p0, public_thoroughfare());
    assert!(
        entered_tapped(&engine, land),
        "\"This land enters tapped\" — read before the enters trigger resolves"
    );

    // The trigger's price is a tap of a permanent of this controller's, so
    // whatever question it raises is answered out of the offer itself.
    match drive_to_rest(&mut engine, p0) {
        Rest::Reached => {}
        other => panic!("the enters trigger never came to rest: {other:?}"),
    }
    assert!(
        on_battlefield(&engine, p0, public_thoroughfare()).is_some(),
        "a land to tap paid the price, so the Thoroughfare is still there"
    );
    assert!(
        is_tapped(&engine, land),
        "and it is still the tapped land it entered as"
    );
    assert!(
        is_tapped(&engine, forest_land),
        "the Forest it named is the permanent that paid for it"
    );

    // ---- across a turn, and then the printed mana ability ----
    reach_their_main_phase(&mut engine, p1);
    assert!(walk_to_own_main(&mut engine, p0), "p0 takes another turn");
    assert!(
        !is_tapped(&engine, land),
        "the untap step stands it back up, which is what makes the tap below possible"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    // The index out of the offer rather than guessed: ability 0 is the
    // enters trigger, which is never activated, and the "{T}: Add one mana
    // of any color" behind it is the only activated ability the card prints.
    let (source, ability_index) = legal
        .abilities
        .iter()
        .copied()
        .find(|(src, _)| *src == land)
        .expect("the untapped land is offered its own mana ability");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .expect("its whole price is its own {T}");
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!("`any color` is a question, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the activating seat is the one that names it");
    assert_eq!(
        options.len(),
        5,
        "the five colors of the game, and colorless is no color at all \
         (CR 105.4): {options:?}"
    );
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Green))
        .expect("green was one of the colors it offered");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1,
        "the color that was named, and not a default"
    );
    assert!(is_tapped(&engine, land), "which tapped the land");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );

    // ---- nothing to tap: the price is unpayable, so nobody is asked ----
    let mut alone = Duel::new(SEED, forest())
        .hand(0, &[public_thoroughfare()])
        .start();
    keep_mulligans(&mut alone);
    assert!(walk_to_own_main(&mut alone, p0), "p0 reaches its own main");
    play_land(&mut alone, p0, public_thoroughfare());
    pass_until(&mut alone, stack_is_empty);
    assert!(
        on_battlefield(&alone, p0, public_thoroughfare()).is_none(),
        "an untapped artifact or land of its controller's is the whole price, \
         and there is none: the question is never asked and the land is sacrificed"
    );
    assert!(
        in_graveyard(&alone, p0, public_thoroughfare()).is_some(),
        "and a sacrificed permanent goes to its owner's graveyard"
    );
}
