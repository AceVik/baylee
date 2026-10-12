//! Guardian Beast: new attachment permissions, dynamic cohort and control.
use super::counter_resources::prepared;
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;
const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);
fn obj(e: &Engine<RegistryLookup>, p: PlayerId, c: CardIndex) -> ObjectId {
    on_battlefield(e, p, c).unwrap()
}
fn mana(e: &mut Engine<RegistryLookup>) {
    for p in &mut e.state.players {
        for c in ManaColor::ALL {
            p.mana_pool.add(c, 30);
        }
    }
    e.refresh_offer();
}
fn priority(e: &mut Engine<RegistryLookup>, player: PlayerId) {
    pass_until(
        e,
        |e| matches!(e.pending(),Pending::Priority{player:p,..}if *p==player),
    );
}
fn cast(e: &mut Engine<RegistryLookup>, c: CardIndex) {
    priority(e, P0);
    mana(e);
    let card = in_hand(e, P0, c).unwrap();
    e.apply(P0, PlayerAction::CastSpell { card }).unwrap();
}
fn target(e: &mut Engine<RegistryLookup>, id: ObjectId) {
    let p = e.pending().asked().unwrap();
    e.apply(
        p,
        PlayerAction::ChooseTargets {
            objects: vec![id],
            players: vec![],
        },
    )
    .unwrap();
}
fn cast_target(e: &mut Engine<RegistryLookup>, c: CardIndex, id: ObjectId) {
    cast(e, c);
    target(e, id);
    pass_until(e, stack_is_empty);
}
fn next_main(e: &mut Engine<RegistryLookup>, player: PlayerId) {
    let turn = e.state.turn.number;
    pass_until(e, |e| {
        e.state.turn.number > turn
            && e.state.turn.active == player
            && e.state.turn.phase == crate::turn::Phase::FirstMain
            && matches!(e.pending(),Pending::Priority{player:p,..}if *p==player)
    });
}
fn tap_with_twiddle(e: &mut Engine<RegistryLookup>, id: ObjectId) {
    cast(e, index::TWIDDLE);
    target(e, id);
    pass_until(e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    e.apply(P0, PlayerAction::YesNo(true)).unwrap();
    pass_until(e, stack_is_empty);
}
#[test]
fn guardian_keeps_old_aura_but_refuses_new_aura_then_tapped_beast_allows_destruction() {
    let mut e = prepared(
        &[
            index::POWER_ARTIFACT,
            index::POWER_ARTIFACT,
            index::GUARDIAN_BEAST,
            index::SHATTER,
            index::SHATTER,
            index::TWIDDLE,
        ],
        &[index::SOL_RING],
        &[index::SOL_RING],
    );
    let ring = obj(&e, P0, index::SOL_RING);
    let other = obj(&e, P1, index::SOL_RING);
    cast_target(&mut e, index::POWER_ARTIFACT, ring);
    let aura = obj(&e, P0, index::POWER_ARTIFACT);
    cast(&mut e, index::GUARDIAN_BEAST);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.object(aura).unwrap().attached_to, Some(ring));
    assert!(keywords(&e, ring).contains(KeywordSet::INDESTRUCTIBLE));
    cast(&mut e, index::POWER_ARTIFACT);
    let Pending::ChooseTargets { options, .. } = e.pending() else {
        panic!("{:?}", e.pending())
    };
    assert!(!options.contains(&ring));
    assert!(options.contains(&other));
    target(&mut e, other);
    pass_until(&mut e, stack_is_empty);
    cast_target(&mut e, index::SHATTER, ring);
    assert_eq!(
        e.state.object(ring).unwrap().zone,
        crate::zone::Zone::Battlefield
    );
    let beast = obj(&e, P0, index::GUARDIAN_BEAST);
    tap_with_twiddle(&mut e, beast);
    cast_target(&mut e, index::SHATTER, ring);
    assert_eq!(
        e.state.object(ring).unwrap().zone,
        crate::zone::Zone::Graveyard
    );
}

