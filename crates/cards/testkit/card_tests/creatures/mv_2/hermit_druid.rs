//! `cards/creatures/mv_2/hermit_druid.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;
use baylee_engine::zone::ZonePosition;

fn hermit_druid() -> CardIndex {
    card_index("16f6438d-2a29-41cb-bf0c-4d02bd66112b")
}

/// Hermit Druid ({1}{G} 1/1 Human Druid):
/// "{G}, {T}: Reveal cards from the top of your library until you reveal a
/// basic land card. Put that card into your hand and all other cards revealed
/// this way into your graveyard."
#[test]
fn hermit_druid_skips_spells_and_nonbasic_lands_to_put_first_basic_into_hand() {
    let mut e = Duel::new(SEED, index::WASTELAND)
        .battlefield(0, &[hermit_druid(), forest()])
        .hand(0, &[plains(), index::WASTELAND, index::SHOCK])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, lk::P0);

    // Move hand cards onto top of P0's library so top-to-bottom begins:
    // Shock (spell), Wasteland (nonbasic land), Plains (basic land), then 60 Wastelands.
    let basic = in_hand(&e, lk::P0, plains()).unwrap();
    let nonbasic = in_hand(&e, lk::P0, index::WASTELAND).unwrap();
    let spell = in_hand(&e, lk::P0, index::SHOCK).unwrap();
    {
        let state = e.dev_state_mut(lk::P0).unwrap();
        for id in [basic, nonbasic, spell] {
            state
                .move_object(
                    id,
                    ZoneLocation::Library(lk::P0),
                    ZonePosition::Top,
                    baylee_engine::event::Cause::Setup,
                )
                .unwrap();
        }
    }
    e.refresh_offer();

    lk::float(&mut e, lk::P0, &[(ManaColor::Green, 1)]);
    lk::use_ability(&mut e, lk::P0, hermit_druid(), 0, &[], &[]);

    assert!(in_hand(&e, lk::P0, plains()).is_some());
    assert_eq!(e.state().zones.list(ZoneLocation::Hand(lk::P0)).len(), 1);
    assert!(in_graveyard(&e, lk::P0, index::SHOCK).is_some());
    assert!(in_graveyard(&e, lk::P0, index::WASTELAND).is_some());
    assert_eq!(
        e.state().zones.list(ZoneLocation::Graveyard(lk::P0)).len(),
        2
    );
    assert_eq!(
        e.state().zones.list(ZoneLocation::Library(lk::P0)).len(),
        60
    );
}

#[test]
fn hermit_druid_mills_entire_library_when_it_contains_no_basic_land() {
    let mut e = Duel::new(SEED, index::WASTELAND)
        .battlefield(0, &[hermit_druid(), forest()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, lk::P0);

    lk::float(&mut e, lk::P0, &[(ManaColor::Green, 1)]);
    lk::use_ability(&mut e, lk::P0, hermit_druid(), 0, &[], &[]);

    assert_eq!(e.state().zones.list(ZoneLocation::Hand(lk::P0)).len(), 0);
    assert_eq!(e.state().zones.list(ZoneLocation::Library(lk::P0)).len(), 0);
    assert_eq!(
        e.state().zones.list(ZoneLocation::Graveyard(lk::P0)).len(),
        60
    );
}
