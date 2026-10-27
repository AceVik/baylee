//! Encoder 7 (`ENCODER_VERSION7 = 7`) complete state encoder (`encode_state7`).
//!
//! Expands entity features to `128` floats and global features to `80` floats:
//! - Exact unclipped numeric quantities (`raw_unit` / `raw_signed` fed to `symlog`),
//! - Lossless battlefield permanent grouping (`PermanentGroup7`),
//! - Deduplicated inspection/reveal zones (`SharedHand`, `ControlledHand`, `LookingAt`, `LibraryTop`),
//! - Explicit `Stack` and `Graveyard` top-down ordering (`order_from_top`, `normalized_depth`),
//! - Public `CommanderDesignation` entities for commanders in hidden zones,
//! - Own-deck `DeckInventory` and `SideboardInventory` entities from `InventoryLedger7`,
//! - Disambiguated 21 `CounterKind` channels, full projected `KeywordSet`, basic land subtypes,
//!   `board_mana`, `granted_mana`, `ability_mana_spending`, `word_changes`, and exact `owed` symbols.

use std::collections::BTreeSet;

use baylee_cards_dsl::KeywordSet;
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::land as land_subtypes;
use baylee_core::ids::{CardIndex, Defender, ObjectId, PlayerId, TargetRef};
use baylee_core::mana::{ManaColor, ManaPayment, ManaSpending, ManaSymbol};
use baylee_core::types::{SupertypeSet, TypeSet};
use baylee_engine::choice::Pending;
use baylee_view::{
    CommanderView, CounterKind, DayNight, HandObject, ObjectStatus, Phase, PlayerView,
    PublicObject, StackItem, Step,
};

use crate::carddsl::UNKNOWN_CARD;
use crate::features4::{
    ENTITY_TOKEN_OFFSET, EdgeRel4, EncodedState4, EntityZone4, KnownCardMemory4, MAX_EDGES,
    MAX_ENTITIES, MAX_SEATS, PendingKind4, SEAT_FEAT_WIDTH, SEAT_TOKEN_OFFSET, SeatKnowledge4,
    abs_seat, pack_object_id, rel_seat,
};
use crate::features7::grouping::group_battlefield_permanents;

/// Encoder version identifier for Encoder 7 (`7`).
pub const ENCODER_VERSION7: u32 = 7;
/// Per-entity numeric feature width in Encoder 7 (`128`).
pub const ENTITY_FEAT_WIDTH7: usize = 128;
/// Per-seat numeric feature width in Encoder 7 (`64`).
pub const SEAT_FEAT_WIDTH7: usize = SEAT_FEAT_WIDTH;
/// Global numeric feature width in Encoder 7 (`80`).
pub const GLOBAL_FEAT_WIDTH7: usize = 80;
/// Per-candidate numeric feature width in Encoder 7 (`48`).
pub const CAND_FEAT_WIDTH7: usize = 48;

#[inline]
fn raw_unit(v: f32, scale: f32) -> f32 {
    if scale <= 0.0 {
        0.0
    } else {
        (v / scale).clamp(0.0, 1.0)
    }
}

#[inline]
fn raw_signed(v: f32, scale: f32) -> f32 {
    if scale <= 0.0 {
        0.0
    } else {
        (v / scale).clamp(-1.0, 1.0)
    }
}

/// Encodes a [`PlayerView`] into an Encoder 7 [`EncodedState4`] (`128` entity dims, `80` global dims).
#[must_use]
#[allow(
    clippy::too_many_lines,
    reason = "Ordered zone traversal and complete Encoder 7 state contract"
)]
pub fn encode_state7(
    view: &PlayerView,
    pending: Option<&Pending>,
    knowledge: Option<&SeatKnowledge4>,
    teams: &[Option<u8>],
) -> EncodedState4 {
    let mut state = EncodedState4 {
        seat_feat: vec![vec![0.0; SEAT_FEAT_WIDTH7]; MAX_SEATS],
        global_feat: vec![0.0; GLOBAL_FEAT_WIDTH7],
        ..EncodedState4::default()
    };
    let viewer = view.seat;
    let seat_count = view.seats.len().clamp(1, MAX_SEATS);
    let mut emitted_object_ids = BTreeSet::<u64>::new();

    // 1. Grouped Battlefield permanents
    let bf_groups = group_battlefield_permanents(view, pending, knowledge);
    for group in &bf_groups {
        let obj = &view.battlefield[group.rep_index];
        let packed_members: Vec<u64> = group
            .member_ids
            .iter()
            .map(|&id| pack_object_id(id))
            .collect();
        for &raw in &packed_members {
            emitted_object_ids.insert(raw);
        }
        push_public_object7(
            &mut state,
            view,
            obj,
            EntityZone4::Battlefield,
            viewer,
            seat_count,
            pending,
            knowledge,
            packed_members,
            group.count() as f32,
            group.max_age_turns as f32,
            f32::from(u8::from(group.entered_this_turn)),
        );
    }

    // 2. Stack (never grouped; explicit top-down ordering)
    let stack_len = view.stack.len();
    for (idx, obj) in view.stack.iter().enumerate() {
        let raw = pack_object_id(obj.id);
        emitted_object_ids.insert(raw);
        let from_top = (stack_len - 1 - idx) as f32;
        let norm_depth = if stack_len > 1 {
            from_top / ((stack_len - 1) as f32)
        } else {
            0.0
        };
        push_public_object7(
            &mut state,
            view,
            obj,
            EntityZone4::Stack,
            viewer,
            seat_count,
            pending,
            knowledge,
            vec![raw],
            1.0,
            from_top,
            norm_depth,
        );
    }

    // 3. Viewing seat's own Hand
    for (idx, h_obj) in view.hand.iter().enumerate() {
        let raw = pack_object_id(h_obj.id);
        if !emitted_object_ids.insert(raw) {
            continue;
        }
        push_hand_object7(
            &mut state,
            h_obj,
            viewer,
            EntityZone4::Hand,
            viewer,
            seat_count,
            pending,
            knowledge,
            idx as f32,
        );
    }

    // 4. Command, Graveyards (explicit top-down ordering), and Exile
    for seat_cmd in &view.command {
        for obj in seat_cmd {
            let raw = pack_object_id(obj.id);
            if !emitted_object_ids.insert(raw) {
                continue;
            }
            push_public_object7(
                &mut state,
                view,
                obj,
                EntityZone4::Command,
                viewer,
                seat_count,
                pending,
                knowledge,
                vec![raw],
                1.0,
                0.0,
                0.0,
            );
        }
    }
    for seat_gy in &view.graveyards {
        let gy_len = seat_gy.len();
        for (idx, obj) in seat_gy.iter().enumerate() {
            let raw = pack_object_id(obj.id);
            if !emitted_object_ids.insert(raw) {
                continue;
            }
            let from_top = (gy_len - 1 - idx) as f32;
            let norm_depth = if gy_len > 1 {
                from_top / ((gy_len - 1) as f32)
            } else {
                0.0
            };
            push_public_object7(
                &mut state,
                view,
                obj,
                EntityZone4::Graveyard,
                viewer,
                seat_count,
                pending,
                knowledge,
                vec![raw],
                1.0,
                from_top,
                norm_depth,
            );
        }
    }
    for seat_ex in &view.exile {
        for obj in seat_ex {
            let raw = pack_object_id(obj.id);
            if !emitted_object_ids.insert(raw) {
                continue;
            }
            push_public_object7(
                &mut state,
                view,
                obj,
                EntityZone4::Exile,
                viewer,
                seat_count,
                pending,
                knowledge,
                vec![raw],
                1.0,
                0.0,
                0.0,
            );
        }
    }

    // 5. Deduplicated SharedHands, ControlledHands, LookingAt, and LibraryTops
    for sh in &view.shared_hands {
        for (idx, h_obj) in sh.cards.iter().enumerate() {
            let raw = pack_object_id(h_obj.id);
            if !emitted_object_ids.insert(raw) {
                continue;
            }
            push_hand_object7(
                &mut state,
                h_obj,
                sh.player,
                EntityZone4::SharedHand,
                viewer,
                seat_count,
                pending,
                knowledge,
                idx as f32,
            );
        }
    }
    for ch in &view.controlled_hands {
        for (idx, h_obj) in ch.cards.iter().enumerate() {
            let raw = pack_object_id(h_obj.id);
            if !emitted_object_ids.insert(raw) {
                continue;
            }
            push_hand_object7(
                &mut state,
                h_obj,
                ch.player,
                EntityZone4::ControlledHand,
                viewer,
                seat_count,
                pending,
                knowledge,
                idx as f32,
            );
        }
    }
    let look_len = view.looking_at.len();
    for (idx, obj) in view.looking_at.iter().enumerate() {
        let raw = pack_object_id(obj.id);
        if !emitted_object_ids.insert(raw) {
            continue;
        }
        let norm_pos = if look_len > 1 {
            (idx as f32) / ((look_len - 1) as f32)
        } else {
            0.0
        };
        push_public_object7(
            &mut state,
            view,
            obj,
            EntityZone4::LookingAt,
            viewer,
            seat_count,
            pending,
            knowledge,
            vec![raw],
            1.0,
            idx as f32,
            norm_pos,
        );
    }
    for obj in &view.library_tops {
        let raw = pack_object_id(obj.id);
        if !emitted_object_ids.insert(raw) {
            continue;
        }
        push_public_object7(
            &mut state,
            view,
            obj,
            EntityZone4::LibraryTop,
            viewer,
            seat_count,
            pending,
            knowledge,
            vec![raw],
            1.0,
            0.0,
            0.0,
        );
    }

    // 6. Public CommanderDesignation (zone 10) for commanders in hidden zones
    for sv in &view.seats {
        for cmd in &sv.commanders {
            let raw = pack_object_id(cmd.object);
            if !emitted_object_ids.insert(raw) {
                continue;
            }
            push_commander_designation7(&mut state, sv.player, cmd, viewer, seat_count);
        }
    }

    // 7. Remembered opponent cards from SeatKnowledge4
    if let Some(kn) = knowledge {
        for (&(_owner, obj_id), mem) in &kn.known_cards {
            if state.entity_object_ids.len() >= MAX_ENTITIES {
                break;
            }
            if !emitted_object_ids.insert(obj_id) {
                continue;
            }
            push_remembered_card7(&mut state, view, obj_id, mem, viewer, seat_count);
        }
    }

    // 8. Seat features in relative seat order `0..seat_count`
    encode_seats7(&mut state, view, knowledge, teams, viewer, seat_count);

    // 10. Global features (`80` dims)
    encode_global7(
        &mut state, view, pending, knowledge, teams, viewer, seat_count,
    );

    // 11. Directed relational edges
    build_edges7(&mut state, view, viewer, seat_count);

    state
}

