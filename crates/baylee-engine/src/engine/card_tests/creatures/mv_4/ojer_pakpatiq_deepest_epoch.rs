//! `cards/creatures/mv_4/ojer_pakpatiq_deepest_epoch.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::engine::card_tests::legends_kit as kit;
use baylee_core::generated::index;

/// The death ability returns this card directly, tapped, with its back face up.
#[test]
fn ojer_pakpatiq_dies_and_comes_back_as_the_land_on_its_other_face() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp()])
        .hand(0, &[heroes_downfall()])
        .battlefield(1, &[ojer_pakpatiq_deepest_epoch()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let god = on_battlefield(&engine, p1, ojer_pakpatiq_deepest_epoch()).expect("the god is out");
    assert_eq!(pt(&engine, god), (4, 3), "a 4/3 before anything happens");
    assert!(
        keywords(&engine, god).contains(KeywordSet::FLYING),
        "and it flies"
    );

    cast_from_hand(&mut engine, p0, heroes_downfall());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("the Downfall asks for a target, got {:?}", engine.pending())
    };
    assert!(options.contains(&god), "a creature is a legal target");
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![god],
                players: vec![],
            },
        )
        .expect("a target the spell offered");
    pass_until(&mut engine, |e| at_rest(e, p0));

    let temple =
        on_battlefield(&engine, p1, ojer_pakpatiq_deepest_epoch()).expect("it came back at once");
    let t = types(&engine, temple);
    assert!(
        t.contains(TypeSet::LAND),
        "\"transformed\" — the back face is Temple of Cyclical Time"
    );
    assert!(
        !t.contains(TypeSet::CREATURE),
        "and the creature stayed on the front"
    );
    assert!(
        in_graveyard(&engine, p1, ojer_pakpatiq_deepest_epoch()).is_none(),
        "the god is on the battlefield and not in a graveyard"
    );

    assert!(
        is_tapped(&engine, temple),
        "the return explicitly enters tapped"
    );
}

fn stack_size(e: &Engine<RegistryLookup>) -> usize {
    e.state().zones.list(ZoneLocation::Stack).len()
}

/// "Whenever you cast an instant spell from your hand, it gains rebound": the
/// control, so that the absence of the trigger below means something. The
/// Giant Growth and Ojer's trigger are both on the stack, and the Growth is
/// exiled as it resolves.
#[test]
fn ojer_triggers_for_your_own_instant_cast_from_your_hand() {
    let p0 = kit::P0;
    let mut e = kit::game(
        SEED,
        &[index::OJER_PAKPATIQ_DEEPEST_EPOCH, index::GRIZZLY_BEARS],
        &[],
        &[index::GIANT_GROWTH],
        &[],
    );
    let bears = kit::obj(&e, p0, index::GRIZZLY_BEARS);
    kit::float(&mut e, p0, &[(ManaColor::Green, 1)]);
    cast_with_floating(&mut e, p0, index::GIANT_GROWTH);
    kit::aim(&mut e, &[bears], &[]);
    assert_eq!(stack_size(&e), 2, "the Growth and Ojer's rebound trigger");
    kit::settle(&mut e);
    let growth_zone = |e: &Engine<RegistryLookup>| {
        e.state()
            .zones
            .list(ZoneLocation::Exile(p0))
            .iter()
            .any(|id| {
                e.state()
                    .object(*id)
                    .is_some_and(|o| o.card.is_some_and(|c| c.index == index::GIANT_GROWTH))
            })
    };
    assert!(growth_zone(&e), "rebound exiled it as it resolved");
}

