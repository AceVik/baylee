//! The snapshot hash and the loop signature, and everything they write.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary
use super::*;

impl GameState {
    /// Streaming xxh3 hash over the entire deterministic state.
    ///
    /// Every struct on the way is taken apart by name, here and in the
    /// helpers it calls, so a field added tomorrow does not compile until it
    /// is either hashed or bound to `_` beside the reason it is left out. The
    /// list this replaced was written by hand and went blind to every field
    /// added after it, among them the X, the kicker and the chosen mode of a
    /// spell on the stack (#122). A type whose every field counts derives
    /// `Hash` instead, which reaches a new field without being asked.
    ///
    /// Nothing here depends on the order a map iterates in (`hash_unordered`)
    /// or on an address: a `&'static` definition hashes what it says, or the
    /// printed face that names it (`hash_ability_list`).
    #[must_use]
    #[allow(clippy::too_many_lines)] // one line per field: the list is the guard
    pub fn snapshot_hash(&self) -> u64 {
        let Self {
            arena,
            zones,
            graveyard_order,
            sba_legend_decisions,
            players,
            turn,
            combat,
            per_turn,
            delayed,
            counter_links,
            ltb_versions,
            damage_deaths,
            pending_miracle,
            draws_to_offer,
            discard_answers,
            discards_on_top,
            extra_turns,
            resume_after,
            skip_followups,
            reanimated_auras,
            reanimation_finishes,
            restriction_info,
            next_restriction_id,
            commander_casts,
            commander_redirect,
            pending_copied_faces,
            ltb_mana_values,
            ltb_controllers,
            ltb_powers,
            ltb_abilities,
            ltb_attachments,
            ltb_counters,
            ltb_characteristics,
            // Empty again before any question is out; the field says why.
            ceased: _,
            // Empty whenever a question is out, by a rule the build
            // enforces; the field says which.
            reflexive: _,
            discovered,
            divided,
            synthetic_copies,
            commanders,
            monarch,
            day_night,
            previous_turn,
            starting_player,
            ability_fires,
            rng,
            // A record of what happened, not an input to what happens next.
            // The two rules that used to read this turn's entries now read
            // `per_turn` instead (#241). The engine's remaining readers are
            // anchored to state the `Engine` holds: `trigger_scan_seq`,
            // `entry_scan_seq`, and the resolution in progress
            // (`resolve::reflexive`). Hashing those is #238.
            journal: _,
            names,
            // Printed faces shared between objects. Each object's face is
            // hashed with the object, whether or not it is shared.
            bases: _,
            timestamp,
            effects,
            text_changes,
            effect_text_overrides,
            copy_snapshots,
            numeric_failure,
            constrained_payments,
            replacement_rules,
            shields,
            granted_actions,
            next_granted_action,
            next_damage_batch,
            damage_sources,
            source_memory,
            characteristics_generation,
            // Scratch, always left empty.
            projection_ids: _,
            // A cache flag, derived from the effects hashed below.
            projected_cross_zone: _,
            // Read off the cards at setup and never written again; each
            // object's card is hashed with the object.
            printed_pt_cda: _,
            // Drained by every pass, before anyone can look.
            token_cleanup: _,
            // This hash's cache of its own bytes, read below.
            snapshot_memo,
        } = self;
        // Contended only if two threads hash one state at once, and then
        // the second simply writes everything: the same bytes either way.
        let mut memo = snapshot_memo.0.try_lock().ok();
        let mut h = Hasher::new();
        h.u64(*timestamp);
        h.u64(*characteristics_generation);
        graveyard_order.hash(&mut h);
        sba_legend_decisions.hash(&mut h);
        hash_effects(&mut h, effects);
        text_changes.hash(&mut h);
        effect_text_overrides.hash(&mut h);
        numeric_failure.hash(&mut h);
        constrained_payments.hash(&mut h);
        h.usize(copy_snapshots.len());
        for (id, characteristics) in copy_snapshots {
            id.hash(&mut h);
            hash_characteristics(&mut h, characteristics);
        }
        replacement_rules.hash(&mut h);
        shields.hash(&mut h);
        granted_actions.hash(&mut h);
        next_granted_action.hash(&mut h);
        next_damage_batch.hash(&mut h);
        source_memory.hash(&mut h);
        h.usize(damage_sources.len());
        match memo.as_deref_mut() {
            Some(memo) => hash_sources_remembering(&mut h, damage_sources, &mut memo.sources),
            None => {
                for source in damage_sources {
                    hash_object(&mut h, source);
                }
            }
        }
        turn.hash(&mut h);
        day_night.hash(&mut h);
        previous_turn.hash(&mut h);
        monarch.hash(&mut h);
        starting_player.hash(&mut h);
        per_turn.hash(&mut h);
        delayed.hash(&mut h);
        counter_links.hash(&mut h);
        ltb_versions.hash(&mut h);
        damage_deaths.len().hash(&mut h);
        for death in damage_deaths {
            death.seq.hash(&mut h);
            hash_object(&mut h, &death.victim);
            hash_characteristics(&mut h, death.victim.characteristics());
            death.sources.len().hash(&mut h);
            for observer in &death.sources {
                hash_object(&mut h, &observer.object);
                hash_characteristics(&mut h, observer.object.characteristics());
                observer.grants.hash(&mut h);
                observer.text.hash(&mut h);
                observer.times.hash(&mut h);
            }
        }
        pending_miracle.hash(&mut h);
        draws_to_offer.hash(&mut h);
        discard_answers.hash(&mut h);
        discards_on_top.hash(&mut h);
        extra_turns.hash(&mut h);
        resume_after.hash(&mut h);
        skip_followups.hash(&mut h);
        reanimated_auras.hash(&mut h);
        reanimation_finishes.hash(&mut h);
        rng.hash(&mut h);
        // The interner's order is the game's history, and every object
        // hashes the `NameRef` it carries; the count keeps apart two
        // histories that interned a different number of names.
        h.usize(names.len());
        h.usize(players.len());
        for player in players {
            hash_player(&mut h, player);
        }
        match memo.as_deref_mut() {
            Some(memo) => hash_arena_remembering(&mut h, arena, &mut memo.chunks),
            None => {
                for (slot, generation, value) in arena.slots() {
                    hash_slot(&mut h, slot, generation, value);
                }
            }
        }
        zones.hash(&mut h);
        combat.hash(&mut h);
        // The commander counters, which no zone can stand in for: a
        // commander cast and then returned leaves the command zone exactly
        // as it found it, and the only difference between the two states is
        // what the next cast costs (CR 903.8). `Commander::answered` rides
        // along: two states that differ only in whether the owner has said
        // no yet (CR 903.9a) are two states.
        commander_casts.hash(&mut h);
        commanders.hash(&mut h);
        // Answers taken but not yet spent (CR 903.9b), and copies created
        // but not yet handed the rules text they copied. A resolution can
        // suspend on a choice with either outstanding, which is a moment
        // this hash is taken at.
        commander_redirect.hash(&mut h);
        pending_copied_faces.hash(&mut h);
        // A discovered card waiting for the resolution that found it to end.
        discovered.hash(&mut h);
        // How an ability on the stack divides its damage.
        divided.hash(&mut h);
        // The look-back lists are not scan bookkeeping that a priority
        // grant clears: an entry stays until its object moves again, and
        // `eval::matches` consults `ltb_attachments` in general.
        h.usize(ltb_abilities.len());
        for (object, list) in ltb_abilities {
            object.hash(&mut h);
            hash_ability_list(&mut h, &list.abilities, list.printed);
            list.token.hash(&mut h);
        }
        ltb_attachments.hash(&mut h);
        ltb_counters.hash(&mut h);
        ltb_mana_values.hash(&mut h);
        h.usize(ltb_characteristics.len());
        for (object, was) in ltb_characteristics {
            object.hash(&mut h);
            hash_characteristics(&mut h, was);
        }
        ltb_controllers.hash(&mut h);
        ltb_powers.hash(&mut h);
        synthetic_copies.hash(&mut h);
        hash_unordered(
            &mut h,
            restriction_info.iter(),
            |(id, (source, filter, rider))| {
                let mut one = Hasher::new();
                id.hash(&mut one);
                source.hash(&mut one);
                filter_hash(&mut one, filter);
                rider.hash(&mut one);
                one.finish()
            },
        );
        next_restriction_id.hash(&mut h);
        // Seventeen bytes an entry, digested in one call: a streaming state
        // set up per entry costs more than the entry does.
        hash_unordered(&mut h, ability_fires.iter(), |((object, index), uses)| {
            let mut entry = [0u8; 17];
            entry[..4].copy_from_slice(&object.object.slot().to_le_bytes());
            entry[4] = object.object.generation();
            entry[5..9].copy_from_slice(&object.version.to_le_bytes());
            entry[9..13].copy_from_slice(&index.to_le_bytes());
            entry[13..].copy_from_slice(&uses.to_le_bytes());
            xxhash_rust::xxh3::xxh3_64(&entry)
        });
        h.finish()
    }

