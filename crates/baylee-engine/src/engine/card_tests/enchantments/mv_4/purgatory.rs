//! `cards/enchantments/mv_4/purgatory.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "2fb8e100-58a1-4dc6-bc38-ad7f115e3e46"
fn purgatory() -> CardIndex {
    card_index("2fb8e100-58a1-4dc6-bc38-ad7f115e3e46")
}

/// The card in `seat`'s exile zone, if any.
fn in_exile(engine: &Engine<RegistryLookup>, seat: PlayerId, card: CardIndex) -> Option<ObjectId> {
    engine
        .state()
        .zones
        .list(ZoneLocation::Exile(seat))
        .iter()
        .copied()
        .find(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == card))
        })
}

/// A duel at P0's first main phase with Purgatory already on the table. Its
/// upkeep question ("pay {4} and 2 life?") is asked at the very first upkeep,
/// with nothing exiled yet, and is declined here.
#[track_caller]
fn game_with_purgatory_in_play(
    p0_board: &[CardIndex],
    p1_board: &[CardIndex],
    p0_hand: &[CardIndex],
) -> Engine<RegistryLookup> {
    lk::game_with(SEED, p0_board, p1_board, p0_hand, &[], |e| {
        lk::yes_no(e, false)
    })
}

#[test]
fn purgatory_enters_the_battlefield() {
    let mut engine = lk::game(SEED, &[], &[], &[purgatory()], &[]);

    lk::float(
        &mut engine,
        P0,
        &[
            (ManaColor::Colorless, 2),
            (ManaColor::White, 1),
            (ManaColor::Black, 1),
        ],
    );
    cast_with_floating(&mut engine, P0, purgatory());
    lk::settle(&mut engine);

    assert!(on_battlefield(&engine, P0, purgatory()).is_some());
}

/// "Put into your graveyard": your own nontoken creature is exiled instead
/// of resting in the graveyard, an opponent's creature dies as usual.
#[test]
fn purgatory_exiles_only_creatures_that_go_to_your_graveyard() {
    let mut engine = game_with_purgatory_in_play(
        &[purgatory(), index::HILL_GIANT],
        &[index::HILL_GIANT],
        &[index::LIGHTNING_BOLT, index::LIGHTNING_BOLT],
    );
    let mine = lk::obj(&engine, P0, index::HILL_GIANT);
    let theirs = lk::obj(&engine, P1, index::HILL_GIANT);

    lk::float(&mut engine, P0, &[(ManaColor::Red, 2)]);
    lk::cast_at(&mut engine, P0, index::LIGHTNING_BOLT, &[mine], &[]);
    assert!(
        in_exile(&engine, P0, index::HILL_GIANT).is_some(),
        "my dying Hill Giant is exiled"
    );
    assert!(in_graveyard(&engine, P0, index::HILL_GIANT).is_none());

    lk::cast_at(&mut engine, P0, index::LIGHTNING_BOLT, &[theirs], &[]);
    assert!(
        in_graveyard(&engine, P1, index::HILL_GIANT).is_some(),
        "the opponent's dying Hill Giant goes to their graveyard"
    );
    assert!(in_exile(&engine, P1, index::HILL_GIANT).is_none());
}

/// Puts a Grizzly Bears of P0's on the way to exile, then walks to P0's next
/// upkeep, where Purgatory asks about {4} and 2 life.
fn bears_in_exile_at_the_next_upkeep() -> Engine<RegistryLookup> {
    let mut engine = game_with_purgatory_in_play(
        &[purgatory(), index::GRIZZLY_BEARS],
        &[],
        &[index::LIGHTNING_BOLT],
    );
    let bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);
    lk::float(&mut engine, P0, &[(ManaColor::Red, 1)]);
    lk::cast_at(&mut engine, P0, index::LIGHTNING_BOLT, &[bears], &[]);
    assert!(in_exile(&engine, P0, index::GRIZZLY_BEARS).is_some());

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    assert_eq!(
        (engine.state().turn.number, engine.state().turn.active),
        (3, P0),
        "the upkeep is its controller's own"
    );
    engine
}

/// "You may pay {4} and 2 life. If you do, return a card exiled with this
/// enchantment to the battlefield."
#[test]
fn purgatory_paying_returns_the_exiled_creature_for_four_mana_and_two_life() {
    let mut engine = bears_in_exile_at_the_next_upkeep();
    let exiled = in_exile(&engine, P0, index::GRIZZLY_BEARS).unwrap();

    lk::float(&mut engine, P0, &[(ManaColor::Colorless, 4)]);
    lk::drive(&mut engine, &[exiled], true, 0);

    assert!(on_battlefield(&engine, P0, index::GRIZZLY_BEARS).is_some());
    assert!(in_exile(&engine, P0, index::GRIZZLY_BEARS).is_none());
    assert_eq!(lk::life(&engine, P0), 18, "2 life paid");
    assert_eq!(lk::pool_total(&engine, P0), 0, "{{4}} paid");
}

#[test]
fn purgatory_declining_leaves_the_creature_exiled_and_the_life_alone() {
    let mut engine = bears_in_exile_at_the_next_upkeep();
    let exiled = in_exile(&engine, P0, index::GRIZZLY_BEARS).unwrap();

    lk::drive(&mut engine, &[exiled], false, 0);

    assert!(on_battlefield(&engine, P0, index::GRIZZLY_BEARS).is_none());
    assert!(in_exile(&engine, P0, index::GRIZZLY_BEARS).is_some());
    assert_eq!(lk::life(&engine, P0), 20);
}
