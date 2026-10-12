//! Actual combat and spell/ability damage for Johan and Bronze Horse.
use super::counter_resources::prepared;
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;
const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);
fn obj(e: &Engine<RegistryLookup>, p: PlayerId, c: CardIndex) -> ObjectId {
    on_battlefield(e, p, c).unwrap()
}
fn top_of_stack(e: &Engine<RegistryLookup>) -> ObjectId {
    *e.state.zones.list(ZoneLocation::Stack).last().unwrap()
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
    let player = e.pending().asked().unwrap();
    e.apply(
        player,
        PlayerAction::ChooseTargets {
            objects: vec![id],
            players: vec![],
        },
    )
    .unwrap();
}
fn answer(e: &mut Engine<RegistryLookup>, yes: bool) {
    let player = e.pending().asked().unwrap();
    e.apply(player, PlayerAction::YesNo(yes)).unwrap();
}
fn choice(e: &mut Engine<RegistryLookup>, yes: bool) {
    pass_until(e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    answer(e, yes);
    pass_until(e, stack_is_empty);
}
fn attackers(e: &mut Engine<RegistryLookup>) {
    pass_until(
        e,
        |e| matches!(e.pending(),Pending::ChooseAttackers{player,..}if *player==P0),
    );
}
fn attack(e: &mut Engine<RegistryLookup>, creature: ObjectId) {
    e.apply(
        P0,
        PlayerAction::DeclareAttackers {
            attackers: vec![(creature, Defender::Player(P1))],
        },
    )
    .unwrap();
}
#[test]
fn johan_accepts_optional_choice_and_only_untapped_source_prevents_attack_tap() {
    for tapped in [false, true] {
        let mut e = prepared(
            &[index::TWIDDLE],
            &[index::JOHAN, index::GRIZZLY_BEARS],
            &[],
        );
        let johan = obj(&e, P0, index::JOHAN);
        let bear = obj(&e, P0, index::GRIZZLY_BEARS);
        choice(&mut e, true);
        if tapped {
            cast(&mut e, index::TWIDDLE);
            target(&mut e, johan);
            choice(&mut e, true);
        }
        attackers(&mut e);
        let hash = e.snapshot_hash();
        assert!(
            e.apply(
                P0,
                PlayerAction::DeclareAttackers {
                    attackers: vec![(johan, Defender::Player(P1))]
                }
            )
            .is_err()
        );
        assert_eq!(e.snapshot_hash(), hash);
        attack(&mut e, bear);
        assert_eq!(is_tapped(&e, bear), tapped);
        assert!(
            !keywords(&e, bear).contains(KeywordSet::VIGILANCE),
            "attack tap exemption is a rule, not gained ability"
        );
    }
}
#[test]
fn johan_decline_leaves_it_able_to_attack_and_normal_attack_tap() {
    let mut e = prepared(&[], &[index::JOHAN, index::GRIZZLY_BEARS], &[]);
    let johan = obj(&e, P0, index::JOHAN);
    let bear = obj(&e, P0, index::GRIZZLY_BEARS);
    choice(&mut e, false);
    attackers(&mut e);
    e.apply(
        P0,
        PlayerAction::DeclareAttackers {
            attackers: vec![(johan, Defender::Player(P1)), (bear, Defender::Player(P1))],
        },
    )
    .unwrap();
    assert!(is_tapped(&e, johan) && is_tapped(&e, bear));
}
#[test]
fn johan_tapped_then_untapped_before_attack_reenables_dynamic_rule() {
    let mut e = prepared(
        &[index::TWIDDLE, index::TWIDDLE],
        &[index::JOHAN, index::GRIZZLY_BEARS],
        &[],
    );
    let johan = obj(&e, P0, index::JOHAN);
    let bear = obj(&e, P0, index::GRIZZLY_BEARS);
    choice(&mut e, true);
    for tapped in [true, false] {
        cast(&mut e, index::TWIDDLE);
        target(&mut e, johan);
        choice(&mut e, true);
        assert_eq!(is_tapped(&e, johan), tapped);
    }
    attackers(&mut e);
    attack(&mut e, bear);
    assert!(!is_tapped(&e, bear));
}
#[test]
fn johan_real_departure_before_attack_ends_source_condition() {
    let mut e = prepared(
        &[index::UNSUMMON],
        &[index::JOHAN, index::GRIZZLY_BEARS],
        &[],
    );
    let johan = obj(&e, P0, index::JOHAN);
    let bear = obj(&e, P0, index::GRIZZLY_BEARS);
    choice(&mut e, true);
    cast(&mut e, index::UNSUMMON);
    target(&mut e, johan);
    pass_until(&mut e, stack_is_empty);
    attackers(&mut e);
    attack(&mut e, bear);
    assert!(is_tapped(&e, bear));
}
#[test]
fn bronze_horse_prevents_targeting_spell_damage_only_with_another_creature() {
    for companion in [false, true] {
        let mine: &[CardIndex] = if companion {
            &[index::BRONZE_HORSE, index::GRIZZLY_BEARS]
        } else {
            &[index::BRONZE_HORSE]
        };
        let mut e = prepared(&[index::LIGHTNING_BOLT], mine, &[]);
        let horse = obj(&e, P0, index::BRONZE_HORSE);
        cast(&mut e, index::LIGHTNING_BOLT);
        target(&mut e, horse);
        pass_until(&mut e, stack_is_empty);
        assert_eq!(
            e.state.object(horse).unwrap().damage,
            if companion { 0 } else { 3 }
        );
    }
}
#[test]
fn bronze_horse_targeted_activated_ability_damage_is_not_a_spell() {
    let mut e = prepared(
        &[],
        &[
            index::BRONZE_HORSE,
            index::GRIZZLY_BEARS,
            index::PRODIGAL_SORCERER,
        ],
        &[],
    );
    let horse = obj(&e, P0, index::BRONZE_HORSE);
    activate(&mut e, P0, index::PRODIGAL_SORCERER, 0);
    target(&mut e, horse);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.object(horse).unwrap().damage, 1);
}
#[test]
fn bronze_horse_untargeted_spell_damage_is_not_prevented() {
    let mut e = prepared(
        &[index::SANDSTORM],
        &[index::BRONZE_HORSE, index::GRIZZLY_BEARS],
        &[],
    );
    let horse = obj(&e, P0, index::BRONZE_HORSE);
    attackers(&mut e);
    attack(&mut e, horse);
    cast(&mut e, index::SANDSTORM);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.object(horse).unwrap().damage, 1);
}
#[test]
fn bronze_horse_prevention_reevaluates_companion_at_damage_time() {
    let mut e = prepared(
        &[index::LIGHTNING_BOLT, index::UNSUMMON],
        &[index::BRONZE_HORSE, index::GRIZZLY_BEARS],
        &[],
    );
    let horse = obj(&e, P0, index::BRONZE_HORSE);
    let bear = obj(&e, P0, index::GRIZZLY_BEARS);
    cast(&mut e, index::LIGHTNING_BOLT);
    target(&mut e, horse);
    cast(&mut e, index::UNSUMMON);
    target(&mut e, bear);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.object(horse).unwrap().damage, 3);
}
#[test]
fn bronze_horse_copied_spell_uses_new_selected_targets() {
    for original_horse in [false, true] {
        let mut e = prepared(
            &[index::LIGHTNING_BOLT, index::FORK],
            &[index::BRONZE_HORSE, index::CARNAGE_TYRANT],
            &[],
        );
        let horse = obj(&e, P0, index::BRONZE_HORSE);
        let companion = obj(&e, P0, index::CARNAGE_TYRANT);
        let original_target = if original_horse { horse } else { companion };
        let new_target = if original_horse { companion } else { horse };
        cast(&mut e, index::LIGHTNING_BOLT);
        target(&mut e, original_target);
        let bolt = top_of_stack(&e);
        cast(&mut e, index::FORK);
        target(&mut e, bolt);
        pass_until(&mut e, |e| {
            matches!(e.pending(), Pending::ChooseTargets { .. })
        });
        let copy = top_of_stack(&e);
        assert_ne!(copy, bolt);
        target(&mut e, new_target);
        // Check the live spell target set after the actual retarget action; merely
        // retaining the original's target would silently protect the wrong source.
        assert_eq!(
            crate::eval::matches(
                &baylee_cards_dsl::Filter::SpellTargetingAffected,
                e.state(),
                e.state.object(copy).unwrap(),
                P0,
                horse
            ),
            !original_horse
        );
        pass_until(&mut e, stack_is_empty);
        assert_eq!(e.state.object(horse).unwrap().damage, 0);
        assert_eq!(e.state.object(companion).unwrap().damage, 3);
    }
}

