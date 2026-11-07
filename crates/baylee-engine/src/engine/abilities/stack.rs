//! Capturing activated and triggered abilities on the stack.
#[allow(clippy::wildcard_imports)] // shared ability vocabulary
use super::*;

impl<L: CardLookup> Engine<L> {
    /// The card and the name an ability on the stack goes by: its source's.
    ///
    /// An emblem (CR 114.2), a token and a token copy have no card, and the
    /// handle a client is given says so; what resolves is the list captured
    /// beside it (CR 608.2), as for any ability. A spell's copy has ceased to
    /// exist by the time a delayed ability it created comes due, and that list
    /// is then what still says which card it was.
    ///
    /// Only a spell's list. A token gone with a look-back trigger in hand has
    /// always gone by the first name interned and no card, and those are on
    /// the stack object the hash reads, so a written record holds them.
    fn ability_identity(
        &mut self,
        source: ObjectId,
        captured: Option<&crate::object::AbilityList>,
    ) -> (Option<baylee_core::ids::CardIndex>, NameRef) {
        if let Some(object) = self.state.object(source) {
            let card = object
                .card
                .filter(|_| !object.status.contains(crate::object::Status::FACE_DOWN))
                .map(|c| c.index);
            return (card, object.base.name);
        }
        let printed = captured
            .filter(|list| {
                list.abilities.iter().any(|ability| {
                    matches!(
                        ability,
                        baylee_cards_dsl::AbilityDef::Spell { .. }
                            | baylee_cards_dsl::AbilityDef::ModalSpell { .. }
                    )
                })
            })
            .and_then(|list| list.printed);
        let name = printed
            .and_then(|printed| {
                let def = self.lookup.card(printed.card())?;
                Some(def.faces.get(usize::from(printed.face()))?.name)
            })
            .map_or(NameRef::new(0), |name| self.state.names.intern(name));
        (printed.map(crate::object::PrintedFace::card), name)
    }

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
        let captured = self
            .activating_abilities
            .take()
            .and_then(|(id, abilities)| (id == source).then_some(abilities));
        let (card, name) = self.ability_identity(source, captured.as_ref());
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
        //
        // "If it was kicked" and "for each time it was kicked" ride along by
        // the same door and for the same reason: the ability is asked what
        // was paid for the spell that became its source (CR 400.7d, 607.2i),
        // and that is `Effect::IfKicked`'s question of the object on the
        // stack.
        let announced = matches!(
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
        .then(|| self.state.object(source))
        .flatten()
        .map_or((0, 0), |o| (o.x_value, o.times_kicked));
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
            (obj.x_value, obj.times_kicked) = announced;
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

/// The engine's own steps, named for the card test crates (`test-support`) and
/// their kit: the steps stay the engine's, these only call them.
#[cfg(any(test, feature = "test-support"))]
impl<L: CardLookup> Engine<L> {
    #[doc(hidden)]
    pub fn push_ability_to_stack_for_tests(
        &mut self,
        controller: PlayerId,
        source: ObjectId,
        ability_index: u32,
        targets: SmallVec<[ObjectId; 2]>,
    ) -> ObjectId {
        self.push_ability_to_stack(controller, source, ability_index, targets)
    }
}
