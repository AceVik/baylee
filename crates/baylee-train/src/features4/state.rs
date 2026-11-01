/// Zone identifiers for tokenized entities (`0..12`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum EntityZone4 {
    /// Shared battlefield (`0`).
    Battlefield = 0,
    /// The stack (`1`).
    Stack = 1,
    /// Viewing seat's own hand (`2`).
    Hand = 2,
    /// Voluntarily or publicly shared hand (`3`).
    SharedHand = 3,
    /// Controlled player's hand (`4`).
    ControlledHand = 4,
    /// Graveyard (`5`).
    Graveyard = 5,
    /// Public exile (`6`).
    Exile = 6,
    /// Command zone (`7`).
    Command = 7,
    /// Temporary inspection zone (`PlayerView::looking_at`, `8`).
    LookingAt = 8,
    /// Revealed top of library (`PlayerView::library_tops`, `9`).
    LibraryTop = 9,
    /// Designated commander tracker (`SeatView::commanders`, `10`).
    CommanderDesignation = 10,
    /// Previously revealed opponent card remembered by `SeatKnowledge4` (`11`).
    RememberedOpponentCard = 11,
    /// Viewing seat's remaining library card group from `InventoryLedger7` (`12`).
    DeckInventory = 12,
    /// Viewing seat's remaining sideboard wish group from `InventoryLedger7` (`13`).
    SideboardInventory = 13,
}

/// Directed edge relation kinds (`0..14`) between tokens in the unified
/// transformer sequence (`0` = Global, `1..=8` = Relative Seats `0..7`,
/// `9..` = Entities `0..num_entities`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[repr(i16)]
pub enum EdgeRel4 {
    /// `entity -> controller_seat` (`0`).
    Controller = 0,
    /// `entity -> owner_seat` (`1`).
    Owner = 1,
    /// `entity -> attached_to_entity` (`2`).
    AttachedTo = 2,
    /// `attacker_entity -> defending_seat_or_entity` (`3`).
    Attacking = 3,
    /// `blocker_entity -> blocked_attacker_entity` (`4`).
    Blocking = 4,
    /// `attacker_entity -> blocking_entity` (`5`).
    BlockedBy = 5,
    /// `stack_entity -> target_entity_or_seat` (`6`).
    StackTarget = 6,
    /// `stack_entity -> source_entity` (`7`).
    StackSource = 7,
    /// `granted_entity -> grantor_entity` (`8`).
    GrantSource = 8,
    /// `band_member_entity -> peer_band_member_entity` (`9`).
    BandMember = 9,
    /// `battle_entity -> protector_seat` (`10`).
    ProtectorSeat = 10,
    /// `entity -> chosen_opponent_seat` (`11`).
    ChosenOpponentSeat = 11,
    /// `entity -> other_entity_sharing_card_index` (`12`).
    SameCardCopy = 12,
    /// `targeting_source_entity -> global_token` (`13`).
    TargetingContextSource = 13,
}

/// Complete v4 encoded state frame (`EncodedState4`).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EncodedState4 {
    /// Per-entity numeric features (`num_entities x 96`).
    pub entity_feat: Vec<Vec<f32>>,
    /// Engine `ObjectId` for each entity row (`0` for synthetic memory rows).
    pub entity_object_ids: Vec<u64>,
    /// Packed `ObjectId`s of all members when a battlefield entity represents a permanent group.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub entity_group_members: Vec<Vec<u64>>,
    /// Active rules `CardIndex` (`rules.card` when copying, else `card.index`, `UNKNOWN_CARD` if unknown/token).
    pub entity_card_ids: Vec<u32>,
    /// Active face (`0` or `1`).
    pub entity_face_ids: Vec<u8>,
    /// Registry token ID (`0` if not a registry token).
    pub entity_token_ids: Vec<u16>,
    /// All public printed/selected/granted rule references; no ability cap.
    pub entity_rule_refs: Vec<Vec<crate::cardstructure::refs::RuleRef>>,
    /// Zone ID (`EntityZone4 as u8`).
    pub entity_zone_ids: Vec<u8>,
    /// Relative controller seat (`0..7`, `8` = unknown).
    pub entity_rel_controller: Vec<u8>,
    /// Per-seat features in relative seat order `0..7` (`8 x 64`).
    pub seat_feat: Vec<Vec<f32>>,
    /// Global table/turn/payment/deck features (`64`).
    pub global_feat: Vec<f32>,
    /// Directed relational edges `[src_token, dst_token, rel_kind]`.
    pub edges: Vec<[i16; 3]>,
}

