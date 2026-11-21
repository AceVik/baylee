//! `cards/sorceries/mv_2/hull_breach.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn hull_breach() -> CardIndex {
    card_index("2da232d8-580f-4116-b977-2c59cd21b5a4")
}

fn cast_hull_mode(
    engine: &mut Engine<RegistryLookup>,
    mode: usize,
    first: ObjectId,
    second: Option<ObjectId>,
) {
    tap_all_mana(engine, P0);
    cast_with_floating(engine, P0, hull_breach());
    let slot = choose_cast_kind(engine, CastModeKind::Mode(mode));
    engine.apply(P0, PlayerAction::ChooseMode(slot)).unwrap();
    aim_at(engine, P0, first);
    if let Some(sec) = second {
        aim_at(engine, P0, sec);
    }
    lk::settle(engine);
}

/// Hull Breach (2da232d8-580f-4116-b977-2c59cd21b5a4) — {R}{G} Sorcery:
/// Mode 0: Destroy target artifact; Mode 1: Destroy target enchantment;
/// Mode 2: Destroy target artifact and target enchantment.
#[test]
fn hull_breach_plays_all_three_modes_destroying_artifact_enchantment_or_both() {
    let mut m0 = lk::game(
        SEED,
        &[mountain(), forest()],
        &[sol_ring(), mirri_s_guile()],
        &[hull_breach()],
        &[],
    );
    let ring0 = lk::obj(&m0, P1, sol_ring());
    let ench0 = lk::obj(&m0, P1, mirri_s_guile());
    cast_hull_mode(&mut m0, 0, ring0, None);
    assert_eq!(lk::zone_of(&m0, ring0), Zone::Graveyard);
    assert_eq!(lk::zone_of(&m0, ench0), Zone::Battlefield);

    let mut m1 = lk::game(
        SEED,
        &[mountain(), forest()],
        &[sol_ring(), mirri_s_guile()],
        &[hull_breach()],
        &[],
    );
    let ring1 = lk::obj(&m1, P1, sol_ring());
    let ench1 = lk::obj(&m1, P1, mirri_s_guile());
    cast_hull_mode(&mut m1, 1, ench1, None);
    assert_eq!(lk::zone_of(&m1, ring1), Zone::Battlefield);
    assert_eq!(lk::zone_of(&m1, ench1), Zone::Graveyard);

    let mut m2 = lk::game(
        SEED,
        &[mountain(), forest()],
        &[sol_ring(), mirri_s_guile()],
        &[hull_breach()],
        &[],
    );
    let ring2 = lk::obj(&m2, P1, sol_ring());
    let ench2 = lk::obj(&m2, P1, mirri_s_guile());
    cast_hull_mode(&mut m2, 2, ring2, Some(ench2));
    assert_eq!(lk::zone_of(&m2, ring2), Zone::Graveyard);
    assert_eq!(lk::zone_of(&m2, ench2), Zone::Graveyard);
}