#[allow(
    clippy::too_many_arguments,
    clippy::too_many_lines,
    reason = "Contiguous versioned public-object feature layout for Encoder 7"
)]
fn push_public_object7(
    state: &mut EncodedState4,
    view: &PlayerView,
    obj: &PublicObject,
    zone: EntityZone4,
    viewer: PlayerId,
    seat_count: usize,
    pending: Option<&Pending>,
    knowledge: Option<&SeatKnowledge4>,
    group_members: Vec<u64>,
    group_count: f32,
    order_or_age: f32,
    depth_or_entered: f32,
) {
    if state.entity_feat.len() >= MAX_ENTITIES {
        return;
    }
    let mut f = vec![0.0_f32; ENTITY_FEAT_WIDTH7];
    if (zone as usize) < 12 {
        f[zone as usize] = 1.0;
    }

    let rel_ctrl = rel_seat(viewer, Some(obj.controller), seat_count);
    let rel_own = rel_seat(viewer, Some(obj.owner), seat_count);
    let rel_prot = rel_seat(viewer, obj.battle_protector, seat_count);
    let rel_c_opp = rel_seat(viewer, obj.chosen_opponent, seat_count);

    // [12..20]: relative seat relationships
    f[12] = f32::from(u8::from(rel_ctrl == 0));
    f[13] = raw_unit(rel_ctrl as f32, 8.0);
    f[14] = f32::from(u8::from(rel_own == 0));
    f[15] = raw_unit(rel_own as f32, 8.0);
    f[16] = f32::from(u8::from(obj.battle_protector.is_some()));
    f[17] = raw_unit(rel_prot as f32, 8.0);
    f[18] = f32::from(u8::from(obj.chosen_opponent.is_some()));
    f[19] = raw_unit(rel_c_opp as f32, 8.0);

    // [20..27]: all 7 ObjectStatus bits
    let st = obj.status;
    f[20] = f32::from(u8::from(st.contains(ObjectStatus::TAPPED)));
    f[21] = f32::from(u8::from(st.contains(ObjectStatus::FACE_DOWN)));
    f[22] = f32::from(u8::from(st.contains(ObjectStatus::PHASED_OUT)));
    f[23] = f32::from(u8::from(st.contains(ObjectStatus::FLIPPED)));
    f[24] = f32::from(u8::from(st.contains(ObjectStatus::NO_ABILITIES)));
    f[25] = f32::from(u8::from(st.contains(ObjectStatus::MELDED_CARD)));
    f[26] = f32::from(u8::from(st.contains(ObjectStatus::TOKEN)));

    // [27..34]: offered bits + payable flag in f[127]
    if let Some(p) = pending {
        match p {
            Pending::Priority { legal, .. } => {
                if legal.castable.contains(&obj.id) || legal.suspendable.contains(&obj.id) {
                    f[27] = 1.0;
                }
                if legal.payable.contains(&obj.id) {
                    f[127] = 1.0;
                }
                if legal.lands.contains(&obj.id) {
                    f[28] = 1.0;
                }
                if legal.mana_abilities.contains(&obj.id)
                    || legal.abilities.iter().any(|&(src, _)| src == obj.id)
                {
                    f[29] = 1.0;
                }
            }
            Pending::ChooseAttackers { attackers, .. } => {
                if attackers.contains(&obj.id) {
                    f[31] = 1.0;
                }
            }
            Pending::ChooseBlockers { blockers, .. } => {
                if blockers.iter().any(|b| b.blocker == obj.id) {
                    f[32] = 1.0;
                }
            }
            Pending::ChooseCards { options, .. }
            | Pending::ChooseTargets { options, .. }
            | Pending::LegendChoice { options, .. }
            | Pending::Arrange { cards: options, .. }
                if options.contains(&obj.id) =>
            {
                f[30] = 1.0;
            }
            _ => {}
        }
    }
    if view.casting == Some(obj.id) {
        f[33] = 1.0;
    }

    // [34..46]: face, rules override, prepared, chosen_subtype, chosen_name, doors, suspended, commander, summoning_sick
    let face_idx = obj
        .rules
        .map(|r| r.face)
        .or_else(|| obj.card.map(|c| c.face))
        .unwrap_or(0);
    let active_card_id = obj
        .rules
        .map(|r| r.card.get())
        .or_else(|| obj.card.map(|c| c.index.get()))
        .unwrap_or(UNKNOWN_CARD);
    let is_copy_override = obj
        .rules
        .zip(obj.card)
        .is_some_and(|(r, c)| r.card != c.index || r.face != c.face);
    f[34] = f32::from(face_idx);
    f[35] = f32::from(u8::from(is_copy_override));
    f[36] = f32::from(u8::from(obj.prepared_spell.is_some()));
    f[37] = obj
        .chosen_subtype
        .map_or(0.0, |st| f32::from(st.get()) / 1024.0);
    f[38] = f32::from(u8::from(obj.chosen_name.is_some()));
    if let Some([d0, d1]) = obj.unlocked_doors {
        f[39] = f32::from(u8::from(d0));
        f[40] = f32::from(u8::from(d1));
    }
    f[41] = f32::from(u8::from(obj.suspended));
    f[42] = f32::from(u8::from(obj.commander));
    f[43] = f32::from(u8::from(obj.summoning_sick));
    f[44] = f32::from(u8::from(obj.attached_to.is_some()));
    f[45] = raw_unit(obj.mana_value as f32, 12.0);

    // [46..56]: unclipped projected & base P/T, damage, remaining toughness, lethal, loyalty
    f[46] = f32::from(u8::from(obj.power.is_some()));
    f[47] = obj.power.map_or(0.0, |p| raw_signed(f32::from(p), 12.0));
    f[48] = obj
        .toughness
        .map_or(0.0, |t| raw_signed(f32::from(t), 12.0));
    f[49] = obj
        .base_power
        .map_or(0.0, |p| raw_signed(f32::from(p), 12.0));
    f[50] = obj
        .base_toughness
        .map_or(0.0, |t| raw_signed(f32::from(t), 12.0));
    f[51] = raw_unit(f32::from(obj.damage), 10.0);
    f[52] = obj
        .remaining_toughness()
        .map_or(0.0, |r| raw_signed(f32::from(r), 12.0));
    f[53] = f32::from(u8::from(obj.is_lethally_damaged()));
    f[54] = f32::from(u8::from(obj.loyalty.is_some()));
    f[55] = obj.loyalty.map_or(0.0, |l| raw_unit(f32::from(l), 10.0));

    // [56..70]: types (9) + colors (5)
    f[56] = f32::from(u8::from(obj.types.contains(TypeSet::CREATURE)));
    f[57] = f32::from(u8::from(obj.types.contains(TypeSet::INSTANT)));
    f[58] = f32::from(u8::from(obj.types.contains(TypeSet::SORCERY)));
    f[59] = f32::from(u8::from(obj.types.contains(TypeSet::ARTIFACT)));
    f[60] = f32::from(u8::from(obj.types.contains(TypeSet::ENCHANTMENT)));
    f[61] = f32::from(u8::from(obj.types.contains(TypeSet::LAND)));
    f[62] = f32::from(u8::from(obj.types.contains(TypeSet::PLANESWALKER)));
    f[63] = f32::from(u8::from(obj.types.contains(TypeSet::BATTLE)));
    f[64] = f32::from(u8::from(obj.supertypes.contains(SupertypeSet::LEGENDARY)));
    f[65] = f32::from(u8::from(obj.colors.contains(Color::White)));
    f[66] = f32::from(u8::from(obj.colors.contains(Color::Blue)));
    f[67] = f32::from(u8::from(obj.colors.contains(Color::Black)));
    f[68] = f32::from(u8::from(obj.colors.contains(Color::Red)));
    f[69] = f32::from(u8::from(obj.colors.contains(Color::Green)));

    // [70..82] + [96..105]: All 21 CounterKind categories (unclipped & disambiguated)
    for entry in &obj.counters {
        let amt = f32::from(entry.count);
        match entry.kind {
            CounterKind::Plus { power, toughness } => {
                f[70] += raw_unit(amt * f32::from(power.max(toughness).max(1)), 8.0);
                f[96] += raw_signed(amt * f32::from(power), 8.0);
                f[97] += raw_signed(amt * f32::from(toughness), 8.0);
            }
            CounterKind::Minus { power, toughness } => {
                f[71] += raw_unit(amt * f32::from(power.max(toughness).max(1)), 8.0);
                f[98] += raw_signed(amt * f32::from(power), 8.0);
                f[99] += raw_signed(amt * f32::from(toughness), 8.0);
            }
            CounterKind::Loyalty => f[72] += raw_unit(amt, 10.0),
            CounterKind::Defense => f[73] += raw_unit(amt, 10.0),
            CounterKind::Lore => f[74] += raw_unit(amt, 6.0),
            CounterKind::Level => {
                f[74] += raw_unit(amt, 6.0);
                f[100] += raw_unit(amt, 6.0);
            }
            CounterKind::Time => f[75] += raw_unit(amt, 6.0),
            CounterKind::Charge => f[76] += raw_unit(amt, 8.0),
            CounterKind::Bore => {
                f[76] += raw_unit(amt, 8.0);
                f[101] += raw_unit(amt, 8.0);
            }
            CounterKind::Energy => {
                f[76] += raw_unit(amt, 8.0);
                f[102] += raw_unit(amt, 8.0);
            }
            CounterKind::Stun => f[77] += raw_unit(amt, 4.0),
            CounterKind::Shield => f[78] += raw_unit(amt, 4.0),
            CounterKind::Indestructible => {
                f[78] += raw_unit(amt, 4.0);
                f[103] += raw_unit(amt, 4.0);
            }
            CounterKind::Finality => f[79] += raw_unit(amt, 2.0),
            CounterKind::Flying => f[80] += raw_unit(amt, 4.0),
            CounterKind::Lifelink => {
                f[80] += raw_unit(amt, 4.0);
                f[104] += 1.0;
            }
            CounterKind::Shadow => {
                f[80] += raw_unit(amt, 4.0);
                f[104] += 2.0;
            }
            CounterKind::Hone => {
                f[80] += raw_unit(amt, 4.0);
                f[104] += 4.0;
            }
            CounterKind::Poison => {
                f[81] += raw_unit(amt, 6.0);
                f[104] += 8.0;
            }
            CounterKind::Rad => {
                f[81] += raw_unit(amt, 6.0);
                f[104] += 16.0;
            }
            CounterKind::Custom(_) => {
                f[81] += raw_unit(amt, 6.0);
            }
        }
    }

    // [82..90]: Combat state & mana/grant capabilities
    let is_attacking = view.combat.attackers.iter().find(|a| a.creature == obj.id);
    let is_blocking = view.combat.blockers.iter().any(|b| b.blocker == obj.id);
    f[82] = f32::from(u8::from(is_attacking.is_some()));
    f[83] = f32::from(u8::from(is_attacking.is_some_and(|a| a.blocked)));
    f[84] = f32::from(u8::from(is_blocking));
    f[85] = f32::from(u8::from(
        obj.granted_mana.is_some() || obj.board_mana.is_some(),
    ));
    f[86] = f32::from(u8::from(obj.flashback.is_some()));
    f[87] = obj
        .flashback
        .as_ref()
        .map_or(0.0, |mc| raw_unit(mc.cmc() as f32, 10.0));
    f[88] = raw_unit(obj.grants.len() as f32, 4.0);
    f[89] = f32::from(u8::from(obj.ability_mana_spending != ManaSpending::EXACT));

    // [90..96]: StackItem & projected keywords summary
    match obj.stack_item {
        Some(StackItem::Spell) => {
            f[90] = 1.0;
        }
        Some(StackItem::Ability { ability, token, .. }) => {
            f[91] = 1.0;
            let ab_idx = ability
                .map(|a| a.index)
                .or_else(|| token.map(|t| t.index))
                .unwrap_or(0);
            f[92] = raw_unit(ab_idx as f32, 8.0);
        }
        None => {}
    }
    f[93] = raw_unit(obj.targets.len() as f32, 4.0);
    f[94] = f32::from(u8::from((obj.keywords & (1 << 0)) != 0)); // Flying
    f[95] = f32::from(u8::from(obj.keywords != 0));

    // [105..116]: Full projected KeywordSet bits
    encode_keywords7(obj.keywords, &mut f);

    // [116..122]: Projected WUBRG mana production (basic land subtypes, board_mana, granted_mana) + word_changes
    let mut mana_colors = obj
        .board_mana
        .as_ref()
        .map_or(ColorSet::EMPTY, |bm| mana_colors_to_set(&bm.colors));
    if let Some(gm) = &obj.granted_mana {
        mana_colors = mana_colors.union(mana_colors_to_set(&gm.colors));
    }
    if obj.subtypes.contains(land_subtypes::PLAINS) || mana_colors.contains(Color::White) {
        f[116] = 1.0;
    }
    if obj.subtypes.contains(land_subtypes::ISLAND) || mana_colors.contains(Color::Blue) {
        f[117] = 1.0;
    }
    if obj.subtypes.contains(land_subtypes::SWAMP) || mana_colors.contains(Color::Black) {
        f[118] = 1.0;
    }
    if obj.subtypes.contains(land_subtypes::MOUNTAIN) || mana_colors.contains(Color::Red) {
        f[119] = 1.0;
    }
    if obj.subtypes.contains(land_subtypes::FOREST) || mana_colors.contains(Color::Green) {
        f[120] = 1.0;
    }
    let grant_amt = obj.granted_mana.as_ref().map_or(0, |gm| gm.amount);
    f[121] = f32::from(grant_amt)
        + (obj.word_changes.len() as f32)
        + mana_spending_flexibility(obj.ability_mana_spending);

    // [122..127]: group_count, remaining copies in own library, draw probability, ordering/age, depth/entered
    f[122] = group_count;
    if obj.owner == viewer
        && active_card_id != UNKNOWN_CARD
        && let Some(kn) = knowledge
    {
        f[123] = kn.inventory.remaining_copies_in_library(active_card_id) as f32;
        f[124] = kn.inventory.draw_probability(active_card_id);
    }
    f[125] = order_or_age;
    f[126] = depth_or_entered;

    let token_slot = obj.token.map_or(0, |id| {
        if (id as usize) < baylee_cards::tokens::ALL.len() {
            id.saturating_add(1)
        } else {
            0
        }
    });
    state.entity_feat.push(f);
    state.entity_object_ids.push(pack_object_id(obj.id));
    state.entity_group_members.push(group_members);
    state.entity_card_ids.push(active_card_id);
    state.entity_face_ids.push(face_idx);
    state.entity_token_ids.push(token_slot);
    state
        .entity_rule_refs
        .push(crate::cardstructure::refs::public_refs(
            obj,
            active_card_id,
            face_idx,
            token_slot,
        ));
    state.entity_zone_ids.push(zone as u8);
    state.entity_rel_controller.push(rel_ctrl as u8);
}

