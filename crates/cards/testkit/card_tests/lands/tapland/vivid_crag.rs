//! `cards/lands/tapland/vivid_crag.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// The other half of the Vivid land, which is the counter being **spent**.
///
/// One land, three turns, and the arc is the whole point: two counters means
/// exactly two activations of the any-colour line, and the third turn is
/// where the ability stops being offered while the land's own `{T}: Add {R}`
/// stays. That last assertion is what separates "the counter ran out" from
/// "the land is tapped", which every earlier turn would have confused.
///
/// Nothing is asked of the player about the cost, and that is deliberate:
/// the counters come off the source and the source is not a choice, so
/// [`CostPart::RemoveCounterSelf`] goes straight to the colour question
/// (CR 605.3b) rather than through a chooser with one legal answer.
///
/// [`CostPart::RemoveCounterSelf`]: baylee_cards_dsl::CostPart::RemoveCounterSelf
#[test]
fn a_vivid_land_pays_a_counter_for_a_colour_it_could_not_otherwise_make() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(542, forest()).hand(0, &[vivid_crag()]).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, vivid_crag());
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.contains(&(land, 1)),
        "the turn it arrives it is tapped, so neither line is payable: {:?}",
        legal.abilities
    );

    for turn in 0..2u16 {
        cross_into_the_next_own_main(&mut engine, p0);
        let Pending::Priority { legal, .. } = engine.pending().clone() else {
            panic!("expected priority, got {:?}", engine.pending())
        };
        assert!(
            legal.abilities.contains(&(land, 1)),
            "turn {turn}: {} counter(s) left, so the any-colour line is on \
             offer: {:?}",
            counters_on(&engine, land, CounterKind::Charge),
            legal.abilities
        );

        engine
            .apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: land,
                    ability_index: 1,
                },
            )
            .expect("a counter is there to pay with");

        // Straight to the colour: the cost asked nobody anything.
        let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
            panic!(
                "turn {turn}: any colour is a choice and the cost is not: {:?}",
                engine.pending()
            )
        };
        assert_eq!(options.len(), 5, "turn {turn}: all five colours");
        engine
            .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
            .expect("a colour the engine offered");

        assert_eq!(
            counters_on(&engine, land, CounterKind::Charge),
            1 - turn,
            "turn {turn}: exactly one counter came off"
        );
        assert_eq!(
            engine.state().players[0]
                .mana_pool
                .available(ManaColor::Blue),
            1,
            "turn {turn}: and a blue mana a Vivid Crag's own line cannot make"
        );
        assert!(
            is_tapped(&engine, land),
            "turn {turn}: the {{T}} was paid too"
        );
    }

    cross_into_the_next_own_main(&mut engine, p0);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert_eq!(
        counters_on(&engine, land, CounterKind::Charge),
        0,
        "both counters are spent"
    );
    assert!(
        !legal.abilities.contains(&(land, 1)),
        "an untapped land with no counters cannot pay the any-colour line: \
         {:?}",
        legal.abilities
    );
    assert!(
        legal.abilities.contains(&(land, 0)),
        "and its own {{T}}: Add {{R}} is untouched, which is what says the \
         refusal above is about the counter and not about the tap: {:?}",
        legal.abilities
    );
}

/// CR 614.16 has a direction, and this is it.
///
/// A counter-doubling replacement applies to counters being **put** on a
/// permanent — the two a Vivid land enters with are exactly that (CR 614.1c),
/// so under a Doubling Season it enters with four. Nothing in Magic
/// multiplies a counter being *removed*, which is why
/// `replacement::remove_counters` takes no multiplier at all: the cost is one
/// counter under any number of Doubling Seasons.
///
/// Both halves in one game, because a removal that doubled would be invisible
/// against a land that entered with the wrong number anyway.
#[test]
fn a_doubler_doubles_the_counters_a_land_arrives_with_and_never_the_cost() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(545, forest())
        .battlefield(0, &[doubling_season()])
        .hand(0, &[vivid_crag()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, vivid_crag());
    assert_eq!(
        counters_on(&engine, land, CounterKind::Charge),
        4,
        "two printed counters, put on as the land enters, doubled once"
    );

    cross_into_the_next_own_main(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: land,
                ability_index: 1,
            },
        )
        .expect("four counters is enough for one");
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .expect("a colour the engine offered");

    assert_eq!(
        counters_on(&engine, land, CounterKind::Charge),
        3,
        "a cost of one counter is a cost of one counter — a doubler has \
         nothing to say about a removal"
    );
}