    /// A hash of the *rules-visible situation*, blind to object identity and
    /// to time.
    ///
    /// [`Self::snapshot_hash`] answers "is this the same game state?" — it is
    /// what resync and replay compare, and it deliberately hashes object
    /// slots, generations and timestamps. That makes it useless for the
    /// question the loop detector asks. Slots are never recycled and
    /// timestamps only go up, so a permanent that dies and comes back is a
    /// different object at a later time: a genuine endless loop never hashes
    /// the same twice.
    ///
    /// This hashes what a player would see instead — who is where, with what
    /// characteristics, counters, damage and status — and reduces every
    /// object reference (attachments, targets, combat, an ability's source)
    /// to a position in a canonical ordering of the zones, so that a
    /// re-created permanent looks like the one it replaced.
    ///
    /// The turn *number* is left out for the same reason: a loop that spans a
    /// turn boundary would otherwise look different every time round.
    ///
    /// See [`crate::loops`] for how the detector uses it.
    #[must_use]
    pub fn loop_signature(&self) -> u64 {
        let zones = self.signature_zones();

        // Canonical position of every object, so references can be hashed
        // without their ids. Sorted by slot for a binary search; slots are
        // unique, so the mapping is exact.
        let mut by_slot: Vec<(u32, u32)> = Vec::new();
        for loc in &zones {
            for id in self.zones.list(*loc) {
                let position = by_slot.len() as u32;
                by_slot.push((id.slot(), position));
            }
        }
        by_slot.sort_unstable_by_key(|(slot, _)| *slot);
        let position = |id: ObjectId| -> u32 {
            by_slot
                .binary_search_by_key(&id.slot(), |(slot, _)| *slot)
                .map_or(u32::MAX, |i| by_slot[i].1)
        };

        let mut h = Hasher::new();
        h.u8(self.turn.active.get());
        h.u8(self.turn.phase as u8);
        h.u8(self.turn.step as u8);
        // A later effect can read this historical count even when the
        // present battlefield is identical, so it distinguishes situations.
        h.u32(self.per_turn.untapped_lands_at_start);
        self.live_damage_pairs(&position).hash(&mut h);
        h.u8(self.monarch.map_or(255, PlayerId::get));
        // The designation is rules-visible and a loop that flips it is a
        // loop that changes what daybound permanents are (CR 731). The
        // previous turn belongs here for a subtler reason: it is what the
        // *next* untap step will read, so two states alike in everything
        // else but disagreeing about it are not the same state.
        h.u8(self.day_night.map_or(255, |d| d as u8));
        h.u8(self.previous_turn.map_or(255, |p| p.active.get()));
        h.u32(self.previous_turn.map_or(0, |p| p.spells_cast));
        h.u32(self.previous_turn.map_or(0, |p| p.spells_by_all));
        h.u32(self.previous_turn.map_or(0, |p| p.most_by_one));
        h.usize(self.players.len());
        for p in &self.players {
            h.u8(p.id.get());
            h.i32(p.life);
            h.u16(p.poison);
            h.u16(p.energy);
            h.boolean(p.enduring_story);
            h.boolean(p.citys_blessing);
            h.i8(p.hand_modifier);
            h.boolean(p.has_lost());
            for color in ManaColor::ALL {
                h.u32(p.mana_pool.available(color));
                h.u32(p.mana_pool.snow_available(color));
            }
            // The commander tax belongs here even though nothing else that
            // only grows does. It is rules-visible — a player can see what
            // the next cast costs — and it only ever rises (CR 903.8), so a
            // "loop" that casts a commander is a game still making progress
            // and must not be called a draw. The ids need no canonical
            // position: the list is fixed for the whole game, so its order
            // already identifies each commander.
            //
            // Commander damage belongs here for the same reason and by the
            // same argument (CR 903.10a): it is rules-visible, it only ever
            // rises, and a "loop" that lands another swing from a commander
            // is a game walking towards a loss rather than standing still.
            // The order is the order it was first dealt in, which is the
            // same on every machine.
            h.usize(p.commander_damage.len());
            for (source, amount) in &p.commander_damage {
                h.u32(source.slot());
                h.u16(*amount);
            }
            // `Commander::answered` deliberately does *not* join it, in
            // either form. As a timestamp it would be the tax bug in
            // reverse — a number that only grows makes every situation
            // unique and no loop detectable — and as a "has this arrival
            // been offered" bit it is a constant here: this hash is taken
            // at priority grants, the SBA fixpoint has finished by then,
            // and a commander sitting in a graveyard has therefore always
            // been offered already. `commander_redirect` stays out for the
            // second of those reasons: it is filled and spent inside one
            // resolution, so it is empty every time this hash is taken.
            // `snapshot_hash` carries it because that one runs at any
            // moment, including a suspended one.
            let seat = p.id.get() as usize;
            h.u32(self.commander_casts.get(seat).copied().unwrap_or(0));
            for c in self.commanders.get(seat).into_iter().flatten() {
                h.u32(c.casts);
            }
        }
        for loc in &zones {
            let list = self.zones.list(*loc);
            h.usize(list.len());
            for id in list {
                match self.object(*id) {
                    Some(obj) => hash_object_situation(&mut h, obj, self, &position),
                    None => h.u8(0),
                }
            }
        }
        h.usize(self.combat.participants.len());
        for &(id, version) in &self.combat.participants {
            h.u32(position(id));
            h.boolean(self.object(id).is_some_and(|o| o.version == version));
        }
        h.usize(self.combat.attackers().len());
        for a in self.combat.attackers() {
            h.u32(position(a.creature));
            hash_defender(&mut h, a.defending, position);
            h.boolean(a.blocked);
        }
        h.usize(self.combat.blockers.len());
        for b in &self.combat.blockers {
            h.u32(position(b.blocker));
            h.u32(position(b.attacker));
        }
        // What has already been used this turn, which is rules-visible: an
        // ability that prints "activate only once each turn" is *offered* in
        // one of these states and not in the other, so two boards alike in
        // everything else are not the same state. Left out, a loop detector
        // would call them one and could declare a draw on a game that still
        // had a move in it.
        //
        // Sorted, because iterating the map in its own order would make the
        // signature depend on insertion — the rule this engine keeps
        // everywhere for determinism. The object is hashed by its canonical
        // position for the reason every other reference here is.
        hash_ability_tallies(self, &mut h, &position);
        // Prevention shields are what damage will do next, so two states
        // that differ only in what is shielded are two states.
        hash_shields(&mut h, self, &position);
        hash_granted_actions(&mut h, self, &position);
        hash_source_references(&mut h, self, &position);
        // A land already cleaned by this incarnation is not eligible again,
        // even if the visible board and counter totals are identical.
        self.counter_links.hash(&mut h);
        h.finish()
    }

