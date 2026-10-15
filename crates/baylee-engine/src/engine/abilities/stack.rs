//! Capturing activated and triggered abilities on the stack.
#[allow(clippy::wildcard_imports)] // shared ability vocabulary
use super::*;

impl<L: CardLookup> Engine<L> {
    /// Puts one of `source`'s abilities on the stack (CR 603.3 for a
    /// trigger, CR 602.2a for an activation).
    ///
    /// It cannot fail, and that is the point. It used to refuse a source
    /// with no card — and both of its trigger callers discarded the refusal
    /// with `let _ =`, so a token's triggered ability was thrown away
    /// silently and the line after the call then wrote the trigger's
    /// `event_object` onto whatever happened to be on top of the stack
    /// instead, which is a bystander's target context taking a stranger's.
    /// The emblem case was walked around rather than fixed: a second,
    /// almost identical function existed for it and the callers chose
    /// between them on `ObjectKind::Emblem`, which is the wrong question —
    /// what this needs to know is whether the source has a card, and an
    /// emblem is only one of the things that has none.
    ///
    /// Returns the ability object it made, because two callers need it back
    /// and both used to find it by reading the top of the stack — true, and
    /// true only because nothing runs in between.
    pub(crate) fn push_ability_to_stack(
        &mut self,
        controller: PlayerId,
        source: ObjectId,
        ability_index: u32,
        targets: SmallVec<[ObjectId; 2]>,
    ) -> ObjectId {
        // An emblem (CR 114.2), a token and a token copy have no card, and
        // the handle a client is given says so. What resolves is the list
        // captured below (CR 608.2), as for any ability.
        let card = self
            .state
            .object(source)
            .filter(|o| !o.status.contains(crate::object::Status::FACE_DOWN))
            .and_then(|o| o.card)
            .map(|c| c.index);
        let name = self
            .state
            .object(source)
            .map_or(NameRef::new(0), |o| o.base.name);
        let base = self.state.bare_base(name);
        // CR 113.7a: the ability on the stack exists independently of the
        // permanent it came from, so it takes the list `index` points into
        // with it. Reading it back off the source at resolution time was
        // right only while a source's abilities could not change under it —
        // a copy that dies with its ability on the stack stops being a copy
        // (CR 400.7), and the index would then be read against the printed
        // card, which is a different ability or none at all.
        //
        // The slot holds the list for the *next* push, and two callers fill
        // it: an activation, which captured its list before paying a cost
        // that may already have moved the source, and a look-back trigger
        // (CR 603.10a), whose source stopped being a copy on the way off the
        // battlefield. That second one is the hazard the paragraph above
        // describes, and it happened — Phyrexian Metamorph copying Solemn
        // Simulacrum died and drew nobody a card, because the list read back
        // here was the printed Metamorph's and has no dies trigger in it.
        // Everything else still reads the source, which is right by
        // definition while it is still standing there.
        let captured = self
            .activating_abilities
            .take()
            .and_then(|(id, abilities)| (id == source).then_some(abilities));
        let list = captured.unwrap_or_else(|| {
            self.state
                .object(source)
                .map_or(crate::object::AbilityList::NONE, |o| {
                    o.ability_list(&self.lookup)
                        .with_base_text(self.state.text_changes.get(
                            baylee_core::ids::DamageSourceRef {
                                object: source,
                                version: o.version,
                            },
                        ))
                })
        });
        let captured_text = list.base_text(ability_index as usize);
        let abilities = list.abilities.clone();
        // Retargeting and copying read the captured requirement without a card
        // lookup. Modal triggers bind theirs when their selected mode is stored.
        let target_req = if matches!(
            abilities.get(ability_index as usize),
            Some(
                baylee_cards_dsl::AbilityDef::ModalTriggered { .. }
                    | baylee_cards_dsl::AbilityDef::ModalActivated { .. }
            )
        ) {
            None
        } else {
            crate::object::ability_target_req(abilities.clone(), ability_index, None)
        };
        let source_version = self.state.object(source).map(|o| o.version);
        let counter_version = crate::resolve::linked_counters::uses_links(&abilities)
            .then(|| self.state.object(source).map_or(0, |o| o.version));
        // CR 107.3m: an object's **own** enters-the-battlefield triggered
        // ability that refers to X uses the X chosen for the spell that
        // became that object, although X for the permanent itself is 0. The
        // Meathook Massacre is the card that reads it — `{X}{B}{B}`, "when
        // this enters, each creature gets -X/-X" — and without this the
        // trigger resolved at nought: a `Coverage::Implemented` enchantment
        // that swept no board. `Trigger::ETB` and not any entering trigger,
        // because the rule says *its* enter trigger: a landfall
        // `EntersBattlefield(&Filter::YOUR_LAND)` is about another permanent
        // and announces nothing. Nothing is guarded here: a permanent's
        // `x_value` is normalised the moment it enters
        // (`progress::apply_enter_modifiers`), so on the battlefield the
        // field already means "the X of the spell that became this, or
        // nothing" and a reanimated body announces a 0.
        let announced_x = matches!(
            abilities.get(ability_index as usize),
            Some(
                baylee_cards_dsl::AbilityDef::Triggered {
                    trigger: baylee_cards_dsl::Trigger::ETB,
                    ..
                } | baylee_cards_dsl::AbilityDef::ModalTriggered {
                    trigger: baylee_cards_dsl::Trigger::ETB,
                    ..
                }
            )
        )
        .then(|| self.state.object(source).map_or(0, |o| o.x_value))
        .unwrap_or(0);
        let id = self.state.arena.insert_with(|id| {
            let mut obj = GameObject::new_ability_on_stack(
                id,
                controller,
                AbilityLoc {
                    card,
                    index: ability_index,
                    source,
                },
                targets,
                base,
            );
            obj.take_abilities(list);
            obj.target_req = target_req;
            if let Some(version) = source_version {
                obj.riders
                    .push(crate::object::Rider::AbilitySourceVersion(version));
            }
            obj.x_value = announced_x;
            if let Some(version) = counter_version {
                obj.riders
                    .push(crate::object::Rider::CounterSourceVersion(version));
            }
            obj
        });
        self.state
            .zones
            .insert(id, ZoneLocation::Stack, ZonePosition::Top, false);
        if let Some(reference) = self.state.source_identity(id) {
            self.state.text_changes.set(reference, captured_text);
        }
        // The targets a trigger was put on the stack with are the ones it
        // announced, so they are remembered before the event that says so:
        // "becomes the target" and ward read this event against those
        // references, and a Phantasmal Image copy aimed at by a Flametongue
        // Kavu's trigger was never sacrificed.
        self.state.capture_source_references();
        self.state.journal.record(GameEvent::AbilityTriggered {
            object: id,
            source,
            ability_index,
            controller,
        });
        id
    }
}
