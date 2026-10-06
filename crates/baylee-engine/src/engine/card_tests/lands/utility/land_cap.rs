//! `cards/lands/utility/land_cap.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Land Cap: "{T}: Add {W} or {U}. Put a depletion counter on this land."
/// One activation, two effects, and the card is only itself when both
/// happen — a land that made the mana and skipped the counter is a
/// Tundra, and one that placed the counter and made nothing is a blank.
///
/// The choice is the other half: "{W} **or** {U}" is a question asked as the
/// ability resolves, so the answer is given and the pool is then read for
/// that colour *and* for its total, because a land that quietly made both
/// would satisfy either colour alone.
///
/// The file is `Coverage::Partial` for the two clauses that read the counter
/// back — the untap lock and the upkeep removal — and this pins the first:
/// the land untaps on the next turn with the counter still on it, which the
/// printing forbids. That assertion is meant to be deleted the day a
/// `Modifier` can make an untap lock conditional on a counter.
#[test]
fn land_cap_pays_for_its_mana_with_a_depletion_counter_it_cannot_yet_read() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest()).hand(0, &[land_cap()]).start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let land = play_land(&mut engine, p0, land_cap());
    assert!(
        !entered_tapped(&engine, land),
        "Land Cap prints no enters-tapped clause"
    );

    activate(&mut engine, p0, land_cap(), 0);
    // Not `if let`: a question that is never asked and a question answered
    // wrongly read the same through one, and the whole point of "or" is that
    // it is asked.
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "\"Add {{W}} or {{U}}\" asks which, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that tapped it chooses");
    assert_eq!(
        options,
        vec![ManaColor::White, ManaColor::Blue],
        "the two the card prints, in the order it prints them"
    );
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::White))
        .expect("a colour the ability offered");
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::White),
        1,
        "the colour that was named"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "and one mana in total: \"or\" is a choice and not a pair"
    );
    assert_eq!(
        counters_on(&engine, land, counters::DEPLETION),
        1,
        "\"Put a depletion counter on this land\" is the price of the colour"
    );

    // The pin. "This land doesn't untap during your untap step if it has a
    // depletion counter on it" has no `Modifier` that reads a counter, so it
    // is not built — and this is where that shows.
    cross_into_the_next_own_main(&mut engine, p0);
    assert_eq!(
        counters_on(&engine, land, counters::DEPLETION),
        1,
        "nothing removed it: the upkeep clause is not built either"
    );
    assert!(
        !is_tapped(&engine, land),
        "and it untapped anyway, which the printing forbids — delete this \
         assertion when a Modifier can hang an untap lock on a counter"
    );
}
