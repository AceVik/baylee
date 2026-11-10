//! `cards/creatures/mv_4/nameless_race.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn nameless_race() -> CardIndex {
    card_index("35a717ea-bd32-4714-b402-de7f15398e52")
}

fn savannah_lions() -> CardIndex {
    card_index("60ba93eb-39e6-4af2-9c66-cd38f72daff2")
}

fn serra_angel() -> CardIndex {
    card_index("4b7ac066-e5c7-43e6-9e7e-2739b24a905d")
}

fn pearled_unicorn() -> CardIndex {
    card_index("c071be90-0531-40cc-af46-0cbe80c4ddd4")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn float(engine: &mut Engine<RegistryLookup>, seat: usize, mana: &[(ManaColor, u32)]) {
    let state = engine
        .dev_state_mut(PlayerId::new(0))
        .expect("the harness may set boards up");
    for (color, n) in mana {
        state.players[seat].mana_pool.add(*color, *n);
    }
    engine.refresh_offer();
}

/// Casts the Race with p0's life-payment question open; returns the bounds.
fn cast_until_asked(engine: &mut Engine<RegistryLookup>) -> (u32, u32) {
    let p0 = PlayerId::new(0);
    float(
        engine,
        0,
        &[(ManaColor::Colorless, 3), (ManaColor::Black, 1)],
    );
    cast_with_floating(engine, p0, nameless_race());
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseNumber { .. })
    });
    let Pending::ChooseNumber { min, max, .. } = engine.pending().clone() else {
        unreachable!("the pass waited for exactly this")
    };
    (min, max)
}

fn board(mine: &[CardIndex], theirs: &[CardIndex]) -> Engine<RegistryLookup> {
    let mut engine = Duel::new(SEED, swamp())
        .hand(0, &[nameless_race()])
        .battlefield(0, mine)
        .battlefield(1, theirs)
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, PlayerId::new(0)),
        "p0 reaches its own main"
    );
    engine
}

/// Nameless Race (35a717ea-bd32-4714-b402-de7f15398e52) — "Trample / As this
/// creature enters, pay any amount of life. The amount you pay can't be more
/// than the total number of white nontoken permanents your opponents control
/// plus the total number of white cards in their graveyards. / Nameless
/// Race's power and toughness are each equal to the life paid as it entered."
#[test]
fn nameless_race_pays_life_up_to_the_opponents_white_permanents_and_takes_it_as_size() {
    let p0 = PlayerId::new(0);
    let mut engine = board(&[], &[savannah_lions(), serra_angel(), grizzly_bears()]);
    let (min, max) = cast_until_asked(&mut engine);
    assert_eq!(
        (min, max),
        (0, 2),
        "two white permanents, the Bears are green"
    );
    engine
        .apply(p0, PlayerAction::ChooseNumber(2))
        .expect("2 is within bounds");
    pass_until(&mut engine, stack_is_empty);

    let race = on_battlefield(&engine, p0, nameless_race()).expect("the Race is out");
    assert_eq!(pt(&engine, race), (2, 2), "as big as the life paid");
    assert_eq!(engine.state().players[0].life, 18, "2 life paid");
    assert!(keywords(&engine, race).contains(KeywordSet::TRAMPLE));
}

/// Paying more than the cap is refused and the question stands.
#[test]
fn nameless_race_cannot_pay_more_than_the_cap() {
    let p0 = PlayerId::new(0);
    let mut engine = board(&[], &[savannah_lions()]);
    let (_, max) = cast_until_asked(&mut engine);
    assert_eq!(max, 1);
    assert!(
        engine.apply(p0, PlayerAction::ChooseNumber(2)).is_err(),
        "2 is above the cap of 1"
    );
    assert!(matches!(engine.pending(), Pending::ChooseNumber { .. }));
    engine
        .apply(p0, PlayerAction::ChooseNumber(1))
        .expect("1 is within bounds");
    pass_until(&mut engine, stack_is_empty);
    let race = on_battlefield(&engine, p0, nameless_race()).expect("the Race is out");
    assert_eq!(pt(&engine, race), (1, 1));
}

/// Paying nothing leaves a 0/0, which dies.
#[test]
fn nameless_race_paying_nothing_dies_as_a_zero_zero() {
    let p0 = PlayerId::new(0);
    let mut engine = board(&[], &[savannah_lions()]);
    cast_until_asked(&mut engine);
    engine
        .apply(p0, PlayerAction::ChooseNumber(0))
        .expect("0 is within bounds");
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, nameless_race()).is_none());
    assert!(in_graveyard(&engine, p0, nameless_race()).is_some());
    assert_eq!(engine.state().players[0].life, 20, "no life paid");
}

/// Only the opponents' white cards count: white permanents of its own
/// controller and white cards in its own graveyard do not, and an opponent's
/// white card in a graveyard does.
#[test]
fn nameless_race_counts_opposing_white_graveyard_cards_and_not_its_own() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = board(
        &[savannah_lions(), pearled_unicorn()],
        &[savannah_lions(), serra_angel(), grizzly_bears()],
    );
    let my_unicorn = on_battlefield(&engine, p0, pearled_unicorn()).expect("my Unicorn is out");
    let their_angel = on_battlefield(&engine, p1, serra_angel()).expect("their Angel is out");
    bury(&mut engine, &[my_unicorn, their_angel]);
    engine.refresh_offer();
    assert!(in_graveyard(&engine, p0, pearled_unicorn()).is_some());
    assert!(in_graveyard(&engine, p1, serra_angel()).is_some());

    let (min, max) = cast_until_asked(&mut engine);
    assert_eq!(
        (min, max),
        (0, 2),
        "their Lions on the battlefield plus their Angel in the graveyard"
    );
    engine
        .apply(p0, PlayerAction::ChooseNumber(2))
        .expect("2 is within bounds");
    pass_until(&mut engine, stack_is_empty);
    let race = on_battlefield(&engine, p0, nameless_race()).expect("the Race is out");
    assert_eq!(pt(&engine, race), (2, 2));
}