impl Default for EncodedState4 {
    fn default() -> Self {
        Self {
            entity_feat: Vec::new(),
            entity_object_ids: Vec::new(),
            entity_group_members: Vec::new(),
            entity_card_ids: Vec::new(),
            entity_face_ids: Vec::new(),
            entity_token_ids: Vec::new(),
            entity_rule_refs: Vec::new(),
            entity_zone_ids: Vec::new(),
            entity_rel_controller: Vec::new(),
            seat_feat: vec![vec![0.0; SEAT_FEAT_WIDTH]; MAX_SEATS],
            global_feat: vec![0.0; GLOBAL_FEAT_WIDTH],
            edges: Vec::new(),
        }
    }
}

impl EncodedState4 {
    /// Finds the entity index (`0..entity_object_ids.len()`) for an [`ObjectId`].
    #[must_use]
    pub fn find_entity(&self, id: ObjectId) -> Option<usize> {
        let raw = pack_object_id(id);
        if raw == u64::MAX {
            return None;
        }
        if !self.entity_group_members.is_empty()
            && let Some(pos) = self
                .entity_group_members
                .iter()
                .position(|members| members.contains(&raw))
        {
            return Some(pos);
        }
        self.entity_object_ids
            .iter()
            .position(|&oid| oid != u64::MAX && oid == raw)
    }

    /// Materializes the `CARD_DSL_WIDTH` (`160`) vector for entity `idx` using
    /// `entity_card_ids`, `entity_face_ids`, and `entity_token_ids` (`0` = none,
    /// `1 + token_id` = registry token).
    #[must_use]
    pub fn entity_dsl_summary(&self, idx: usize) -> [f32; CARD_DSL_WIDTH] {
        let card_id = self
            .entity_card_ids
            .get(idx)
            .copied()
            .unwrap_or(UNKNOWN_CARD);
        let face_id = self.entity_face_ids.get(idx).copied().unwrap_or(0) as usize;
        if card_id != UNKNOWN_CARD
            && let Some(def) = baylee_cards::by_index(CardIndex::new(card_id))
        {
            return encode_card_face(def, face_id).summary;
        }
        let tok_slot = self.entity_token_ids.get(idx).copied().unwrap_or(0);
        if tok_slot != 0
            && let Some(tok) = baylee_cards::tokens::by_token_id(tok_slot - 1)
        {
            return encode_token_def(tok).summary;
        }
        [0.0; CARD_DSL_WIDTH]
    }
}