    /// Every zone, in a fixed order — the canonical ordering object
    /// positions in [`Self::loop_signature`] are taken from.
    fn signature_zones(&self) -> Vec<ZoneLocation> {
        let mut locs = Vec::with_capacity(2 + self.players.len() * 6);
        locs.push(ZoneLocation::Battlefield);
        locs.push(ZoneLocation::Stack);
        for p in &self.players {
            locs.push(ZoneLocation::Library(p.id));
            locs.push(ZoneLocation::Hand(p.id));
            locs.push(ZoneLocation::Graveyard(p.id));
            locs.push(ZoneLocation::Exile(p.id));
            locs.push(ZoneLocation::Command(p.id));
            locs.push(ZoneLocation::OutsideGame(p.id));
        }
        locs
    }
}

/// A loss as [`GameState::snapshot_hash`] folds it in: `0` for none, which
/// is what the boolean it replaced wrote for `false`, and one value per
/// reason after it.
///
/// Spelled out rather than `reason as u8`, so reordering the enum cannot
/// move a hash and a new reason has to be given a byte here on purpose.
const fn loss_byte(loss: Option<LossReason>) -> u8 {
    match loss {
        None => 0,
        Some(LossReason::Life) => 1,
        Some(LossReason::EmptyDraw) => 2,
        Some(LossReason::Poison) => 3,
        Some(LossReason::CommanderDamage) => 4,
        Some(LossReason::Conceded) => 5,
        Some(LossReason::Effect) => 6,
    }
}

/// The shields as [`GameState::loop_signature`] hashes them: every object
/// they name by its canonical `position`, like every other reference there,
/// and by how far it has moved on since the shield named it
/// ([`moves_since`]): a shield on a permanent protects only the object it
/// was made on (CR 400.7), and a chosen source is the source only as that
/// object or, chosen as a spell, the permanent it became (CR 609.7a). The
/// version itself is identity, which the signature is blind to; how far the
/// object has moved on from it is what the shield reads.
fn hash_shields(h: &mut Hasher, state: &GameState, position: &dyn Fn(ObjectId) -> u32) {
    h.usize(state.shields.len());
    for shield in &state.shields {
        match shield.protects {
            crate::prevention::Shielded::Player(p) => {
                h.u8(0);
                h.u8(p.get());
            }
            crate::prevention::Shielded::Object(id, version) => {
                h.u8(1);
                h.u32(position(id));
                h.u8(moves_since(state.object(id), version));
            }
            crate::prevention::Shielded::Everything => h.u8(2),
        }
        match shield.kind {
            crate::prevention::ShieldKind::Next(n) => {
                h.u8(0);
                h.u32(n);
            }
            crate::prevention::ShieldKind::RedirectNext { remaining, to } => {
                h.u8(4);
                h.u32(remaining);
                h.u8(to.get());
            }
            crate::prevention::ShieldKind::AllCombat => h.u8(1),
            crate::prevention::ShieldKind::NextFrom {
                source,
                all_but,
                gain_life,
                combat_only,
            } => {
                h.u8(2);
                hash_chosen_source(h, state, &source, position);
                h.u32(all_but);
                h.boolean(gain_life);
                h.boolean(combat_only);
            }
            crate::prevention::ShieldKind::RedirectNextFrom { source, to } => {
                h.u8(3);
                hash_chosen_source(h, state, &source, position);
                h.u8(to.get());
            }
        }
        h.u8(shield.controller.get());
    }
}

/// A chosen source as [`hash_shields`] hashes it: everything
/// `ChosenSource::deals_as` reads when the damage comes.
fn hash_chosen_source(
    h: &mut Hasher,
    state: &GameState,
    source: &crate::prevention::ChosenSource,
    position: &dyn Fn(ObjectId) -> u32,
) {
    h.u32(position(source.id));
    h.u8(moves_since(
        state.object_or_departed(source.id),
        source.version,
    ));
    h.boolean(source.was_spell);
    h.boolean(state.object(source.id).is_some_and(|current| {
        state.is_resolved_source(
            baylee_core::ids::DamageSourceRef {
                object: source.id,
                version: source.version,
            },
            baylee_core::ids::DamageSourceRef {
                object: current.id,
                version: current.version,
            },
        )
    }));
    filter_hash(h, source.filter);
    source.text.hash(h);
    h.u8(source.you.get());
    h.u32(position(source.this));
    hash_damage_source(h, state, source.id, source.version);
}

