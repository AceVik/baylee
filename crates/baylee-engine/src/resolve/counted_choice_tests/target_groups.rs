//! Independent target instances and bounded life gain, without printed cards.
use super::*;
use baylee_cards_dsl::{Amount, Filter, PlayerRel, TargetSlot, ZoneSel};
use baylee_core::types::TypeSet;

const OTHER: PlayerId = PlayerId::new(1);

fn object(state: &mut GameState, owner: PlayerId, zone: ZoneLocation, types: TypeSet) -> ObjectId {
    let name = state.names.intern("Synthetic subject");
    let kind = match zone {
        ZoneLocation::Battlefield => ObjectKind::Permanent,
        ZoneLocation::Stack => ObjectKind::Spell,
        _ => ObjectKind::Card,
    };
    let id = state.create_bare(owner, kind, name, zone);
    let base = state.object_mut(id).unwrap().base_mut();
    base.types = types;
    base.power = Some(3);
    base.toughness = Some(3);
    id
}

#[test]
fn second_player_does_not_alias_first_player_or_chosen_player() {
    let (mut state, _) = library_of(0);
    let mut res = mine(Effect::GainLifeFor {
        amount: Amount::Fixed(3),
        who: PlayerRel::SecondTargetPlayerOrController,
    });
    res.target_players.insert(me());
    res.chosen_player = Some(me());
    let mut second = baylee_core::ids::SeatSet::new();
    second.insert(OTHER);
    res.subject = SubjectContext::default().with_second_players(second);
    assert!(matches!(run(&mut state, &mut res), Flow::Complete));
    assert_eq!((state.players[0].life, state.players[1].life), (20, 23));
    let mut missing = mine(Effect::GainLifeFor {
        amount: Amount::Fixed(7),
        who: PlayerRel::SecondTargetPlayerOrController,
    });
    missing.target_players.insert(me());
    missing.chosen_player = Some(OTHER);
    assert!(matches!(run(&mut state, &mut missing), Flow::Complete));
    assert_eq!(
        (state.players[0].life, state.players[1].life),
        (20, 23),
        "missing second target must not borrow another player selection"
    );
}

#[test]
fn second_target_recipient_retains_stolen_controller_after_destruction() {
    let (mut state, _) = library_of(0);
    let first = object(
        &mut state,
        OTHER,
        ZoneLocation::Battlefield,
        TypeSet::CREATURE,
    );
    let second = object(
        &mut state,
        OTHER,
        ZoneLocation::Battlefield,
        TypeSet::CREATURE,
    );
    gain_control(&mut state, &[(second, me())]);
    let mut res = mine(Effect::DestroyTargetSlots {
        targets: &[TargetSlot::Second],
        no_regen: true,
    });
    res.targets.push(first);
    res.second_targets.push(second);
    res.chosen_player = Some(OTHER);
    res.effects.push(Effect::GainLifeFor {
        amount: Amount::Fixed(4),
        who: PlayerRel::SecondTargetPlayerOrController,
    });
    assert!(matches!(run(&mut state, &mut res), Flow::Complete));
    assert_eq!(
        state.object(first).unwrap().zone,
        crate::zone::Zone::Battlefield
    );
    assert_eq!(
        state.object(second).unwrap().zone,
        crate::zone::Zone::Graveyard
    );
    assert_eq!(
        (state.players[0].life, state.players[1].life),
        (24, 20),
        "the former controller benefits, not the owner or first target's controller"
    );
}

