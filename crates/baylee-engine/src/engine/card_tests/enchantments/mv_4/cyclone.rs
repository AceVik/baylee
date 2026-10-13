//! `cards/enchantments/mv_4/cyclone.rs`, played.
//!
//! Cyclone ({2}{G}{G}): "At the beginning of your upkeep, put a wind counter
//! on this enchantment, then sacrifice this enchantment unless you pay {G}
//! for each wind counter on it. If you pay, this enchantment deals damage
//! equal to the number of wind counters on it to each creature and each
//! player."
//!
//! Written from the printed sentence and the Comprehensive Rules alone:
//! "[Do something] unless [a player does something else]" means "[a player]
//! may do something else; if they don't, [do something]" (CR 118.12a), and
//! the damage is dealt by the enchantment to every creature and every player
//! (CR 120.3a, 120.3e). Cyclone sits on the starting battlefield; the board
//! is seeded and everything after is played through `Engine::apply`.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::engine::card_tests::partials_w1b_kit as kit;

// oracle_id = "fc9e82b2-f148-431b-9312-97b7101cffe4"
fn cyclone() -> CardIndex {
    card_index("fc9e82b2-f148-431b-9312-97b7101cffe4")
}

// A vanilla green 2/2: enough toughness to survive one point of damage and
// not two.
fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// What Cyclone asked when it asked for its price.
struct Asked {
    /// Whose turn it is.
    active: PlayerId,
    /// Who was asked.
    player: PlayerId,
    /// What the question put the price at.
    cost: baylee_core::mana::ManaCost,
}

/// Passes priority until somebody is asked whether to pay mana, and says who
/// was asked what, on whose turn.
///
/// The pass stops *on* the question, so a question that is asked at the wrong
/// upkeep is seen as such by the caller (`active`), and one that is never
/// asked runs the game on to wherever the caller's other condition ends it.
#[track_caller]
fn walk_to_the_price(engine: &mut Engine<RegistryLookup>) -> Asked {
    pass_until(engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayMana { .. },
                ..
            }
        )
    });
    let Pending::YesNo {
        player,
        prompt: YesNoPrompt::PayMana { cost },
        ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stopped on the question")
    };
    Asked {
        active: engine.state().turn.active,
        player,
        cost,
    }
}

/// Says yes to the question, makes the mana out of every land the player
/// has the way a player does (the window the "yes" opens), and closes the
/// window: whatever the pool then holds either pays the price or does not.
#[track_caller]
fn say_yes_and_pay_with_every_land(engine: &mut Engine<RegistryLookup>, seat: PlayerId) {
    engine
        .apply(seat, PlayerAction::YesNo(true))
        .expect("the question can be answered");
    tap_all_mana(engine, seat);
    engine
        .apply(seat, PlayerAction::PassPriority)
        .expect("closing the payment window");
    pass_until(engine, stack_is_empty);
}

fn price(text: &str) -> baylee_core::mana::ManaCost {
    baylee_core::mana::ManaCost::parse(text)
}

/// A duel with Cyclone and `lands` on p0's side of the table and a Grizzly
/// Bears on each side, so that damage "to each creature" has a witness that
/// is the controller's and one that is not.
fn cyclone_game(lands: &[CardIndex]) -> Engine<RegistryLookup> {
    let mut mine = vec![cyclone(), grizzly_bears()];
    mine.extend_from_slice(lands);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &mine)
        .battlefield(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    engine
}

