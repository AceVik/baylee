//! Casting the spell associated with a prepared permanent.
#[allow(clippy::wildcard_imports)] // shared ability vocabulary
use super::*;

impl<L: CardLookup> Engine<L> {
    /// Whether `player` could begin casting a copy of `spell_def` right now.
    ///
    /// A prepared permanent says "you may **cast** a copy of its spell", and
    /// grants no exception to when that spell may be cast — so the copy is
    /// held to the linked card's own timing, through the same
    /// [`casting::timing_allows`] every other cast goes through. Emeritus of
    /// Woe's spell is Demonic Tutor, a sorcery (CR 307.1), and the offer
    /// asked about the mana and nothing else: a prepared Warlock was a tutor
    /// at instant speed on anybody's turn.
    ///
    /// The linked card's own timing rather than a flat sorcery speed,
    /// because nothing about being prepared slows a spell down — a prepared
    /// permanent whose spell were an instant would rightly offer it at
    /// instant speed. Sharing the function is what carries the two
    /// player-scoped effects a hand-rolled copy of the rule would have lost:
    /// Teferi's +1 gives that player's sorceries flash, and his static pulls
    /// every opponent's spell back to sorcery speed.
    ///
    /// `keywords_for_face(0)` and not `faces[0].keywords`: a single-faced
    /// card states its keywords once at card level and leaves the face's own
    /// list empty, so reading the face directly would lose the flash on
    /// every card that has one.
    pub(super) fn prepared_cast_is_timely(
        &self,
        player: PlayerId,
        spell_def: &baylee_cards_dsl::CardDef,
    ) -> bool {
        casting::may_begin_casting(&self.state, player)
            && casting::timing_allows(
                &self.state,
                player,
                spell_def.faces[0].types,
                spell_def.keywords_for_face(0),
            )
    }

    /// A newly cast copy has its own characteristics and no previous object
    /// identity. In particular, the prepared permanent's color is irrelevant.
    pub(super) fn prepared_spell_price(
        &self,
        player: PlayerId,
        def: &baylee_cards_dsl::CardDef,
    ) -> baylee_core::mana::ManaCost {
        let base = crate::object::Characteristics::from_face(def, 0, crate::state::NAMELESS);
        let spell = GameObject::new_bare(ObjectId::NO_SOURCE, player, ObjectKind::Spell, base);
        casting::spell_adjustment(&self.state, player, &spell).price(
            def.faces[0].mana_cost,
            casting::printed_reduction(&self.state, &def.faces[0], player, ObjectId::NO_SOURCE),
        )
    }

    /// Prepared cast (Emeritus of Woe): pays the linked spell's cost,
    /// puts a copy of it on the stack, and removes the prepared marker.
    pub(super) fn start_prepared_cast(
        &mut self,
        player: PlayerId,
        source: ObjectId,
    ) -> Result<(), EngineError> {
        let linked_card = self
            .state
            .object(source)
            .and_then(|o| o.card)
            .and_then(|c| {
                let face = self
                    .state
                    .object(source)
                    .map_or(0, |o| o.face_index as usize);
                self.lookup.card(c.index).and_then(|def| {
                    def.abilities_for_face(face).iter().find_map(|a| match a {
                        AbilityDef::Prepared { card } => Some(*card),
                        _ => None,
                    })
                })
            })
            .ok_or(EngineError::IllegalAction("no prepared spell"))?;
        let spell_def = self
            .lookup
            .card(linked_card)
            .ok_or(EngineError::IllegalAction("unknown linked card"))?;
        let face = &spell_def.faces[0];
        // The same question the offer asked, asked again here so the two
        // probes cannot disagree: a client that named this action out of a
        // stale `LegalActions` is refused rather than handed a sorcery on
        // the opponent's turn.
        if !self.prepared_cast_is_timely(player, spell_def) {
            return Err(EngineError::IllegalAction(
                "the prepared spell cannot be cast right now",
            ));
        }
        let cost = self.prepared_spell_price(player, spell_def);
        if !casting::pay_mana(&mut self.state, player, &cost) {
            return Err(EngineError::IllegalAction("cannot pay the spell's cost"));
        }
        // Unprepare the source.
        if let Some(obj) = self.state.object_mut(source) {
            obj.riders
                .retain(|r| !matches!(r, crate::object::Rider::Prepared));
        }
        // The copy of the linked spell, built the way `Effect::CopyTargetSpell`
        // builds one (CR 707.10): a fresh object carrying the copied card.
        let name = self.state.names.intern(face.name);
        let base = crate::object::Characteristics::from_face(spell_def, 0, name);
        let ts = self.state.next_timestamp();
        let id = self.state.arena.insert_with(|oid| {
            let mut obj = GameObject::new_bare(oid, player, ObjectKind::Spell, base);
            obj.timestamp = ts;
            obj.controlled_since = ts;
            obj.cast_from_hand = false;
            // A fresh object starts in its owner's library, and putting it
            // somewhere with `Zones::insert` does not say otherwise —
            // `move_object` reads the object, not the zone lists. Left at
            // the default, the spell resolved *out of the library*: the
            // stack kept the id, the spell ceased to exist beneath it, and
            // `stack_projectable` was still pointing at nothing.
            obj.zone = Zone::Stack;
            // And it has to *be* the linked card. `resolve_stack_top` reads a
            // spell's effects off `obj.card` rather than through
            // `GameObject::abilities`, so a card-less spell resolves to
            // nothing at all — which is what this one did: Demonic Tutor went
            // on the stack, both seats passed, and no library was ever
            // searched.
            obj.card = Some(crate::object::CardRef {
                index: linked_card,
                // The rules identity is the linked card; the *printing* is
                // one nobody brought to the table, and `PrintRef::new(0)`
                // here would be another card's art under this one's name —
                // and would hand the opponent a row of the print table out
                // of a deck they have never seen.
                print: baylee_core::ids::PrintRef::UNKNOWN,
            });
            // And it is still a copy, however it got here (CR 704.5e): the
            // spell was never a card anyone owns, so a graveyard is the one
            // place it must not end up.
            obj.riders.push(crate::object::Rider::SpellCopy);
            obj
        });
        self.state.put_new_spell_on_stack(id);
        // Per-turn tracking, exactly as an ordinary cast keeps it. The card
        // says "you may **cast** a copy of its spell", so this is a cast and
        // the turn has to count it: without these two the prepared spell was
        // invisible to Storm of Saruman's "your second spell each turn" and
        // handed Esper Sentinel's "first noncreature spell" to whatever was
        // cast next.
        if !face.types.contains(baylee_core::types::TypeSet::CREATURE)
            && let Some(v) = self
                .state
                .per_turn
                .noncreature_spells
                .get_mut(player.get() as usize)
        {
            *v = v.saturating_add(1);
        }
        if let Some(v) = self
            .state
            .per_turn
            .spells_cast
            .get_mut(player.get() as usize)
        {
            *v = v.saturating_add(1);
        }
        self.state
            .journal
            .record(GameEvent::SpellCast { object: id, player });
        self.after_action(player);
        Ok(())
    }
}
