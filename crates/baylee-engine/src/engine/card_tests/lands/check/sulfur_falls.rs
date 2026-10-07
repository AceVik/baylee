//! `cards/lands/check/sulfur_falls.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Sulfur Falls prints "This land enters tapped unless you control an Island
/// or a Mountain" and "{T}: Add {U} or {R}", and the whole entry clause is a
/// question about *whose* table it reads — so the board makes that the only
/// variable: p1 holds both named types from the start and p0 holds neither,
/// and the first Falls played into exactly that has to arrive tapped. A turn
/// cycle later p0 plays a basic Island of its own, and the second Falls — the
/// same card against the same unchanged opposing board — arrives untapped,
/// which is what tells the printed condition from a land that simply enters
/// tapped. The untapped Falls is then tapped for its own {T} with nothing
/// floated beforehand, and red is the color named: no other land on p0's side
/// can make it, so the mana in the pool came off this card and nowhere else.
#[test]
fn sulfur_falls_reads_its_controllers_lands_and_taps_for_red() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        // The opponent holds both types the card names, from the first turn.
        .battlefield(1, &[island(), mountain()])
        .hand(0, &[sulfur_falls(), sulfur_falls(), island()])
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "p0 reaches its own main phase"
    );

    // Turn one: p0 controls no land at all, and the Island and Mountain
    // across the table are not p0's to count.
    let first = play_land(&mut engine, p0, sulfur_falls());
    assert!(
        entered_tapped(&engine, first),
        "\"enters tapped unless you control an Island or a Mountain\" — p0 \
         controls neither, and p1's pair is the opponent's"
    );

    // A turn cycle later, a basic Island of p0's own: the other half of the
    // condition, and the only thing that changes between the two plays.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    let shore = play_land(&mut engine, p0, island());
    assert!(
        !entered_tapped(&engine, shore),
        "a basic Island has nothing to check and arrives untapped"
    );

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    let second = play_land(&mut engine, p0, sulfur_falls());
    assert!(
        !entered_tapped(&engine, second),
        "with an Island under p0 the same card arrives untapped, while the \
         opponent's lands have not moved"
    );

    // Its only printed price is its own {T}, so nothing has to be floated
    // first and nothing else on the board is tapped to pay for it.
    activate(&mut engine, p0, sulfur_falls(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "\"Add {{U}} or {{R}}\" is a question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that tapped the land names the color");
    assert!(
        options.contains(&ManaColor::Blue) && options.contains(&ManaColor::Red),
        "\"{{U}} or {{R}}\": {options:?}"
    );
    assert_eq!(options.len(), 2, "two colors and no third: {options:?}");

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Red))
        .expect("red was one of the two colors the ability offered");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Red),
        1,
        "the color that was named, and not the blue sitting beside it on the menu"
    );
    assert_eq!(
        pool.available(ManaColor::Blue),
        0,
        "the other half of \"{{U}} or {{R}}\" is not made as well"
    );
    assert_eq!(
        pool.total(),
        1,
        "one mana off one tap: the Island and the first Falls are untapped and stayed so"
    );
}