// Exact referenced identities affect future choices even when every visible
// permanent is identical. Canonical ordinal labels keep absolute generations
// and monotonically increasing choice IDs out of loop equivalence.
fn hash_source_references(h: &mut Hasher, state: &GameState, position: &dyn Fn(ObjectId) -> u32) {
    let mut rows: Vec<_> = state
        .source_memory
        .stack
        .iter()
        .filter(|(holder, _)| {
            state
                .object(holder.object)
                .is_some_and(|o| o.version == holder.version && o.zone == Zone::Stack)
        })
        .flat_map(|(holder, refs)| {
            refs.iter()
                .map(move |(slot, reference)| (position(holder.object), *slot, *reference))
        })
        .collect();
    rows.sort_unstable_by_key(|(holder, slot, _)| (*holder, *slot));
    let mut identities = Vec::new();
    h.usize(rows.len());
    for (holder, slot, reference) in rows {
        h.u32(holder);
        slot.hash(h);
        let label = identities
            .iter()
            .position(|r| *r == reference)
            .unwrap_or_else(|| {
                identities.push(reference);
                identities.len() - 1
            });
        h.usize(label);
        h.u32(position(reference.object));
        h.u8(moves_since(
            state.object(reference.object),
            reference.version,
        ));
        hash_damage_source(h, state, reference.object, reference.version);
        if let Some(source) = state.source_object(reference)
            && source.kind == ObjectKind::Spell
        {
            // A surviving copy trigger observes these announcement decisions
            // even after the original spell has left the stack.
            h.u32(source.x_value);
            source.kicked.hash(h);
            source.replicated.hash(h);
            source.mode_index.hash(h);
            source.modes.hash(h);
            h.u8(source.face_index);
        }
        if let Some(shares) = state.source_memory.divisions.get(&reference) {
            h.usize(shares.len());
            for (target, amount) in shares {
                h.u32(position(target.object));
                h.u8(moves_since(state.object(target.object), target.version));
                h.u32(*amount);
            }
        } else {
            h.usize(0);
        }
    }
}

fn hash_damage_source(h: &mut Hasher, state: &GameState, id: ObjectId, version: u32) {
    if let Some(source) = state.damage_source(id, Some(version)) {
        h.u8(1);
        h.u8(source.controller.get());
        hash_characteristics(h, source.characteristics());
    } else {
        h.u8(0);
    }
}

/// How far `obj` has moved on from `version`, in the steps a shield tells
/// apart: 0 is the same object, 1 the next one (the permanent a chosen spell
/// became), 2 any later one, and 3 none at all. Capped, so an object that
/// keeps moving settles, and a loop that blinks it is still a repeat.
fn hash_ability_tallies(state: &GameState, h: &mut Hasher, position: &impl Fn(ObjectId) -> u32) {
    let mut used: Vec<(u32, u8, u32, u32)> = state
        .ability_fires
        .iter()
        .filter(|((id, _), _)| {
            state.source_identity(id.object) == Some(*id)
                || !state.source_referenced_by(*id).is_empty()
        })
        .map(|((id, index), n)| {
            (
                position(id.object),
                moves_since(state.object(id.object), id.version),
                *index,
                *n,
            )
        })
        .collect();
    used.sort_unstable();
    h.usize(used.len());
    for (obj, age, index, n) in used {
        h.u32(obj);
        h.u8(age);
        h.u32(index);
        h.u32(n);
    }
}

fn hash_granted_actions(h: &mut Hasher, state: &GameState, position: &impl Fn(ObjectId) -> u32) {
    state.granted_actions.len().hash(h);
    for grant in &state.granted_actions {
        h.u8(grant.player.get());
        grant.offer.cost.hash(h);
        grant.offer.timing.hash(h);
        grant.offer.ability.hash(h);
        // Source/recipient versions are rules identities, not UI labels.
        h.u32(position(grant.offer.source.object));
        h.u32(grant.offer.source.version);
        match grant.offer.effect {
            crate::choice::GrantedActionKind::AddMana { color, amount } => {
                h.u8(0);
                color.hash(h);
                amount.hash(h);
            }
            crate::choice::GrantedActionKind::PreventNextDamage { target, amount } => {
                h.u8(1);
                amount.hash(h);
                match target {
                    baylee_core::ids::TargetRef::Player(player) => {
                        h.u8(0);
                        h.u8(player.get());
                    }
                    baylee_core::ids::TargetRef::Object(reference) => {
                        h.u8(1);
                        h.u32(position(reference.object));
                        h.u32(reference.version);
                    }
                }
            }
        }
    }
}

fn moves_since(obj: Option<&GameObject>, version: u32) -> u8 {
    match obj.map(|o| o.version.wrapping_sub(version)) {
        Some(0) => 0,
        Some(1) => 1,
        Some(_) => 2,
        None => 3,
    }
}

/// Hashes a defender. `locate` maps an object to whatever identity the
/// caller's hash is built on — the arena slot for the snapshot, a
/// canonical position for the loop signature.
///
/// The discriminant is hashed first so that a planeswalker in slot 3 and
/// the player with id 3 cannot collide.
fn hash_defender(h: &mut Hasher, defender: Defender, locate: impl Fn(ObjectId) -> u32) {
    match defender {
        Defender::Player(p) => {
            h.u8(0);
            h.u32(u32::from(p.get()));
        }
        Defender::Planeswalker(id) => {
            h.u8(1);
            h.u32(locate(id));
        }
    }
}

/// Hashes one object as a *situation*: everything a player could observe
/// about it, with object references reduced to canonical positions.
///
/// The identity fields `hash_object` includes — slot, generation — are
/// exactly what has to be left out here; see
/// [`GameState::loop_signature`].
fn hash_object_situation(
    h: &mut Hasher,
    obj: &GameObject,
    state: &GameState,
    position: &impl Fn(ObjectId) -> u32,
) {
    h.u8(1);
    h.u8(obj.owner.get());
    h.u8(obj.controller.get());
    // Two boards that look identical but differ in who gets the permanent
    // back when a control effect ends are different situations.
    h.u8(obj.base_controller.get());
    h.u8(obj.zone as u8);
    h.u8(obj.zone_owner.map_or(255, PlayerId::get));
    h.u8(obj.kind as u8);
    h.u8(obj.face_index);
    h.u8(obj.doors.bits());
    h.u8(obj.chosen_opponent().map_or(255, PlayerId::get));
    h.boolean(obj.prototyped);
    match &obj.card {
        Some(c) => {
            h.u8(1);
            h.u32(c.index.get());
        }
        None => h.u8(0),
    }
    let b = &obj.base;
    h.u32(b.name.get());
    hash_mana_cost(h, &b.mana_cost);
    h.u8(b.colors.bits());
    h.u16(b.types.bits());
    h.u8(b.supertypes.bits());
    for word in b.subtypes.words() {
        h.u64(*word);
    }
    h.u128(b.keywords.bits());
    h.option_u32(b.power.map(|v| v as u32));
    h.option_u32(b.toughness.map(|v| v as u32));
    h.option_u32(b.loyalty.map(u32::from));
    let counters: Vec<_> = obj.counters.iter().collect();
    h.usize(counters.len());
    for (kind, n) in counters {
        hash_counter(h, kind);
        h.u16(n);
    }
    h.u16(obj.damage);
    // Status and the deathtouch mark share one word; bit 8 is out of the
    // status byte, so packing them cannot collide.
    h.u16(u16::from(obj.status.bits()) | (u16::from(obj.deathtouched) << 8));
    // Its own byte rather than a third thing packed into the word above:
    // a shield is a count and not a flag, so there is no width to argue
    // about, and two boards that differ only in how many destructions a
    // creature will survive are different situations.
    h.u8(obj.regeneration_shields);
    h.option_u32(obj.source_power_lki.map(|p| p as u32));
    hash_rider_situation(h, obj, state, position);
    // What was paid is part of what the spell will do: Neoform after a
    // two-drop and after a five-drop are two different futures.
    let sacrificed = obj.paid.as_ref().and_then(|p| p.sacrificed_lki);
    h.option_u32(sacrificed.map(|s| s.mana_value));
    h.u32(sacrificed.map_or(0, |s| s.power as u32));
    h.u32(sacrificed.map_or(0, |s| s.toughness as u32));
    h.u32(obj.paid.as_ref().map_or(0, |p| p.mana_spent));
    h.u8(obj.paid.as_ref().map_or(0, |p| p.colors_spent.bits()));
    for amount in obj.paid.as_ref().map_or([0; 6], |p| p.mana_types_spent) {
        h.u32(amount);
    }
    h.u64(obj.paid.as_ref().map_or(0, |paid| {
        structural_fingerprint(&(&paid.mana_paid, paid.fixed_mana_cost, paid.mana_spending))
    }));
    // And which creature station tapped: its power is what the counters
    // will be.
    let followed = obj.paid.as_ref().and_then(|p| p.source_after_cost);
    h.option_u32(followed.map(|r| position(r.object)));
    if let Some(r) = followed {
        hash_damage_source(h, state, r.object, r.version);
    }
    let sacrificed = obj.paid.as_ref().and_then(|p| p.sacrificed);
    h.option_u32(sacrificed.map(|(id, _)| position(id)));
    if let Some((id, version)) = sacrificed {
        hash_damage_source(h, state, id, version);
    }
    let tapped = obj.paid.as_ref().and_then(|p| p.tapped);
    h.option_u32(tapped.map(|(id, _)| position(id)));
    h.option_u32(tapped.map(|(_, version)| version));
    h.option_u32(obj.attached_to.map(position));
    h.usize(obj.targets.len());
    for t in &obj.targets {
        h.u32(position(*t));
    }
    h.usize(obj.second_targets().len());
    for t in obj.second_targets() {
        h.u32(position(*t));
    }
    match &obj.ability {
        Some(loc) => {
            h.u8(1);
            h.option_u32(loc.card.map(baylee_core::ids::CardIndex::get));
            h.u32(loc.index);
            h.u32(position(loc.source));
        }
        None => h.u8(0),
    }
}

