//! `cards/sorceries/mv_4/wash_out.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn wash_out() -> CardIndex {
    card_index("54748cb1-d92a-4212-ad76-417ee79b5ef1")
}

fn savannah_lions() -> CardIndex {
    card_index("60ba93eb-39e6-4af2-9c66-cd38f72daff2")
}

/// Wash Out (54748cb1-d92a-4212-ad76-417ee79b5ef1) — {3}{U} Sorcery:
/// "Return all permanents of the color of your choice to their owners' hands."
#[test]
fn wash_out_bounces_all_permanents_of_chosen_color_to_their_owners_hands() {
    let mut engine = lk::game(
        SEED,
        &[
            island(),
            island(),
            island(),
            island(),
            aurochs(),
            savannah_lions(),
            sol_ring(),
        ],
        &[lotleth_troll(), fangren_hunter(), forest()],
        &[wash_out()],
        &[],
    );
    let stolen_green = lk::obj(&engine, P0, aurochs());
    let my_white = lk::obj(&engine, P0, savannah_lions());
    let my_ring = lk::obj(&engine, P0, sol_ring());
    let their_black_green = lk::obj(&engine, P1, lotleth_troll());
    let their_green = lk::obj(&engine, P1, fangren_hunter());
    let their_land = lk::obj(&engine, P1, forest());

    // P1 controls P0's Aurochs -> Wash Out returns it to P0's (owner's) hand!
    engine
        .dev_state_mut(P0)
        .unwrap()
        .object_mut(stolen_green)
        .unwrap()
        .set_controller(P1);
    engine.refresh_offer();

    tap_mana_except(&mut engine, P0, my_ring);
    cast_with_floating(&mut engine, P0, wash_out());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseColor { .. })
    });
    engine
        .apply(P0, PlayerAction::ChooseColor(ManaColor::Green))
        .unwrap();
    lk::settle(&mut engine);

    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Hand(P0))
            .contains(&stolen_green),
        "returned to its owner's hand, not its controller's"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Hand(P1))
            .contains(&their_black_green),
        "multicolored green permanent is also bounced"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Hand(P1))
            .contains(&their_green)
    );
    assert_eq!(lk::zone_of(&engine, my_white), Zone::Battlefield);
    assert_eq!(lk::zone_of(&engine, my_ring), Zone::Battlefield);
    assert_eq!(
        lk::zone_of(&engine, their_land),
        Zone::Battlefield,
        "Forests are colorless permanents"
    );
}