#[test]
fn johan_rule_applies_to_a_later_real_flash_creature_without_granting_abilities() {
    let mut e = prepared(&[index::SCRYB_RANGER], &[index::JOHAN], &[]);
    choice(&mut e, true);
    cast(&mut e, index::SCRYB_RANGER);
    pass_until(&mut e, stack_is_empty);
    let ranger = obj(&e, P0, index::SCRYB_RANGER);
    // Only haste is a fixture, permitting the real late entrant to attack this turn.
    e.state.object_mut(ranger).unwrap().base_mut().keywords = KeywordSet::HASTE;
    e.state.invalidate_projections();
    e.refresh_offer();
    attackers(&mut e);
    attack(&mut e, ranger);
    assert!(!is_tapped(&e, ranger));
    assert!(!keywords(&e, ranger).contains(KeywordSet::VIGILANCE));
}

#[test]
fn johan_blink_while_trigger_waits_cannot_bind_its_new_incarnation() {
    let mut e = prepared(
        &[index::EPHEMERATE],
        &[index::JOHAN, index::GRIZZLY_BEARS],
        &[],
    );
    let johan = obj(&e, P0, index::JOHAN);
    let bear = obj(&e, P0, index::GRIZZLY_BEARS);
    let before = e.state.object(johan).unwrap().version;
    pass_until(&mut e, |e| {
        !stack_is_empty(e) && matches!(e.pending(), Pending::Priority { .. })
    });
    cast(&mut e, index::EPHEMERATE);
    target(&mut e, johan);
    pass_until(&mut e, |e| {
        e.state.zones.list(ZoneLocation::Stack).len() == 1
            && matches!(e.pending(), Pending::Priority { .. })
    });
    assert_ne!(e.state.object(johan).unwrap().version, before);
    choice(&mut e, true);
    assert!(!keywords(&e, johan).contains(KeywordSet::CANT_ATTACK));
    attackers(&mut e);
    attack(&mut e, bear);
    assert!(is_tapped(&e, bear));
}

#[test]
fn johan_rule_is_in_both_hashes_but_stale_source_cannot_change_the_loop_signature() {
    let mut e = prepared(&[], &[index::JOHAN, index::GRIZZLY_BEARS], &[]);
    choice(&mut e, true);
    let baseline = e.state.clone();
    e.state.effects.remove_where(|fx| {
        matches!(
            fx.modifier,
            baylee_cards_dsl::Modifier::AttackWithoutTappingWhileSourceUntapped { .. }
        )
    });
    assert_ne!(baseline.snapshot_hash(), e.state.snapshot_hash());
    assert_ne!(baseline.loop_signature(), e.state.loop_signature());
    let mut stale = baseline.clone();
    let johan = obj(&e, P0, index::JOHAN);
    stale.object_mut(johan).unwrap().version += 1;
    let with_stale = stale.loop_signature();
    stale.effects.remove_where(|fx| {
        matches!(
            fx.modifier,
            baylee_cards_dsl::Modifier::AttackWithoutTappingWhileSourceUntapped { .. }
        )
    });
    assert_eq!(with_stale, stale.loop_signature());
}
