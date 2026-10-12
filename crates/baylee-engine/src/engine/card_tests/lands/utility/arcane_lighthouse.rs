//! `cards/lands/utility/arcane_lighthouse.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Arcane Lighthouse: "{T}: Add {C}." / "{1}, {T}: Until end of turn, creatures your opponents control lose hexproof and shroud and can't have hexproof or shroud."
/// Activating ability 1 removes hexproof from an opponent's creature upon resolution.
#[test]
fn arcane_lighthouse_removes_hexproof_from_opponent_creatures() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(127, forest())
        .battlefield(0, &[arcane_lighthouse(), forest()])
        .battlefield(1, &[carnage_tyrant()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let tyrant = on_battlefield(&engine, p1, carnage_tyrant()).expect("Carnage Tyrant deployed");
    assert!(keywords(&engine, tyrant).contains(KeywordSet::HEXPROOF));

    let lighthouse = on_battlefield(&engine, p0, arcane_lighthouse()).unwrap();
    tap_mana_except(&mut engine, p0, lighthouse);
    activate(&mut engine, p0, arcane_lighthouse(), 1);
    pass_until(&mut engine, stack_is_empty);

    assert!(!keywords(&engine, tyrant).contains(KeywordSet::HEXPROOF));
    assert!(is_tapped(&engine, lighthouse));
}

use baylee_core::generated::index;
const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);
fn q_prepared(
    hand: &[CardIndex],
    mine: &[CardIndex],
    theirs: &[CardIndex],
) -> Engine<RegistryLookup> {
    let mut e = Duel::new(SEED, index::FOREST)
        .hand(0, hand)
        .battlefield(0, mine)
        .battlefield(1, theirs)
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    q_mana(&mut e);
    e
}
fn q_mana(e: &mut Engine<RegistryLookup>) {
    for p in &mut e.state.players {
        for c in ManaColor::ALL {
            p.mana_pool.add(c, 30);
        }
    }
    e.refresh_offer();
}
fn q_obj(e: &Engine<RegistryLookup>, p: PlayerId, c: CardIndex) -> ObjectId {
    on_battlefield(e, p, c).unwrap()
}
fn q_priority(e: &mut Engine<RegistryLookup>, p: PlayerId) {
    pass_until(
        e,
        |e| matches!(e.pending(),Pending::Priority{player,..} if *player==p),
    );
}
fn q_cast(e: &mut Engine<RegistryLookup>, p: PlayerId, c: CardIndex) {
    q_priority(e, p);
    q_mana(e);
    let card = in_hand(e, p, c).unwrap();
    e.apply(p, PlayerAction::CastSpell { card }).unwrap();
}
fn q_targets(e: &mut Engine<RegistryLookup>, objects: Vec<ObjectId>, players: Vec<PlayerId>) {
    let p = e.pending().asked().unwrap();
    e.apply(p, PlayerAction::ChooseTargets { objects, players })
        .unwrap();
}

fn q_activate(e: &mut Engine<RegistryLookup>, c: CardIndex, i: u32) {
    q_priority(e, P0);
    q_mana(e);
    let source = q_obj(e, P0, c);
    e.apply(
        P0,
        PlayerAction::ActivateAbility {
            source,
            ability_index: i,
        },
    )
    .unwrap();
}
fn q_next_main(e: &mut Engine<RegistryLookup>, p: PlayerId) {
    let turn = e.state.turn.number;
    pass_until(e, |e| {
        e.state.turn.number > turn
            && e.state.turn.active == p
            && e.state.turn.phase == crate::turn::Phase::FirstMain
            && matches!(e.pending(),Pending::Priority{player,..} if *player==p)
    });
}

