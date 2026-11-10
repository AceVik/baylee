//! `cards/lands/restricted/spire_of_industry.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Spire of Industry and Rivendell, whose clauses ask about something that
/// is not a land at all: an artifact, and a legendary creature.
///
/// Both are laid out twice rather than played into, because what satisfies
/// them is not a land and could not be put on the table by playing one.
/// Rivendell's ability is not a mana ability either — it goes on the stack
/// like any other — so what the second half asserts is that it was allowed
/// on at all.
#[test]
fn a_clause_may_ask_about_an_artifact_or_a_legend_instead() {
    let p0 = PlayerId::new(0);

    let mut engine = Duel::new(950, forest())
        .battlefield(0, &[spire_of_industry()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    assert!(
        !offered(&engine, spire_of_industry(), 1),
        "no artifact, no coloured mana"
    );

    let mut engine = Duel::new(951, forest())
        .battlefield(0, &[spire_of_industry(), lightning_greaves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    assert!(
        offered(&engine, spire_of_industry(), 1),
        "an Equipment is an artifact"
    );
    let life = engine.state().players[0].life;
    activate(&mut engine, p0, spire_of_industry(), 1);
    let Pending::ChooseColor { .. } = engine.pending().clone() else {
        panic!("expected any colour, got {:?}", engine.pending())
    };
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Green))
        .expect("any colour includes green");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1,
        "one mana of the colour chosen"
    );
    assert_eq!(
        engine.state().players[0].life,
        life - 1,
        "and the life the cost asked for"
    );

    let mut engine = Duel::new(952, forest())
        .battlefield(0, &[rivendell(), island(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    tap_all_mana_but(&mut engine, p0, Some(rivendell()));
    assert!(
        !offered(&engine, rivendell(), 1),
        "no legendary creature, no scry"
    );

    let mut engine = Duel::new(953, forest())
        .battlefield(0, &[rivendell(), jin_gitaxias(), island(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    // Rivendell's own `{T}` is what its second ability costs, so it is kept
    // back from the tap (#159).
    tap_all_mana_but(&mut engine, p0, Some(rivendell()));
    assert!(
        offered(&engine, rivendell(), 1),
        "Jin-Gitaxias is a legendary creature"
    );
    activate(&mut engine, p0, rivendell(), 1);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Stack).len(),
        1,
        "the scry is an ordinary activated ability and uses the stack"
    );
}
