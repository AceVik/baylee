//! Lossless battlefield permanent grouping for Encoder 7.
//!
//! Two battlefield permanents are grouped into a single entity token only when
//! their visible projected characteristics, status, counters, grants,
//! attachments, combat participation, targeting context, and currently offered
//! legal actions are identical.

use std::collections::{BTreeMap, BTreeSet};

use baylee_core::ids::{ObjectId, PlayerId, SubtypeId, TargetRef};
use baylee_core::mana::ManaSpending;
use baylee_core::types::SubtypeSet;
use baylee_engine::choice::Pending;
use baylee_view::{
    AbilitySource, BoardMana, CounterKind, GrantSource, GrantedMana, PlayerView, PublicObject,
    StackItem,
};

use crate::features4::SeatKnowledge4;

/// A canonical group of functionally indistinguishable battlefield permanents.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PermanentGroup7 {
    /// Index into `view.battlefield` of the canonical representative (first member).
    pub rep_index: usize,
    /// All member `ObjectId`s in `view.battlefield` traversal order.
    pub member_ids: Vec<ObjectId>,
    /// Maximum battlefield age (in turns) among members of the group.
    pub max_age_turns: u32,
    /// Whether the members of this group entered the battlefield this turn.
    pub entered_this_turn: bool,
}

impl PermanentGroup7 {
    /// Returns the representative [`ObjectId`] (first member in battlefield order).
    #[must_use]
    pub fn representative_id(&self) -> ObjectId {
        self.member_ids[0]
    }

