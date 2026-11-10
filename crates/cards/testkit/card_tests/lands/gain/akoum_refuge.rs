//! `cards/lands/gain/akoum_refuge.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Akoum Refuge prints three lines and a land drop is the only way to reach
/// all three: it enters tapped, it gains its controller 1 life as it enters,
/// and it taps for {B} or {R}. The two halves are read against each other,
/// because either one alone is satisfied by the wrong card — a life gain
/// asserted without the tap would pass on any lifegain land, and the colour
/// question asserted without the tap would pass on a land that entered
/// untapped. Nothing is put on the battlefield but the Refuge, so the single
/// red in the pool afterwards cannot have come from anywhere else, and the
/// `{T}` is only payable a full turn later, which is what says the entry
/// really was tapped.
#[test]
fn akoum_refuge_enters_tapped_gains_one_life_and_taps_for_black_or_red() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(812, forest())
        .hand(0, &[akoum_refuge()])
        .life(0, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // A real `PlayLand` and not `starting_battlefield`: an entry is what the
    // replacement effect reads, and a placed permanent never has one.
    let land = play_land(&mut engine, p0, akoum_refuge());
    assert!(
        entered_tapped(&engine, land),
        "\"This land enters tapped\" applies to the entry the land drop made"
    );
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[0].life,
        21,
        "\"When this land enters, you gain 1 life\" — once, off the one \
         entry it had, and for the seat that played it"
    );

    // The other side of the tapped entry: `{T}` is unpayable while the
    // permanent is down, so the ability is not in the offer at all.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.iter().any(|(id, _)| *id == land),
        "a land that entered tapped gives nothing this turn: {:?}",
        legal.abilities
    );

    // Across the opponent's turn and back, so that the untap step has run
    // and the `{T}` price is payable for the first time.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(!entered_tapped(&engine, land), "the untap step stood it up");

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let (source, ability_index) = legal
        .abilities
        .iter()
        .copied()
        .find(|(id, _)| *id == land)
        .expect("the printed {T} is offered once the land is untapped");
    assert!(
        !legal.mana_abilities.contains(&land),
        "a nonbasic land's own printed tap is an `abilities` entry and not \
         the CR 305.6 shortcut: {:?}",
        legal.mana_abilities
    );
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .expect("an untapped land and no other price");

    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "`{{T}}: Add {{B}} or {{R}}` is a question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the land's controller is the one who names it");
    assert_eq!(
        options,
        vec![ManaColor::Black, ManaColor::Red],
        "the two colours the card prints, and no third"
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
    assert_eq!(
        pool.total(),
        1,
        "one mana off one tap: the Refuge is the only source on this board"
    );
    assert!(is_tapped(&engine, land), "the Refuge paid its own {{T}}");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack"
    );
}
