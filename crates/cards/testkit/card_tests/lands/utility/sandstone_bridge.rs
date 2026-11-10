//! `cards/lands/utility/sandstone_bridge.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Sandstone Bridge prints three sentences and two of them need a board to be
/// read: it enters tapped, its enter trigger gives one creature +1/+1 and
/// vigilance until end of turn, and it taps for {W}. Both Elves stand on the
/// table so "target creature" is shown to reach across it and yet to land on
/// only one, and the tapped arrival is what keeps the printed mana line off the
/// offer for a whole turn — a `{T}` a tapped land cannot pay is not something
/// the list may carry.
#[test]
#[allow(clippy::too_many_lines)]
fn sandstone_bridge_enters_tapped_and_pumps_one_creature_with_vigilance() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[sandstone_bridge()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    assert_eq!(pt(&engine, mine), (1, 1), "a printed 1/1 before the pump");
    assert!(
        !keywords(&engine, mine).contains(KeywordSet::VIGILANCE),
        "nothing has granted anything yet"
    );

    // The land is *played*, so its own enter modifier is a real entry and not
    // a placement — a `starting_battlefield` seed would arrive untapped.
    let land = play_land(&mut engine, p0, sandstone_bridge());
    assert!(entered_tapped(&engine, land), "\"This land enters tapped\"");

    // The enter trigger asks which creature is pumped, and the land it came
    // from is not a creature it may name.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stopped on nothing but the target question")
    };
    assert_eq!(player, p0, "the seat that played the land names the target");
    assert_eq!(
        (min, max),
        (1, 1),
        "one creature, and the trigger asks once"
    );
    assert!(
        options.contains(&mine) && options.contains(&theirs),
        "\"target creature\" is any creature, on either side of the table: {options:?}"
    );
    assert!(
        !options.contains(&land),
        "the land is no creature, and it is not a legal target for its own \
         trigger: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mine],
            },
        )
        .expect("the creature the question offered was chosen");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, mine),
        (2, 2),
        "+1/+1 on the creature the trigger named"
    );
    assert!(
        keywords(&engine, mine).contains(KeywordSet::VIGILANCE),
        "and vigilance, from the same grant"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "the pump reaches the creature it targeted and never across the table"
    );
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::VIGILANCE),
        "nor does the keyword"
    );

    // The land came in tapped, so its `{T}: Add {W}` is not even offered this
    // turn: `legal.abilities` is filtered through `can_afford`, which cannot
    // pay a tap symbol that is already spent.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("the seat holds a quiet main phase: {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.iter().any(|(src, _)| *src == land),
        "a land that entered tapped has no {{T}} to spend: {:?}",
        legal.abilities
    );

    // Across the opponent's turn and back: the untap step stands it up, and
    // only then is the printed mana line a route the seat may take.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, land),
        "the untap step stood the Bridge back up"
    );
    assert!(
        !keywords(&engine, mine).contains(KeywordSet::VIGILANCE),
        "\"until end of turn\" expired with the turn that made the grant"
    );
    assert_eq!(pt(&engine, mine), (1, 1), "and the +1/+1 went with it");

    // Ability 1 is the printed "{T}: Add {W}" — a mana ability a card prints,
    // so it is an ordinary entry in `legal.abilities` with an index to name.
    activate(&mut engine, p0, sandstone_bridge(), 1);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::White), 1, "{{T}}: Add {{W}}");
    assert_eq!(pool.total(), 1, "one mana, off one tap");
    assert!(is_tapped(&engine, land), "the Bridge paid its own {{T}}");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
}
