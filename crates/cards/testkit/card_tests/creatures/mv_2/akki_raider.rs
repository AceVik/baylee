//! `cards/creatures/mv_2/akki_raider.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Akki Raider is a `2/1` Goblin Warrior for `{1}{R}`, whose entire text
/// reads: "Whenever a land is put into a graveyard from the battlefield,
/// this creature gets +1/+0 until end of turn."
///
/// That is two zones and exactly one event, so both are played: the
/// test kit puts a land card from the library into the graveyard of the
/// same seat — the same card, the same graveyard, no pump —, and afterward
/// the same seat casts Vindicate on its own Forest, which is the printed
/// event and pumps. The Elves next to it are the third reading: the text
/// says "this creature", so the 1/1 remains, which has seen the same
/// death.
#[test]
fn akki_raider_grows_when_a_land_leaves_the_battlefield_for_the_graveyard() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                swamp(),
                forest(),
                akki_raider(),
                llanowar_elves(),
            ],
        )
        .hand(0, &[vindicate()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let raider = on_battlefield(&engine, p0, akki_raider()).expect("der Raider steht");
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves stand");
    let doomed = on_battlefield(&engine, p0, forest()).expect("one's own Forest stands");
    assert_eq!(pt(&engine, raider), (2, 1), "eine gedruckte 2/1");
    assert_eq!(pt(&engine, elves), (1, 1), "und eine gedruckte 1/1 daneben");

    // The same card and the same graveyard — only out of the library
    // rather than off the battlefield. "Dies" is CR 700.4's event and this
    // is not it.
    seed_graveyard(&mut engine, p0, 1);
    let graveyard_before = engine.state().zones.list(ZoneLocation::Graveyard(p0)).len();
    assert_eq!(graveyard_before, 1, "a land card is in the graveyard");
    assert_eq!(
        pt(&engine, raider),
        (2, 1),
        "\"put into a graveyard from the battlefield\" — a card that \
         was never on the battlefield has not died"
    );

    // The event itself, created by a real spell and not by the
    // test kit. Vindicate is a sorcery, so everything happens in the
    // Raider owner's own first main phase.
    cast_from_hand(&mut engine, p0, vindicate());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        unreachable!("pass_until stops on nothing except a target choice")
    };
    assert!(
        options.contains(&doomed),
        "\"destroy target permanent\" erreicht ein eigenes Land: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![doomed],
            },
        )
        .expect("the Forest was one of the options the question listed");
    pass_until(&mut engine, stack_is_empty);

    // Two cards and not one: the destroyed Forest **and** Vindicate itself,
    // which goes to the same graveyard as a resolved sorcery (CR 608.2m).
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p0)).len(),
        graveyard_before + 2,
        "the destroyed Forest went from the battlefield to its owner's \
         graveyard, and the spell that destroyed it lies next to it"
    );
    assert!(
        in_graveyard(&engine, p0, forest()).is_some(),
        "and it is the Forest that the number above counts"
    );
    assert_eq!(
        pt(&engine, raider),
        (3, 1),
        "+1/+0 für das eine Land, das starb, bis zum Ende des Zuges"
    );
    assert_eq!(
        pt(&engine, elves),
        (1, 1),
        "\"this creature\" is the Raider and not every creature you control"
    );
}