/// Hash identity-sensitive riders by whether their remembered incarnation
/// still exists, rather than by the counter value of its version.
fn hash_rider_situation(
    h: &mut Hasher,
    obj: &GameObject,
    state: &GameState,
    position: &impl Fn(ObjectId) -> u32,
) {
    let departure = obj.riders.iter().find_map(|r| match r {
        crate::object::Rider::EventDeparture(p, t) => Some((*p, *t)),
        _ => None,
    });
    departure.hash(h);
    for rider in &obj.riders {
        match rider {
            crate::object::Rider::AbilitySourceVersion(version) => {
                h.u8(21);
                h.boolean(
                    obj.ability
                        .and_then(|a| state.object(a.source))
                        .is_some_and(|source| source.version == *version),
                );
                if let Some(ability) = obj.ability {
                    hash_damage_source(h, state, ability.source, *version);
                }
            }
            crate::object::Rider::EventObjectIdentity(version, power) => {
                h.u8(22);
                let still_here = obj
                    .event_object
                    .and_then(|id| state.object(id))
                    .is_some_and(|event| event.version == *version);
                h.boolean(still_here);
                if !still_here {
                    h.u16(*power as u16);
                }
                if let Some(id) = obj.event_object {
                    hash_damage_source(h, state, id, *version);
                }
            }
            crate::object::Rider::SourceAttachmentLki(id) => {
                h.u8(23);
                h.u32(position(*id));
                h.boolean(state.object(*id).is_some_and(|o| {
                    obj.riders
                        .contains(&crate::object::Rider::SourceAttachmentVersion(o.version))
                }));
            }
            crate::object::Rider::AttachmentHostLeft => h.u8(24),
            crate::object::Rider::EventAmount(amount) => {
                h.u8(26);
                h.u32(*amount);
            }
            _ => {}
        }
    }
}

/// Hashes a map's entries without depending on the order it iterates in.
///
/// `digest` hashes one entry on its own and the digests are summed, so any
/// iteration order gives one total and nothing is allocated. Sorting the
/// entries first would put a `Vec` on every call, and the harness takes
/// this hash after every action (#213).
fn hash_unordered<T>(
    h: &mut Hasher,
    entries: impl ExactSizeIterator<Item = T>,
    digest: impl Fn(T) -> u64,
) {
    h.usize(entries.len());
    let sum = entries.fold(0u64, |sum, entry| sum.wrapping_add(digest(entry)));
    h.u64(sum);
}

/// Hashes an ability list by what names it, which is never its address.
///
/// A printed list is named by its face: [`AbilityList`] carries the two
/// together, and every place that builds one takes both from the same face,
/// so the face stands for the list. A token's or an emblem's has no face,
/// and there the list's content is hashed, because an address is no name:
/// it differs between builds, and the compiler merges identical lists into
/// one ([`PrintedFace`]).
///
/// [`AbilityList`]: crate::object::AbilityList
fn hash_ability_list(
    h: &mut Hasher,
    abilities: impl Into<crate::copiable_abilities::AbilityDefs>,
    printed: Option<PrintedFace>,
) {
    printed.hash(h);
    abilities.into().hash(h);
}

fn hash_effects(h: &mut Hasher, table: &crate::effects::EffectTable) {
    let (effects, parked, next_id, generation) = table.hashed_parts();
    next_id.hash(h);
    generation.hash(h);
    h.usize(effects.len());
    for fx in effects {
        hash_effect(h, fx);
    }
    // The statics parked while their sources are phased out. Nothing is
    // written while nothing is parked, so a game without phasing hashes as
    // it did before the table could park; the length above already moves
    // when an effect is parked.
    if !parked.is_empty() {
        h.usize(parked.len());
        for fx in parked {
            hash_effect(h, fx);
        }
    }
}

fn hash_effect(h: &mut Hasher, fx: &crate::effects::ContinuousEffect) {
    let crate::effects::ContinuousEffect {
        id,
        source,
        controller,
        origin,
        layer,
        timestamp,
        duration,
        filter,
        modifier,
    } = fx;
    id.hash(h);
    source.hash(h);
    controller.hash(h);
    origin.hash(h);
    layer.hash(h);
    timestamp.hash(h);
    duration.hash(h);
    match filter {
        crate::effects::EffectFilter::Dsl(filter) => {
            h.u8(0);
            filter_hash(h, filter);
        }
        crate::effects::EffectFilter::ObjectIs(id, version) => {
            h.u8(1);
            id.hash(h);
            version.hash(h);
        }
    }
    hash_modifier(h, modifier);
}

