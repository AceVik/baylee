//! `cards/instants/mv_2/assassin_s_trophy.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Assassin's Trophy: "Destroy target permanent an opponent controls. Its
/// controller may search their library for a basic land card, put it onto
/// the battlefield, then shuffle."
///
/// This proves that the permanent is destroyed, that the offer goes to the
/// *target's controller* (p1, not p0), over p1's own library, and that the
/// fetched land arrives untapped — it arrived tapped while the card borrowed
/// Path to Exile's search.
#[test]
fn assassins_trophy_destroys_the_target_and_offers_its_controller_a_basic_land_search() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(77, forest())
        .battlefield(0, &[swamp(), forest()])
        .hand(0, &[assassin_s_trophy()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let victim = on_battlefield(&engine, p1, llanowar_elves()).expect("the Elf is deployed");
    let lands_before = lands_of(&engine, p1).len();

    cast_from_hand(&mut engine, p0, assassin_s_trophy());
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!("Trophy asks for a target, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "their spell, their target choice");
    assert!(
        options.contains(&victim),
        "the opponent's creature is a permanent an opponent controls: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![victim],
            },
        )
        .expect("the Elf is a legal target");

    // Wait for the search question, which belongs to the Elf's controller (p1).
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected the basic-land search, got {:?}", engine.pending())
    };
    assert_eq!(
        player, p1,
        "\"Its controller\" is the target's controller (p1), not the caster (p0)"
    );
    assert_eq!((min, max), (0, 1), "\"may search\" — zero or one card");
    let theirs = engine.state().zones.list(ZoneLocation::Library(p1)).clone();
    assert!(
        !options.is_empty() && options.iter().all(|o| theirs.contains(o)),
        "their own library holds basics to search for"
    );

    // The destroy half happened before the search was offered.
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_none(),
        "\"Destroy target permanent\" — the Elf is no longer on the battlefield"
    );

    // Take the land; it arrives untapped.
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .expect("p1 takes the basic land");
    pass_until(&mut engine, |e| lands_of(e, p1).len() > lands_before);

    let fetched = *lands_of(&engine, p1)
        .last()
        .expect("the fetched land arrived");
    assert!(
        !is_tapped(&engine, fetched),
        "\"put it onto the battlefield\" says nothing of tapped"
    );
}