/// Encodes a [`PlayerView`] (plus optional [`Pending`] and [`SeatKnowledge4`])
/// into an [`EncodedState4`].
#[must_use]
#[allow(
    clippy::too_many_lines,
    reason = "Ordered zone traversal is part of the versioned encoder"
)]
pub fn encode_state4(
    view: &PlayerView,
    pending: Option<&Pending>,
    knowledge: Option<&SeatKnowledge4>,
    teams: &[Option<u8>],
) -> EncodedState4 {
    let mut state = EncodedState4::default();
    let viewer = view.seat;
    let seat_count = view.seats.len().clamp(1, MAX_SEATS);

    // 1. Collect entities in deterministic priority order up to MAX_ENTITIES:
    //    Battlefield -> Stack -> Own Hand -> Shared/Controlled Hands ->
    //    LookingAt -> LibraryTops -> Command -> Graveyards -> Exile ->
    //    Known opponent cards from SeatKnowledge4.
    for obj in &view.battlefield {
        push_public_object(
            &mut state,
            view,
            obj,
            EntityZone4::Battlefield,
            viewer,
            seat_count,
            pending,
        );
    }
    for obj in &view.stack {
        push_public_object(
            &mut state,
            view,
            obj,
            EntityZone4::Stack,
            viewer,
            seat_count,
            pending,
        );
    }
    for h_obj in &view.hand {
        push_hand_object(
            &mut state,
            h_obj,
            viewer,
            EntityZone4::Hand,
            viewer,
            seat_count,
            pending,
        );
    }
    for sh in &view.shared_hands {
        for h_obj in &sh.cards {
            push_hand_object(
                &mut state,
                h_obj,
                sh.player,
                EntityZone4::SharedHand,
                viewer,
                seat_count,
                pending,
            );
        }
    }
    for ch in &view.controlled_hands {
        for h_obj in &ch.cards {
            push_hand_object(
                &mut state,
                h_obj,
                ch.player,
                EntityZone4::ControlledHand,
                viewer,
                seat_count,
                pending,
            );
        }
    }
    for obj in &view.looking_at {
        push_public_object(
            &mut state,
            view,
            obj,
            EntityZone4::LookingAt,
            viewer,
            seat_count,
            pending,
        );
    }
    for obj in &view.library_tops {
        push_public_object(
            &mut state,
            view,
            obj,
            EntityZone4::LibraryTop,
            viewer,
            seat_count,
            pending,
        );
    }
    for seat_cmd in &view.command {
        for obj in seat_cmd {
            push_public_object(
                &mut state,
                view,
                obj,
                EntityZone4::Command,
                viewer,
                seat_count,
                pending,
            );
        }
    }
    for seat_gy in &view.graveyards {
        for obj in seat_gy {
            push_public_object(
                &mut state,
                view,
                obj,
                EntityZone4::Graveyard,
                viewer,
                seat_count,
                pending,
            );
        }
    }
    for seat_ex in &view.exile {
        for obj in seat_ex {
            push_public_object(
                &mut state,
                view,
                obj,
                EntityZone4::Exile,
                viewer,
                seat_count,
                pending,
            );
        }
    }
    if let Some(kn) = knowledge {
        for (&(_owner, obj_id), mem) in &kn.known_cards {
            if state.entity_object_ids.len() >= MAX_ENTITIES {
                break;
            }
            if state.entity_object_ids.contains(&obj_id) {
                continue;
            }
            push_remembered_card(&mut state, obj_id, mem, viewer, seat_count);
        }
    }

    // 2. Encode Seat features in relative seat order `0..seat_count`
    encode_seats4(&mut state, view, knowledge, teams, viewer, seat_count);

    // 3. Encode Global features
    encode_global4(
        &mut state, view, pending, knowledge, teams, viewer, seat_count,
    );

    // 4. Build directed relational edges
    build_edges4(&mut state, view, viewer, seat_count);

    state
}

/// Entity feature 27: `legal` offers to cast `id`, or to exile it from hand
/// by a special action for a later cast (suspend, foretell).
pub(crate) fn offered_to_cast(legal: &LegalActions, id: ObjectId) -> bool {
    legal.castable.contains(&id)
        || legal.suspendable.contains(&id)
        || legal.foretellable.contains(&id)
}

