//! The manlands whose animation names the colours it makes, played together.
//!
//! "This land becomes a 3/3 black Beholder creature" *sets* the colours of the
//! land (CR 105.3: an effect that says an object "becomes" a colour gives it
//! that colour, replacing any it had); it does not add a colour to whatever
//! the land already was. A land that something else has already coloured
//! shows the difference: after the animation it has exactly the printed
//! colours and not those plus the earlier one.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::engine::card_tests::partials_w1b_kit as kit;
use baylee_core::color::{Color, ColorSet};

/// The five "lace" instants of Alpha, each making its target a colour: the
/// colour-setting effect that gives a manland a colour before it is animated.
/// `(colour, oracle id)`, in WUBRG order.
const LACES: [(Color, &str); 5] = [
    (Color::White, "3773001a-8868-49ec-a406-298cf72359c2"), // Purelace
    (Color::Blue, "6452b6a6-6235-46a3-a712-a26592450438"),  // Thoughtlace
    (Color::Black, "fb80aaba-352a-4b58-8db2-1e02d542819c"), // Deathlace
    (Color::Red, "08842aa3-f923-46e9-a106-f542331e9cc1"),   // Chaoslace
    (Color::Green, "eec1de80-4b3d-481d-a235-c299e0381830"), // Lifelace
];

/// The manlands: `(name, oracle id, the colours its animation prints)`. The
/// colours are read off the Oracle text in `data/scryfall-cache/<slug>.json`:
/// "becomes a 3/3 **black** Beholder creature", "a 3/4 **red and green**
/// Dinosaur creature" and so on.
const MANLANDS: [(&str, &str, &[Color]); 12] = [
    (
        "Hive of the Eye Tyrant",
        "d17163d4-dd43-4de6-b7cf-576448160b7f",
        &[Color::Black],
    ),
    (
        "Den of the Bugbear",
        "f451b8f0-1ff5-4e8d-9f30-9352d83ed687",
        &[Color::Red],
    ),
    (
        "Restless Ridgeline",
        "4c0f4a63-586a-4dde-9621-b0dd9118b2e5",
        &[Color::Red, Color::Green],
    ),
    (
        "Creeping Tar Pit",
        "250cb58b-2924-4dff-92fe-ac0ebbbeb218",
        &[Color::Blue, Color::Black],
    ),
    (
        "Restless Vinestalk",
        "0935faa2-fb90-48db-8a92-906ba0f374c7",
        &[Color::Green, Color::Blue],
    ),
    (
        "Hall of Storm Giants",
        "087c8c0e-a91c-4e3c-8387-9312db01f343",
        &[Color::Blue],
    ),
    (
        "Spawning Pool",
        "f3bf22cf-0a6f-4fb6-ba82-63ce290308d6",
        &[Color::Black],
    ),
    (
        "Svogthos, the Restless Tomb",
        "a34a70b8-02e5-4e8c-a9e7-b21c5a11dddf",
        &[Color::Black, Color::Green],
    ),
    (
        "Raging Ravine",
        "8d38194e-b607-4ff4-9c19-0e8636d463bf",
        &[Color::Red, Color::Green],
    ),
    (
        "Wandering Fumarole",
        "741c51f1-cfbe-4c29-ac8f-ca6bcd2652f9",
        &[Color::Blue, Color::Red],
    ),
    (
        "Restless Anchorage",
        "91320daf-f69c-4350-b0fc-4bb37a6904b1",
        &[Color::White, Color::Blue],
    ),
    (
        "Lair of the Hydra",
        "126e9140-2c05-4c00-8b01-5653456c736a",
        &[Color::Green],
    ),
];

/// Colours `manland` before and after its animation, with a lace cast on it
/// first: `(the lace's colour, colours after the lace, colours after the
/// animation)`. The lace is the first one whose colour the animation does not
/// print, so that an animation that only added a colour would show it.
fn colours_around_the_animation(
    manland: CardIndex,
    printed: ColorSet,
) -> (Color, ColorSet, ColorSet) {
    let p0 = kit::P0;
    let (lace_colour, lace_id) = LACES
        .iter()
        .copied()
        .find(|(colour, _)| !printed.contains(*colour))
        .expect("no manland prints all five colours");
    let lace = card_index(lace_id);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                manland,
                plains(),
                plains(),
                island(),
                island(),
                swamp(),
                swamp(),
                mountain(),
                mountain(),
                forest(),
                forest(),
            ],
        )
        .hand(0, &[lace, llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let land = on_battlefield(&engine, p0, manland).expect("the manland");
    // One creature card in the graveyard: Svogthos is as big as the number of
    // them and would die as a 0/0 the moment it was animated.
    hand_to_graveyard(&mut engine, p0, llanowar_elves());

    cast_from_hand(&mut engine, p0, lace);
    aim_at(&mut engine, p0, land);
    pass_until(&mut engine, stack_is_empty);
    let after_the_lace = kit::colors_of(&engine, land);

    // Casting the lace tapped every land for mana, the manland's own included,
    // and what the lace did not use is still floating: it pays for the
    // animation, the second ability each of these prints (the mana ability,
    // the first, is not offered while the land is tapped).
    let offered = kit::offered(&engine, land);
    assert!(
        offered.contains(&1),
        "the animation is affordable and offered: {offered:?}"
    );
    kit::activate_object(&mut engine, p0, land, 1);
    kit::settle(&mut engine, p0, |_, pending| match pending {
        // Lair of the Hydra: `{X}{G}`, any X from 1 up.
        Pending::ChooseNumber { min, .. } => Some(PlayerAction::ChooseNumber(*min)),
        _ => None,
    });
    assert!(
        types(&engine, land).contains(TypeSet::CREATURE),
        "the animation made it a creature"
    );
    (lace_colour, after_the_lace, kit::colors_of(&engine, land))
}

/// "Until end of turn, this land becomes a … <colours> … creature": the
/// colours the Oracle text prints, and no others (CR 105.3).
///
/// For each of twelve manlands a lace has first made the land another colour,
/// one the animation does not print. Afterwards its colours are exactly the
/// printed ones: neither the lace's colour nor any other has stayed.
#[test]
fn a_manland_animation_sets_the_printed_colours_it_does_not_add_to_them() {
    let mut wrong = Vec::new();
    for (name, oracle_id, colours) in MANLANDS {
        let printed = ColorSet::from_slice(colours);
        let manland = card_index(oracle_id);
        let ran = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            colours_around_the_animation(manland, printed)
        }));
        match ran {
            Err(_) => wrong.push(format!(
                "{name}: could not be animated (see the panic above)"
            )),
            Ok((lace, after_the_lace, after)) => {
                if after_the_lace != ColorSet::of(lace) {
                    wrong.push(format!(
                        "{name}: the lace did not make it exactly {lace:?}: {after_the_lace:?}"
                    ));
                }
                if after != printed {
                    wrong.push(format!(
                        "{name}: animated it has {after:?}, the Oracle text prints {printed:?} \
                         (it was {lace:?} before)"
                    ));
                }
            }
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