fn hash_player(h: &mut Hasher, player: &Player) {
    let Player {
        id,
        life,
        poison,
        energy,
        enduring_story,
        citys_blessing,
        mana_pool,
        hand_modifier,
        lands_played_this_turn,
        turn_start_timestamp,
        tried_empty_draw,
        commander_damage,
        loss,
        // Preset-constant, so it tells no two states of one game apart
        // (`docs/engine-internals.md`).
        team: _,
    } = player;
    id.hash(h);
    life.hash(h);
    poison.hash(h);
    energy.hash(h);
    enduring_story.hash(h);
    citys_blessing.hash(h);
    mana_pool.hash(h);
    hand_modifier.hash(h);
    lands_played_this_turn.hash(h);
    turn_start_timestamp.hash(h);
    tried_empty_draw.hash(h);
    // Commander damage (CR 903.10a), which no life total records: two
    // seats on the same life with twelve and twenty points from the same
    // commander are one attack apart from different games.
    commander_damage.hash(h);
    // Why, and not only whether: two engines that eliminated the same seat
    // by different state-based actions ran different rules, and once that
    // seat's objects have left the game the reason is the only trace of
    // which one fired.
    h.u8(loss_byte(*loss));
}

/// Canonical characteristics hash for a held resolution's target snapshot.
pub(crate) fn characteristics_fingerprint(characteristics: &Characteristics) -> u64 {
    let mut hash = Hasher::new();
    hash_characteristics(&mut hash, characteristics);
    hash.finish()
}

fn hash_characteristics(h: &mut Hasher, characteristics: &Characteristics) {
    let Characteristics {
        name,
        mana_cost,
        colors,
        types,
        supertypes,
        subtypes,
        keywords,
        power,
        toughness,
        loyalty,
        color_identity,
        produced_colors,
        produced_colorless,
        produced_chosen,
        has_mana_ability,
        rules_text_lost,
        abilities_lost,
        front_mana_value,
    } = characteristics;
    name.hash(h);
    hash_mana_cost(h, mana_cost);
    colors.hash(h);
    types.hash(h);
    supertypes.hash(h);
    // `[u64; N]`'s own `Hash`, word by word: its length, then each word's
    // native bytes, as fixed-size writes instead of one of runtime length.
    h.words(subtypes.words());
    keywords.hash(h);
    power.hash(h);
    toughness.hash(h);
    loyalty.hash(h);
    color_identity.hash(h);
    produced_colors.hash(h);
    produced_colorless.hash(h);
    produced_chosen.hash(h);
    has_mana_ability.hash(h);
    rules_text_lost.hash(h);
    abilities_lost.hash(h);
    front_mana_value.hash(h);
}

/// What [`GameState::snapshot_hash`] last wrote for each arena chunk: the
/// chunk's key and the bytes its slots made.
///
/// A game is hashed after every input its record keeps, and between two
/// inputs most chunks are not written at all. A chunk whose key still
/// matches has the same slots ([`crate::arena::ChunkKey`] says why), and
/// its slots' bytes depend on nothing else, so its bytes are fed again
/// instead of written again. The stream is byte for byte the one writing
/// would make; `the_remembered_hash_is_the_written_hash` holds the two
/// together over whole games.
///
/// A clone starts with an empty memo: a decision checkpoint is a clone, and
/// a memo's keys would hold its chunks shared for nothing.
///
/// The retained damage sources are remembered the same way, each by its
/// `Arc`: nothing writes one once it is retained, and the memo's reference
/// would make a write copy it anyway.
#[derive(Default)]
pub(crate) struct SnapshotMemo(pub(super) std::sync::Mutex<MemoTables>);

#[derive(Default)]
pub(super) struct MemoTables {
    chunks: Vec<Option<ArenaTape>>,
    /// In `damage_sources` order, which only ever drops entries or adds
    /// them at the end.
    sources: Vec<(Arc<GameObject>, Vec<u8>)>,
}

/// One chunk's key and bytes.
struct ArenaTape {
    key: crate::arena::ChunkKey<GameObject>,
    bytes: Vec<u8>,
}

impl Clone for SnapshotMemo {
    fn clone(&self) -> Self {
        Self::default()
    }
}

impl std::fmt::Debug for SnapshotMemo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SnapshotMemo")
    }
}

/// One arena slot as the snapshot hash writes it.
fn hash_slot(h: &mut Hasher, slot: u32, generation: u8, value: Option<&GameObject>) {
    h.u32(slot);
    h.u8(generation);
    h.boolean(value.is_some());
    if let Some(obj) = value {
        hash_object(h, obj);
    }
}

/// The arena's slots in order, each chunk's bytes taken from `memo` while
/// the chunk is the one they were written for, and written (and kept)
/// otherwise.
fn hash_arena_remembering(
    h: &mut Hasher,
    arena: &Arena<GameObject>,
    memo: &mut Vec<Option<ArenaTape>>,
) {
    let mut chunks = 0;
    for (i, view) in arena.chunk_views().enumerate() {
        chunks = i + 1;
        if memo.len() <= i {
            memo.push(None);
        }
        match &mut memo[i] {
            Some(tape) if view.is(&tape.key) => h.bytes(&tape.bytes),
            entry => {
                let mut written = Hasher::tape_into(entry.take().map(|t| t.bytes));
                for (slot, generation, value) in view.slots() {
                    hash_slot(&mut written, slot, generation, value);
                }
                let bytes = written.into_tape();
                h.bytes(&bytes);
                *entry = Some(ArenaTape {
                    key: view.key(),
                    bytes,
                });
            }
        }
    }
    memo.truncate(chunks);
}

/// The retained damage sources in order, each one's bytes taken from
/// `memo` while it is the same allocation they were written for. The
/// list only loses entries or gains them at its end, so one cursor through
/// the memo finds every one still there.
fn hash_sources_remembering(
    h: &mut Hasher,
    sources: &[Arc<GameObject>],
    memo: &mut Vec<(Arc<GameObject>, Vec<u8>)>,
) {
    let mut old = std::mem::take(memo).into_iter();
    let mut next = old.next();
    for source in sources {
        // Past every remembered entry the list has since dropped. Were the
        // order ever different, an entry would only be written again: a
        // remembered entry is used for the very allocation it was made of.
        let mut found = None;
        while let Some((kept, bytes)) = next.take() {
            next = old.next();
            if Arc::ptr_eq(&kept, source) {
                found = Some(bytes);
                break;
            }
        }
        let bytes = found.unwrap_or_else(|| {
            let mut written = Hasher::tape_into(None);
            hash_object(&mut written, source);
            written.into_tape()
        });
        h.bytes(&bytes);
        memo.push((Arc::clone(source), bytes));
    }
}