#[test]
fn distinct_target_groups_destroy_together_and_duplicate_groups_destroy_once() {
    use baylee_cards_dsl::{AbilityDef, Trigger, triggered};
    static WATCH: &[AbilityDef] = &[triggered!(
        Trigger::Dies(&Filter::And(&[
            Filter::CREATURE,
            Filter::Not(&Filter::This)
        ])),
        &[Effect::GainLife {
            amount: Amount::Fixed(1)
        }]
    )];
    let (mut state, _) = library_of(0);
    let watcher = object(
        &mut state,
        me(),
        ZoneLocation::Battlefield,
        TypeSet::CREATURE,
    );
    state
        .object_mut(watcher)
        .unwrap()
        .take_abilities(crate::object::AbilityList::from_static(WATCH, None, None));
    let victim = object(
        &mut state,
        OTHER,
        ZoneLocation::Battlefield,
        TypeSet::CREATURE,
    );
    let since = state.journal.last_seq();
    let mut res = mine(Effect::DestroyTargetSlots {
        targets: &[TargetSlot::First, TargetSlot::Second],
        no_regen: false,
    });
    res.targets.push(watcher);
    res.second_targets.push(victim);
    assert!(matches!(run(&mut state, &mut res), Flow::Complete));
    let triggers = crate::trigger::collect(&state, &SyntheticLookup::new(vec![]), since);
    assert_eq!(
        triggers
            .iter()
            .filter(|trigger| trigger.source == watcher)
            .count(),
        1,
        "the departing watcher sees the other simultaneous death"
    );
    let repeated = object(
        &mut state,
        me(),
        ZoneLocation::Battlefield,
        TypeSet::CREATURE,
    );
    state.object_mut(repeated).unwrap().regeneration_shields = 1;
    let mut res = mine(Effect::DestroyTargetSlots {
        targets: &[TargetSlot::First, TargetSlot::Second],
        no_regen: false,
    });
    res.targets.push(repeated);
    res.second_targets.push(repeated);
    assert!(matches!(run(&mut state, &mut res), Flow::Complete));
    assert_eq!(
        state.object(repeated).unwrap().zone,
        crate::zone::Zone::Battlefield,
        "one regeneration handles an object selected by both groups"
    );
    assert_eq!(state.object(repeated).unwrap().regeneration_shields, 0);
}

#[test]
fn graveyard_counts_are_per_target_player_and_union_counts_each_card_once() {
    static TYPES: Filter = Filter::Or(&[Filter::ARTIFACT, Filter::CREATURE]);
    const RECIPIENT_COUNT: Amount = Amount::CountOf {
        filter: &TYPES,
        zone: ZoneSel::GraveyardOf(PlayerRel::Recipient),
    };
    let (mut state, _) = library_of(0);
    for _ in 0..3 {
        object(
            &mut state,
            me(),
            ZoneLocation::Graveyard(me()),
            TypeSet::CREATURE,
        );
    }
    object(
        &mut state,
        OTHER,
        ZoneLocation::Graveyard(OTHER),
        TypeSet::ARTIFACT.union(TypeSet::CREATURE),
    );
    object(
        &mut state,
        OTHER,
        ZoneLocation::Graveyard(OTHER),
        TypeSet::LAND,
    );
    let mut res = mine(Effect::GainLifeFor {
        amount: Amount::CountOf {
            filter: &TYPES,
            zone: ZoneSel::GraveyardOf(PlayerRel::SecondTargetPlayerOrController),
        },
        who: PlayerRel::SecondTargetPlayerOrController,
    });
    res.target_players.insert(me());
    res.chosen_player = Some(me());
    let mut second = baylee_core::ids::SeatSet::new();
    second.insert(OTHER);
    res.subject = SubjectContext::default().with_second_players(second);
    res.effects.push(Effect::GainLifeFor {
        amount: Amount::divided(&RECIPIENT_COUNT, 2, true),
        who: PlayerRel::EachPlayer,
    });
    assert!(matches!(run(&mut state, &mut res), Flow::Complete));
    assert_eq!((state.players[0].life, state.players[1].life), (22, 22));
}