/// A sorcery is not an instant, however it is cast.
#[test]
fn ojer_gives_a_sorcery_no_rebound() {
    let p0 = kit::P0;
    let mut e = kit::game(
        SEED,
        &[index::OJER_PAKPATIQ_DEEPEST_EPOCH],
        &[],
        &[index::COUNSEL_OF_THE_SORATAMI],
        &[],
    );
    kit::float(&mut e, p0, &[(ManaColor::Blue, 3)]);
    cast_with_floating(&mut e, p0, index::COUNSEL_OF_THE_SORATAMI);
    assert_eq!(stack_size(&e), 1, "no trigger for a sorcery");
    kit::settle(&mut e);
    assert!(
        in_graveyard(&e, p0, index::COUNSEL_OF_THE_SORATAMI).is_some(),
        "so it ends in the graveyard like any sorcery"
    );
}

/// "Whenever YOU cast": the opponent's instant, cast from the opponent's own
/// hand, is not yours.
#[test]
fn ojer_gives_an_opponents_instant_no_rebound() {
    let (p0, p1) = (kit::P0, kit::P1);
    let mut e = kit::game(
        SEED,
        &[index::OJER_PAKPATIQ_DEEPEST_EPOCH],
        &[index::GRIZZLY_BEARS],
        &[],
        &[index::GIANT_GROWTH],
    );
    let their_bears = kit::obj(&e, p1, index::GRIZZLY_BEARS);
    kit::pass_once(&mut e);
    assert!(
        matches!(e.pending(), Pending::Priority { player, .. } if *player == p1),
        "the opponent may act in our main phase: {:?}",
        e.pending()
    );
    kit::float(&mut e, p1, &[(ManaColor::Green, 1)]);
    cast_with_floating(&mut e, p1, index::GIANT_GROWTH);
    kit::aim(&mut e, &[their_bears], &[]);
    assert_eq!(stack_size(&e), 1, "Ojer watches p0's casts, not p1's");
    kit::settle(&mut e);
    assert!(
        in_graveyard(&e, p1, index::GIANT_GROWTH).is_some(),
        "the opponent's Growth goes to the opponent's graveyard"
    );
    assert!(e.state().zones.list(ZoneLocation::Exile(p1)).is_empty());
    assert!(e.state().zones.list(ZoneLocation::Exile(p0)).is_empty());
}

/// "from your HAND": an instant cast out of the graveyard (here by Kess) does
/// not trigger Ojer.
#[test]
fn ojer_does_not_trigger_for_an_instant_cast_from_the_graveyard() {
    let p0 = kit::P0;
    let mut e = kit::game(
        SEED,
        &[
            index::OJER_PAKPATIQ_DEEPEST_EPOCH,
            index::KESS_DISSIDENT_MAGE,
        ],
        &[],
        &[index::GIANT_GROWTH],
        &[],
    );
    let kess = kit::obj(&e, p0, index::KESS_DISSIDENT_MAGE);
    let growth = hand_to_graveyard(&mut e, p0, index::GIANT_GROWTH);
    kit::float(&mut e, p0, &[(ManaColor::Green, 1)]);
    e.apply(p0, PlayerAction::CastSpell { card: growth })
        .unwrap();
    kit::aim(&mut e, &[kess], &[]);
    assert_eq!(
        stack_size(&e),
        1,
        "cast from the graveyard, not from the hand: no rebound trigger"
    );
    kit::settle(&mut e);
}

/// Ojer as the land it returns as: three time counters and tapped, already
/// proven by the tests above and in `batch45`.
fn ojer_as_a_temple() -> (Engine<RegistryLookup>, ObjectId) {
    let p0 = kit::P0;
    let mut e = kit::game(
        SEED,
        &[index::OJER_PAKPATIQ_DEEPEST_EPOCH, index::GRIZZLY_BEARS],
        &[],
        &[index::GIANT_GROWTH],
        &[],
    );
    let ojer = kit::obj(&e, p0, index::OJER_PAKPATIQ_DEEPEST_EPOCH);
    kill(&mut e, ojer);
    assert_eq!(e.state().object(ojer).unwrap().face_index, 1);
    assert_eq!(counters_on(&e, ojer, CounterKind::Time), 3);
    assert!(kit::tapped(&e, ojer));
    (e, ojer)
}