#[allow(clippy::too_many_lines)] // one line per field: the list is the guard
fn hash_object(h: &mut Hasher, obj: &GameObject) {
    let GameObject {
        id,
        owner,
        controller,
        base_controller,
        zone,
        zone_owner,
        kind,
        card,
        base,
        // The layer projection of `base` under the effect table, both of
        // which are hashed; it is recomputed whenever the table moves.
        cache: _,
        counters,
        damage,
        deathtouched,
        regeneration_shields,
        status,
        attached_to,
        timestamp,
        controlled_since,
        version,
        riders,
        targets,
        target_req,
        second,
        original_base,
        ability,
        source_power_lki,
        paid,
        x_value,
        kicked,
        replicated,
        alt_cast,
        prototyped,
        chosen_player,
        target_players,
        mode_index,
        modes,
        chosen_subtype,
        chosen_color,
        chosen_name,
        doors,
        face_index,
        own_abilities,
        own_abilities_until_eot,
        own_origin,
        token,
        pending_face_change,
        event_object,
        cast_from_hand,
    } = obj;
    id.hash(h);
    owner.hash(h);
    controller.hash(h);
    // Not derivable from the projected controller: it is who the permanent
    // goes back to when a control effect ends, so a resync that lost it
    // would hand the permanent to the wrong seat later.
    base_controller.hash(h);
    zone.hash(h);
    zone_owner.hash(h);
    kind.hash(h);
    card.hash(h);
    // Base characteristics (copiable values), and the ones a copy gives
    // back when it changes zones (CR 400.7).
    hash_characteristics(h, base);
    h.boolean(original_base.is_some());
    if let Some(original) = original_base {
        hash_characteristics(h, original);
    }
    counters.hash(h);
    damage.hash(h);
    deathtouched.hash(h);
    regeneration_shields.hash(h);
    status.hash(h);
    attached_to.hash(h);
    timestamp.hash(h);
    controlled_since.hash(h);
    version.hash(h);
    // Exile riders.
    h.usize(riders.len());
    for rider in riders {
        match rider {
            Rider::Linked { host, until } => {
                h.u8(1);
                host.hash(h);
                match until {
                    None => h.u8(0),
                    Some(crate::object::LinkUntil::HostLeaves) => h.u8(1),
                    Some(crate::object::LinkUntil::OpponentBecomesMonarch { of }) => {
                        h.u8(2);
                        h.u8(of.get());
                    }
                }
            }
            Rider::CounterSourceVersion(version) => {
                h.u8(18);
                version.hash(h);
            }
            Rider::EventObjectIdentity(version, power) => {
                h.u8(22);
                version.hash(h);
                power.hash(h);
            }
            Rider::AbilitySourceVersion(version) => {
                h.u8(21);
                version.hash(h);
            }
            Rider::TriggerSourceVersion(version) => {
                h.u8(20);
                version.hash(h);
            }
            Rider::SourceAttachmentLki(id) => {
                h.u8(23);
                id.hash(h);
            }
            Rider::SourceAttachmentVersion(version) => {
                h.u8(25);
                version.hash(h);
            }
            Rider::SourceAuraSuccessor(version) => {
                h.u8(60);
                version.hash(h);
            }
            Rider::AttachmentHostLeft => h.u8(24),
            Rider::Rebound => h.u8(2),
            Rider::Adventure => h.u8(3),
            Rider::Foretold => h.u8(4),
            Rider::Plotted => h.u8(5),
            Rider::Suspend => h.u8(6),
            Rider::Flashback => h.u8(7),
            Rider::Uncounterable => h.u8(8),
            Rider::PlayableFromExileFor(p) => {
                h.u8(9);
                h.u8(p.get());
            }
            Rider::Prepared => h.u8(10),
            Rider::SpellCopy => h.u8(11),
            Rider::ExileInsteadOfGraveyard => h.u8(12),
            Rider::ExiledWith { host, version } => {
                h.u8(13);
                host.hash(h);
                version.hash(h);
            }
            Rider::Dashed => h.u8(14),
            Rider::EventDeparture(p, toughness) => {
                h.u8(19);
                p.hash(h);
                toughness.hash(h);
            }
            Rider::EventPlayer(p) => {
                h.u8(15);
                h.u8(p.get());
            }
            Rider::EventAmount(amount) => {
                h.u8(26);
                amount.hash(h);
            }
            Rider::Escaped => h.u8(16),
            Rider::Masked => h.u8(41),
            Rider::ChosenOpponent(p) => {
                h.u8(17);
                h.u8(p.get());
            }
        }
    }
    // What the spell or ability on the stack was cast or put there with:
    // its targets, what it may retarget to, which ability it is, and every
    // choice made on the way — two copies of one spell with X 3 and X 0 are
    // two different futures.
    targets.hash(h);
    target_req.hash(h);
    second.hash(h);
    ability.hash(h);
    source_power_lki.hash(h);
    paid.hash(h);
    x_value.hash(h);
    kicked.hash(h);
    replicated.hash(h);
    alt_cast.hash(h);
    prototyped.hash(h);
    chosen_player.hash(h);
    target_players.hash(h);
    mode_index.hash(h);
    modes.hash(h);
    chosen_subtype.hash(h);
    chosen_color.hash(h);
    chosen_name.hash(h);
    doors.hash(h);
    face_index.hash(h);
    pending_face_change.hash(h);
    event_object.hash(h);
    cast_from_hand.hash(h);
    // What the object can do when it is not what its card says: a copy's
    // list, an emblem's, an ability's captured one. `own_origin` names it.
    h.boolean(own_abilities.is_some());
    if let Some(list) = own_abilities {
        hash_ability_list(
            h,
            list,
            own_origin.and_then(crate::object::AbilityOrigin::printed),
        );
    }
    own_origin.hash(h);
    own_abilities_until_eot.hash(h);
    // A token's definition, hashed by what it says for the reason
    // `hash_ability_list` gives.
    token.hash(h);
}

