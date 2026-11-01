//! `cards/instants/mv_2/mana_drain.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::engine::card_tests::partials_w2c_kit as kit;
use baylee_core::generated::index;

// oracle_id = "74d3277a-38e5-4732-afed-084a56148f20"

/// Mana Drain is `{U}{U}` and prints two sentences: "Counter target spell",
/// and "At the beginning of your next main phase, add an amount of {C} equal
/// to that spell's mana value."
///
/// The victim is Tidings — `{3}{U}{U}`, five mana and no target — so the
/// delayed mana has to be **five** colourless. That is the whole second
/// sentence: the amount is read off the spell that was countered, which a
/// one-mana victim could not tell from the Drain's own cost or from a card
/// that always adds one. Five Islands pay Tidings' price to the last mana, so
/// the five that arrive later are not leftovers, and reading them at p1's
/// *next* first main phase is the first sentence too: mana made during p0's
/// main phase would have emptied with it (CR 500.5) and the walk would never
/// see it at all.
#[test]
fn mana_drain_counters_a_five_mana_spell_and_pays_five_colorless_a_turn_later() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island(), island(), island()])
        .hand(0, &[tidings()])
        .battlefield(1, &[island(), island()])
        .hand(1, &[mana_drain()])
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "p0 reaches a first main phase of its own"
    );

    let library_before = library_size(&engine, p0);
    cast_from_hand(&mut engine, p0, tidings());
    let victim = on_stack(&engine, tidings()).expect("Tidings went on the stack");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "five Islands are exactly {{3}}{{U}}{{U}}, so nothing is left floating"
    );

    // p0 passes, and the opponent gets the window an instant needs.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Priority { player, .. } if *player == p1)
            && on_stack(e, tidings()).is_some()
    });
    tap_all_mana(&mut engine, p1);
    assert_eq!(
        engine.state().players[1].mana_pool.total(),
        2,
        "the two Islands are {{U}}{{U}}, and the Drain asks for no more"
    );
    cast_with_floating(&mut engine, p1, mana_drain());
    let menu = aim_at(&mut engine, p1, victim);
    assert!(
        menu.contains(&victim),
        "the only spell on the stack is the one being countered: {menu:?}"
    );

    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, p0, tidings()).is_some(),
        "\"Counter target spell\": the sorcery is in its owner's graveyard"
    );
    assert!(
        in_graveyard(&engine, p1, mana_drain()).is_some(),
        "and the counterspell itself resolved rather than fizzling"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before,
        "Tidings draws four cards only if it resolves — a countered spell does nothing"
    );
    assert_eq!(
        engine.state().players[1].mana_pool.total(),
        0,
        "the Drain's price was all p1 had, so the pool starts this walk blank"
    );

    // The second sentence, at the only moment it can be read: p1's own next
    // first main phase, with the mana already made.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1
            && matches!(e.state().turn.phase, Phase::FirstMain)
            && e.state().players[1].mana_pool.total() > 0
    });

    let pool = &engine.state().players[1].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Colorless),
        5,
        "the countered spell's mana value is five, so five {{C}} — not one, \
         and not the two the Drain itself cost"
    );
    assert_eq!(
        pool.total(),
        5,
        "and nothing else is floating: the two Islands paid for the Drain"
    );
    assert_eq!(
        pool.available(ManaColor::Blue),
        0,
        "\"add an amount of {{C}}\": the mana is colourless, and no blue land \
         on this board made any of it"
    );
}

/// p0 casts `spell` off the mana its lands make, answering its questions
/// from `answers`, and passes until p1 holds priority over it; p1 taps its
/// two Islands and casts Mana Drain at it. Returns the spell.
#[track_caller]
fn drain_it(
    engine: &mut Engine<RegistryLookup>,
    spell: CardIndex,
    answers: &mut kit::Answers,
) -> ObjectId {
    let (p0, p1) = (kit::P0, kit::P1);
    tap_all_mana(engine, p0);
    let victim = kit::cast(engine, p0, spell, answers);
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::Priority { player, .. } if *player == p1)
            && on_stack(e, spell).is_some()
    });
    tap_all_mana(engine, p1);
    cast_with_floating(engine, p1, mana_drain());
    aim_at(engine, p1, victim);
    victim
}

/// p1's pool at the first main phase of its own that has mana in it.
fn drained_mana(engine: &mut Engine<RegistryLookup>) -> u32 {
    let p1 = kit::P1;
    pass_until(engine, |e| {
        e.state().turn.active == p1
            && matches!(e.state().turn.phase, Phase::FirstMain)
            && e.state().players[1].mana_pool.total() > 0
    });
    engine.state().players[1]
        .mana_pool
        .available(ManaColor::Colorless)
}

/// "That spell's mana value" is the spell's as it last existed on the stack,
/// X included (CR 202.3e, CR 608.2h): Blaze `{X}{R}` cast for two is a
/// three, though the card it is in the graveyard is a one. The Drain used to
/// read the card after its own first sentence had put it there, and paid one
/// {C} for a three-mana spell.
#[test]
fn mana_drain_pays_for_the_x_the_countered_spell_was_cast_with() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &kit::lands(mountain(), 3))
        .hand(0, &[index::BLAZE])
        .battlefield(1, &[island(), island()])
        .hand(1, &[mana_drain()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, kit::P0));
    drain_it(
        &mut engine,
        index::BLAZE,
        &mut kit::Answers::new().x(2).aim_players(&[kit::P1]),
    );
    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, kit::P0, index::BLAZE).is_some(),
        "Blaze was countered"
    );
    assert_eq!(kit::life(&engine, kit::P1), 20, "and dealt nothing");
    assert_eq!(
        drained_mana(&mut engine),
        3,
        "X = 2 and {{R}}: a mana value of three, not the card's one"
    );
}