#[test]
fn guardian_only_stops_other_players_control_gain_of_untapped_noncreature_cohort() {
    for tapped in [false, true] {
        for creature in [false, true] {
            let card = if creature {
                index::ORNITHOPTER
            } else {
                index::SOL_RING
            };
            let mut e = prepared(
                &[index::TWIDDLE],
                &[index::GUARDIAN_BEAST, card],
                &[index::ALADDIN],
            );
            let protected = obj(&e, P0, card);
            let beast = obj(&e, P0, index::GUARDIAN_BEAST);
            if tapped {
                tap_with_twiddle(&mut e, beast);
            }
            let aladdin = obj(&e, P1, index::ALADDIN);
            priority(&mut e, P1);
            mana(&mut e);
            e.apply(
                P1,
                PlayerAction::ActivateAbility {
                    source: aladdin,
                    ability_index: 0,
                },
            )
            .unwrap();
            target(&mut e, protected);
            pass_until(&mut e, stack_is_empty);
            assert_eq!(
                e.state.object(protected).unwrap().controller,
                if tapped || creature { P1 } else { P0 }
            );
        }
    }
}

#[test]
fn guardian_exchange_with_real_oko_fails_atomically_if_other_player_would_gain_protected_artifact()
{
    let mut e = prepared(
        &[],
        &[
            index::GUARDIAN_BEAST,
            index::SOL_RING,
            index::OKO_THIEF_OF_CROWNS,
        ],
        &[index::GRIZZLY_BEARS],
    );
    let ring = obj(&e, P0, index::SOL_RING);
    let bear = obj(&e, P1, index::GRIZZLY_BEARS);
    activate(&mut e, P0, index::OKO_THIEF_OF_CROWNS, 0);
    pass_until(&mut e, stack_is_empty);
    next_main(&mut e, P0);
    activate(&mut e, P0, index::OKO_THIEF_OF_CROWNS, 2);
    target(&mut e, ring);
    target(&mut e, bear);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.object(ring).unwrap().controller, P0);
    assert_eq!(
        e.state.object(bear).unwrap().controller,
        P1,
        "one forbidden gain makes entire exchange fail"
    );
}

#[test]
fn guardian_noncreature_protection_returns_after_real_animation_aura_leaves() {
    let mut e = prepared(
        &[
            index::ANIMATE_ARTIFACT,
            index::GUARDIAN_BEAST,
            index::BOOMERANG,
            index::SHATTER,
        ],
        &[index::SOL_RING],
        &[],
    );
    let ring = obj(&e, P0, index::SOL_RING);
    cast_target(&mut e, index::ANIMATE_ARTIFACT, ring);
    cast(&mut e, index::GUARDIAN_BEAST);
    pass_until(&mut e, stack_is_empty);
    assert!(
        e.state
            .object(ring)
            .unwrap()
            .characteristics()
            .types
            .contains(TypeSet::CREATURE)
    );
    assert!(!keywords(&e, ring).contains(KeywordSet::INDESTRUCTIBLE));
    let animation = obj(&e, P0, index::ANIMATE_ARTIFACT);
    cast_target(&mut e, index::BOOMERANG, animation);
    assert!(
        !e.state
            .object(ring)
            .unwrap()
            .characteristics()
            .types
            .contains(TypeSet::CREATURE)
    );
    assert!(keywords(&e, ring).contains(KeywordSet::INDESTRUCTIBLE));
    cast_target(&mut e, index::SHATTER, ring);
    assert_eq!(
        e.state.object(ring).unwrap().zone,
        crate::zone::Zone::Battlefield
    );
}

#[test]
fn guardian_does_not_block_control_magic_of_artifact_creature() {
    let mut e = Duel::new(SEED, index::FOREST)
        .battlefield(0, &[index::GUARDIAN_BEAST, index::ORNITHOPTER])
        .hand(1, &[index::CONTROL_MAGIC])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    next_main(&mut e, P1);
    let thopter = obj(&e, P0, index::ORNITHOPTER);
    mana(&mut e);
    let magic = in_hand(&e, P1, index::CONTROL_MAGIC).unwrap();
    e.apply(P1, PlayerAction::CastSpell { card: magic })
        .unwrap();
    target(&mut e, thopter);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        e.state.object(thopter).unwrap().controller,
        P1,
        "artifact creatures are outside Guardian's dynamic cohort"
    );
}

mod reattachment_fixture;