/// The first sentence, played to its end: the first upkeep puts one wind
/// counter on the enchantment *before* the question is asked, the price is
/// {G} for that one counter, and paying it deals 1 damage — to each player,
/// the controller included, and to each creature, the opponent's and the
/// controller's own — and keeps the enchantment.
#[test]
fn cyclone_first_upkeep_puts_one_wind_counter_asks_g_and_pays_with_one_damage_to_everything() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = cyclone_game(&[forest(), forest(), forest()]);
    let cyclone_id = on_battlefield(&engine, p0, cyclone()).expect("Cyclone is out");
    assert_eq!(
        counters_on(&engine, cyclone_id, counters::WIND),
        0,
        "no upkeep has begun: no wind counter yet"
    );

    let asked = walk_to_the_price(&mut engine);
    assert_eq!(asked.active, p0, "\"your upkeep\": it is p0's turn");
    assert_eq!(
        asked.player, p0,
        "the controller of Cyclone is the one who is asked"
    );
    assert_eq!(
        asked.cost,
        price("{G}"),
        "one wind counter: {{G}} for each of them"
    );
    assert_eq!(
        counters_on(&engine, cyclone_id, counters::WIND),
        1,
        "\"put a wind counter on this enchantment, then ...\": the counter is on before the price is asked"
    );
    assert_eq!(
        kit::life(&engine, p0),
        20,
        "nothing is dealt before payment"
    );
    assert_eq!(
        kit::life(&engine, p1),
        20,
        "nothing is dealt before payment"
    );

    say_yes_and_pay_with_every_land(&mut engine, p0);

    assert_eq!(
        on_battlefield(&engine, p0, cyclone()),
        Some(cyclone_id),
        "the price was paid: Cyclone stays"
    );
    assert_eq!(counters_on(&engine, cyclone_id, counters::WIND), 1);
    assert_eq!(
        kit::life(&engine, p0),
        19,
        "each player: the controller too"
    );
    assert_eq!(kit::life(&engine, p1), 19, "each player: the opponent");
    let mine = on_battlefield(&engine, p0, grizzly_bears()).expect("my Bears survives 1 damage");
    let theirs = on_battlefield(&engine, p1, grizzly_bears()).expect("their Bears survives 1");
    assert_eq!(
        kit::damage_on(&engine, mine),
        1,
        "each creature: the controller's own"
    );
    assert_eq!(
        kit::damage_on(&engine, theirs),
        1,
        "each creature: the opponent's"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        2,
        "three Forests made {{G}}{{G}}{{G}} and the price took exactly one {{G}}"
    );
}

/// "for each wind counter on it", and the counters accumulate: the second
/// upkeep is the second counter, so the price is {G}{G} and the damage is 2 —
/// which is what a 2/2 cannot survive. The opponent's upkeep between the two
/// is the control for "at the beginning of *your* upkeep": it adds no
/// counter, asks nothing and costs nobody a point of life.
#[test]
fn cyclone_second_upkeep_is_two_counters_gg_and_two_damage_and_the_opponents_upkeep_is_nothing() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = cyclone_game(&[forest(), forest(), forest()]);
    let cyclone_id = on_battlefield(&engine, p0, cyclone()).expect("Cyclone is out");

    let first = walk_to_the_price(&mut engine);
    assert_eq!(first.cost, price("{G}"));
    say_yes_and_pay_with_every_land(&mut engine, p0);
    assert_eq!(kit::life(&engine, p0), 19);

    // The opponent's turn, upkeep included: nothing is asked of anybody.
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayMana { .. },
                ..
            }
        ) || (e.state().turn.active == p1 && matches!(e.state().turn.phase, Phase::FirstMain))
    });
    assert!(
        matches!(engine.state().turn.phase, Phase::FirstMain) && engine.state().turn.active == p1,
        "p1's upkeep went by without Cyclone asking anything, got {:?}",
        engine.pending()
    );
    assert_eq!(
        counters_on(&engine, cyclone_id, counters::WIND),
        1,
        "the opponent's upkeep puts no wind counter on it"
    );
    assert_eq!(kit::life(&engine, p0), 19, "and deals nothing to p0");
    assert_eq!(kit::life(&engine, p1), 19, "and nothing to p1");

    let second = walk_to_the_price(&mut engine);
    assert_eq!(second.active, p0, "back at p0's own upkeep");
    assert_eq!(second.player, p0);
    assert_eq!(
        second.cost,
        price("{G}{G}"),
        "two wind counters: {{G}} for each"
    );
    assert_eq!(counters_on(&engine, cyclone_id, counters::WIND), 2);

    say_yes_and_pay_with_every_land(&mut engine, p0);

    assert_eq!(
        on_battlefield(&engine, p0, cyclone()),
        Some(cyclone_id),
        "paid twice: still here"
    );
    assert_eq!(kit::life(&engine, p0), 17, "1 + 2 damage to the controller");
    assert_eq!(kit::life(&engine, p1), 17, "1 + 2 damage to the opponent");
    assert!(
        on_battlefield(&engine, p0, grizzly_bears()).is_none()
            && in_graveyard(&engine, p0, grizzly_bears()).is_some(),
        "2 damage to my own 2/2: it dies (the damage of the first upkeep wore off at cleanup)"
    );
    assert!(
        on_battlefield(&engine, p1, grizzly_bears()).is_none()
            && in_graveyard(&engine, p1, grizzly_bears()).is_some(),
        "2 damage to their 2/2: it dies"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1,
        "three Forests made three {{G}} and the price took two"
    );
}