#[test]
fn life_gain_obeys_actual_damage_previous_player_life_and_explicit_cap() {
    use crate::prevention::{Shield, ShieldKind, Shielded};
    for (life, prevented, limit, expected_gain) in [
        (3, 0, 9, 3),
        (0, 0, 9, 0),
        (20, 0, 4, 4),
        (20, 7, 9, 2),
        (20, 0, 0, 0),
        (20, 9, 9, 0),
    ] {
        let (mut state, _) = library_of(0);
        state.players[1].life = life;
        let source = object(
            &mut state,
            me(),
            ZoneLocation::Battlefield,
            TypeSet::ARTIFACT,
        );
        if prevented > 0 {
            state.shields.push(Shield {
                protects: Shielded::Player(OTHER),
                kind: ShieldKind::Next(prevented),
                controller: OTHER,
            });
        }
        let mut res = mine(Effect::DealDamageWithLimitedLifeGain {
            amount: Amount::Fixed(9),
            life_limit: Amount::Fixed(limit),
        });
        res.source = source;
        res.target_players.insert(OTHER);
        res.chosen_player = Some(OTHER);
        assert!(matches!(run(&mut state, &mut res), Flow::Complete));
        assert_eq!(
            state.players[0].life,
            20 + expected_gain,
            "life={life}, prevented={prevented}, limit={limit}"
        );
        assert_eq!(state.players[1].life, life - (9 - prevented) as i32);
    }
}

#[test]
fn life_gain_uses_post_damage_toughness_and_pre_damage_loyalty() {
    for (types, toughness, loyalty, infect, damage, expected_gain) in [
        (TypeSet::CREATURE, 4, 0, false, 7, 4),
        (TypeSet::CREATURE, 0, 0, false, 7, 0),
        (TypeSet::CREATURE, 3, 0, true, 2, 1),
        (TypeSet::PLANESWALKER, 0, 3, false, 7, 3),
        (
            TypeSet::PLANESWALKER.union(TypeSet::CREATURE),
            5,
            2,
            false,
            7,
            2,
        ),
    ] {
        let (mut state, _) = library_of(0);
        let source = object(
            &mut state,
            me(),
            ZoneLocation::Battlefield,
            TypeSet::ARTIFACT,
        );
        if infect {
            state.object_mut(source).unwrap().base_mut().keywords =
                baylee_cards_dsl::KeywordSet::INFECT;
        }
        let target = object(&mut state, OTHER, ZoneLocation::Battlefield, types);
        state.object_mut(target).unwrap().base_mut().toughness = Some(toughness);
        state
            .object_mut(target)
            .unwrap()
            .counters
            .set(baylee_cards_dsl::CounterKind::Loyalty, loyalty);
        let mut res = mine(Effect::DealDamageWithLimitedLifeGain {
            amount: Amount::Fixed(damage),
            life_limit: Amount::Fixed(10),
        });
        res.source = source;
        res.targets.push(target);
        assert!(matches!(run(&mut state, &mut res), Flow::Complete));
        assert_eq!(
            state.players[0].life,
            20 + expected_gain,
            "types={types:?}, toughness={toughness}, loyalty={loyalty}, infect={infect}"
        );
    }
}

#[test]
fn copied_x_without_a_payment_cannot_reuse_original_black_mana() {
    for paid in [true, false] {
        let (mut state, _) = library_of(0);
        let source = object(&mut state, me(), ZoneLocation::Stack, TypeSet::SORCERY);
        if paid {
            let mut payment = crate::object::PaidRecord::default();
            payment.mana_types_spent_on_x[ManaColor::Black.index()] = 5;
            state.object_mut(source).unwrap().paid = Some(Box::new(payment));
        }
        let mut res = mine(Effect::DealDamageWithLimitedLifeGain {
            amount: Amount::X,
            life_limit: Amount::ManaTypeSpentOnX(ManaColor::Black),
        });
        res.source = source;
        res.on_stack = source;
        res.x = Some(5);
        res.target_players.insert(OTHER);
        res.chosen_player = Some(OTHER);
        assert!(matches!(run(&mut state, &mut res), Flow::Complete));
        assert_eq!(state.players[0].life, if paid { 25 } else { 20 });
        assert_eq!(state.players[1].life, 15, "the copy retains X for damage");
    }
}