#[test]
fn guardian_actual_entry_does_not_reverse_an_existing_control_gain() {
    let mut e = prepared(
        &[index::GUARDIAN_BEAST],
        &[index::ALADDIN],
        &[index::SOL_RING],
    );
    let ring = obj(&e, P1, index::SOL_RING);
    priority(&mut e, P0);
    mana(&mut e);
    activate(&mut e, P0, index::ALADDIN, 0);
    target(&mut e, ring);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.object(ring).unwrap().controller, P0);
    cast(&mut e, index::GUARDIAN_BEAST);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        e.state.object(ring).unwrap().controller,
        P0,
        "can't gain control is not a retroactive reversal"
    );
    assert!(keywords(&e, ring).contains(KeywordSet::INDESTRUCTIBLE));
    let hash = e.snapshot_hash();
    e.sync_static_effects();
    e.refresh_offer();
    assert_eq!(e.snapshot_hash(), hash);
}

#[test]
fn guardian_source_control_change_rebinds_artifact_cohort_without_reversing_old_effects() {
    let mut e = prepared(
        &[index::CONTROL_MAGIC, index::SHATTER],
        &[index::SOL_RING],
        &[index::GUARDIAN_BEAST, index::SOL_RING],
    );
    let mine = obj(&e, P0, index::SOL_RING);
    let theirs = obj(&e, P1, index::SOL_RING);
    let beast = obj(&e, P1, index::GUARDIAN_BEAST);
    assert!(!keywords(&e, mine).contains(KeywordSet::INDESTRUCTIBLE));
    assert!(keywords(&e, theirs).contains(KeywordSet::INDESTRUCTIBLE));
    cast_target(&mut e, index::CONTROL_MAGIC, beast);
    assert_eq!(e.state.object(beast).unwrap().controller, P0);
    assert!(keywords(&e, mine).contains(KeywordSet::INDESTRUCTIBLE));
    assert!(!keywords(&e, theirs).contains(KeywordSet::INDESTRUCTIBLE));
    cast_target(&mut e, index::SHATTER, theirs);
    assert_eq!(
        e.state.object(theirs).unwrap().zone,
        crate::zone::Zone::Graveyard
    );
    assert_eq!(
        e.state.object(mine).unwrap().zone,
        crate::zone::Zone::Battlefield
    );
}

#[test]
fn guardian_blocks_untargeted_aura_entry_from_actual_sun_titan_reanimation() {
    let mut e = prepared(
        &[index::POWER_ARTIFACT, index::SUN_TITAN],
        &[index::GUARDIAN_BEAST, index::SOL_RING],
        &[],
    );
    let aura = in_hand(&e, P0, index::POWER_ARTIFACT).unwrap();
    let ring = obj(&e, P0, index::SOL_RING);
    // Narrow starting-graveyard fixture only. Sun Titan's actual trigger targets
    // the Aura card, never the protected artifact it would enter enchanting.
    e.state
        .move_object(
            aura,
            ZoneLocation::Graveyard(P0),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    e.refresh_offer();
    let version = e.state.object(aura).unwrap().version;
    cast(&mut e, index::SUN_TITAN);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    target(&mut e, aura);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        e.state.object(aura).unwrap().zone,
        crate::zone::Zone::Graveyard,
        "no legal enchant host means Aura stays in its old zone"
    );
    assert_eq!(
        e.state.object(aura).unwrap().version,
        version,
        "Aura must never leave graveyard, not enter illegally then die to SBA"
    );
    assert!(!e.state.per_turn.entered_battlefield.contains(&aura));
    assert_eq!(e.state.object(aura).unwrap().attached_to, None);
    assert!(keywords(&e, ring).contains(KeywordSet::INDESTRUCTIBLE));
}

#[test]
fn guardian_failed_indefinite_aladdin_gain_does_not_wake_when_beast_taps() {
    let mut e = prepared(
        &[index::TWIDDLE],
        &[index::GUARDIAN_BEAST, index::SOL_RING],
        &[index::ALADDIN],
    );
    let ring = obj(&e, P0, index::SOL_RING);
    let beast = obj(&e, P0, index::GUARDIAN_BEAST);
    let aladdin = obj(&e, P1, index::ALADDIN);
    priority(&mut e, P1);
    mana(&mut e);
    e.apply(
        P1,
        PlayerAction::ActivateAbility {
            source: aladdin,
            ability_index: 0,
        },
    )
    .unwrap();
    target(&mut e, ring);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.object(ring).unwrap().controller, P0);
    tap_with_twiddle(&mut e, beast);
    assert_eq!(
        e.state.object(ring).unwrap().controller,
        P0,
        "failed resolving gain instruction must not leave a dormant indefinite effect"
    );
    e.sync_static_effects();
    e.refresh_offer();
    assert_eq!(e.state.object(ring).unwrap().controller, P0);
}

