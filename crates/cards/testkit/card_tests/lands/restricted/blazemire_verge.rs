//! `cards/lands/restricted/blazemire_verge.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// The verge cycle and Nimbus Maze: "{T}: Add {X}. Activate only if you
/// control a <land type> or a <land type>."
///
/// Eleven lands, each played twice — once for each land type its clause
/// names — because the clause is an `Or` and a reader that dropped one arm
/// would still pass a test that only ever tried the other. The board starts
/// with the verge alone, which is the half that says the condition is doing
/// anything at all: the unconditional ability beside it is offered in the
/// same breath, so an absent second ability is the clause and not an empty
/// list.
///
/// The land that satisfies the clause is then **played from hand**, so what
/// is asserted is a condition re-read between two priorities rather than
/// one decided when the game was laid out.
///
/// Bleachbone Verge rides along although it is hand-written and older: it
/// is the same sentence written by a person, and this is the one place the
/// two spellings of `Condition::ControlCount` are held against each other
/// in play.
#[test]
fn a_verge_adds_its_second_colour_only_beside_the_land_its_clause_names() {
    let p0 = PlayerId::new(0);
    for (seed, card, key, colour) in [
        (901, blazemire_verge(), swamp(), ManaColor::Red),
        (902, blazemire_verge(), mountain(), ManaColor::Red),
        (903, bleachbone_verge(), plains(), ManaColor::White),
        (904, bleachbone_verge(), swamp(), ManaColor::White),
        (905, floodfarm_verge(), plains(), ManaColor::Blue),
        (906, floodfarm_verge(), island(), ManaColor::Blue),
        (907, gloomlake_verge(), island(), ManaColor::Black),
        (908, gloomlake_verge(), swamp(), ManaColor::Black),
        (909, hushwood_verge(), forest(), ManaColor::White),
        (910, hushwood_verge(), plains(), ManaColor::White),
        (911, riverpyre_verge(), island(), ManaColor::Blue),
        (912, riverpyre_verge(), mountain(), ManaColor::Blue),
        (913, sunbillow_verge(), mountain(), ManaColor::Red),
        (914, sunbillow_verge(), plains(), ManaColor::Red),
        (915, thornspire_verge(), mountain(), ManaColor::Green),
        (916, thornspire_verge(), forest(), ManaColor::Green),
        (917, wastewood_verge(), swamp(), ManaColor::Black),
        (918, wastewood_verge(), forest(), ManaColor::Black),
        (919, willowrush_verge(), forest(), ManaColor::Green),
        (920, willowrush_verge(), island(), ManaColor::Green),
        // Nimbus Maze prints the pair the other way round — the Island
        // makes the {W} and the Plains the {U} — which is exactly the
        // reading a transcoder could get backwards in silence.
        (921, nimbus_maze(), island(), ManaColor::White),
        (922, nimbus_maze(), plains(), ManaColor::Blue),
    ] {
        let index = if card == nimbus_maze() && colour == ManaColor::Blue {
            2
        } else {
            1
        };
        let mut engine = Duel::new(seed, forest())
            .battlefield(0, &[card])
            .hand(0, &[key])
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);

        assert!(
            offered(&engine, card, 0),
            "seed {seed}: the ability with no clause is offered"
        );
        assert!(
            !offered(&engine, card, index),
            "seed {seed}: and the one with a clause is not, with nothing beside it"
        );

        play_land(&mut engine, p0, key);
        assert!(
            offered(&engine, card, index),
            "seed {seed}: the land it names arrived"
        );
        activate(&mut engine, p0, card, index);
        assert_eq!(
            engine.state().players[0].mana_pool.available(colour),
            1,
            "seed {seed}: and it added {colour:?}"
        );
    }
}