/// A spell that can't be countered is still a legal target, and the mana
/// comes all the same: the second sentence does not depend on the first
/// (Mana Drain's ruling of 2020-11-10). Abrupt Decay resolves after the Drain
/// and destroys the bear, and p1 is paid its two.
#[test]
fn mana_drain_pays_for_a_spell_it_could_not_counter() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[swamp(), forest()])
        .hand(0, &[index::ABRUPT_DECAY])
        .battlefield(1, &[island(), island(), kit::bears()])
        .hand(1, &[mana_drain()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, kit::P0));
    let bear = on_battlefield(&engine, kit::P1, kit::bears()).expect("the bear");
    drain_it(
        &mut engine,
        index::ABRUPT_DECAY,
        &mut kit::Answers::new().aim(&[bear]),
    );
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, kit::P1, kit::bears()).is_none(),
        "Abrupt Decay could not be countered and destroyed the bear"
    );
    assert_eq!(drained_mana(&mut engine), 2, "and the Drain still pays two");
}

/// p0 passes in `phase` of its own turn with nothing on the stack, p1 answers
/// with Lightning Bolt at p0, and p0 Drains the Bolt. Returns the turn.
#[track_caller]
fn drain_a_bolt_in(engine: &mut Engine<RegistryLookup>, phase: Phase) -> u32 {
    let (p0, p1) = (kit::P0, kit::P1);
    pass_until(engine, |e| {
        e.state().turn.active == p0
            && e.state().turn.phase == phase
            && stack_is_empty(e)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p1)
    });
    tap_all_mana(engine, p1);
    let bolt = kit::cast(
        engine,
        p1,
        kit::bolt(),
        &mut kit::Answers::new().aim_players(&[p0]),
    );
    engine.apply(p1, PlayerAction::PassPriority).unwrap();
    tap_all_mana(engine, p0);
    cast_with_floating(engine, p0, mana_drain());
    aim_at(engine, p0, bolt);
    pass_until(engine, stack_is_empty);
    assert_eq!(kit::life(engine, p0), 20, "the Bolt was countered");
    assert_eq!(engine.state().turn.phase, phase, "still the same phase");
    engine.state().turn.number
}

/// A Mana Drain duel: p0 holds the Drain and two Islands, p1 a Mountain and
/// Lightning Bolt. p0 is at its own first main phase.
fn bolt_and_drain() -> Engine<RegistryLookup> {
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island()])
        .hand(0, &[mana_drain()])
        .battlefield(1, &[mountain()])
        .hand(1, &[kit::bolt()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, kit::P0));
    engine
}

/// "At the beginning of your next main phase": a Drain resolved in its
/// controller's first main phase pays at the beginning of that turn's
/// second main phase (the card's ruling of 2020-11-10), not a turn later.
/// The engine waited for the next first main phase.
#[test]
fn mana_drain_cast_in_your_first_main_pays_in_your_second_main() {
    let mut engine = bolt_and_drain();
    let turn = drain_a_bolt_in(&mut engine, Phase::FirstMain);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "nothing yet"
    );
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert_eq!(engine.state().turn.number, turn, "the same turn");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        1,
        "Lightning Bolt's one {{C}}, as the second main phase begins"
    );
}

/// The return trip: a Drain resolved in its controller's second main phase
/// has seen that main phase begin already, so its next main phase is the
/// first one of its controller's next turn, and nothing comes before.
#[test]
fn mana_drain_cast_in_your_second_main_pays_in_your_next_first_main() {
    let mut engine = bolt_and_drain();
    let turn = drain_a_bolt_in(&mut engine, Phase::SecondMain);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "nothing yet"
    );
    pass_until(&mut engine, |e| {
        e.state().turn.active == kit::P0
            && e.state().turn.phase == Phase::FirstMain
            && e.state().turn.number > turn
    });
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        1,
        "p0's next main phase, on its next turn"
    );
}

/// A copy of a spell is a spell with the original's X (CR 707.10), and the
/// Drain pays for that: Fork copies Blaze cast for X = 2, the Drain counters
/// the copy, and p1 is paid three. The countered copy ceases to exist, so
/// it is read as it was when the Drain began to resolve (CR 608.2h). The
/// original resolves and deals its two.
#[test]
fn mana_drain_pays_for_the_x_of_a_copy_it_counters() {
    let (p0, p1) = (kit::P0, kit::P1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &kit::lands(mountain(), 5))
        .hand(0, &[index::BLAZE, index::FORK])
        .battlefield(1, &[island(), island()])
        .hand(1, &[mana_drain()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));
    tap_all_mana(&mut engine, p0);
    let blaze = kit::cast(
        &mut engine,
        p0,
        index::BLAZE,
        &mut kit::Answers::new().x(2).aim_players(&[p1]),
    );
    kit::cast(
        &mut engine,
        p0,
        index::FORK,
        &mut kit::Answers::new().aim(&[blaze]),
    );
    // "You may choose new targets for the copy": none, so it keeps p1.
    kit::drive_with(
        &mut engine,
        &mut kit::Answers::new().aim_players(&[]),
        |e| {
            on_stack(e, index::FORK).is_none()
                && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
        },
    );
    let copy = *engine
        .state()
        .zones
        .list(ZoneLocation::Stack)
        .last()
        .expect("Fork's copy is on the stack");
    assert_ne!(copy, blaze, "the copy is on top, the original below it");
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, p1);
    cast_with_floating(&mut engine, p1, mana_drain());
    aim_at(&mut engine, p1, copy);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        kit::life(&engine, p1),
        18,
        "the original Blaze dealt its two"
    );
    assert_eq!(
        drained_mana(&mut engine),
        3,
        "the copy's mana value: X = 2 and {{R}}"
    );
}