/// Deterministic structural hash of a DSL filter (modifier payloads).
#[allow(clippy::too_many_lines)] // Exhaustive stable tag table for the filter vocabulary.
fn filter_hash(h: &mut Hasher, f: &baylee_cards_dsl::Filter) {
    use baylee_cards_dsl::Filter as F;
    match f {
        F::Any => h.u8(0),
        F::This => h.u8(1),
        F::Another => h.u8(2),
        F::And(parts) | F::Or(parts) => {
            h.u8(if matches!(f, F::And(_)) { 3 } else { 4 });
            for p in *parts {
                filter_hash(h, p);
            }
        }
        F::Not(inner) => {
            h.u8(5);
            filter_hash(h, inner);
        }
        F::HasType(t) => {
            h.u8(6);
            h.u16(t.bits());
        }
        F::LacksType(t) => {
            h.u8(7);
            h.u16(t.bits());
        }
        F::HasSupertype(s) => {
            h.u8(8);
            h.u8(s.bits());
        }
        F::HasSubtype(s) => {
            h.u8(9);
            h.u16(s.get());
        }
        F::HasColor(c) => {
            h.u8(10);
            h.u8(c.bits());
        }
        F::IsColorless => h.u8(11),
        F::Monocolored => h.u8(12),
        F::IsToken => h.u8(13),
        F::ControlledByYou => h.u8(14),
        F::ControlledByOpponent => h.u8(15),
        F::OwnedByYou => h.u8(16),
        F::Tapped => h.u8(17),
        F::Untapped => h.u8(18),
        F::Attacking => h.u8(19),
        F::BandedWithSource => h.u8(50),
        F::NotTargetedByAnotherNamed(name) => {
            h.u8(51);
            h.u32(u32::try_from(name.len()).unwrap_or(u32::MAX));
            h.bytes(name.as_bytes());
        }
        F::Blocking => h.u8(47),
        F::Unblocked => h.u8(39),
        F::ControlledByActivePlayer => h.u8(40),
        F::ControlledByDefendingPlayer => h.u8(46),
        F::CmcExactlyX => h.u8(41),
        F::MatchesChosenTypeOfSource => h.u8(20),
        F::AttachedToBySource => h.u8(25),
        F::AttachedToSource => h.u8(49),
        F::IsAttached => h.u8(38),
        F::SharesSubtypeWithCommander => h.u8(27),
        F::ToughnessAtMost(n) => {
            h.u8(26);
            h.i16(*n);
        }
        // Three tags and not one with a direction byte: the hash is what
        // tells two continuous effects apart, and a shared tag would make
        // "power at least 4" and "power at most 4" the same effect to the
        // cache — which is the one pair of filters on this list that a
        // single board satisfies on opposite sides.
        F::ToughnessAtLeast(n) => {
            h.u8(31);
            h.i16(*n);
        }
        F::PowerAtLeast(n) => {
            h.u8(32);
            h.i16(*n);
        }
        F::PowerAtMost(n) => {
            h.u8(33);
            h.i16(*n);
        }
        F::HasKeyword(k) => {
            h.u8(21);
            h.u128(k.bits());
        }
        // Game state rather than a characteristic, and hashed all the same:
        // two continuous effects that differ only in this filter are two
        // different effects, and a tag table that left it out would make
        // them one.
        F::EnteredThisTurn => h.u8(30),
        F::PutIntoGraveyardThisTurn => h.u8(35),
        F::AttackedThisTurn => h.u8(44),
        F::ControlledSinceTurnBegan => h.u8(45),
        F::HasCounter(kind) => {
            h.u8(36);
            hash_counter(h, *kind);
        }
        F::WithSingleTarget => h.u8(34),
        F::HasManaAbility => h.u8(48),
        // Its own tag rather than a payload on `CmcAtMost`: the bound is
        // read from the source at match time, so two filters that differ
        // only in *where* the number comes from are different filters.
        F::CmcAtMostX => h.u8(28),
        F::CmcAtMostColorsSpent => h.u8(37),
        F::PowerLessThanSourcePower => h.u8(42),
        F::ToughnessLessThanSourcePower => h.u8(43),
        F::CmcAtMost(n) | F::CmcAtLeast(n) => {
            h.u8(if matches!(f, F::CmcAtMost(_)) { 22 } else { 23 });
            h.u32(*n);
        }
        F::InZone(z) => {
            h.u8(24);
            h.u8(*z as u8);
        }
        // Length-prefixed, so `Named("a") + Named("bc")` inside an `And`
        // cannot hash as `Named("ab") + Named("c")`.
        F::Named(name) => {
            h.u8(29);
            h.u32(u32::try_from(name.len()).unwrap_or(u32::MAX));
            h.bytes(name.as_bytes());
        }
    }
}

fn hash_modifier(h: &mut Hasher, modifier: &baylee_cards_dsl::Modifier) {
    // Derived Hash walks every modifier payload, including granted costs,
    // effects, triggers and counter kinds which the old tag table omitted.
    std::hash::Hash::hash(modifier, h);
}

/// Writes a counter kind into a hash.
///
/// A function rather than the one-byte tag it used to be, because a P/T
/// counter carries two numbers: `Plus { power: 0, toughness: 1 }` and
/// `Plus { power: 1, toughness: 0 }` are different counters and folding
/// them onto one byte would make two different boards hash alike.
fn hash_counter(h: &mut Hasher, kind: CounterKind) {
    match kind {
        CounterKind::Plus { power, toughness } => {
            h.u8(1);
            h.u8(power);
            h.u8(toughness);
        }
        CounterKind::Minus { power, toughness } => {
            h.u8(2);
            h.u8(power);
            h.u8(toughness);
        }
        CounterKind::Loyalty => h.u8(3),
        CounterKind::Lore => h.u8(4),
        CounterKind::Time => h.u8(5),
        CounterKind::Charge => h.u8(6),
        CounterKind::Poison => h.u8(7),
        CounterKind::Energy => h.u8(8),
        CounterKind::Rad => h.u8(9),
        CounterKind::Lifelink => h.u8(10),
        CounterKind::Level => h.u8(11),
        // The id is hashed whole. Folding it modulo 100 was safe while every
        // tag was one byte and is not worth keeping now that the function
        // writes as many as it likes.
        CounterKind::Custom(id) => {
            h.u8(12);
            h.u16(id);
        }
    }
}

pub(crate) fn mana_cost_fingerprint(cost: &baylee_core::mana::ManaCost) -> u64 {
    let mut h = Hasher::new();
    hash_mana_cost(&mut h, cost);
    h.finish()
}

/// Fixed-width structural hashing for continuation state shared with wasm.
pub(crate) fn structural_fingerprint(value: &impl Hash) -> u64 {
    let mut h = Hasher::new();
    value.hash(&mut h);
    h.finish()
}

fn hash_mana_cost(h: &mut Hasher, cost: &baylee_core::mana::ManaCost) {
    // The cost is counted (`ManaCost`): how many kinds it holds, then one
    // five-byte record per kind, its tag, what names it and how many of it,
    // rather than one per symbol. The count fits a byte, and a byte is what
    // an empty cost, most objects' (tokens'), costs the stream: a wider
    // prefix measured 7 % slower on `snapshot_hash_3k_tokens`.
    h.u8(cost.kinds());
    cost.for_each_run(|symbol, count| {
        let [count_lo, count_hi] = count.to_le_bytes();
        let (tag, first, second) = match symbol {
            ManaSymbol::Generic(amount) => {
                // Always one generic symbol: its amount says it all.
                let [w0, w1, w2, w3] = amount.to_le_bytes();
                h.put([0, w0, w1, w2, w3]);
                return;
            }
            ManaSymbol::Colorless => (1, 0, 0),
            ManaSymbol::White => (2, 0, 0),
            ManaSymbol::Blue => (3, 0, 0),
            ManaSymbol::Black => (4, 0, 0),
            ManaSymbol::Red => (5, 0, 0),
            ManaSymbol::Green => (6, 0, 0),
            ManaSymbol::Hybrid(pair) => (7, pair.first() as u8, pair.second() as u8),
            ManaSymbol::TwoOrColor(color) => (8, color as u8, 0),
            ManaSymbol::Phyrexian(color) => (9, color as u8, 0),
            ManaSymbol::HybridPhyrexian(pair) => (10, pair.first() as u8, pair.second() as u8),
            ManaSymbol::Snow => (11, 0, 0),
            ManaSymbol::Variable(variable) => (12, variable as u8, 0),
            ManaSymbol::HalfGeneric => (13, 0, 0),
            ManaSymbol::Infinite => (14, 0, 0),
        };
        h.put([tag, first, second, count_lo, count_hi]);
    });
}
