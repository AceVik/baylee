//! `cards/creatures/mv_4/ojer_pakpatiq_deepest_epoch.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Ojer Pakpatiq's whole point is that killing it does not remove it: "When
/// Ojer Pakpatiq dies, return it to the battlefield tapped and transformed
/// under its owner's control with three time counters on it." So the
/// scenario kills a 4/3 with Hero's Downfall and then reads the battlefield
/// rather than the graveyard, because a god that stayed dead and a god that
/// came back as the wrong face are two different failures.
///
/// `Effect::ExileSelfReturnAsFace { face: 1 }` is the built half, and what
/// comes back is a **land** — Temple of Cyclical Time — off a card whose
/// front is a legendary creature, which is also the shape this pool got
/// wrong twenty-one times this morning: the back face is reachable by
/// transforming and by nothing else.
///
/// The file is `Coverage::Partial` for rebound and for removing a time
/// counter, and the pins are here: what returns is untapped and carries no
/// time counters, both of which the printing spells out. They are meant to
/// be deleted the day an effect can put counters on the object it returns.
#[test]
fn ojer_pakpatiq_dies_and_comes_back_as_the_land_on_its_other_face() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp()])
        .hand(0, &[heroes_downfall()])
        .battlefield(1, &[ojer_pakpatiq_deepest_epoch()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let god = on_battlefield(&engine, p1, ojer_pakpatiq_deepest_epoch()).expect("the god is out");
    assert_eq!(pt(&engine, god), (4, 3), "a 4/3 before anything happens");
    assert!(
        keywords(&engine, god).contains(KeywordSet::FLYING),
        "and it flies"
    );

    cast_from_hand(&mut engine, p0, heroes_downfall());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("the Downfall asks for a target, got {:?}", engine.pending())
    };
    assert!(options.contains(&god), "a creature is a legal target");
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![god],
                players: vec![],
            },
        )
        .expect("a target the spell offered");
    pass_until(&mut engine, |e| at_rest(e, p0));

    let temple =
        on_battlefield(&engine, p1, ojer_pakpatiq_deepest_epoch()).expect("it came back at once");
    let t = types(&engine, temple);
    assert!(
        t.contains(TypeSet::LAND),
        "\"transformed\" — the back face is Temple of Cyclical Time"
    );
    assert!(
        !t.contains(TypeSet::CREATURE),
        "and the creature stayed on the front"
    );
    assert!(
        in_graveyard(&engine, p1, ojer_pakpatiq_deepest_epoch()).is_none(),
        "the god is on the battlefield and not in a graveyard"
    );

    // The two pins. Both are printed and neither is built.
    assert!(
        !is_tapped(&engine, temple),
        "\"return it to the battlefield **tapped**\" — delete this when an \
         effect returning a face can tap what it returns"
    );
    // "with three time counters on it" is not pinned by a count, because
    // there is no `time` counter kind in `baylee_cards_dsl::counters` to
    // count — the clause cannot be spelled at all, which is a shorter
    // sentence than a wrong number. The land's own "{T}: Add {U}. Remove a
    // time counter" waits on the same thing.
    assert_eq!(
        counters_on(&engine, temple, baylee_cards_dsl::counters::QUEST),
        0,
        "and it carries no counters of any kind this DSL can name"
    );
}
