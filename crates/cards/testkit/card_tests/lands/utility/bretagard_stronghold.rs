//! `cards/lands/utility/bretagard_stronghold.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Bretagard Stronghold is a land that enters tapped, taps for `{G}`, and can
/// spend itself — `{G}{W}{W}`, its own tap and the card — to put a +1/+1
/// counter on each of up to two creatures its controller controls and give
/// them vigilance and lifelink until end of turn.
///
/// The land is *played* rather than seated, because `starting_battlefield`
/// places a permanent with `Cause::Setup`, which no entry modifier looks at:
/// the printed "This land enters tapped" is only readable off a real land drop,
/// and the same placement is what would leave the `{T}` payable a turn early.
/// The pump then runs in the next main phase, where the target menu is the
/// whole claim — both of this seat's Lotleth Trolls are on it, the identical
/// card across the table is not, and `(min, max)` says "up to" in the only
/// place a client reads it. The price is asserted *after* the target question,
/// because CR 601.2h pays last.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn bretagard_stronghold_enters_tapped_and_sacrifices_itself_for_two_counters() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                forest(),
                lotleth_troll(),
                lotleth_troll(),
            ],
        )
        .battlefield(1, &[lotleth_troll()])
        .hand(0, &[bretagard_stronghold()])
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "p0 reaches a main phase of its own"
    );

    // The card's own entry, and it has to come from a land drop for the printed
    // modifier to be the thing that taps it.
    let land = play_land(&mut engine, p0, bretagard_stronghold());
    assert!(
        entered_tapped(&engine, land),
        "\"This land enters tapped\" is an entry modifier and not a placement"
    );

    // A land that arrives tapped offers no {{T}} until its controller's untap
    // step (CR 502.3), and the tap symbol is half of what the ability charges.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, land),
        "the untap step stood the Stronghold back up"
    );

    let trolls = all_on_battlefield(&engine, p0, lotleth_troll());
    assert_eq!(trolls.len(), 2, "two creatures of this seat's to name");
    let (host, bystander) = (trolls[0], trolls[1]);
    let theirs = on_battlefield(&engine, p1, lotleth_troll()).expect("their Troll is out");
    let before = (pt(&engine, host), pt(&engine, bystander));

    // Mana before the claim: `legal.abilities` is filtered through
    // `can_afford`, which reads the pool rather than the untapped lands.
    tap_all_mana_but(&mut engine, p0, Some(bretagard_stronghold()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "two Plains and one Forest, and the Stronghold kept back for its own {{T}}"
    );
    assert!(
        !is_tapped(&engine, land),
        "the one route kept back is the one the ability charges"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(land, 0)),
        "the printed {{T}}: Add {{G}} is offered: {:?}",
        legal.abilities
    );
    assert!(
        legal.abilities.contains(&(land, 1)),
        "and the pump, whose {{G}}{{W}}{{W}} are floating: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, bretagard_stronghold(), 1);
    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "the pump asks for up to two creatures you control, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat is the one that aims it");
    assert_eq!(
        (min, max),
        (0, 2),
        "\"up to two targets\": none is a legal answer and two is the ceiling"
    );
    assert_eq!(
        options.len(),
        2,
        "and the menu is exactly the creatures this seat controls: {options:?}"
    );
    assert!(
        options.contains(&host) && options.contains(&bystander),
        "both of this seat's creatures may be named: {options:?}"
    );
    assert!(
        !options.contains(&theirs),
        "\"creatures *you* control\" declines the same card across the table: {options:?}"
    );

    // CR 601.2c names the targets and CR 601.2h pays afterwards, so while the
    // question stands the Stronghold is still on the battlefield and the
    // {{G}}{{W}}{{W}} are still in the pool.
    assert!(
        on_battlefield(&engine, p0, bretagard_stronghold()).is_some(),
        "the sacrifice is the last step of the activation, not the first"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "and so is the mana"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host, bystander],
            },
        )
        .expect("both creatures the question offered");

    assert!(
        on_battlefield(&engine, p0, bretagard_stronghold()).is_none(),
        "\"Sacrifice this land\" took the card that carried the ability"
    );
    assert!(
        in_graveyard(&engine, p0, bretagard_stronghold()).is_some(),
        "and a sacrificed permanent goes to its owner's graveyard"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "{{G}}{{W}}{{W}} came out of the pool"
    );
    assert!(
        !stack_is_empty(&engine),
        "putting counters on creatures is no mana ability, so the ability is waiting"
    );

    pass_until(&mut engine, stack_is_empty);

    for (id, was) in [(host, before.0), (bystander, before.1)] {
        assert_eq!(
            counters_on(&engine, id, CounterKind::P1P1),
            1,
            "one +1/+1 counter per creature named, and a counter is not a pump"
        );
        assert_eq!(
            pt(&engine, id),
            (was.0 + 1, was.1 + 1),
            "the body grew by exactly the counter it was given"
        );
        let granted = keywords(&engine, id);
        assert!(
            granted.contains(KeywordSet::VIGILANCE),
            "and the named creature gained vigilance: {granted:?}"
        );
        assert!(
            granted.contains(KeywordSet::LIFELINK),
            "and lifelink: {granted:?}"
        );
    }
    assert_eq!(
        counters_on(&engine, theirs, CounterKind::P1P1),
        0,
        "the creature the ability did not name was given no counter"
    );
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::VIGILANCE),
        "and none of the keywords: the effect targets, it does not sweep the board"
    );

    // "until end of turn" is half the sentence, so the same creatures are read
    // again past the end step: the counters are permanent, the keywords are not.
    reach_their_main_phase(&mut engine, p1);
    assert_eq!(
        counters_on(&engine, host, CounterKind::P1P1),
        1,
        "the +1/+1 counter is not a duration"
    );
    let after = keywords(&engine, host);
    assert!(
        !after.contains(KeywordSet::VIGILANCE) && !after.contains(KeywordSet::LIFELINK),
        "the vigilance and lifelink lasted the turn they were granted in and no \
         longer: {after:?}"
    );
    assert!(
        on_battlefield(&engine, p0, lotleth_troll()).is_some(),
        "and the creature is still standing, so the grant left rather than the creature"
    );
}
