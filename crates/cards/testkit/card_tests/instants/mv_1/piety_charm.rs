//! `cards/instants/mv_1/piety_charm.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "3d4ec57a-8541-42ed-9790-7933f96c8adf"
fn piety_charm() -> CardIndex {
    card_index("3d4ec57a-8541-42ed-9790-7933f96c8adf")
}

/// Mode 1: Destroy target Aura attached to a creature (excluding non-Aura
/// enchantments).
#[test]
fn piety_charm_mode_one_destroys_aura_attached_to_creature() {
    let mut engine = lk::game(
        SEED,
        &[index::GRIZZLY_BEARS],
        &[their_enchantment()],
        &[index::HOLY_STRENGTH, piety_charm()],
        &[],
    );
    let bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);
    let global_ench = lk::obj(&engine, P1, their_enchantment());

    lk::float(&mut engine, P0, &[(ManaColor::White, 2)]);
    lk::cast_at(&mut engine, P0, index::HOLY_STRENGTH, &[bears], &[]);
    let aura = lk::obj(&engine, P0, index::HOLY_STRENGTH);

    cast_with_floating(&mut engine, P0, piety_charm());
    let m0 = choose_cast_kind(&engine, CastModeKind::Mode(0));
    engine.apply(P0, PlayerAction::ChooseMode(m0)).unwrap();
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected Aura target choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&aura));
    assert!(
        !options.contains(&global_ench),
        "non-Aura enchantment is not a legal target: {options:?}"
    );
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![aura],
                players: vec![],
            },
        )
        .unwrap();
    lk::settle(&mut engine);

    assert!(in_graveyard(&engine, P0, index::HOLY_STRENGTH).is_some());
    assert_eq!(pt(&engine, bears), (2, 2));
}

/// Mode 2: Target Soldier creature gets +2/+2 until end of turn (excluding
/// non-Soldier creatures), and wears off next turn.
#[test]
fn piety_charm_mode_two_pumps_only_target_soldier_until_eot() {
    let mut engine = lk::game(
        SEED,
        &[index::BENALISH_HERO, index::GRIZZLY_BEARS],
        &[],
        &[piety_charm()],
        &[],
    );
    let hero = lk::obj(&engine, P0, index::BENALISH_HERO);
    let bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);

    lk::float(&mut engine, P0, &[(ManaColor::White, 1)]);
    cast_with_floating(&mut engine, P0, piety_charm());
    let m1 = choose_cast_kind(&engine, CastModeKind::Mode(1));
    engine.apply(P0, PlayerAction::ChooseMode(m1)).unwrap();
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected Soldier target choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&hero));
    assert!(
        !options.contains(&bears),
        "non-Soldier Bear is not a legal target: {options:?}"
    );
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![hero],
                players: vec![],
            },
        )
        .unwrap();
    lk::settle(&mut engine);

    assert_eq!(pt(&engine, hero), (3, 3), "+2/+2 to Soldier");
    reach_their_main_phase(&mut engine, P1);
    assert_eq!(pt(&engine, hero), (1, 1), "bonus wears off at EOT");
}

/// Mode 3: Creatures you control gain vigilance until end of turn (not
/// opponent's), attack without tapping, and lose vigilance next turn.
#[test]
fn piety_charm_mode_three_grants_vigilance_to_your_creatures_until_eot() {
    let mut engine = lk::game(
        SEED,
        &[index::GRIZZLY_BEARS],
        &[index::HILL_GIANT],
        &[piety_charm()],
        &[],
    );
    let bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);
    let giant = lk::obj(&engine, P1, index::HILL_GIANT);

    lk::float(&mut engine, P0, &[(ManaColor::White, 1)]);
    cast_with_floating(&mut engine, P0, piety_charm());
    if matches!(engine.pending(), Pending::ChooseCastMode { .. }) {
        let m2 = choose_cast_kind(&engine, CastModeKind::Mode(2));
        engine.apply(P0, PlayerAction::ChooseMode(m2)).unwrap();
    }
    lk::settle(&mut engine);

    assert!(keywords(&engine, bears).contains(KeywordSet::VIGILANCE));
    assert!(!keywords(&engine, giant).contains(KeywordSet::VIGILANCE));

    let _ = lk::attack(&mut engine, &[bears]);
    lk::block(&mut engine, &[]);
    lk::through_combat(&mut engine);
    assert!(
        !is_tapped(&engine, bears),
        "vigilant attacker stayed untapped"
    );

    reach_their_main_phase(&mut engine, P1);
    assert!(!keywords(&engine, bears).contains(KeywordSet::VIGILANCE));
}