    /// Returns the number of grouped battlefield permanents.
    #[must_use]
    pub fn count(&self) -> usize {
        self.member_ids.len()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct PermanentActionOfferKey {
    kind: u8,
    slot: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct PermanentGroupKey7 {
    controller: PlayerId,
    owner: PlayerId,
    card_key: Option<(u32, u8)>,
    rules_key: Option<(u32, u8)>,
    token: Option<u16>,
    name: String,
    commander: bool,
    status_bits: u8,
    types_bits: u16,
    supertypes_bits: u8,
    subtypes: SubtypeSet,
    chosen_subtype: Option<SubtypeId>,
    chosen_opponent: Option<PlayerId>,
    chosen_name: Option<(u32, u8)>,
    unlocked_doors: Option<[bool; 2]>,
    prepared_spell: Option<(u32, u8)>,
    colors_bits: u8,
    keywords_bits: u128,
    power: Option<i16>,
    toughness: Option<i16>,
    base_power: Option<i16>,
    base_toughness: Option<i16>,
    loyalty: Option<u16>,
    battle_protector: Option<PlayerId>,
    mana_value: u32,
    damage: u16,
    summoning_sick: bool,
    entered_this_turn: bool,
    counters: Vec<(CounterKind, u16)>,
    word_changes: Vec<(bool, u8, u8)>,
    ability_sources: Vec<AbilitySource>,
    ability_mana_spending: ManaSpending,
    granted_mana: Option<GrantedMana>,
    board_mana: Option<BoardMana>,
    grants: Vec<GrantSource>,
    offered_actions: Vec<PermanentActionOfferKey>,
}

/// Groups `view.battlefield` permanents into lossless equivalence classes while
/// preserving first-seen battlefield traversal order.
#[must_use]
pub fn group_battlefield_permanents(
    view: &PlayerView,
    pending: Option<&Pending>,
    knowledge: Option<&SeatKnowledge4>,
) -> Vec<PermanentGroup7> {
    if view.battlefield.is_empty() {
        return Vec::new();
    }

    let unmergeable = collect_unmergeable_permanents(view, pending);
    let action_offers = collect_offered_actions_by_permanent(view, pending);

    let mut groups = Vec::<PermanentGroup7>::with_capacity(view.battlefield.len());
    let mut key_to_group = std::collections::HashMap::<PermanentGroupKey7, usize>::new();

    for (bf_idx, obj) in view.battlefield.iter().enumerate() {
        let (age_turns, entered_this_turn) = knowledge.map_or((0, false), |k| {
            (
                k.permanent_age_turns(obj.id),
                k.entered_battlefield_this_turn(obj.id),
            )
        });

        if unmergeable.contains(&obj.id) {
            groups.push(PermanentGroup7 {
                rep_index: bf_idx,
                member_ids: vec![obj.id],
                max_age_turns: age_turns,
                entered_this_turn,
            });
            continue;
        }

        let offers = action_offers.get(&obj.id).cloned().unwrap_or_default();
        let key = build_group_key(obj, entered_this_turn, offers);

        if let Some(&group_idx) = key_to_group.get(&key) {
            let group = &mut groups[group_idx];
            group.member_ids.push(obj.id);
            group.max_age_turns = group.max_age_turns.max(age_turns);
        } else {
            let group_idx = groups.len();
            key_to_group.insert(key, group_idx);
            groups.push(PermanentGroup7 {
                rep_index: bf_idx,
                member_ids: vec![obj.id],
                max_age_turns: age_turns,
                entered_this_turn,
            });
        }
    }

    groups
}

fn collect_unmergeable_permanents(
    view: &PlayerView,
    pending: Option<&Pending>,
) -> BTreeSet<ObjectId> {
    let mut singletons = BTreeSet::<ObjectId>::new();

    // 1. Attached permanents and their hosts.
    for obj in &view.battlefield {
        if let Some(host) = obj.attached_to {
            singletons.insert(obj.id);
            singletons.insert(host);
        }
        if !obj.targets.is_empty() {
            singletons.insert(obj.id);
            for target in &obj.targets {
                if let TargetRef::Object(r) = target {
                    singletons.insert(r.object);
                }
            }
        }
    }

    // 2. Stack items targeting a permanent or originating from a permanent.
    for item in &view.stack {
        for target in &item.targets {
            if let TargetRef::Object(r) = target {
                singletons.insert(r.object);
            }
        }
        if let Some(StackItem::Ability { source, .. }) = item.stack_item {
            singletons.insert(source);
        }
    }

    // 3. Active combat participants (attackers and blockers).
    for attacker in &view.combat.attackers {
        singletons.insert(attacker.creature);
    }
    for blocker in &view.combat.blockers {
        singletons.insert(blocker.attacker);
        singletons.insert(blocker.blocker);
    }

    // 4. Sorcery lock, active casting, and active targeting context.
    if let Some(lock) = view.sorcery_lock {
        singletons.insert(lock);
    }
    if let Some(cast_id) = view.casting {
        singletons.insert(cast_id);
    }
    if let Some(targeting) = &view.targeting {
        singletons.insert(targeting.source.id);
    }

    // 5. Pending choice context referencing a specific permanent.
    if let Some(pending) = pending {
        match pending {
            Pending::ChooseCastMode { object, .. } => {
                singletons.insert(*object);
            }
            Pending::ChooseDamageSource { options, .. } => {
                for src in options {
                    singletons.insert(src.object);
                }
            }
            Pending::ChooseDamageEffect { options, .. } => {
                for eff in options {
                    if let Some(src) = eff.source {
                        singletons.insert(src);
                    }
                }
            }
            _ => {}
        }
    }

    singletons
}

fn collect_offered_actions_by_permanent(
    _view: &PlayerView,
    pending: Option<&Pending>,
) -> BTreeMap<ObjectId, Vec<PermanentActionOfferKey>> {
    let mut map = BTreeMap::<ObjectId, Vec<PermanentActionOfferKey>>::new();
    match pending {
        Some(Pending::Priority { legal, .. }) => {
            for &id in &legal.lands {
                map.entry(id)
                    .or_default()
                    .push(PermanentActionOfferKey { kind: 1, slot: 0 });
            }
            for &id in &legal.castable {
                map.entry(id)
                    .or_default()
                    .push(PermanentActionOfferKey { kind: 2, slot: 0 });
            }
            for &id in &legal.payable {
                map.entry(id)
                    .or_default()
                    .push(PermanentActionOfferKey { kind: 3, slot: 0 });
            }
            for &id in &legal.suspendable {
                map.entry(id)
                    .or_default()
                    .push(PermanentActionOfferKey { kind: 4, slot: 0 });
            }
            for &id in &legal.mana_abilities {
                map.entry(id)
                    .or_default()
                    .push(PermanentActionOfferKey { kind: 5, slot: 0 });
            }
            for &(id, slot) in &legal.abilities {
                map.entry(id).or_default().push(PermanentActionOfferKey {
                    kind: 6,
                    slot: u64::from(slot),
                });
            }
            for grant in &legal.granted_actions {
                map.entry(grant.source.object)
                    .or_default()
                    .push(PermanentActionOfferKey {
                        kind: 7,
                        slot: grant.id.get(),
                    });
            }
        }
        Some(Pending::ChooseManaAbility { options, .. }) => {
            for opt in options {
                map.entry(opt.source.object)
                    .or_default()
                    .push(PermanentActionOfferKey {
                        kind: 8,
                        slot: u64::from(opt.ability_index.unwrap_or(u32::MAX)),
                    });
            }
        }
        _ => {}
    }
    for list in map.values_mut() {
        list.sort_unstable();
        list.dedup();
    }
    map
}

fn build_group_key(
    obj: &PublicObject,
    entered_this_turn: bool,
    offered_actions: Vec<PermanentActionOfferKey>,
) -> PermanentGroupKey7 {
    let mut counters: Vec<(CounterKind, u16)> =
        obj.counters.iter().map(|c| (c.kind, c.count)).collect();
    counters.sort_by_key(|(k, _)| counter_sort_key(*k));

    let word_changes = obj
        .word_changes
        .iter()
        .map(|w| (w.basic_land_type, w.from, w.to))
        .collect();

    PermanentGroupKey7 {
        controller: obj.controller,
        owner: obj.owner,
        card_key: obj.card.map(|c| (c.index.get(), c.face)),
        rules_key: obj.rules.map(|r| (r.card.get(), r.face)),
        token: obj.token,
        name: obj.name.clone(),
        commander: obj.commander,
        status_bits: obj.status.bits(),
        types_bits: obj.types.bits(),
        supertypes_bits: obj.supertypes.bits(),
        subtypes: obj.subtypes,
        chosen_subtype: obj.chosen_subtype,
        chosen_opponent: obj.chosen_opponent,
        chosen_name: obj.chosen_name.map(|n| (n.card.get(), n.face)),
        unlocked_doors: obj.unlocked_doors,
        prepared_spell: obj.prepared_spell.map(|r| (r.card.get(), r.face)),
        colors_bits: obj.colors.bits(),
        keywords_bits: obj.keywords,
        power: obj.power,
        toughness: obj.toughness,
        base_power: obj.base_power,
        base_toughness: obj.base_toughness,
        loyalty: obj.loyalty,
        battle_protector: obj.battle_protector,
        mana_value: obj.mana_value,
        damage: obj.damage,
        summoning_sick: obj.summoning_sick,
        entered_this_turn,
        counters,
        word_changes,
        ability_sources: obj.ability_sources.clone(),
        ability_mana_spending: obj.ability_mana_spending,
        granted_mana: obj.granted_mana.clone(),
        board_mana: obj.board_mana.clone(),
        grants: obj.grants.clone(),
        offered_actions,
    }
}

const fn counter_sort_key(kind: CounterKind) -> (u8, u8, u8, u32) {
    match kind {
        CounterKind::Plus { power, toughness } => (0, power, toughness, 0),
        CounterKind::Minus { power, toughness } => (1, power, toughness, 0),
        CounterKind::Loyalty => (2, 0, 0, 0),
        CounterKind::Lore => (3, 0, 0, 0),
        CounterKind::Time => (4, 0, 0, 0),
        CounterKind::Charge => (5, 0, 0, 0),
        CounterKind::Poison => (6, 0, 0, 0),
        CounterKind::Energy => (7, 0, 0, 0),
        CounterKind::Rad => (8, 0, 0, 0),
        CounterKind::Lifelink => (9, 0, 0, 0),
        CounterKind::Level => (10, 0, 0, 0),
        CounterKind::Flying => (11, 0, 0, 0),
        CounterKind::Shadow => (12, 0, 0, 0),
        CounterKind::Indestructible => (13, 0, 0, 0),
        CounterKind::Hone => (14, 0, 0, 0),
        CounterKind::Stun => (15, 0, 0, 0),
        CounterKind::Finality => (16, 0, 0, 0),
        CounterKind::Bore => (17, 0, 0, 0),
        CounterKind::Shield => (18, 0, 0, 0),
        CounterKind::Defense => (19, 0, 0, 0),
        CounterKind::Custom(id) => (20, 0, 0, id),
    }
}