#[allow(
    clippy::too_many_arguments,
    reason = "Contiguous versioned hand-object feature layout for Encoder 7"
)]
fn push_hand_object7(
    state: &mut EncodedState4,
    h_obj: &HandObject,
    owner: PlayerId,
    zone: EntityZone4,
    viewer: PlayerId,
    seat_count: usize,
    pending: Option<&Pending>,
    knowledge: Option<&SeatKnowledge4>,
    hand_order: f32,
) {
    if state.entity_feat.len() >= MAX_ENTITIES {
        return;
    }
    let mut f = vec![0.0_f32; ENTITY_FEAT_WIDTH7];
    if (zone as usize) < 12 {
        f[zone as usize] = 1.0;
    }
    let rel_own = rel_seat(viewer, Some(owner), seat_count);
    f[12] = f32::from(u8::from(rel_own == 0));
    f[13] = raw_unit(rel_own as f32, 8.0);
    f[14] = f32::from(u8::from(rel_own == 0));
    f[15] = raw_unit(rel_own as f32, 8.0);

    if let Some(Pending::Priority { legal, .. }) = pending {
        if legal.castable.contains(&h_obj.id) || legal.suspendable.contains(&h_obj.id) {
            f[27] = 1.0;
        }
        if legal.payable.contains(&h_obj.id) {
            f[127] = 1.0;
        }
        if legal.lands.contains(&h_obj.id) {
            f[28] = 1.0;
        }
        if legal.abilities.iter().any(|(src, _)| *src == h_obj.id) {
            f[29] = 1.0;
        }
    }
    f[34] = f32::from(h_obj.card.face);
    f[42] = f32::from(u8::from(h_obj.commander));
    f[45] = raw_unit(h_obj.mana_value as f32, 12.0);

    if let Some(def) = baylee_cards::by_index(h_obj.card.index)
        && let Some(face) = def
            .faces
            .get(h_obj.card.face as usize)
            .or_else(|| def.faces.first())
    {
        f[46] = f32::from(u8::from(face.power.is_some()));
        f[47] = face.power.map_or(0.0, |p| raw_signed(f32::from(p), 12.0));
        f[48] = face
            .toughness
            .map_or(0.0, |t| raw_signed(f32::from(t), 12.0));
        f[54] = f32::from(u8::from(face.loyalty.is_some()));
        f[55] = face.loyalty.map_or(0.0, |l| raw_unit(f32::from(l), 10.0));
        f[64] = f32::from(u8::from(face.supertypes.contains(SupertypeSet::LEGENDARY)));
        f[94] = f32::from(u8::from(face.keywords.contains(KeywordSet::FLYING)));
        f[95] = f32::from(u8::from(!face.keywords.is_empty()));
        encode_keywords7(face.keywords.bits(), &mut f);
    }

    f[56] = f32::from(u8::from(h_obj.types.contains(TypeSet::CREATURE)));
    f[57] = f32::from(u8::from(h_obj.types.contains(TypeSet::INSTANT)));
    f[58] = f32::from(u8::from(h_obj.types.contains(TypeSet::SORCERY)));
    f[59] = f32::from(u8::from(h_obj.types.contains(TypeSet::ARTIFACT)));
    f[60] = f32::from(u8::from(h_obj.types.contains(TypeSet::ENCHANTMENT)));
    f[61] = f32::from(u8::from(h_obj.types.contains(TypeSet::LAND)));
    f[62] = f32::from(u8::from(h_obj.types.contains(TypeSet::PLANESWALKER)));
    f[63] = f32::from(u8::from(h_obj.types.contains(TypeSet::BATTLE)));
    f[65] = f32::from(u8::from(h_obj.colors.contains(Color::White)));
    f[66] = f32::from(u8::from(h_obj.colors.contains(Color::Blue)));
    f[67] = f32::from(u8::from(h_obj.colors.contains(Color::Black)));
    f[68] = f32::from(u8::from(h_obj.colors.contains(Color::Red)));
    f[69] = f32::from(u8::from(h_obj.colors.contains(Color::Green)));

    let cid = h_obj.card.index.get();
    f[122] = 1.0;
    if owner == viewer
        && let Some(kn) = knowledge
    {
        f[123] = kn.inventory.remaining_copies_in_library(cid) as f32;
        f[124] = kn.inventory.draw_probability(cid);
    }
    f[125] = hand_order;

    let raw = pack_object_id(h_obj.id);
    state.entity_feat.push(f);
    state.entity_object_ids.push(raw);
    state.entity_group_members.push(vec![raw]);
    state.entity_card_ids.push(cid);
    state.entity_face_ids.push(h_obj.card.face);
    state.entity_token_ids.push(0);
    state
        .entity_rule_refs
        .push(crate::cardstructure::refs::card_refs(
            cid,
            h_obj.card.face,
            0,
        ));
    state.entity_zone_ids.push(zone as u8);
    state.entity_rel_controller.push(rel_own as u8);
}