fn untap(e: &mut Engine<RegistryLookup>, id: ObjectId) {
    e.dev_state_mut(kit::P0).unwrap().set_tapped(id, false);
    e.refresh_offer();
}

/// "{T}: Add {U}. Remove a time counter from this land.": exactly one blue
/// mana, one counter fewer (and none below zero), and the land taps.
#[test]
fn ojer_temple_taps_for_one_blue_and_removes_one_time_counter() {
    let p0 = kit::P0;
    let (mut e, ojer) = ojer_as_a_temple();
    for (taps, counters_left) in [(1_u32, 2_u16), (2, 1), (3, 0), (4, 0)] {
        untap(&mut e, ojer);
        activate(&mut e, p0, index::OJER_PAKPATIQ_DEEPEST_EPOCH, 0);
        assert_eq!(
            kit::pool_of(&e, p0, ManaColor::Blue),
            taps,
            "one {{U}} for each tap"
        );
        assert_eq!(
            kit::pool_total(&e, p0),
            u64::from(taps),
            "and nothing but {{U}}"
        );
        assert_eq!(
            counters_on(&e, ojer, CounterKind::Time),
            counters_left,
            "one counter off per tap, never below none"
        );
        assert!(kit::tapped(&e, ojer), "{{T}} is the cost");
    }
}

/// "{2}{U}, {T}: Transform this land. Activate only if it has no time
/// counters on it and only as a sorcery.": refused while any counter is left,
/// refused with a spell on the stack, allowed at none and at sorcery speed.
#[test]
fn ojer_temple_transforms_only_without_time_counters_and_at_sorcery_speed() {
    let p0 = kit::P0;
    let (mut e, ojer) = ojer_as_a_temple();
    let bears = kit::obj(&e, p0, index::GRIZZLY_BEARS);
    // {2}{U} floats from here on, so that no refusal below is a refusal of the cost.
    kit::float(
        &mut e,
        p0,
        &[(ManaColor::Blue, 1), (ManaColor::Colorless, 2)],
    );
    untap(&mut e, ojer);
    assert!(
        !kit::offered(&e, index::OJER_PAKPATIQ_DEEPEST_EPOCH, p0, 1),
        "three time counters are left"
    );
    for _ in 0..2 {
        activate(&mut e, p0, index::OJER_PAKPATIQ_DEEPEST_EPOCH, 0);
        untap(&mut e, ojer);
    }
    assert_eq!(counters_on(&e, ojer, CounterKind::Time), 1);
    assert!(
        !kit::offered(&e, index::OJER_PAKPATIQ_DEEPEST_EPOCH, p0, 1),
        "one time counter is still one too many"
    );
    activate(&mut e, p0, index::OJER_PAKPATIQ_DEEPEST_EPOCH, 0);
    untap(&mut e, ojer);
    assert_eq!(counters_on(&e, ojer, CounterKind::Time), 0);
    assert!(
        kit::offered(&e, index::OJER_PAKPATIQ_DEEPEST_EPOCH, p0, 1),
        "no counters, an empty stack in the main phase, {{2}}{{U}} and an untapped land"
    );

    kit::float(&mut e, p0, &[(ManaColor::Green, 1)]);
    cast_with_floating(&mut e, p0, index::GIANT_GROWTH);
    kit::aim(&mut e, &[bears], &[]);
    assert!(
        !kit::offered(&e, index::OJER_PAKPATIQ_DEEPEST_EPOCH, p0, 1),
        "\"only as a sorcery\": not with a spell on the stack"
    );
    kit::settle(&mut e);
    assert!(kit::offered(&e, index::OJER_PAKPATIQ_DEEPEST_EPOCH, p0, 1));
    kit::use_ability(&mut e, p0, index::OJER_PAKPATIQ_DEEPEST_EPOCH, 1, &[], &[]);
    assert_eq!(
        e.state().object(ojer).unwrap().face_index,
        0,
        "it transformed back to the God"
    );
    assert!(types(&e, ojer).contains(TypeSet::CREATURE));
}
