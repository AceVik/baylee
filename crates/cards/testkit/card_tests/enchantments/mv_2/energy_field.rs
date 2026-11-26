//! `cards/enchantments/mv_2/energy_field.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as kit;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn energy_field() -> CardIndex {
    card_index("fbf7a7f4-9915-4a53-84b3-e55e9f487323")
}

fn island() -> CardIndex {
    card_index("b2c6aa39-2d2a-459c-a555-fb48ba993373")
}

fn mountain() -> CardIndex {
    card_index("a3fb7228-e76b-4e96-a40e-20b5fed75685")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn lightning_bolt() -> CardIndex {
    card_index("4457ed35-7c10-48c8-9776-456485fdf070")
}

#[test]
fn energy_field_prevents_opponent_damage_allows_own_and_sacrifices_on_graveyard_card() {
    let mut e = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), mountain(), grizzly_bears()])
        .battlefield(1, &[mountain(), grizzly_bears()])
        .hand(0, &[energy_field(), lightning_bolt()])
        .hand(1, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);

    cast_from_hand(&mut e, P0, energy_field());
    pass_until(&mut e, stack_is_empty);

    // P1 casts Lightning Bolt at P0: prevented.
    reach_their_main_phase(&mut e, P1);
    cast_from_hand(&mut e, P1, lightning_bolt());
    kit::aim(&mut e, &[], &[P0]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        e.state().players[0].life,
        20,
        "foreign spell damage prevented"
    );
    // Wait: P1's bolt went to P1's graveyard, NOT P0's! So Energy Field stays!
    assert!(on_battlefield(&e, P0, energy_field()).is_some());

    // P1 attacks P0 with Grizzly Bears: combat damage prevented.
    let p1_bears = on_battlefield(&e, P1, grizzly_bears()).unwrap();
    kit::attack(&mut e, &[p1_bears]);
    e.apply(P0, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        e.state().players[0].life,
        20,
        "opponent combat damage prevented"
    );

    // Back to P0's turn: P0 bolts P0 -> NOT prevented (P0 controls Lightning Bolt), then Bolt hits P0's graveyard -> Energy Field sacrificed!
    reach_their_main_phase(&mut e, P0);
    cast_from_hand(&mut e, P0, lightning_bolt());
    kit::aim(&mut e, &[], &[P0]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        e.state().players[0].life,
        17,
        "own source damage not prevented"
    );
    assert!(
        on_battlefield(&e, P0, energy_field()).is_none(),
        "card put into P0 graveyard sacrifices Energy Field"
    );
}