fn push_commander_designation7(
    state: &mut EncodedState4,
    owner: PlayerId,
    cmd: &CommanderView,
    viewer: PlayerId,
    seat_count: usize,
) {
    if state.entity_feat.len() >= MAX_ENTITIES {
        return;
    }
    let mut f = vec![0.0_f32; ENTITY_FEAT_WIDTH7];
    f[EntityZone4::CommanderDesignation as usize] = 1.0;
    let rel_own = rel_seat(viewer, Some(owner), seat_count);
    f[12] = f32::from(u8::from(rel_own == 0));
    f[13] = raw_unit(rel_own as f32, 8.0);
    f[14] = f32::from(u8::from(rel_own == 0));
    f[15] = raw_unit(rel_own as f32, 8.0);
    f[42] = 1.0; // commander flag

    let (cid, face_idx) = cmd
        .card
        .map_or((UNKNOWN_CARD, 0), |c| (c.index.get(), c.face));
    f[34] = f32::from(face_idx);
    populate_printed_face_features(cid, face_idx as usize, &mut f);
    f[122] = 1.0;
    f[125] = cmd.casts as f32;

    let raw = pack_object_id(cmd.object);
    state.entity_feat.push(f);
    state.entity_object_ids.push(raw);
    state.entity_group_members.push(vec![raw]);
    state.entity_card_ids.push(cid);
    state.entity_face_ids.push(face_idx);
    state.entity_token_ids.push(0);
    state
        .entity_rule_refs
        .push(crate::cardstructure::refs::card_refs(cid, face_idx, 0));
    state
        .entity_zone_ids
        .push(EntityZone4::CommanderDesignation as u8);
    state.entity_rel_controller.push(rel_own as u8);
}