#[allow(
    clippy::too_many_lines,
    reason = "Contiguous versioned public-object feature layout"
)]
fn push_public_object(
    state: &mut EncodedState4,
    view: &PlayerView,
    obj: &PublicObject,
    zone: EntityZone4,
    viewer: PlayerId,
    seat_count: usize,
    pending: Option<&Pending>,
) {
    if state.entity_feat.len() >= MAX_ENTITIES {
        return;
    }
    let mut f = vec![0.0_f32; ENTITY_FEAT_WIDTH];
    // [0..12]: zone one-hot
    f[zone as usize] = 1.0;

    let rel_ctrl = rel_seat(viewer, Some(obj.controller), seat_count);
    let rel_own = rel_seat(viewer, Some(obj.owner), seat_count);
    let rel_prot = rel_seat(viewer, obj.battle_protector, seat_count);
    let rel_c_opp = rel_seat(viewer, obj.chosen_opponent, seat_count);

    // [12..20]: relative seat relationships
    f[12] = f32::from(u8::from(rel_ctrl == 0));
    f[13] = unit(rel_ctrl as f32, 8.0);
    f[14] = f32::from(u8::from(rel_own == 0));
    f[15] = unit(rel_own as f32, 8.0);
    f[16] = f32::from(u8::from(obj.battle_protector.is_some()));
    f[17] = unit(rel_prot as f32, 8.0);
    f[18] = f32::from(u8::from(obj.chosen_opponent.is_some()));
    f[19] = unit(rel_c_opp as f32, 8.0);

    // [20..27]: all 7 ObjectStatus bits
    let st = obj.status;
    f[20] = f32::from(u8::from(st.contains(ObjectStatus::TAPPED)));
    f[21] = f32::from(u8::from(st.contains(ObjectStatus::FACE_DOWN)));
    f[22] = f32::from(u8::from(st.contains(ObjectStatus::PHASED_OUT)));
    f[23] = f32::from(u8::from(st.contains(ObjectStatus::FLIPPED)));
    f[24] = f32::from(u8::from(st.contains(ObjectStatus::NO_ABILITIES)));
    f[25] = f32::from(u8::from(st.contains(ObjectStatus::MELDED_CARD)));
    f[26] = f32::from(u8::from(st.contains(ObjectStatus::TOKEN)));

    // [27..34]: all 6 Offered bits + numeric picked derived from Pending
    if let Some(p) = pending {
        match p {
            Pending::Priority { legal, .. } => {
                if offered_to_cast(legal, obj.id) {
                    f[27] = 1.0;
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
    f[45] = unit(obj.mana_value as f32, 12.0);

    // [46..56]: projected & base P/T, damage, remaining toughness, lethal, loyalty
    f[46] = f32::from(u8::from(obj.power.is_some()));
    f[47] = obj.power.map_or(0.0, |p| signed_unit(f32::from(p), 12.0));
    f[48] = obj
        .toughness
        .map_or(0.0, |t| signed_unit(f32::from(t), 12.0));
    f[49] = obj
        .base_power
        .map_or(0.0, |p| signed_unit(f32::from(p), 12.0));
    f[50] = obj
        .base_toughness
        .map_or(0.0, |t| signed_unit(f32::from(t), 12.0));
    f[51] = unit(f32::from(obj.damage), 10.0);
    f[52] = obj
        .remaining_toughness()
        .map_or(0.0, |r| signed_unit(f32::from(r), 12.0));
    f[53] = f32::from(u8::from(obj.is_lethally_damaged()));
    f[54] = f32::from(u8::from(obj.loyalty.is_some()));
    f[55] = obj.loyalty.map_or(0.0, |l| unit(f32::from(l), 10.0));

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

    // [70..82]: CounterKind categories (all 21 mapped into 12 dense channels)
    for entry in &obj.counters {
        let amt = f32::from(entry.count);
        match entry.kind {
            CounterKind::Plus { power, toughness } => {
                f[70] = (f[70] + unit(amt * f32::from(power.max(toughness).max(1)), 8.0)).min(1.0);
            }
            CounterKind::Minus { power, toughness } => {
                f[71] = (f[71] + unit(amt * f32::from(power.max(toughness).max(1)), 8.0)).min(1.0);
            }
            CounterKind::Loyalty => f[72] = unit(amt, 10.0),
            CounterKind::Defense => f[73] = unit(amt, 10.0),
            CounterKind::Lore | CounterKind::Level => f[74] = unit(amt, 6.0),
            CounterKind::Time => f[75] = unit(amt, 6.0),
            CounterKind::Charge | CounterKind::Bore | CounterKind::Energy => f[76] = unit(amt, 8.0),
            CounterKind::Stun => f[77] = unit(amt, 4.0),
            CounterKind::Shield | CounterKind::Indestructible => f[78] = unit(amt, 4.0),
            CounterKind::Finality => f[79] = unit(amt, 2.0),
            CounterKind::Flying
            | CounterKind::Lifelink
            | CounterKind::Shadow
            | CounterKind::Hone => f[80] = unit(amt, 4.0),
            CounterKind::Poison | CounterKind::Rad | CounterKind::Custom(_) => {
                f[81] = (f[81] + unit(amt, 6.0)).min(1.0);
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
        .map_or(0.0, |mc| unit(mc.cmc() as f32, 10.0));
    f[88] = unit(obj.grants.len() as f32, 4.0);
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
            f[92] = unit(ab_idx as f32, 8.0);
        }
        None => {}
    }
    f[93] = unit(obj.targets.len() as f32, 4.0);
    f[94] = f32::from(u8::from((obj.keywords & (1 << 0)) != 0)); // Flying
    f[95] = f32::from(u8::from(obj.keywords != 0));

    let token_slot = obj.token.map_or(0, |id| {
        if (id as usize) < baylee_cards::tokens::ALL.len() {
            id.saturating_add(1)
        } else {
            0
        }
    });
    state.entity_feat.push(f);
    state.entity_object_ids.push(pack_object_id(obj.id));
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

fn push_hand_object(
    state: &mut EncodedState4,
    h_obj: &HandObject,
    owner: PlayerId,
    zone: EntityZone4,
    viewer: PlayerId,
    seat_count: usize,
    pending: Option<&Pending>,
) {
    if state.entity_feat.len() >= MAX_ENTITIES {
        return;
    }
    let mut f = vec![0.0_f32; ENTITY_FEAT_WIDTH];
    f[zone as usize] = 1.0;
    let rel_own = rel_seat(viewer, Some(owner), seat_count);
    f[12] = f32::from(u8::from(rel_own == 0));
    f[13] = unit(rel_own as f32, 8.0);
    f[14] = f32::from(u8::from(rel_own == 0));
    f[15] = unit(rel_own as f32, 8.0);

    // Check if castable or playable land in current Priority pending
    if let Some(Pending::Priority { legal, .. }) = pending {
        if offered_to_cast(legal, h_obj.id) {
            f[27] = 1.0;
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
    f[45] = unit(h_obj.mana_value as f32, 12.0);

    if let Some(def) = baylee_cards::by_index(h_obj.card.index)
        && let Some(face) = def
            .faces
            .get(h_obj.card.face as usize)
            .or_else(|| def.faces.first())
    {
        f[46] = f32::from(u8::from(face.power.is_some()));
        f[47] = face.power.map_or(0.0, |p| signed_unit(f32::from(p), 12.0));
        f[48] = face
            .toughness
            .map_or(0.0, |t| signed_unit(f32::from(t), 12.0));
        f[54] = f32::from(u8::from(face.loyalty.is_some()));
        f[55] = face.loyalty.map_or(0.0, |l| unit(f32::from(l), 10.0));
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

    state.entity_feat.push(f);
    state.entity_object_ids.push(pack_object_id(h_obj.id));
    state.entity_card_ids.push(h_obj.card.index.get());
    state.entity_face_ids.push(h_obj.card.face);
    state.entity_token_ids.push(0);
    state
        .entity_rule_refs
        .push(crate::cardstructure::refs::card_refs(
            h_obj.card.index.get(),
            h_obj.card.face,
            0,
        ));
    state.entity_zone_ids.push(zone as u8);
    state.entity_rel_controller.push(rel_own as u8);
}

fn push_remembered_card(
    state: &mut EncodedState4,
    obj_id: u64,
    mem: &KnownCardMemory4,
    viewer: PlayerId,
    seat_count: usize,
) {
    let mut f = vec![0.0_f32; ENTITY_FEAT_WIDTH];
    f[EntityZone4::RememberedOpponentCard as usize] = 1.0;
    let rel_own = rel_seat(viewer, Some(PlayerId::new(mem.owner_seat)), seat_count);
    f[13] = unit(rel_own as f32, 8.0);
    f[15] = unit(rel_own as f32, 8.0);
    if let Some(def) = baylee_cards::by_index(CardIndex::new(mem.card_index))
        && let Some(face) = def.faces.first()
    {
        f[45] = unit(face.mana_cost.cmc() as f32, 12.0);
        f[56] = f32::from(u8::from(face.types.contains(TypeSet::CREATURE)));
        f[57] = f32::from(u8::from(face.types.contains(TypeSet::INSTANT)));
        f[58] = f32::from(u8::from(face.types.contains(TypeSet::SORCERY)));
        f[59] = f32::from(u8::from(face.types.contains(TypeSet::ARTIFACT)));
        f[60] = f32::from(u8::from(face.types.contains(TypeSet::ENCHANTMENT)));
        f[61] = f32::from(u8::from(face.types.contains(TypeSet::LAND)));
        f[62] = f32::from(u8::from(face.types.contains(TypeSet::PLANESWALKER)));
    }
    state.entity_feat.push(f);
    state.entity_object_ids.push(obj_id);
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

#[allow(
    clippy::too_many_lines,
    reason = "Contiguous versioned seat feature layout"
)]
fn encode_seats4(
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

        // Life, counters, speed, rad
        let start_life = knowledge.map_or(20, |k| k.starting_life[abs.get() as usize]);
        dst[11] = signed_unit(sv.life as f32, 40.0);
        dst[12] = signed_unit((sv.life - start_life) as f32, 40.0);
        dst[13] = unit(f32::from(sv.poison), 10.0);
        dst[14] = unit(f32::from(sv.energy), 10.0);
        dst[15] = unit(f32::from(sv.speed), 4.0);
        dst[16] = unit(f32::from(sv.rad), 10.0);

        // Zone counts & board aggregates
        dst[17] = unit(sv.hand_count as f32, 7.0);
        dst[18] = unit(sv.library_count as f32, 60.0);
        dst[19] = unit(sv.graveyard_count as f32, 20.0);
        let exile_cnt = view.exile.get(abs.get() as usize).map_or(0, Vec::len);
        dst[20] = unit(exile_cnt as f32, 10.0);

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
        dst[21] = unit(creatures, 10.0);
        dst[22] = unit(lands, 10.0);
        dst[23] = unit(untapped_lands, 10.0);
        dst[24] = unit(other_perms, 10.0);
        dst[25] = unit(board_power, 25.0);
        dst[26] = unit(board_tough, 25.0);

        // ManaPoolView: W, U, B, R, G, C, total, spending, restricted[0..6]
        let mp = &sv.mana_pool;
        dst[27] = unit(mp.white as f32, 8.0);
        dst[28] = unit(mp.blue as f32, 8.0);
        dst[29] = unit(mp.black as f32, 8.0);
        dst[30] = unit(mp.red as f32, 8.0);
        dst[31] = unit(mp.green as f32, 8.0);
        dst[32] = unit(mp.colorless as f32, 8.0);
        dst[33] = unit(mp.total() as f32, 12.0);
        dst[34] = f32::from(u8::from(mp.spending != ManaSpending::EXACT));
        for (r_idx, &r_val) in mp.restricted.iter().enumerate().take(6) {
            dst[35 + r_idx] = unit(r_val as f32, 6.0);
        }

        // Commander info & 8-seat commander damage matrix
        dst[41] = unit(sv.commanders.len() as f32, 2.0);
        let cmd_casts: u32 = sv.commanders.iter().map(|c| c.casts).sum();
        dst[42] = unit(cmd_casts as f32, 5.0);
        let max_cmd_dmg = sv
            .commander_damage
            .iter()
            .map(|cd| cd.amount)
            .max()
            .unwrap_or(0);
        dst[43] = unit(f32::from(max_cmd_dmg), 21.0);
        // Map each CommanderDamage source back to the relative seat of that commander's owner
        for cd in &sv.commander_damage {
            if let Some((owner_idx, _)) = view
                .seats
                .iter()
                .enumerate()
                .find(|(_, other_s)| other_s.commanders.iter().any(|c| c.object == cd.source))
            {
                let rel_src = rel_seat(viewer, Some(PlayerId::new(owner_idx as u8)), seat_count);
                if rel_src < MAX_SEATS {
                    dst[44 + rel_src] =
                        (dst[44 + rel_src] + unit(f32::from(cd.amount), 21.0)).min(1.0);
                }
            }
        }

        // Public history statistics from SeatKnowledge4 [52..64]
        if let Some(kn) = knowledge {
            let h = &kn.seat_history[abs.get() as usize];
            dst[52] = unit(f32::from(h.spells_cast_total), 20.0);
            dst[53] = unit(f32::from(h.spells_cast_this_turn), 5.0);
            dst[54] = unit(f32::from(h.lands_played_total), 12.0);
            dst[55] = unit(f32::from(h.creatures_entered_total), 15.0);
            dst[56] = unit(f32::from(h.attacks_declared_total), 10.0);
            dst[57] = unit(f32::from(h.life_gained_total), 20.0);
            dst[58] = unit(f32::from(h.life_lost_total), 30.0);
            dst[59] = unit(f32::from(h.graveyard_peak), 20.0);
            let known_in_hand = kn
                .known_cards
                .values()
                .filter(|m| m.owner_seat == abs.get() && m.origin_zone == 1)
                .count();
            dst[60] = unit(known_in_hand as f32, 7.0);
        }
    }
}

fn encode_global4(
    state: &mut EncodedState4,
    view: &PlayerView,
    pending: Option<&Pending>,
    knowledge: Option<&SeatKnowledge4>,
    teams: &[Option<u8>],
    viewer: PlayerId,
    seat_count: usize,
) {
    let g = &mut state.global_feat;
    g[0] = unit(view.turn as f32, 20.0);
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
    g[14] = unit(rel_act as f32, 8.0);
    g[15] = f32::from(u8::from(rel_act == 0));
    g[16] = unit(rel_awt as f32, 8.0);
    g[17] = f32::from(u8::from(rel_awt == 0));
    g[18] = unit(seat_count as f32, 8.0);

    let surviving = view.seats.iter().filter(|s| !s.has_lost()).count();
    g[19] = unit(surviving as f32, 8.0);
    let is_team = teams.iter().any(Option::is_some);
    let is_cmd = view.seats.iter().any(|s| !s.commanders.is_empty());
    g[20] = f32::from(u8::from(is_team));
    g[21] = f32::from(u8::from(is_cmd));
    g[22] = match view.day_night {
        None => 0.0,
        Some(DayNight::Day) => 0.5,
        Some(DayNight::Night) => 1.0,
    };
    g[23] = unit(view.stack.len() as f32, 6.0);
    g[24] = f32::from(u8::from(view.combat.is_active()));
    g[25] = unit(view.combat.attackers.len() as f32, 10.0);
    g[26] = unit(view.combat.blockers.len() as f32, 10.0);
    g[27] = f32::from(u8::from(view.sorcery_lock.is_some()));
    g[28] = f32::from(u8::from(view.sorceries_have_flash));
    g[29] = f32::from(u8::from(view.casting.is_some()));
    g[30] = f32::from(u8::from(view.targeting.is_some()));

    // Owed payment window (CR 605.3a)
    match &view.owed {
        None => {}
        Some(ManaPayment::Fixed(mc)) => {
            g[31] = 1.0;
            g[33] = unit(mc.cmc() as f32, 10.0);
        }
        Some(ManaPayment::AnyAmount { preventable_damage }) => {
            g[32] = 1.0;
            g[33] = unit(*preventable_damage as f32, 10.0);
        }
    }

    if let Some(p) = pending {
        let pk = PendingKind4::of(p);
        g[34] = unit(f32::from(pk as u8), 28.0);
    }
    if let Some(kn) = knowledge {
        g[35] = unit(kn.cursor as f32, 200.0);
        // [40..64]: 24-dim own deck summary
        g[40..64].copy_from_slice(&kn.own_deck_summary);
    }
}

#[allow(
    clippy::too_many_lines,
    reason = "Explicit relation schema and stable edge ordering"
)]
fn build_edges4(state: &mut EncodedState4, view: &PlayerView, viewer: PlayerId, seat_count: usize) {
    let entity_ids = state.entity_object_ids.clone();
    let obj_token = |id: ObjectId| -> Option<i16> {
        let raw = pack_object_id(id);
        entity_ids
            .iter()
            .position(|&oid| oid == raw)
            .map(|idx| ENTITY_TOKEN_OFFSET + idx as i16)
    };
    let mut push_edge = |src: i16, dst: i16, rel: EdgeRel4| {
        if state.edges.len() < MAX_EDGES {
            state.edges.push([src, dst, rel as i16]);
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
                if let (Some(ti), Some(tj)) = (obj_token(band[i]), obj_token(band[j])) {
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

    // 4. Same-card-copy edges between battlefield/hand/stack entities sharing a CardIndex
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
}
