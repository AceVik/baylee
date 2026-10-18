//! `cards/sorceries/mv_4/all_hallow_s_eve.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::engine::card_tests::partials_w2c_kit::{bears, hill_giant};
use baylee_cards_dsl::counters;

fn all_hallow_s_eve() -> CardIndex {
    card_index("ebed900f-6de3-4ae7-9795-fae34882298d")
}

fn find_in_exile(
    engine: &Engine<RegistryLookup>,
    seat: PlayerId,
    card: CardIndex,
) -> Option<ObjectId> {
    engine
        .state()
        .zones
        .list(ZoneLocation::Exile(seat))
        .iter()
        .copied()
        .find(|&id| {
            engine
                .state()
                .object(id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == card))
        })
}

#[track_caller]
fn bury_from_hand(
    engine: &mut Engine<RegistryLookup>,
    seat: PlayerId,
    card: CardIndex,
) -> ObjectId {
    let held = in_hand(engine, seat, card).expect("card in hand");
    let buried = engine
        .dev_state_mut(seat)
        .expect("harness")
        .move_object(
            held,
            ZoneLocation::Graveyard(seat),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .expect("move to graveyard");
    engine.refresh_offer();
    buried
}

/// All Hallow's Eve: "Exile All Hallow's Eve with two scream counters on it.
/// At the beginning of your upkeep, if this card is exiled with a scream
/// counter on it, remove a scream counter from it. If there are no more
/// scream counters on it, put it into your graveyard and each player returns
/// all creature cards from their graveyard to the battlefield."
#[test]
fn all_hallow_s_eve_counts_down_two_upkeeps_in_exile_then_returns_each_players_creatures() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(41, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp()])
        .hand(0, &[all_hallow_s_eve(), bears()])
        .hand(1, &[hill_giant()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches main phase");

    bury_from_hand(&mut engine, p0, bears());
    bury_from_hand(&mut engine, p1, hill_giant());

    cast_from_hand(&mut engine, p0, all_hallow_s_eve());
    pass_until(&mut engine, stack_is_empty);

    let exiled = find_in_exile(&engine, p0, all_hallow_s_eve()).expect("exiled on resolution");
    assert_eq!(
        counters_on(&engine, exiled, counters::SCREAM),
        2,
        "exiled with two scream counters"
    );
    assert!(in_graveyard(&engine, p0, all_hallow_s_eve()).is_none());

    // Advance to p0's next upkeep (turn 3): first scream counter is removed,
    // one remains, so the spell stays in exile and creatures stay buried.
    pass_until(&mut engine, |e| {
        e.state().turn.number == 3
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert_eq!(
        counters_on(&engine, exiled, counters::SCREAM),
        1,
        "first upkeep removed one scream counter"
    );
    assert!(on_battlefield(&engine, p0, bears()).is_none());
    assert!(on_battlefield(&engine, p1, hill_giant()).is_none());

    // Advance to p0's following main phase (turn 5): second scream counter is
    // removed, All Hallow's Eve moves to p0's graveyard, and each player's
    // creature cards return from their graveyard to the battlefield under
    // their own control.
    pass_until(&mut engine, |e| {
        e.state().turn.number == 5
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(
        find_in_exile(&engine, p0, all_hallow_s_eve()).is_none(),
        "All Hallow's Eve left exile once the last scream counter came off"
    );
    assert!(
        in_graveyard(&engine, p0, all_hallow_s_eve()).is_some(),
        "put into its owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p0, bears()).is_some(),
        "p0's Grizzly Bears returned under p0's control"
    );
    assert!(
        on_battlefield(&engine, p1, hill_giant()).is_some(),
        "p1's Hill Giant returned under p1's control"
    );
}
