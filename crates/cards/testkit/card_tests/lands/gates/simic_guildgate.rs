//! `cards/lands/gates/simic_guildgate.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Simic Guildgate prints two lines and nothing else: "This land enters
/// tapped" and "{T}: Add {G} or {U}." Both halves are one land drop, and the
/// drop has to be a real `PlayLand` — a permanent seeded through
/// `starting_battlefield` is placed with `Cause::Setup`, no replacement
/// effect looks at it, and a board built that way arrives untapped whatever
/// the card says.
///
/// The turn cycle is what separates the halves. While the Gate lies down, its
/// `{T}` is not merely unwise to pay — it is withheld from the offer, since
/// the price is its own tap and the permanent is tapped. Only its controller's
/// untap step stands it back up, and then the same ability asks which of the
/// two printed colours is wanted, so the pool afterwards says which answer was
/// read rather than assuming one.
#[test]
fn simic_guildgate_arrives_tapped_and_taps_for_either_of_its_two_colors() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[simic_guildgate(), forest()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let gate = play_land(&mut engine, p0, simic_guildgate());
    assert!(
        entered_tapped(&engine, gate),
        "a real land drop goes through the replacement effect: \
         `EnterModifier::Tapped` is the printed first sentence"
    );

    // "A land that enters tapped gives nothing this turn": the {T} is not
    // merely unwise to pay, it is not on the offer at all.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!(
            "the land drop leaves the seat holding priority: {:?}",
            engine.pending()
        )
    };
    assert!(
        !legal.abilities.iter().any(|(source, _)| *source == gate),
        "the Gate stands tapped, so its own {{T}} is refused: {:?}",
        legal.abilities
    );

    // Across the opponent's turn and back, because the Gate's controller's
    // untap step is the only thing that can stand it up again.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, gate),
        "its controller's untap step is what hands the mana ability back"
    );

    // Ability 0 is the printed "{T}: Add {G} or {U}." The whole price is its
    // own tap, so nothing has to be floated before it is pressed — and no
    // other source on this board is tapped, so the pool below can only have
    // come off the Gate.
    activate(&mut engine, p0, simic_guildgate(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "\"Add {{G}} or {{U}}\" is a question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat is the one that names it");
    assert!(
        options.contains(&ManaColor::Green) && options.contains(&ManaColor::Blue),
        "both halves of the printed line are on the menu: {options:?}"
    );
    assert_eq!(options.len(), 2, "and nothing else: {options:?}");

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .expect("blue was one of the colours it offered");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Blue),
        1,
        "the colour that was named, and not a default"
    );
    assert_eq!(pool.available(ManaColor::Green), 0, "and not the other one");
    assert_eq!(pool.total(), 1, "one mana, off one tap");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
    assert!(is_tapped(&engine, gate), "the Gate paid its own {{T}}");
}
