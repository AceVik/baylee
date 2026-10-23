//! `cards/sorceries/mv_5/reign_of_terror.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::partials_w2c_kit::{bears as grizzly_bears, savannah_lions};

fn reign_of_terror() -> CardIndex {
    card_index("67d12a5a-9e0c-4550-86ad-cd1e173fea8f")
}

/// Wall of Brambles: a green 2/3 with `{G}: Regenerate`.
fn wall_of_brambles() -> CardIndex {
    card_index("f8d82a00-c10e-4b9e-9642-d05706900a97")
}

/// Casts Reign of Terror off an open board and answers the "green or white"
/// question with `mode` (0 is green, 1 is white).
#[track_caller]
fn cast_and_choose(engine: &mut Engine<RegistryLookup>, mode: usize) {
    cast_from_hand(engine, PlayerId::new(0), reign_of_terror());
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseCastMode { .. })
    });
    engine
        .apply(PlayerId::new(0), PlayerAction::ChooseMode(mode))
        .unwrap();
    pass_until(engine, stack_is_empty);
}

/// Reign of Terror (`{3}{B}{B}`): choosing green destroys the green creature
/// and nothing else, and the caster loses 2 life for the one creature that
/// died.
#[test]
fn reign_of_terror_destroys_chosen_color_creatures() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[swamp(); 5])
        .battlefield(1, &[grizzly_bears(), savannah_lions(), gray_ogre()])
        .hand(0, &[reign_of_terror()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_and_choose(&mut engine, 0);

    assert!(
        on_battlefield(&engine, p1, grizzly_bears()).is_none(),
        "green creature is destroyed when mode 0 is chosen"
    );
    assert!(
        on_battlefield(&engine, p1, savannah_lions()).is_some(),
        "white creature survives mode 0"
    );
    assert!(
        on_battlefield(&engine, p1, gray_ogre()).is_some(),
        "red creature survives mode 0"
    );
    assert_eq!(
        engine.state().players[0].life,
        18,
        "2 life for the one creature that died this way"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "\"you lose\": the opponent loses nothing"
    );
}

/// The second choice destroys white creatures of both players, and the caster
/// loses 2 life for each one, its own included.
#[test]
fn reign_of_terror_white_choice_destroys_both_sides_white_creatures_and_costs_two_each() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                savannah_lions(),
            ],
        )
        .battlefield(1, &[grizzly_bears(), savannah_lions(), gray_ogre()])
        .hand(0, &[reign_of_terror()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_and_choose(&mut engine, 1);

    assert!(
        on_battlefield(&engine, p0, savannah_lions()).is_none(),
        "the caster's own white creature is destroyed too"
    );
    assert!(on_battlefield(&engine, p1, savannah_lions()).is_none());
    assert!(
        on_battlefield(&engine, p1, grizzly_bears()).is_some(),
        "the green creature survives the white choice"
    );
    assert!(on_battlefield(&engine, p1, gray_ogre()).is_some());
    assert_eq!(
        engine.state().players[0].life,
        16,
        "two creatures died this way: 2 life each"
    );
    assert_eq!(engine.state().players[1].life, 20);
}

/// "You lose 2 life for each creature that died this way": with no creature of
/// the chosen color nothing died, so nothing is lost.
#[test]
fn reign_of_terror_costs_nothing_when_no_creature_dies() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[swamp(); 5])
        .battlefield(1, &[savannah_lions(), gray_ogre()])
        .hand(0, &[reign_of_terror()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_and_choose(&mut engine, 0);

    assert!(on_battlefield(&engine, p1, savannah_lions()).is_some());
    assert!(on_battlefield(&engine, p1, gray_ogre()).is_some());
    assert_eq!(
        engine.state().players[0].life,
        20,
        "no green creature died, so no life is lost"
    );
}

/// "They can't be regenerated": a green Wall of Brambles with a regeneration
/// shield up still dies, and counts toward the life lost.
#[test]
fn reign_of_terror_destroys_a_creature_that_has_a_regeneration_shield() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                forest(),
                wall_of_brambles(),
            ],
        )
        .battlefield(1, &[grizzly_bears()])
        .hand(0, &[reign_of_terror()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let wall = on_battlefield(&engine, p0, wall_of_brambles()).expect("the Wall is seated");

    // `{G}: Regenerate` is bought off the Forest, then the spell is cast out of
    // the black mana left floating.
    tap_all_mana(&mut engine, p0);
    raise_a_shield(&mut engine, p0, wall, 0);
    cast_with_floating(&mut engine, p0, reign_of_terror());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCastMode { .. })
    });
    engine.apply(p0, PlayerAction::ChooseMode(0)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, wall_of_brambles()).is_none(),
        "the shield cannot replace this destruction"
    );
    assert!(in_graveyard(&engine, p0, wall_of_brambles()).is_some());
    assert!(on_battlefield(&engine, p1, grizzly_bears()).is_none());
    assert_eq!(
        engine.state().players[0].life,
        16,
        "the Wall and the Bears both died this way"
    );
}