fn push_remembered_card7(
    state: &mut EncodedState4,
    view: &PlayerView,
    obj_id: u64,
    mem: &KnownCardMemory4,
    viewer: PlayerId,
    seat_count: usize,
) {
    let mut f = vec![0.0_f32; ENTITY_FEAT_WIDTH7];
    f[EntityZone4::RememberedOpponentCard as usize] = 1.0;
    let rel_own = rel_seat(viewer, Some(PlayerId::new(mem.owner_seat)), seat_count);
    f[13] = raw_unit(rel_own as f32, 8.0);
    f[15] = raw_unit(rel_own as f32, 8.0);
    populate_printed_face_features(mem.card_index, 0, &mut f);
    f[122] = 1.0;
    f[125] = view.turn.saturating_sub(mem.last_seen_turn) as f32;
    f[127] = f32::from(mem.origin_zone);

    state.entity_feat.push(f);
    state.entity_object_ids.push(obj_id);
    state.entity_group_members.push(vec![obj_id]);
    state.entity_card_ids.push(mem.card_index);
    state.entity_face_ids.push(0);
    state.entity_token_ids.push(0);
    state
        .entity_rule_refs
        .push(crate::cardstructure::refs::card_refs(mem.card_index, 0, 0));
    state
        .entity_zone_ids
        .push(EntityZone4::RememberedOpponentCard as u8);
    state.entity_rel_controller.push(rel_own as u8);
}

fn populate_printed_face_features(card_id: u32, face_idx: usize, f: &mut [f32]) {
    if card_id == UNKNOWN_CARD {
        return;
    }
    let Some(def) = baylee_cards::by_index(CardIndex::new(card_id)) else {
        return;
    };
    let Some(face) = def.faces.get(face_idx).or_else(|| def.faces.first()) else {
        return;
    };
    f[45] = raw_unit(face.mana_cost.cmc() as f32, 12.0);
    f[46] = f32::from(u8::from(face.power.is_some()));
    f[47] = face.power.map_or(0.0, |p| raw_signed(f32::from(p), 12.0));
    f[48] = face
        .toughness
        .map_or(0.0, |t| raw_signed(f32::from(t), 12.0));
    f[54] = f32::from(u8::from(face.loyalty.is_some()));
    f[55] = face.loyalty.map_or(0.0, |l| raw_unit(f32::from(l), 10.0));
    f[56] = f32::from(u8::from(face.types.contains(TypeSet::CREATURE)));
    f[57] = f32::from(u8::from(face.types.contains(TypeSet::INSTANT)));
    f[58] = f32::from(u8::from(face.types.contains(TypeSet::SORCERY)));
    f[59] = f32::from(u8::from(face.types.contains(TypeSet::ARTIFACT)));
    f[60] = f32::from(u8::from(face.types.contains(TypeSet::ENCHANTMENT)));
    f[61] = f32::from(u8::from(face.types.contains(TypeSet::LAND)));
    f[62] = f32::from(u8::from(face.types.contains(TypeSet::PLANESWALKER)));
    f[63] = f32::from(u8::from(face.types.contains(TypeSet::BATTLE)));
    f[64] = f32::from(u8::from(face.supertypes.contains(SupertypeSet::LEGENDARY)));
    let face_colors = face.mana_cost.colors().union(face.color_indicator);
    f[65] = f32::from(u8::from(face_colors.contains(Color::White)));
    f[66] = f32::from(u8::from(face_colors.contains(Color::Blue)));
    f[67] = f32::from(u8::from(face_colors.contains(Color::Black)));
    f[68] = f32::from(u8::from(face_colors.contains(Color::Red)));
    f[69] = f32::from(u8::from(face_colors.contains(Color::Green)));
    let kw = def.keywords.union(face.keywords);
    f[94] = f32::from(u8::from(kw.contains(KeywordSet::FLYING)));
    f[95] = f32::from(u8::from(!kw.is_empty()));
    encode_keywords7(kw.bits(), f);
}

