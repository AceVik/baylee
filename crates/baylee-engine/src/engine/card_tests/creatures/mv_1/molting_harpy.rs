//! `cards/creatures/mv_1/molting_harpy.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Molting Harpy — {B} 2/1 Harpy Mercenary with flying: "At the beginning of
/// your upkeep, sacrifice this creature unless you pay {2}." Both halves of
/// that sentence need playing, because each is the other's control. The
/// trigger has to arrive as a *question* that names the printed {2} — an
/// unconditional sacrifice would run the same board and never ask — and the
/// two answers have to diverge: {2} off the two Islands kept back for it
/// leaves the 2/1 flier standing to be asked again, and declining the next
/// upkeep puts it in the graveyard. The mana is made while the question
/// stands (CR 605.3a), which is exactly why the Islands were not spent on
/// the Harpy itself.
#[test]
fn molting_harpy_asks_for_two_at_each_upkeep_and_sacrifices_itself_if_it_is_not_paid() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[swamp(), swamp(), island(), island()])
        .hand(0, &[molting_harpy()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // The Swamps pay the {B} out of the pool, and the two Islands are held
    // back on purpose: the upkeep's {2} is made in the window the question
    // opens, and a board tapped dry here could not pay it.
    tap_all_mana_but(&mut engine, p0, Some(island()));
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        2,
        "two Swamps tapped for the black the Harpy costs"
    );
    cast_with_floating(&mut engine, p0, molting_harpy());
    pass_until(&mut engine, stack_is_empty);

    let harpy = on_battlefield(&engine, p0, molting_harpy()).expect("the Harpy resolved");
    assert_eq!(pt(&engine, harpy), (2, 1), "the body the card prints");
    assert!(
        keywords(&engine, harpy).contains(KeywordSet::FLYING),
        "and the flying it prints, on the permanent the spell became"
    );

    // The first upkeep it sees: the sentence arrives as a question, with the
    // price on it. Nothing else in this game asks p0 a yes/no.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    let Pending::YesNo { player, prompt, .. } = engine.pending().clone() else {
        unreachable!("the predicate just matched")
    };
    assert_eq!(player, p0, "\"at the beginning of *your* upkeep\"");
    assert_eq!(
        prompt,
        YesNoPrompt::PayTax { mana: 2 },
        "the question names the {{2}} the card prints"
    );

    // Pay it: the two Islands are the whole of the price, made while the
    // question stands (CR 605.3a).
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    tap_all_mana(&mut engine, p0);

    // The next upkeep asks the same question, which is all the {2} bought.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    assert!(
        on_battlefield(&engine, p0, molting_harpy()).is_some(),
        "the upkeep it was paid for left it standing, so the same question can be asked again"
    );

    // And declining it is the other half of the printed sentence.
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    assert!(
        matches!(drive_to_rest(&mut engine, p0), Rest::Reached),
        "the sacrifice resolves and the seat is back at a quiet priority"
    );
    assert!(
        on_battlefield(&engine, p0, molting_harpy()).is_none(),
        "\"sacrifice this creature unless you pay\" — declined, it leaves"
    );
    assert!(
        in_graveyard(&engine, p0, molting_harpy()).is_some(),
        "and a sacrificed permanent goes to its owner's graveyard"
    );
}