#[test]
fn guardian_temporary_control_expiry_preserves_controller_only_while_protection_applies() {
    let mut e = prepared(
        &[index::GUARDIAN_BEAST, index::TWIDDLE],
        &[index::ALADDIN],
        &[index::SOL_RING],
    );
    let ring = obj(&e, P1, index::SOL_RING);
    let source = obj(&e, P0, index::ALADDIN);
    // Narrow spell-effect fixture: artifact-targeting temporary-control helper is
    // absent. Register only a normal GainControl/UntilEndOfTurn effect, not the
    // Guardian protection or a controller-holding implementation under test.
    let timestamp = e.state.next_timestamp();
    let filter = crate::effects::EffectFilter::object(&e.state, ring);
    e.state.effects.register(crate::effects::ContinuousEffect {
        id: baylee_core::ids::EffectId::new(0),
        source: Some(source),
        controller: P0,
        origin: crate::effects::EffectOrigin::Resolution,
        layer: baylee_cards_dsl::Modifier::GainControl.layer(),
        timestamp,
        duration: baylee_cards_dsl::Duration::UntilEndOfTurn,
        filter,
        modifier: baylee_cards_dsl::Modifier::GainControl,
    });
    e.state.invalidate_projections();
    e.sync_static_effects();
    e.refresh_offer();
    assert_eq!(e.state.object(ring).unwrap().controller, P0);
    cast(&mut e, index::GUARDIAN_BEAST);
    pass_until(&mut e, stack_is_empty);
    next_main(&mut e, P1);
    assert_eq!(
        e.state.object(ring).unwrap().controller,
        P0,
        "expired effect does not authorize forbidden gain by another player"
    );
    assert!(keywords(&e, ring).contains(KeywordSet::INDESTRUCTIBLE));
    let hash = e.snapshot_hash();
    e.sync_static_effects();
    e.refresh_offer();
    assert_eq!(e.snapshot_hash(), hash);
    let beast = obj(&e, P0, index::GUARDIAN_BEAST);
    tap_with_twiddle(&mut e, beast);
    assert_eq!(
        e.state.object(ring).unwrap().controller,
        P1,
        "ending protection reveals the already-expired control effect; it was not extended"
    );
}

#[test]
fn guardian_sun_titan_untargeted_aura_entry_chooses_only_unprotected_legal_host() {
    let mut e = prepared(
        &[index::POWER_ARTIFACT, index::SUN_TITAN],
        &[index::GUARDIAN_BEAST, index::SOL_RING],
        &[index::SOL_RING],
    );
    let aura = in_hand(&e, P0, index::POWER_ARTIFACT).unwrap();
    let protected = obj(&e, P0, index::SOL_RING);
    let legal = obj(&e, P1, index::SOL_RING);
    // Starting graveyard fixture; actual Sun Titan performs return instruction.
    e.state
        .move_object(
            aura,
            ZoneLocation::Graveyard(P0),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    e.refresh_offer();
    let version = e.state.object(aura).unwrap().version;
    cast(&mut e, index::SUN_TITAN);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    target(&mut e, aura);
    // Untargeted host choice uses the ordinary object-choice wire shape.
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, .. } = e.pending() else {
        unreachable!()
    };
    assert!(!options.contains(&protected));
    assert!(options.contains(&legal));
    assert_eq!(
        e.state.object(aura).unwrap().zone,
        crate::zone::Zone::Graveyard,
        "host choice precedes entry"
    );
    assert_eq!(e.state.object(aura).unwrap().version, version);
    e.apply(
        P0,
        PlayerAction::ChooseObjects {
            objects: vec![legal],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        e.state.object(aura).unwrap().zone,
        crate::zone::Zone::Battlefield
    );
    assert_eq!(e.state.object(aura).unwrap().attached_to, Some(legal));
    assert_ne!(e.state.object(aura).unwrap().version, version);
}

mod phasing;
