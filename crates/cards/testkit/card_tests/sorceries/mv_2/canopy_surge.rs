//! `cards/sorceries/mv_2/canopy_surge.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn canopy_surge() -> CardIndex {
    card_index("a151c45f-0b9e-41af-90b3-fafa12e3bbce")
}

fn serra_angel() -> CardIndex {
    card_index("4b7ac066-e5c7-43e6-9e7e-2739b24a905d")
}

fn cast_surge(engine: &mut Engine<RegistryLookup>, kicked: bool) {
    cast_with_floating(engine, P0, canopy_surge());
    match engine.pending().clone() {
        Pending::YesNo { .. } => lk::yes_no(engine, kicked),
        Pending::ChooseCastMode { .. } => {
            let kind = if kicked {
                CastModeKind::Kicked
            } else {
                CastModeKind::Normal
            };
            let slot = choose_cast_kind(engine, kind);
            engine.apply(P0, PlayerAction::ChooseMode(slot)).unwrap();
        }
        _ => assert!(
            !kicked,
            "expected kicker prompt, got {:?}",
            engine.pending()
        ),
    }
    lk::settle(engine);
}

/// Canopy Surge (a151c45f-0b9e-41af-90b3-fafa12e3bbce) — {1}{G} Sorcery,
/// Kicker {2}: deals 1 damage (or 4 if kicked) to each creature with flying
/// and each player.
#[test]
fn canopy_surge_deals_one_unkicked_and_four_kicked_hitting_only_flyers_and_players() {
    let mut unkicked = lk::game(
        SEED,
        &[forest(), forest(), forest(), forest(), aurochs()],
        &[serra_angel()],
        &[canopy_surge()],
        &[],
    );
    let ground = lk::obj(&unkicked, P0, aurochs());
    let flyer = lk::obj(&unkicked, P1, serra_angel());

    tap_all_mana(&mut unkicked, P0);
    cast_surge(&mut unkicked, false);
    assert_eq!(lk::pool_total(&unkicked, P0), 2);
    assert_eq!(lk::damage_on(&unkicked, flyer), 1);
    assert_eq!(
        lk::damage_on(&unkicked, ground),
        0,
        "ground creature spared"
    );
    assert_eq!((lk::life(&unkicked, P0), lk::life(&unkicked, P1)), (19, 19));

    let mut kicked = lk::game(
        SEED,
        &[forest(), forest(), forest(), forest(), aurochs()],
        &[serra_angel()],
        &[canopy_surge()],
        &[],
    );
    let ground2 = lk::obj(&kicked, P0, aurochs());
    let flyer2 = lk::obj(&kicked, P1, serra_angel());

    tap_all_mana(&mut kicked, P0);
    cast_surge(&mut kicked, true);
    assert_eq!(lk::pool_total(&kicked, P0), 0);
    assert_eq!(lk::zone_of(&kicked, flyer2), Zone::Graveyard);
    assert_eq!(lk::zone_of(&kicked, ground2), Zone::Battlefield);
    assert_eq!(lk::damage_on(&kicked, ground2), 0);
    assert_eq!((lk::life(&kicked, P0), lk::life(&kicked, P1)), (16, 16));
}
