//! `cards/enchantments/mv_3/tidal_influence.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::turn::Step;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn influence_card() -> CardIndex {
    card_index("b1393388-b8b3-4c54-a086-f5e6d9908972")
}
fn homarid_warrior_card() -> CardIndex {
    // {4}{U}, 3/3, a blue creature.
    card_index("6facb0f6-fa49-4ce6-9f95-15bb3ce4bcf2")
}
fn water_elemental_card() -> CardIndex {
    // {3}{U}{U}, 5/4, a blue creature.
    card_index("c470fbe1-0538-46ec-8741-3143f5e78af8")
}
fn bears_card() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn tide(e: &Engine<RegistryLookup>, id: ObjectId) -> u16 {
    e.state()
        .object(id)
        .expect("Tidal Influence is out")
        .counters
        .get(baylee_cards_dsl::counters::TIDE)
}

fn fill_pool(e: &mut Engine<RegistryLookup>) {
    for p in &mut e.state.players {
        for c in ManaColor::ALL {
            p.mana_pool.add(c, 30);
        }
    }
    e.refresh_offer();
}

/// Walks to p0's next upkeep and lets its triggers resolve.
fn next_upkeep(e: &mut Engine<RegistryLookup>) {
    let began = e.state().turn.number;
    pass_until(e, |e| {
        e.state().turn.number > began
            && e.state().turn.active == P0
            && e.state().turn.step == Step::Upkeep
    });
    pass_until(e, stack_is_empty);
}

struct Board {
    e: Engine<RegistryLookup>,
    influence: ObjectId,
    my_blue: ObjectId,
    their_blue: ObjectId,
    green: ObjectId,
}

/// p0: Tidal Influence (x2) in hand, three Islands, a Homarid Warrior and a
/// Grizzly Bears; p1: a Water Elemental. The first Tidal Influence is cast
/// and resolved.
fn influence_cast() -> Board {
    let mut e = Duel::new(SEED, swamp())
        .hand(0, &[influence_card(), influence_card()])
        .battlefield(
            0,
            &[
                island(),
                island(),
                island(),
                homarid_warrior_card(),
                bears_card(),
            ],
        )
        .battlefield(1, &[water_elemental_card()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    let my_blue = on_battlefield(&e, P0, homarid_warrior_card()).expect("my blue creature");
    let their_blue = on_battlefield(&e, P1, water_elemental_card()).expect("their blue creature");
    let green = on_battlefield(&e, P0, bears_card()).expect("the Bears");
    assert_eq!(pt(&e, my_blue), (3, 3));
    assert_eq!(pt(&e, their_blue), (5, 4));

    cast_from_hand(&mut e, P0, influence_card());
    pass_until(&mut e, stack_is_empty);
    let influence = on_battlefield(&e, P0, influence_card()).expect("Tidal Influence resolved");
    Board {
        e,
        influence,
        my_blue,
        their_blue,
        green,
    }
}

/// Tidal Influence (b1393388-b8b3-4c54-a086-f5e6d9908972) — "This enchantment
/// enters with a tide counter on it." "As long as there is exactly one tide
/// counter on this enchantment, all blue creatures get -2/-0." Everyone's blue
/// creatures, not the green ones.
#[test]
fn tidal_influence_enters_with_one_tide_counter_and_shrinks_all_blue_creatures() {
    let b = influence_cast();
    assert_eq!(tide(&b.e, b.influence), 1);
    assert_eq!(pt(&b.e, b.my_blue), (1, 3), "-2/-0 on my blue creature");
    assert_eq!(pt(&b.e, b.their_blue), (3, 4), "and on theirs");
    assert_eq!(pt(&b.e, b.green), (2, 2), "the green one is untouched");
}

/// Tidal Influence — "At the beginning of your upkeep, put a tide counter on
/// this enchantment", with the exact counts: one is -2/-0, two is nothing,
/// three is +2/+0, and "whenever there are four or more tide counters" the
/// counters are all removed, which starts the cycle again.
#[test]
fn tidal_influence_cycles_through_its_tide_counters() {
    let Board {
        mut e,
        influence,
        my_blue,
        their_blue,
        green,
    } = influence_cast();

    next_upkeep(&mut e);
    assert_eq!(tide(&e, influence), 2, "an upkeep adds one");
    assert_eq!(pt(&e, my_blue), (3, 3), "two counters: no modification");
    assert_eq!(pt(&e, their_blue), (5, 4));

    next_upkeep(&mut e);
    assert_eq!(tide(&e, influence), 3);
    assert_eq!(pt(&e, my_blue), (5, 3), "exactly three: +2/+0");
    assert_eq!(pt(&e, their_blue), (7, 4), "on theirs as well");
    assert_eq!(pt(&e, green), (2, 2), "never on a non-blue creature");

    next_upkeep(&mut e);
    assert_eq!(
        tide(&e, influence),
        0,
        "the fourth counter trips the trigger that removes them all"
    );
    assert_eq!(
        pt(&e, my_blue),
        (3, 3),
        "and no counters means no modification"
    );

    next_upkeep(&mut e);
    assert_eq!(tide(&e, influence), 1, "a new cycle begins");
    assert_eq!(pt(&e, my_blue), (1, 3));
}

/// Tidal Influence — "Cast this spell only if no permanents named Tidal
/// Influence are on the battlefield." The second copy cannot be cast with one
/// out, even with all the mana in the world, and can once the first is gone.
#[test]
fn tidal_influence_cannot_be_cast_while_another_is_on_the_battlefield() {
    let Board {
        mut e, influence, ..
    } = influence_cast();
    fill_pool(&mut e);
    let second = in_hand(&e, P0, influence_card()).expect("the second copy in hand");
    assert!(
        e.apply(P0, PlayerAction::CastSpell { card: second })
            .is_err(),
        "one is already out"
    );

    bury(&mut e, &[influence]);
    e.refresh_offer();
    assert!(on_battlefield(&e, P0, influence_card()).is_none());
    fill_pool(&mut e);
    e.apply(P0, PlayerAction::CastSpell { card: second })
        .expect("with none out, the second copy can be cast");
    pass_until(&mut e, stack_is_empty);
    let again = on_battlefield(&e, P0, influence_card()).expect("it resolved");
    assert_eq!(
        tide(&e, again),
        1,
        "and it enters with its own tide counter"
    );
}

/// Tidal Influence — control for the restriction: a first copy is castable on
/// an empty battlefield, so the refusal above is the restriction's.
#[test]
fn tidal_influence_is_castable_when_none_is_out() {
    let mut e = Duel::new(SEED, swamp())
        .hand(0, &[influence_card()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    fill_pool(&mut e);
    let card = in_hand(&e, P0, influence_card()).expect("in hand");
    e.apply(P0, PlayerAction::CastSpell { card })
        .expect("the same mana, no other copy: castable");
    pass_until(&mut e, stack_is_empty);
    assert!(on_battlefield(&e, P0, influence_card()).is_some());
}
