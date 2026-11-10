//! `cards/lands/gain/wind_scarred_crag.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Wind-Scarred Crag prints three lines and this plays all of them in one
/// game: it enters tapped, its entry trigger gains 1 life, and it taps for
/// {R} or {W}. The life is read as a move — twenty before the land is played,
/// twenty-one once the trigger has resolved — and the tapped entry is read
/// off the permanent, which is what makes the mana line unofferable in the
/// turn it arrived: `{T}` is a price a tapped land cannot pay, and the untap
/// step is the only thing that lifts it. A turn cycle later the same offer
/// does name the land, and the tap is played out: a colour question holding
/// exactly the two colours the card prints, the mana in the pool the moment
/// the answer lands, and no stack (CR 605.3b).
#[test]
fn wind_scarred_crag_enters_tapped_gains_a_life_and_taps_for_red_or_white() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[wind_scarred_crag()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let life_before = engine.state().players[0].life;

    let crag = play_land(&mut engine, p0, wind_scarred_crag());
    pass_until(&mut engine, stack_is_empty);

    assert!(entered_tapped(&engine, crag), "This land enters tapped");
    assert_eq!(
        engine.state().players[0].life,
        life_before + 1,
        "\"When this land enters, you gain 1 life\" — the trigger is the only \
         thing that moved the total"
    );

    // The price of the card's only activated ability is its own `{T}`, so a
    // land that entered tapped has nothing to offer this turn — and with no
    // mana in the pool there is no second reading to confuse the answer.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.iter().any(|(source, _)| *source == crag),
        "a tapped Crag offers no {{T}} to pay: {:?}",
        legal.abilities
    );

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, crag),
        "the untap step stands it back up, which is the only thing that makes \
         the assertion above a claim about the entry and not about a missing turn"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let (source, ability_index) = legal
        .abilities
        .iter()
        .copied()
        .find(|(id, _)| *id == crag)
        .expect("an untapped Crag offers its printed mana ability");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .expect("the ability was offered, so the offer is taken");

    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "`Add {{R}} or {{W}}` is a question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat names the colour");
    assert!(
        options.contains(&ManaColor::Red) && options.contains(&ManaColor::White),
        "both colours the card prints are on the menu: {options:?}"
    );
    assert_eq!(
        options.len(),
        2,
        "and nothing else — a third option would be a different card: {options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Red))
        .expect("red was one of the colours it offered");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Red),
        1,
        "the colour that was named, and not a default"
    );
    assert_eq!(pool.total(), 1, "one land, one tap, one mana");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
    assert!(is_tapped(&engine, crag), "and the Crag paid its own {{T}}");
}
