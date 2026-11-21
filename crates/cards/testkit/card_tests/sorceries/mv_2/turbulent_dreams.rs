//! `cards/sorceries/mv_2/turbulent_dreams.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn turbulent_dreams() -> CardIndex {
    card_index("83134570-debe-4bc0-9656-2a16801a3592")
}

/// Turbulent Dreams (83134570-debe-4bc0-9656-2a16801a3592) — {U}{U} Sorcery:
/// "As an additional cost to cast this spell, discard X cards. Return X target
/// nonland permanents to their owners' hands."
#[test]
fn turbulent_dreams_discards_x_and_bounces_x_nonland_permanents() {
    let mut engine = lk::game(
        SEED,
        &[island(), island(), sol_ring()],
        &[aurochs(), mirri_s_guile(), forest()],
        &[turbulent_dreams(), plains(), swamp()],
        &[],
    );
    let my_ring = lk::obj(&engine, P0, sol_ring());
    let their_beast = lk::obj(&engine, P1, aurochs());
    let their_ench = lk::obj(&engine, P1, mirri_s_guile());
    let their_land = lk::obj(&engine, P1, forest());
    let pitch1 = in_hand(&engine, P0, plains()).unwrap();
    let pitch2 = in_hand(&engine, P0, swamp()).unwrap();

    tap_mana_except(&mut engine, P0, my_ring);
    cast_with_floating(&mut engine, P0, turbulent_dreams());
    engine.apply(P0, PlayerAction::ChooseNumber(2)).unwrap();

    let Pending::ChooseTargets {
        options, min, max, ..
    } = engine.pending().clone()
    else {
        panic!("expected ChooseTargets, got {:?}", engine.pending());
    };
    assert_eq!((min, max), (2, 2));
    assert!(
        options.contains(&my_ring)
            && options.contains(&their_beast)
            && options.contains(&their_ench)
    );
    assert!(!options.contains(&their_land), "lands excluded");
    lk::aim(&mut engine, &[their_beast, their_ench], &[]);

    for pitch in [pitch1, pitch2] {
        engine
            .apply(
                P0,
                PlayerAction::ChooseObjects {
                    objects: vec![pitch],
                },
            )
            .unwrap();
    }
    lk::settle(&mut engine);

    assert_eq!(lk::zone_of(&engine, their_beast), Zone::Hand);
    assert_eq!(lk::zone_of(&engine, their_ench), Zone::Hand);
    assert_eq!(lk::zone_of(&engine, my_ring), Zone::Battlefield);
}