fn mana_colors_to_set(colors: &[ManaColor]) -> ColorSet {
    let mut s = ColorSet::EMPTY;
    for c in colors {
        match c {
            ManaColor::White => s = s.union(ColorSet::of(Color::White)),
            ManaColor::Blue => s = s.union(ColorSet::of(Color::Blue)),
            ManaColor::Black => s = s.union(ColorSet::of(Color::Black)),
            ManaColor::Red => s = s.union(ColorSet::of(Color::Red)),
            ManaColor::Green => s = s.union(ColorSet::of(Color::Green)),
            ManaColor::Colorless => {}
        }
    }
    s
}

fn mana_spending_flexibility(spending: ManaSpending) -> f32 {
    if spending == ManaSpending::EXACT {
        return 0.0;
    }
    let all = [
        ManaColor::White,
        ManaColor::Blue,
        ManaColor::Black,
        ManaColor::Red,
        ManaColor::Green,
        ManaColor::Colorless,
    ];
    let mut extra = 0_u32;
    for a in all {
        for r in all {
            if a != r && spending.permits(a, r) {
                extra += 1;
            }
        }
    }
    raw_unit(extra as f32, 30.0)
}

fn encode_keywords7(bits: u128, f: &mut [f32]) {
    let kw = |k: KeywordSet| -> bool { (bits & k.bits()) != 0 };
    f[105] = if kw(KeywordSet::DOUBLE_STRIKE) {
        1.0
    } else if kw(KeywordSet::FIRST_STRIKE) {
        0.5
    } else {
        0.0
    };
    f[106] = f32::from(u8::from(kw(KeywordSet::DEATHTOUCH)));
    f[107] = f32::from(u8::from(kw(KeywordSet::HASTE)));
    f[108] = f32::from(u8::from(kw(KeywordSet::HEXPROOF)))
        + 2.0 * f32::from(u8::from(kw(KeywordSet::SHROUD)));
    f[109] = f32::from(u8::from(kw(KeywordSet::INDESTRUCTIBLE)));
    f[110] = f32::from(u8::from(kw(KeywordSet::LIFELINK)));
    f[111] = f32::from(u8::from(kw(KeywordSet::MENACE)))
        + 2.0 * f32::from(u8::from(kw(KeywordSet::UNBLOCKABLE)))
        + 0.5
            * f32::from(u8::from(
                kw(KeywordSet::SHADOW) || kw(KeywordSet::FEAR) || kw(KeywordSet::INTIMIDATE),
            ));
    f[112] = f32::from(u8::from(kw(KeywordSet::REACH)));
    f[113] = f32::from(u8::from(kw(KeywordSet::TRAMPLE)));
    f[114] = f32::from(u8::from(kw(KeywordSet::VIGILANCE)));
    f[115] = f32::from(u8::from(kw(KeywordSet::DEFENDER)))
        + 2.0 * f32::from(u8::from(kw(KeywordSet::CANT_ATTACK)))
        + 4.0 * f32::from(u8::from(kw(KeywordSet::CANT_BLOCK)))
        + 0.5 * f32::from(u8::from(kw(KeywordSet::FLASH)))
        + 0.25 * f32::from(u8::from(kw(KeywordSet::UNCOUNTERABLE)));
}