/// "Sacrifice this enchantment unless you pay": declining is the other
/// branch, and it deals nothing — no life lost, no damage marked — where
/// paying (the sibling tests) deals damage. Nothing is asked again after the
/// sacrifice, because there is no Cyclone left to ask.
#[test]
fn cyclone_declined_is_sacrificed_and_deals_no_damage() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = cyclone_game(&[forest(), forest()]);
    let cyclone_id = on_battlefield(&engine, p0, cyclone()).expect("Cyclone is out");

    let asked = walk_to_the_price(&mut engine);
    assert_eq!(asked.player, p0);
    assert_eq!(asked.cost, price("{G}"));
    engine
        .apply(p0, PlayerAction::YesNo(false))
        .expect("declining is an answer");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, cyclone()).is_none(),
        "not paid: sacrificed"
    );
    assert!(
        in_graveyard(&engine, p0, cyclone()).is_some(),
        "sacrificed is the graveyard (CR 701.21a), not gone from the game: {cyclone_id:?}"
    );
    assert_eq!(kit::life(&engine, p0), 20, "no damage to the controller");
    assert_eq!(kit::life(&engine, p1), 20, "no damage to the opponent");
    let mine = on_battlefield(&engine, p0, grizzly_bears()).expect("my Bears is untouched");
    let theirs = on_battlefield(&engine, p1, grizzly_bears()).expect("their Bears is untouched");
    assert_eq!(kit::damage_on(&engine, mine), 0);
    assert_eq!(kit::damage_on(&engine, theirs), 0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "declining spends no mana"
    );

    // Gone for good: neither seat's later upkeeps are asked anything.
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayMana { .. },
                ..
            }
        ) || (e.state().turn.number >= 4 && matches!(e.state().turn.phase, Phase::FirstMain))
    });
    assert!(
        !matches!(engine.pending(), Pending::YesNo { .. }),
        "a sacrificed Cyclone asks nothing at later upkeeps, got {:?}",
        engine.pending()
    );
}

/// The second upkeep's price is {G}{G}: two *green* mana, not "two mana" and
/// not "one {G}". With two Forests it is paid (the control); with a Forest and
/// a Mountain there are two mana and only one is green; with one Forest there
/// is one mana. In both short cases the player said yes, made all the mana
/// they could and still did not pay, so the enchantment is sacrificed and the
/// second round of damage is not dealt (life stays where the first round left
/// it).
#[test]
fn cyclone_second_upkeep_needs_two_green_not_two_mana_and_not_one_green() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for (lands, can_pay, why) in [
        (vec![forest(), forest()], true, "two Forests: {G}{G}"),
        (
            vec![forest(), mountain()],
            false,
            "a Forest and a Mountain: two mana, one of them green",
        ),
        (vec![forest()], false, "one Forest: one green mana"),
    ] {
        let mut engine = cyclone_game(&lands);
        let first = walk_to_the_price(&mut engine);
        assert_eq!(first.cost, price("{G}"), "{why}");
        say_yes_and_pay_with_every_land(&mut engine, p0);
        assert_eq!(
            kit::life(&engine, p0),
            19,
            "{why}: the first round was paid with the green mana there was"
        );

        let second = walk_to_the_price(&mut engine);
        assert_eq!(second.cost, price("{G}{G}"), "{why}");
        assert_eq!(second.active, p0, "{why}");
        say_yes_and_pay_with_every_land(&mut engine, p0);

        if can_pay {
            assert!(
                on_battlefield(&engine, p0, cyclone()).is_some(),
                "{why}: paid, kept"
            );
            assert_eq!(kit::life(&engine, p0), 17, "{why}: 2 more damage");
            assert_eq!(kit::life(&engine, p1), 17, "{why}: 2 more damage");
        } else {
            assert!(
                on_battlefield(&engine, p0, cyclone()).is_none(),
                "{why}: the price was not paid, so it is sacrificed"
            );
            assert!(
                in_graveyard(&engine, p0, cyclone()).is_some(),
                "{why}: into the graveyard"
            );
            assert_eq!(
                kit::life(&engine, p0),
                19,
                "{why}: nothing is dealt when the price is not paid"
            );
            assert_eq!(
                kit::life(&engine, p1),
                19,
                "{why}: nothing is dealt when the price is not paid"
            );
        }
    }
}
