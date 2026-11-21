//! `cards/sorceries/mv_2/breath_of_darigaaz.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn breath_of_darigaaz() -> CardIndex {
    card_index("1954994f-17bf-4ea5-af72-60f9bfcb6569")
}

fn serra_angel() -> CardIndex {
    card_index("4b7ac066-e5c7-43e6-9e7e-2739b24a905d")
}

fn cast_breath(engine: &mut Engine<RegistryLookup>, kicked: bool) {
    cast_with_floating(engine, P0, breath_of_darigaaz());
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

/// Breath of Darigaaz (1954994f-17bf-4ea5-af72-60f9bfcb6569) — {1}{R} Sorcery,
/// Kicker {2}: deals 1 damage (or 4 if kicked) to each creature without flying
/// and each player.
#[test]
fn breath_of_darigaaz_deals_one_unkicked_and_four_kicked_sparing_flyers() {
    let mut unkicked = lk::game(
        SEED,
        &[mountain(), mountain(), mountain(), mountain(), aurochs()],
        &[serra_angel(), llanowar_elves()],
        &[breath_of_darigaaz()],
        &[],
    );
    let beast = lk::obj(&unkicked, P0, aurochs());
    let angel = lk::obj(&unkicked, P1, serra_angel());
    let elf = lk::obj(&unkicked, P1, llanowar_elves());

    tap_all_mana(&mut unkicked, P0);
    cast_breath(&mut unkicked, false);
    assert_eq!(lk::pool_total(&unkicked, P0), 2, "only {{1}}{{R}} spent");
    assert_eq!(lk::damage_on(&unkicked, beast), 1);
    assert_eq!(lk::damage_on(&unkicked, angel), 0, "flyer spared");
    assert_eq!(lk::zone_of(&unkicked, elf), Zone::Graveyard);
    assert_eq!((lk::life(&unkicked, P0), lk::life(&unkicked, P1)), (19, 19));

    let mut kicked = lk::game(
        SEED,
        &[
            mountain(),
            mountain(),
            mountain(),
            mountain(),
            fangren_hunter(),
        ],
        &[serra_angel()],
        &[breath_of_darigaaz()],
        &[],
    );
    let hunter = lk::obj(&kicked, P0, fangren_hunter());
    let angel2 = lk::obj(&kicked, P1, serra_angel());

    tap_all_mana(&mut kicked, P0);
    cast_breath(&mut kicked, true);
    assert_eq!(
        lk::pool_total(&kicked, P0),
        0,
        "all {{3}}{{R}} spent kicked"
    );
    assert_eq!(
        lk::zone_of(&kicked, hunter),
        Zone::Graveyard,
        "4 damage instead of 1"
    );
    assert_eq!(lk::zone_of(&kicked, angel2), Zone::Battlefield);
    assert_eq!(lk::damage_on(&kicked, angel2), 0);
    assert_eq!((lk::life(&kicked, P0), lk::life(&kicked, P1)), (16, 16));
}
