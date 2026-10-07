//! `cards/lands/surveil/undercity_sewers.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Undercity Sewers is a Land — Island Swamp whose printed sentences are
/// "This land enters tapped", "{T}: Add {U} or {B}" and an enters-surveil.
/// The surveil is the subject of the two tests above and is merely walked
/// past here — `pass_until` keeps everything, which is why this test may
/// count a library at all. The land is *played* rather than
/// seeded onto the battlefield, which is the only way the tapped entry is a
/// rule at all: `starting_battlefield` places a permanent with
/// `Cause::Setup`, and no replacement effect looks at a placement. The turn
/// cycle that follows keeps the second half honest — the same permanent is
/// untapped again afterwards, so the {T} this test pays is a real cost on a
/// real land and not a free tap on a frozen one.
#[test]
fn undercity_sewers_enters_tapped_and_taps_for_blue_or_black() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);

    // The entry, with the control beside it: the same helper, the same seat,
    // the same first main phase — a Forest, which arrives untapped. Without
    // that, the assertion below would hold for a harness that taps whatever
    // it plays.
    let (mut engine, land) = play_land_face(undercity_sewers(), 0)
        .expect("a land in hand, on an empty board, in a first main phase");
    let (control_engine, forest) =
        play_land_face(basic_forest(), 0).expect("and the same for a basic Forest");
    assert!(
        entered_tapped(&engine, land),
        "\"This land enters tapped.\""
    );
    assert!(
        !entered_tapped(&control_engine, forest),
        "the harness' own land drop taps nothing by itself"
    );

    // Through the opponent's turn and back: `walk_to_own_main` answers "you
    // are already there" from the main phase this started in, so the walk has
    // to leave it first.
    reach_their_main_phase(&mut engine, p1);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "p0 takes another turn and reaches its own main phase"
    );
    assert!(
        !is_tapped(&engine, land),
        "the untap step gives the Sewers back, so the sentence above was the \
         entry and not a permanent that never untaps"
    );

    // {T}: Add {U} or {B}. Both basic land types sit on one face, so the card
    // prints the ability itself and it is pressed like any other — and it
    // asks which of the two colours this tap is for.
    activate(&mut engine, p0, undercity_sewers(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "\"Add {{U}} or {{B}}\" is a question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that tapped names the colour");
    assert_eq!(
        options.len(),
        2,
        "the two colours it prints and no third: {options:?}"
    );
    assert!(options.contains(&ManaColor::Blue), "{options:?}");
    assert!(options.contains(&ManaColor::Black), "{options:?}");

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .expect("black was one of the two it offered");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Black),
        1,
        "the colour that was named, and not a default"
    );
    assert_eq!(
        pool.available(ManaColor::Blue),
        0,
        "the other half of the choice was not paid"
    );
    assert_eq!(pool.total(), 1, "one mana off one tap");
    assert!(is_tapped(&engine, land), "the land paid its own {{T}}");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to \
         resolve"
    );
}