#[allow(
    clippy::too_many_lines,
    reason = "Contiguous versioned seat feature layout for Encoder 7"
)]
fn encode_seats7(
    state: &mut EncodedState4,
    view: &PlayerView,
    knowledge: Option<&SeatKnowledge4>,
    teams: &[Option<u8>],
    viewer: PlayerId,
    seat_count: usize,
) {
    let my_team = teams.get(viewer.get() as usize).copied().flatten();
    for rel in 0..seat_count.min(MAX_SEATS) {
        let abs = abs_seat(viewer, rel, seat_count);
        let Some(sv) = view.seats.get(abs.get() as usize) else {
            continue;
        };
        let dst = &mut state.seat_feat[rel];
        let seat_team = teams.get(abs.get() as usize).copied().flatten();
        let is_self = rel == 0;
        let is_teammate = !is_self && my_team.is_some() && my_team == seat_team;
        let is_opponent = !is_self && !is_teammate;

        dst[0] = 1.0; // seat present
        dst[1] = f32::from(u8::from(is_self));
        dst[2] = f32::from(u8::from(is_teammate));
        dst[3] = f32::from(u8::from(is_opponent));
        dst[4] = f32::from(u8::from(sv.has_lost()));
        dst[5] = f32::from(u8::from(view.active == abs));
        dst[6] = f32::from(u8::from(view.awaiting == Some(abs)));
        dst[7] = f32::from(u8::from(view.decision_player == Some(abs)));
        dst[8] = f32::from(u8::from(view.monarch == Some(abs)));
        dst[9] = f32::from(u8::from(sv.no_max_hand_size));
        dst[10] = f32::from(u8::from(view.deciding.contains(abs)));

        // Unclipped life, counters, speed, rad
        let start_life = knowledge.map_or(20, |k| k.starting_life[abs.get() as usize]);
        dst[11] = raw_signed(sv.life as f32, 40.0);
        dst[12] = raw_signed((sv.life - start_life) as f32, 40.0);
        dst[13] = raw_unit(f32::from(sv.poison), 10.0);
        dst[14] = raw_unit(f32::from(sv.energy), 10.0);
        dst[15] = raw_unit(f32::from(sv.speed), 4.0);
        dst[16] = raw_unit(f32::from(sv.rad), 10.0);

        // Unclipped zone counts & board aggregates
        dst[17] = raw_unit(sv.hand_count as f32, 7.0);
        dst[18] = raw_unit(sv.library_count as f32, 60.0);
        dst[19] = raw_unit(sv.graveyard_count as f32, 20.0);
        let exile_cnt = view.exile.get(abs.get() as usize).map_or(0, Vec::len);
        dst[20] = raw_unit(exile_cnt as f32, 10.0);

        let mut creatures = 0.0_f32;
        let mut lands = 0.0_f32;
        let mut untapped_lands = 0.0_f32;
        let mut other_perms = 0.0_f32;
        let mut board_power = 0.0_f32;
        let mut board_tough = 0.0_f32;
        for obj in view.battlefield.iter().filter(|o| o.controller == abs) {
            if obj.types.contains(TypeSet::CREATURE) {
                creatures += 1.0;
                board_power += f32::from(obj.power.unwrap_or(0).max(0));
                board_tough += f32::from(obj.toughness.unwrap_or(0).max(0));
            } else if obj.types.contains(TypeSet::LAND) {
                lands += 1.0;
                if !obj.status.contains(ObjectStatus::TAPPED) {
                    untapped_lands += 1.0;
                }
            } else {
                other_perms += 1.0;
            }
        }
        dst[21] = raw_unit(creatures, 10.0);
        dst[22] = raw_unit(lands, 10.0);
        dst[23] = raw_unit(untapped_lands, 10.0);
        dst[24] = raw_unit(other_perms, 10.0);
        dst[25] = raw_unit(board_power, 25.0);
        dst[26] = raw_unit(board_tough, 25.0);

        // Unclipped ManaPoolView: W, U, B, R, G, C, total, spending, restricted[0..6]
        let mp = &sv.mana_pool;
        dst[27] = raw_unit(mp.white as f32, 8.0);
        dst[28] = raw_unit(mp.blue as f32, 8.0);
        dst[29] = raw_unit(mp.black as f32, 8.0);
        dst[30] = raw_unit(mp.red as f32, 8.0);
        dst[31] = raw_unit(mp.green as f32, 8.0);
        dst[32] = raw_unit(mp.colorless as f32, 8.0);
        dst[33] = raw_unit(mp.total() as f32, 12.0);
        dst[34] = f32::from(u8::from(mp.spending != ManaSpending::EXACT));
        for (r_idx, &r_val) in mp.restricted.iter().enumerate().take(6) {
            dst[35 + r_idx] = raw_unit(r_val as f32, 6.0);
        }

        // Commander info & 8-seat commander damage matrix
        dst[41] = raw_unit(sv.commanders.len() as f32, 2.0);
        let cmd_casts: u32 = sv.commanders.iter().map(|c| c.casts).sum();
        dst[42] = raw_unit(cmd_casts as f32, 5.0);
        let max_cmd_dmg = sv
            .commander_damage
            .iter()
            .map(|cd| cd.amount)
            .max()
            .unwrap_or(0);
        dst[43] = raw_unit(f32::from(max_cmd_dmg), 21.0);
        for cd in &sv.commander_damage {
            if let Some((owner_idx, _)) = view
                .seats
                .iter()
                .enumerate()
                .find(|(_, other_s)| other_s.commanders.iter().any(|c| c.object == cd.source))
            {
                let rel_src = rel_seat(viewer, Some(PlayerId::new(owner_idx as u8)), seat_count);
                if rel_src < MAX_SEATS {
                    dst[44 + rel_src] += raw_unit(f32::from(cd.amount), 21.0);
                }
            }
        }

        // Public history statistics & inventory summary [52..64]
        dst[61] = mana_spending_flexibility(mp.spending);
        if let Some(kn) = knowledge {
            let h = &kn.seat_history[abs.get() as usize];
            dst[52] = raw_unit(f32::from(h.spells_cast_total), 20.0);
            dst[53] = raw_unit(f32::from(h.spells_cast_this_turn), 5.0);
            dst[54] = raw_unit(f32::from(h.lands_played_total), 12.0);
            dst[55] = raw_unit(f32::from(h.creatures_entered_total), 15.0);
            dst[56] = raw_unit(f32::from(h.attacks_declared_total), 10.0);
            dst[57] = raw_unit(f32::from(h.life_gained_total), 20.0);
            dst[58] = raw_unit(f32::from(h.life_lost_total), 30.0);
            dst[59] = raw_unit(f32::from(h.graveyard_peak), 20.0);
            let known_in_hand = kn
                .known_cards
                .values()
                .filter(|m| m.owner_seat == abs.get() && m.origin_zone == 1)
                .count();
            dst[60] = raw_unit(known_in_hand as f32, 7.0);
            if is_self {
                dst[62] = raw_unit(kn.inventory.total_remaining_library_cards() as f32, 60.0);
                dst[63] = kn.inventory.unidentified_outside_library() as f32;
            } else {
                let known_top = kn
                    .known_cards
                    .values()
                    .filter(|m| m.owner_seat == abs.get() && m.origin_zone == 2)
                    .count();
                dst[62] = known_top as f32;
            }
        }
    }
}

fn encode_global7(
    state: &mut EncodedState4,
    view: &PlayerView,
    pending: Option<&Pending>,
    knowledge: Option<&SeatKnowledge4>,
    teams: &[Option<u8>],
    viewer: PlayerId,
    seat_count: usize,
) {
    let g = &mut state.global_feat;
    g[0] = raw_unit(view.turn as f32, 20.0);
    let step_idx = match view.step {
        Step::Untap => 0,
        Step::Upkeep => 1,
        Step::Draw => 2,
        Step::Main => {
            if matches!(view.phase, Phase::SecondMain) {
                10
            } else {
                3
            }
        }
        Step::CombatBegin => 4,
        Step::DeclareAttackers => 5,
        Step::DeclareBlockers => 6,
        Step::CombatDamageFirst => 7,
        Step::CombatDamage => 8,
        Step::CombatEnd => 9,
        Step::End => 11,
        Step::Cleanup => 12,
    };
    g[1 + step_idx] = 1.0; // [1..14]

    let rel_act = rel_seat(viewer, Some(view.active), seat_count);
    let rel_awt = rel_seat(viewer, view.awaiting, seat_count);
    g[14] = raw_unit(rel_act as f32, 8.0);
    g[15] = f32::from(u8::from(rel_act == 0));
    g[16] = raw_unit(rel_awt as f32, 8.0);
    g[17] = f32::from(u8::from(rel_awt == 0));
    g[18] = raw_unit(seat_count as f32, 8.0);

    let surviving = view.seats.iter().filter(|s| !s.has_lost()).count();
    g[19] = raw_unit(surviving as f32, 8.0);
    let is_team = teams.iter().any(Option::is_some);
    let is_cmd = view.seats.iter().any(|s| !s.commanders.is_empty());
    g[20] = f32::from(u8::from(is_team));
    g[21] = f32::from(u8::from(is_cmd));
    g[22] = match view.day_night {
        None => 0.0,
        Some(DayNight::Day) => 0.5,
        Some(DayNight::Night) => 1.0,
    };
    g[23] = raw_unit(view.stack.len() as f32, 6.0);
    g[24] = f32::from(u8::from(view.combat.is_active()));
    g[25] = raw_unit(view.combat.attackers.len() as f32, 10.0);
    g[26] = raw_unit(view.combat.blockers.len() as f32, 10.0);
    g[27] = f32::from(u8::from(view.sorcery_lock.is_some()));
    g[28] = f32::from(u8::from(view.sorceries_have_flash));
    g[29] = f32::from(u8::from(view.casting.is_some()));
    g[30] = f32::from(u8::from(view.targeting.is_some()));

    // Owed payment window (CR 605.3a): both legacy summary [31..34] and exact symbol breakdown [64..76]
    match &view.owed {
        None => {}
        Some(ManaPayment::Fixed(mc)) => {
            g[31] = 1.0;
            g[33] = raw_unit(mc.cmc() as f32, 10.0);
            for sym in mc.symbols() {
                match sym {
                    ManaSymbol::White => g[64] += 1.0,
                    ManaSymbol::Blue => g[65] += 1.0,
                    ManaSymbol::Black => g[66] += 1.0,
                    ManaSymbol::Red => g[67] += 1.0,
                    ManaSymbol::Green => g[68] += 1.0,
                    ManaSymbol::Colorless => g[69] += 1.0,
                    ManaSymbol::Generic(n) => g[70] += n as f32,
                    ManaSymbol::Hybrid(_) | ManaSymbol::TwoOrColor(_) => g[71] += 1.0,
                    ManaSymbol::Phyrexian(_) | ManaSymbol::HybridPhyrexian(_) => g[72] += 1.0,
                    ManaSymbol::Snow => g[73] += 1.0,
                    ManaSymbol::Variable(_) => g[74] += 1.0,
                    ManaSymbol::HalfGeneric | ManaSymbol::Infinite => g[70] += 0.5,
                }
            }
            g[75] = mc.cmc() as f32;
        }
        Some(ManaPayment::AnyAmount { preventable_damage }) => {
            g[32] = 1.0;
            g[33] = raw_unit(*preventable_damage as f32, 10.0);
            g[75] = *preventable_damage as f32;
        }
    }

    if let Some(tc) = &view.targeting {
        g[76] = f32::from(u8::from(tc.whole_spell)) + 2.0 * f32::from(u8::from(tc.second));
        g[77] = tc.batch_count as f32;
    }

    if let Some(p) = pending {
        let pk = PendingKind4::of(p);
        g[34] = raw_unit(f32::from(pk as u8), 28.0);
    }
    if let Some(kn) = knowledge {
        g[35] = raw_unit(kn.cursor as f32, 200.0);
        g[40..64].copy_from_slice(&kn.own_deck_summary);
        g[78] = kn.inventory.total_remaining_library_cards() as f32;
        g[79] = kn
            .inventory
            .remaining_sideboard_groups()
            .iter()
            .map(|grp| grp.remaining_copies)
            .sum::<u32>() as f32;
    }
}

