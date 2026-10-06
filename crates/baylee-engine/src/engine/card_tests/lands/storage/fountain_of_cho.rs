//! `cards/lands/storage/fountain_of_cho.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Fountain of Cho: `{T}: Put a storage counter on this land.` and
/// `{T}, Remove any number of storage counters from this land: Add {W} for
/// each storage counter removed this way.`
///
/// The number in that second line is **announced as the ability is
/// activated** (CR 601.2b, reached from CR 602.2b), which is a stage the
/// engine did not have: every cost part until now was a fixed quantity a
/// card printed, so `start_activation` walked from the zone check straight
/// to targets. `CostPart::RemoveCounterSelfX` is the first part that asks
/// the player a question *before* the payment, and X is then the same X the
/// effect reads — the cost and the mana are two halves of one number.
///
/// What this test is built around is the **bound**, because the bound is the
/// whole of the legality. Zero is a legal announcement, so `can_afford` has
/// nothing to refuse and the ability is offered whatever the land carries;
/// the only wrong answer is one larger than the counters actually there.
/// So the arc is four activations of the same land across four of its
/// controller's turns, and each of them asserts the bound the engine offers:
///
/// - stored twice, `max` is 2, and 3 is refused;
/// - spend 1, and the *next* press offers 1 — which is the assertion the
///   test exists for. It fails two different ways: an `activation_x` left
///   over from the first press would skip the question entirely, and a
///   payment that never removed the counters would still offer 2;
/// - spend the last one, and the press after that offers 0, the land still
///   answering for an ability whose only legal announcement is nothing at
///   all. A zero announcement adds no mana, which is the other half of
///   "zero is legal" and the half a `1.max(x)` anywhere would break.
#[test]
#[allow(clippy::too_many_lines)] // four activations of one land, and each is an assertion
fn a_storage_land_asks_how_many_counters_and_the_bound_shrinks_with_them() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(881, forest())
        .hand(0, &[fountain_of_cho()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, fountain_of_cho());
    assert!(
        entered_tapped(&engine, land),
        "Fountain of Cho prints `This land enters tapped`"
    );
    assert_eq!(
        counters_on(&engine, land, counters::STORAGE),
        0,
        "and it arrives empty: the counters are banked one turn at a time"
    );

    // Two turns of `{T}: Put a storage counter on this land.`
    for banked in 1..=2u16 {
        cross_into_the_next_own_main(&mut engine, p0);
        store_a_counter(&mut engine, p0, land, 0);
        assert_eq!(
            counters_on(&engine, land, counters::STORAGE),
            banked,
            "one counter per activation, and the land is spent for the turn"
        );
        assert!(is_tapped(&engine, land), "which is what `{{T}}` means");
    }

    // Two counters, so two is the most that may be announced.
    cross_into_the_next_own_main(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: land,
                ability_index: 1,
            },
        )
        .expect("the storage line is activatable");
    let Pending::ChooseNumber { min, max, .. } = engine.pending().clone() else {
        panic!("expected a number, got {:?}", engine.pending())
    };
    assert_eq!(
        (min, max),
        (0, 2),
        "`any number` is bounded below by nothing and above by the counters \
         that are actually on the land"
    );
    assert!(
        engine.apply(p0, PlayerAction::ChooseNumber(3)).is_err(),
        "three counters is a cost nothing on this land could pay"
    );
    engine
        .apply(p0, PlayerAction::ChooseNumber(1))
        .expect("one of the two is an answer inside the range");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::White),
        1,
        "one counter removed this way is one {{W}}"
    );
    assert_eq!(
        counters_on(&engine, land, counters::STORAGE),
        1,
        "and exactly the announced number came off — the other is still banked"
    );

    // The assertion the test is for: the bound moved with the counters.
    cross_into_the_next_own_main(&mut engine, p0);
    assert_eq!(
        spend_storage(&mut engine, p0, land, 1, 1),
        (0, 1),
        "one counter left, so one is the most the next activation may announce"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::White),
        1,
        "and the mana follows the number that was just announced, not the \
         one announced last turn"
    );
    assert_eq!(counters_on(&engine, land, counters::STORAGE), 0);

    // Empty, and still a legal activation — for nothing.
    cross_into_the_next_own_main(&mut engine, p0);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(land, 1)),
        "`any number` includes none, so there is no counter count at which \
         the ability stops being affordable: {:?}",
        legal.abilities
    );
    assert_eq!(
        spend_storage(&mut engine, p0, land, 1, 0),
        (0, 0),
        "with nothing banked, nothing is the only thing that may be announced"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::White),
        0,
        "and none of a counter is none of a mana"
    );
    assert!(
        is_tapped(&engine, land),
        "the tap was still paid, whatever the announced number was"
    );
    assert!(
        in_graveyard(&engine, p0, fountain_of_cho()).is_none(),
        "and the land is still a land: unlike the depletion cycle, nothing \
         here sacrifices anything when the counters run out"
    );
}
