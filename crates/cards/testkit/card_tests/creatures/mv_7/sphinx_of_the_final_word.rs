//! `cards/creatures/mv_7/sphinx_of_the_final_word.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Sphinx of the Final Word: hexproof, read as a target offer.
///
/// An opponent's Swords to Plowshares is offered the Elf beside the Sphinx
/// and not the Sphinx — a difference in one creature's keywords and in
/// nothing else about the board. The two "can't be countered" sentences are
/// the two tests below.
#[test]
fn sphinx_of_the_final_word_is_no_target_for_an_opponents_spell() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(394, forest())
        .battlefield(0, &[sphinx_of_the_final_word(), llanowar_elves()])
        .battlefield(1, &[plains()])
        .hand(1, &[swords_to_plowshares()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let sphinx =
        on_battlefield(&engine, p0, sphinx_of_the_final_word()).expect("the Sphinx is seated");
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elf is seated");
    assert!(
        keywords(&engine, sphinx).contains(KeywordSet::FLYING),
        "flying is printed"
    );

    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, p1);
    cast_with_floating(&mut engine, p1, swords_to_plowshares());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected a target choice, got {:?}", engine.pending())
    };
    assert!(
        options.contains(&elf),
        "the Elf across the table is a target, so the spell reaches this side"
    );
    assert!(
        !options.contains(&sphinx),
        "and the Sphinx is not, which is hexproof (CR 702.11b): {options:?}"
    );
}

/// Sphinx of the Final Word: "This spell can't be countered."
///
/// Cast into two Islands and a Counterspell, which resolves and counters
/// nothing: the Sphinx arrives.
#[test]
fn sphinx_of_the_final_word_resolves_through_a_counterspell() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(396, island())
        .battlefield(0, &[island(); 7])
        .hand(0, &[sphinx_of_the_final_word()])
        .battlefield(1, &[island(), island()])
        .hand(1, &[counterspell()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let sphinx = in_hand(&engine, p0, sphinx_of_the_final_word()).expect("the Sphinx in hand");
    cast_from_hand(&mut engine, p0, sphinx_of_the_final_word());
    let islands = all_on_battlefield(&engine, p1, island());
    counter_with_two(&mut engine, sphinx, &islands);

    assert!(
        in_graveyard(&engine, p1, counterspell()).is_some(),
        "the Counterspell resolved"
    );
    assert!(
        on_battlefield(&engine, p0, sphinx_of_the_final_word()).is_some(),
        "and the Sphinx arrived anyway"
    );
}

/// Sphinx of the Final Word: "Instant and sorcery spells you control can't be
/// countered."
///
/// With the Sphinx on the battlefield, a Lightning Bolt goes through a
/// Counterspell and a Llanowar Elves does not: the grant is to instants and
/// sorceries, and the Elves are the half of the sentence that says so.
#[test]
fn sphinx_of_the_final_word_keeps_your_instant_uncounterable_and_not_your_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(397, island())
        .battlefield(0, &[sphinx_of_the_final_word(), mountain(), forest()])
        .hand(0, &[lightning_bolt(), llanowar_elves()])
        .battlefield(1, &[island(); 4])
        .hand(1, &[counterspell(), counterspell()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let islands = all_on_battlefield(&engine, p1, island());
    let mountain_id = on_battlefield(&engine, p0, mountain()).expect("the Mountain");
    let forest_id = on_battlefield(&engine, p0, forest()).expect("the Forest");

    let bolt = in_hand(&engine, p0, lightning_bolt()).expect("the Bolt in hand");
    tap_mana_where(&mut engine, p0, |id| id == mountain_id);
    cast_with_floating(&mut engine, p0, lightning_bolt());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .expect("the opponent is a legal target");
    counter_with_two(&mut engine, bolt, &islands[..2]);
    assert_eq!(
        engine.state().players[1].life,
        17,
        "the Bolt resolved through the Counterspell"
    );

    let elves = in_hand(&engine, p0, llanowar_elves()).expect("the Elves in hand");
    tap_mana_where(&mut engine, p0, |id| id == forest_id);
    cast_with_floating(&mut engine, p0, llanowar_elves());
    counter_with_two(&mut engine, elves, &islands[2..]);
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_none(),
        "a creature spell is not an instant or sorcery: countered"
    );
    assert!(in_graveyard(&engine, p0, llanowar_elves()).is_some());
}