#[test]
fn lighthouse_removes_and_prevents_later_shroud_grants() {
    let mut e = q_prepared(
        &[],
        &[index::ARCANE_LIGHTHOUSE],
        &[index::GRIZZLY_BEARS, index::LIGHTNING_GREAVES],
    );
    let bear = q_obj(&e, P1, index::GRIZZLY_BEARS);
    q_next_main(&mut e, P1);
    q_activate(&mut e, index::ARCANE_LIGHTHOUSE, 1);
    pass_until(&mut e, stack_is_empty);
    q_priority(&mut e, P1);
    let greaves = q_obj(&e, P1, index::LIGHTNING_GREAVES);
    e.apply(
        P1,
        PlayerAction::ActivateAbility {
            source: greaves,
            ability_index: 1,
        },
    )
    .unwrap();
    q_targets(&mut e, vec![bear], vec![]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.object(greaves).unwrap().attached_to, Some(bear));
    assert!(
        !keywords(&e, bear).contains(KeywordSet::SHROUD),
        "later timestamp cannot regrant forbidden keyword"
    );
    assert!(keywords(&e, bear).contains(KeywordSet::HASTE));
    q_next_main(&mut e, P0);
    assert!(
        keywords(&e, bear).contains(KeywordSet::SHROUD),
        "prohibition expires at cleanup"
    );
}

#[test]
fn lighthouse_does_not_affect_creatures_entering_after_resolution() {
    let mut e = Duel::new(SEED, index::FOREST)
        .battlefield(0, &[index::ARCANE_LIGHTHOUSE])
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .hand(1, &[index::CARNAGE_TYRANT])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    q_next_main(&mut e, P1);
    q_activate(&mut e, index::ARCANE_LIGHTHOUSE, 1);
    pass_until(&mut e, stack_is_empty);
    q_cast(&mut e, P1, index::CARNAGE_TYRANT);
    pass_until(&mut e, stack_is_empty);
    let late = q_obj(&e, P1, index::CARNAGE_TYRANT);
    assert!(keywords(&e, late).contains(KeywordSet::HEXPROOF));
}

// Append to lands/utility/arcane_lighthouse.rs (existing q_* helpers).
#[test]
fn lighthouse_resolved_cohort_remains_forbidden_after_real_control_change() {
    let mut e = q_prepared(
        &[index::CONTROL_MAGIC],
        &[index::ARCANE_LIGHTHOUSE],
        &[index::CARNAGE_TYRANT],
    );
    let tyrant = q_obj(&e, P1, index::CARNAGE_TYRANT);
    q_activate(&mut e, index::ARCANE_LIGHTHOUSE, 1);
    pass_until(&mut e, stack_is_empty);
    q_cast(&mut e, P0, index::CONTROL_MAGIC);
    q_targets(&mut e, vec![tyrant], vec![]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.object(tyrant).unwrap().controller, P0);
    assert!(
        !keywords(&e, tyrant).contains(KeywordSet::HEXPROOF),
        "cohort binds incarnation, not current opponent control"
    );
    assert!(keywords(&e, tyrant).contains(KeywordSet::TRAMPLE));
    q_next_main(&mut e, P1);
    assert!(
        keywords(&e, tyrant).contains(KeywordSet::HEXPROOF),
        "normal expiry restores printed keyword under new controller"
    );
}
#[test]
fn lighthouse_resolved_cohort_survives_noncreature_interval() {
    let mut e = q_prepared(&[], &[index::ARCANE_LIGHTHOUSE], &[index::CARNAGE_TYRANT]);
    let tyrant = q_obj(&e, P1, index::CARNAGE_TYRANT);
    q_activate(&mut e, index::ARCANE_LIGHTHOUSE, 1);
    pass_until(&mut e, stack_is_empty);
    // Transparent type fixtures isolate cohort persistence. No Implemented
    // helper reproduces both transformations without unrelated removal rules.
    let original = e.state.object(tyrant).unwrap().characteristics().types;
    e.state.object_mut(tyrant).unwrap().base_mut().types = TypeSet::LAND;
    e.state.invalidate_projections();
    e.sync_static_effects();
    e.refresh_offer();
    assert!(!keywords(&e, tyrant).contains(KeywordSet::HEXPROOF));
    e.state.object_mut(tyrant).unwrap().base_mut().types = original;
    e.state.invalidate_projections();
    e.sync_static_effects();
    e.refresh_offer();
    assert!(
        !keywords(&e, tyrant).contains(KeywordSet::HEXPROOF),
        "becoming a creature again cannot escape same-incarnation cohort"
    );
    q_next_main(&mut e, P1);
    assert!(keywords(&e, tyrant).contains(KeywordSet::HEXPROOF));
}
