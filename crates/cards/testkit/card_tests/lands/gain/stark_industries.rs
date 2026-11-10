//! `cards/lands/gain/stark_industries.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Stark Industries prints three lines — it enters tapped, its arrival gains
/// its controller 1 life, and it taps for {U} or {R} — and one land drop
/// plays all three without any of them standing in for another. The entry
/// replacement only runs on a real `PlayLand` (`starting_battlefield` seeds a
/// placement, which no replacement effect sees), so the tap is read off a
/// land that actually arrived; the mana line then has to wait a turn,
/// because a land that came in tapped has an unpayable `{T}` and is not even
/// in the offer. Reading that offer twice — empty now, live after the untap
/// step — is what says the ability is written rather than merely absent.
#[test]
fn stark_industries_enters_tapped_gains_a_life_and_taps_for_blue_or_red() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[stark_industries()])
        .life(0, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let life_before = engine.state().players[0].life;
    let land = play_land(&mut engine, p0, stark_industries());
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(
        entered_tapped(&engine, land),
        "the printed entry replacement, read off a land drop that really ran it"
    );
    assert_eq!(
        engine.state().players[0].life,
        life_before + 1,
        "\"when this land enters, you gain 1 life\", and the trigger had to \
         resolve off the stack before the life was there"
    );

    // The `{T}` is unpaid while the land is tapped, so its one mana ability
    // is not offered at all. Nothing else on this board makes blue or red,
    // and the same predicate is true again the moment CR 502.3 stands the
    // land up — which is what keeps the negative from being vacuous.
    let mana_offered = |engine: &Engine<RegistryLookup>, land: ObjectId| -> bool {
        matches!(
            engine.pending(),
            Pending::Priority { legal, .. }
                if legal.abilities.iter().any(|(source, _)| *source == land)
        )
    };
    assert!(
        !mana_offered(&engine, land),
        "a land that arrived tapped gives nothing this turn: {:?}",
        engine.pending()
    );

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, land), "the untap step stood it up");
    assert!(
        mana_offered(&engine, land),
        "and now the printed ability is offered: {:?}",
        engine.pending()
    );

    // Ability 0 is the enters trigger, which is never activated; 1 is the
    // mana ability.
    activate(&mut engine, p0, stark_industries(), 1);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "one of two colours is a question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat is the one that names it");
    assert_eq!(options.len(), 2, "two colours and no third: {options:?}");
    assert!(
        options.contains(&ManaColor::Blue) && options.contains(&ManaColor::Red),
        "{{U}} or {{R}}, exactly: {options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Red))
        .expect("red was one of the two it offered");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Red),
        1,
        "the colour that was named, and not a default"
    );
    assert_eq!(
        pool.available(ManaColor::Blue),
        0,
        "one tap is one mana, and not one of each"
    );
    assert_eq!(pool.total(), 1, "and nothing else is floating");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting"
    );
    assert!(is_tapped(&engine, land), "the land paid its own {{T}}");
}
