//! `cards/creatures/mv_7/windreader_sphinx.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn windreader_sphinx() -> CardIndex {
    card_index("5e038a3a-fbab-46c2-8773-1327d953da16")
}

/// Windreader Sphinx — a 3/7 flier for {5}{U}{U}.
#[test]
fn windreader_sphinx_costs_seven_and_is_a_three_seven_flier() {
    let mut engine = table7(&lots(island(), 7), &[windreader_sphinx()], &[]);
    let sphinx = cast_and_resolve(&mut engine, windreader_sphinx());
    assert_body(&engine, sphinx, (3, 7), KeywordSet::FLYING);
}

fn my_hand(engine: &Engine<RegistryLookup>) -> usize {
    engine
        .state()
        .zones
        .list(ZoneLocation::Hand(PlayerId::new(0)))
        .len()
}

/// "Whenever a creature with flying attacks, you may draw a card": its own
/// attack is one, and the "may" is answered yes on the way.
#[test]
fn windreader_sphinx_draws_when_it_attacks() {
    let p0 = PlayerId::new(0);
    let mut engine = table7(&[windreader_sphinx()], &[], &[]);
    let sphinx = on_battlefield(&engine, p0, windreader_sphinx()).expect("out");
    let hand = my_hand(&engine);
    fight(&mut engine, &[sphinx], &[]);
    assert_eq!(my_hand(&engine), hand + 1);
}

/// The trigger is any creature with flying, an opposing one included; a
/// ground attacker draws nothing.
#[test]
fn windreader_sphinx_draws_when_their_flier_attacks_but_not_their_ground_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[windreader_sphinx()])
        .battlefield(1, &[flier(), gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    let theirs_flier = on_battlefield(&engine, p1, flier()).expect("out");
    let theirs_ground = on_battlefield(&engine, p1, gray_ogre()).expect("out");

    // The ground creature first, on the opponent's turn: the hand is read
    // between my draw step and their attack.
    pass_until(&mut engine, |e| e.state().turn.active == p1);
    let before = my_hand(&engine);
    let _ = their_attack_blocks(&mut engine, theirs_ground);
    assert_eq!(my_hand(&engine), before, "a ground attacker draws nothing");
    block_and_finish(&mut engine, &[]);

    // Their next turn: the flier.
    pass_until(&mut engine, |e| e.state().turn.active == p0);
    pass_until(&mut engine, |e| e.state().turn.active == p1);
    let before = my_hand(&engine);
    let _ = their_attack_blocks(&mut engine, theirs_flier);
    assert_eq!(
        my_hand(&engine),
        before + 1,
        "their flier attacking draws one"
    );
}