#[allow(
    clippy::too_many_lines,
    reason = "Explicit relation schema and deduplicated edge ordering for grouped entities"
)]
fn build_edges7(state: &mut EncodedState4, view: &PlayerView, viewer: PlayerId, seat_count: usize) {
    let mut edges = std::mem::take(&mut state.edges);
    let obj_token = |id: ObjectId| -> Option<i16> {
        state
            .find_entity(id)
            .map(|idx| ENTITY_TOKEN_OFFSET + idx as i16)
    };
    let mut seen_edges = BTreeSet::<[i16; 3]>::new();
    let mut push_edge = |src: i16, dst: i16, rel: EdgeRel4| {
        if edges.len() < MAX_EDGES {
            let edge = [src, dst, rel as i16];
            if seen_edges.insert(edge) {
                edges.push(edge);
            }
        }
    };
    let seat_token = |p: PlayerId| -> i16 {
        let rel = rel_seat(viewer, Some(p), seat_count);
        SEAT_TOKEN_OFFSET + (rel.min(MAX_SEATS - 1) as i16)
    };

    // 1. Entity relationships on battlefield & stack
    for obj in view.battlefield.iter().chain(&view.stack) {
        let Some(src_tok) = obj_token(obj.id) else {
            continue;
        };
        push_edge(src_tok, seat_token(obj.controller), EdgeRel4::Controller);
        if obj.owner != obj.controller {
            push_edge(src_tok, seat_token(obj.owner), EdgeRel4::Owner);
        }
        if let Some(parent) = obj.attached_to
            && let Some(dst_tok) = obj_token(parent)
        {
            push_edge(src_tok, dst_tok, EdgeRel4::AttachedTo);
        }
        if let Some(prot) = obj.battle_protector {
            push_edge(src_tok, seat_token(prot), EdgeRel4::ProtectorSeat);
        }
        if let Some(c_opp) = obj.chosen_opponent {
            push_edge(src_tok, seat_token(c_opp), EdgeRel4::ChosenOpponentSeat);
        }
        for grant in &obj.grants {
            if let Some(grantor) = grant.source
                && let Some(dst_tok) = obj_token(grantor)
            {
                push_edge(src_tok, dst_tok, EdgeRel4::GrantSource);
            }
        }
        if let Some(StackItem::Ability { source, .. }) = obj.stack_item
            && let Some(dst_tok) = obj_token(source)
        {
            push_edge(src_tok, dst_tok, EdgeRel4::StackSource);
        }
        for &t in &obj.targets {
            match t {
                TargetRef::Object(ds) => {
                    if let Some(dst_tok) = obj_token(ds.object) {
                        push_edge(src_tok, dst_tok, EdgeRel4::StackTarget);
                    }
                }
                TargetRef::Player(pid) => {
                    push_edge(src_tok, seat_token(pid), EdgeRel4::StackTarget);
                }
            }
        }
    }

    // 2. Combat edges: attackers, blockers, bands
    for atk in &view.combat.attackers {
        if let Some(atk_tok) = obj_token(atk.creature) {
            let dst_tok = match atk.defending {
                Defender::Player(pid) => Some(seat_token(pid)),
                Defender::Planeswalker(oid) | Defender::Battle(oid) => obj_token(oid),
            };
            if let Some(dst) = dst_tok {
                push_edge(atk_tok, dst, EdgeRel4::Attacking);
            }
        }
    }
    for blk in &view.combat.blockers {
        if let (Some(b_tok), Some(a_tok)) = (obj_token(blk.blocker), obj_token(blk.attacker)) {
            push_edge(b_tok, a_tok, EdgeRel4::Blocking);
            push_edge(a_tok, b_tok, EdgeRel4::BlockedBy);
        }
    }
    for band in &view.combat.bands {
        for i in 0..band.len() {
            for j in (i + 1)..band.len() {
                if let (Some(ti), Some(tj)) = (obj_token(band[i]), obj_token(band[j]))
                    && ti != tj
                {
                    push_edge(ti, tj, EdgeRel4::BandMember);
                    push_edge(tj, ti, EdgeRel4::BandMember);
                }
            }
        }
    }

    // 3. Targeting context source -> Global token (0)
    if let Some(tc) = &view.targeting
        && let Some(src_tok) = obj_token(tc.source.id)
    {
        push_edge(src_tok, 0, EdgeRel4::TargetingContextSource);
    }

    // 4. Same-card-copy edges between entities sharing a CardIndex (connecting board/hand/stack/inventory)
    let n_ent = state.entity_card_ids.len();
    for i in 0..n_ent {
        let cid = state.entity_card_ids[i];
        if cid == UNKNOWN_CARD {
            continue;
        }
        for j in (i + 1)..n_ent {
            if state.entity_card_ids[j] == cid {
                let ti = ENTITY_TOKEN_OFFSET + i as i16;
                let tj = ENTITY_TOKEN_OFFSET + j as i16;
                push_edge(ti, tj, EdgeRel4::SameCardCopy);
                break;
            }
        }
    }
    state.edges = edges;
}
