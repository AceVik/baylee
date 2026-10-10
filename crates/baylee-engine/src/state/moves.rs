//! Objects changing zones, and what is remembered of them as they go.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary
use super::*;

impl GameState {
    /// Returns every exiled card whose link `ends` says has ended to the
    /// battlefield, under its owner's control, and forgets the link.
    ///
    /// The one way back for a card exiled with a link, whatever ended it:
    /// an effect of the host (`Effect::ReturnLinkedToBattlefield`), a new
    /// monarch ([`Self::set_monarch`]), or the host leaving the battlefield
    /// ([`Self::return_what_departed_hosts_held`]). `ends` is asked with the
    /// host and the link's `until`.
    ///
    /// Under its owner's control because every sentence that reaches here
    /// says so or says nothing (CR 610.3c), and written where it arrives: the
    /// default the card last had on the battlefield is whoever put it there,
    /// which after a reanimation or a blink "under your control" may not be
    /// its owner.
    pub(crate) fn return_linked(
        &mut self,
        ends: impl Fn(&Self, ObjectId, Option<crate::object::LinkUntil>) -> bool,
    ) {
        let mut returning = Vec::new();
        for seat in 0..self.players.len() {
            let p = PlayerId::new(seat as u8);
            for &card in self.zones.list(ZoneLocation::Exile(p)) {
                let ended = self.object(card).and_then(|o| {
                    o.riders.iter().find_map(|r| match *r {
                        crate::object::Rider::Linked { host, until } if ends(self, host, until) => {
                            Some(host)
                        }
                        _ => None,
                    })
                });
                if let Some(host) = ended {
                    returning.push((card, host));
                }
            }
        }
        for (card, host) in returning {
            if let Some(obj) = self.object_mut(card) {
                obj.kind = crate::object::ObjectKind::Permanent;
                obj.riders.retain(
                    |r| !matches!(r, crate::object::Rider::Linked { host: h, .. } if *h == host),
                );
                obj.set_controller(obj.owner);
            }
            let _ = self.move_object(
                card,
                ZoneLocation::Battlefield,
                ZonePosition::Top,
                Cause::Effect,
            );
        }
    }

    /// Returns what was exiled "until this creature leaves the battlefield"
    /// by a host that is no longer on it (CR 610.3).
    ///
    /// The return is the second one-shot effect CR 610.3 creates
    /// "immediately after the specified event", and not a triggered ability,
    /// so it is done at the event itself: [`Self::move_object`] calls this
    /// as a permanent leaves the battlefield, before anything else can
    /// happen (a state-based action, a trigger, the rest of the resolution
    /// that moved it), and `sba::eliminate_player` calls it once what the
    /// departed player owned has left the game with them (CR 800.4a). Those
    /// are the only two ways off the battlefield, which is what makes asking
    /// "is the host still there" enough: a host that was blinked is asked
    /// in the moment it is in exile, before it comes back as a new object.
    pub(crate) fn return_what_departed_hosts_held(&mut self) {
        self.return_linked(|state, host, until| {
            until == Some(crate::object::LinkUntil::HostLeaves)
                && state
                    .object(host)
                    .is_none_or(|h| h.zone != Zone::Battlefield)
        });
    }

    fn flashback_destination(&self, id: ObjectId, from: Zone, to: ZoneLocation) -> ZoneLocation {
        if from == Zone::Stack
            && to.zone() != Zone::Stack
            && let Some(obj) = self.object(id)
            && obj.riders.contains(&crate::object::Rider::Flashback)
        {
            ZoneLocation::Exile(obj.owner)
        } else {
            to
        }
    }

    /// Keep the source's last attachment on waiting abilities before it
    /// leaves or phases out (CR 113.7a, 608.2h). Later appearances of either
    /// handle cannot substitute for the captured incarnations.
    pub(crate) fn remember_source_attachment(&mut self, id: ObjectId) {
        let Some(source) = self.object(id) else {
            return;
        };
        let source_version = source.version;
        let attachment = source.attached_to.and_then(|host| {
            self.object(host).map(|object| {
                let version = if object.zone == Zone::Battlefield {
                    object.version
                } else {
                    self.ltb_versions
                        .iter()
                        .find(|(id, _)| *id == host)
                        .map_or(object.version, |(_, version)| *version)
                };
                [
                    crate::object::Rider::SourceAttachmentLki(host),
                    crate::object::Rider::SourceAttachmentVersion(version),
                ]
            })
        });
        for waiting in self.zones.list(ZoneLocation::Stack).clone() {
            if let Some(ability) = self.object_mut(waiting)
                && ability.ability.is_some_and(|loc| loc.source == id)
                && ability
                    .riders
                    .contains(&crate::object::Rider::AbilitySourceVersion(source_version))
            {
                ability.riders.retain(|rider| {
                    !matches!(
                        rider,
                        crate::object::Rider::SourceAttachmentLki(_)
                            | crate::object::Rider::SourceAttachmentVersion(_)
                    )
                });
                ability.riders.extend(attachment.into_iter().flatten());
            }
        }
    }

    /// Capture source information on its waiting abilities before this incarnation leaves.
    fn remember_ability_source(&mut self, id: ObjectId) {
        self.remember_source_attachment(id);
        let power = self
            .object(id)
            .and_then(|o| o.characteristics().power)
            .unwrap_or(0);
        for waiting in self.zones.list(ZoneLocation::Stack).clone() {
            if let Some(obj) = self.object_mut(waiting)
                && obj.ability.is_some_and(|loc| loc.source == id)
                && obj.source_power_lki.is_none()
            {
                obj.source_power_lki = Some(power);
            }
        }
    }

    /// Writes down what `id` was, one statement before the move erases it.
    ///
    /// The three look-back stores are one job and are done in one place
    /// because they share the moment: a leaves-the-battlefield or dies
    /// trigger fires from the zone the permanent has *arrived* in, and the
    /// game looks back at what was true "immediately prior to the event"
    /// (CR 603.10a). That is here — before the block at the bottom of
    /// [`move_object`] gives the copy back, empties `counters` and clears
    /// every other field only a permanent can have, and before the
    /// state-based actions that would unattach the Equipment.
    ///
    /// Each store is cleared for this object on **every** move and written
    /// again only on a departure from the battlefield, so each describes the
    /// last such departure and no earlier one.
    fn record_last_known(&mut self, id: ObjectId, from_zone: Zone) {
        self.ltb_mana_values.retain(|(other, _)| *other != id);
        self.ltb_controllers.retain(|(other, _)| *other != id);
        self.ltb_powers.retain(|(other, _)| *other != id);
        if from_zone == Zone::Battlefield
            && let Some(object) = self.object(id)
        {
            let controller = object.controller;
            let characteristics = object.characteristics();
            let (mana_value, power) = (characteristics.mana_value(), characteristics.power);
            self.ltb_mana_values.push((id, mana_value));
            if let Some(power) = power {
                self.ltb_powers.push((id, power));
            }
            self.ltb_controllers.push((id, controller));
        }
        if from_zone == Zone::Battlefield {
            self.remember_ability_source(id);
        }
        if from_zone == Zone::Battlefield
            && let Some(source) = self.object(id)
        {
            let (version, power) = (source.version, source.characteristics().power.unwrap_or(0));
            for waiting in self.zones.list(ZoneLocation::Stack).clone() {
                if let Some(ability) = self.object_mut(waiting)
                    && ability.event_object == Some(id)
                {
                    for rider in &mut ability.riders {
                        if let crate::object::Rider::EventObjectIdentity(v, p) = rider
                            && *v == version
                        {
                            *p = power;
                        }
                    }
                }
            }
        }
        // What the object could *do*.
        let departing = self.object(id).and_then(|o| {
            o.own_abilities
                .as_ref()
                .map(|abilities| crate::object::AbilityList {
                    abilities: abilities.into(),
                    printed: o.own_origin.and_then(crate::object::AbilityOrigin::printed),
                    token: o.own_origin.and_then(crate::object::AbilityOrigin::token),
                })
        });
        if from_zone == Zone::Battlefield {
            self.ltb_versions.retain(|(other, _)| *other != id);
            if let Some(o) = self.object(id) {
                self.ltb_versions.push((id, o.version));
            }
        }
        self.ltb_abilities.retain(|(other, _)| *other != id);
        if from_zone == Zone::Battlefield
            && let Some(abilities) = departing
        {
            self.ltb_abilities.push((id, abilities));
        }
        // What it wore. Undying and persist ask this of a creature that is
        // already in the graveyard, where the answer no longer exists.
        let departing_counters = self.object(id).map(|o| o.counters.clone());
        self.ltb_counters.retain(|(other, _)| *other != id);
        if from_zone == Zone::Battlefield
            && let Some(counters) = departing_counters
            && !counters.is_empty()
        {
            self.ltb_counters.push((id, counters));
        }
        // What it was.
        self.ltb_characteristics.retain(|(other, _)| *other != id);
        if from_zone == Zone::Battlefield
            && let Some(was) = self.object(id).map(|o| o.characteristics().clone())
        {
            self.ltb_characteristics.push((id, was));
        }
        // What was attached to it.
        self.ltb_attachments.retain(|(other, _)| *other != id);
        if from_zone == Zone::Battlefield {
            // A phased-out Aura was attached to nothing the rules can
            // see (CR 702.26b), so nothing looks back at it.
            let worn: Vec<ObjectId> = self
                .battlefield_seen()
                .filter(|other| {
                    self.object(*other)
                        .is_some_and(|o| o.attached_to == Some(id))
                })
                .collect();
            if !worn.is_empty() {
                self.ltb_attachments.push((id, worn));
            }
        }
    }

    /// Who controlled `id`, the way CR 608.2h reads an object an effect
    /// needs information from: the controller it had as it last existed on
    /// the battlefield if that is where it last left, and its controller now
    /// otherwise. `None` for an object that is gone and left no record.
    ///
    /// "Exile target creature. Its controller creates …" (Crib Swap) and
    /// "Exile target creature. Its controller gains life …" (Swords to
    /// Plowshares) read the second sentence after the first has moved the
    /// creature, and the field on the exiled card is no answer: nothing
    /// controls a card in exile, the field holds whatever the last refresh
    /// left there, and a refresh that reaches every zone (any effect whose
    /// filter names another zone) settles it to the card's default. After a
    /// steal that is the player it was stolen from.
    ///
    /// [`Self::ltb_controllers`] is cleared at every move and written only
    /// by a departure from the battlefield, so an entry is always the last
    /// word: a creature still on the battlefield, or a spell on the stack,
    /// has none and answers with the controller it has now.
    #[must_use]
    pub fn last_known_controller(&self, id: ObjectId) -> Option<PlayerId> {
        self.ltb_controllers
            .iter()
            .find(|(object, _)| *object == id)
            .map(|(_, seat)| *seat)
            .or_else(|| self.object(id).map(|o| o.controller))
    }

    /// Whether a static permission makes this player's library top public.
    #[must_use]
    pub fn library_top_revealed(&self, player: PlayerId) -> bool {
        self.effects.iter().any(|fx| {
            fx.controller == player
                && matches!(fx.modifier, baylee_cards_dsl::Modifier::RevealLibraryTop)
                && fx.source.is_none_or(|source| {
                    self.object(source)
                        .is_some_and(|o| o.zone == Zone::Battlefield)
                })
        })
    }

    /// Battlefield information before a simultaneous destruction starts.
    pub(crate) fn departure_snapshot(
        &self,
        id: ObjectId,
    ) -> Option<crate::damage_history::BattlefieldDeparture> {
        let object = self.object(id).filter(|o| o.zone == Zone::Battlefield)?;
        Some(crate::damage_history::BattlefieldDeparture {
            damage: self.damage_death_snapshot(id),
            event: crate::event::Departure {
                version: object.version,
                power: object.characteristics().power.unwrap_or(0),
                controller: object.controller,
                toughness: object.characteristics().toughness.unwrap_or(0),
                attachments: self
                    .battlefield_seen()
                    .filter(|other| {
                        self.object(*other)
                            .is_some_and(|o| o.attached_to == Some(id))
                    })
                    .collect(),
            },
        })
    }

    /// Moves an object between zones (CR 400.7: `version` bumps — it
    /// becomes a new object for rules that track identity).
    ///
    /// # Errors
    /// [`StateError::NoSuchObject`] for stale or unknown handles.
    ///
    /// # Panics
    /// Internal invariant violations (existence is checked above).
    #[allow(clippy::too_many_lines)] // one reset per field CR 400.7 clears
    pub fn move_object(
        &mut self,
        id: ObjectId,
        to: ZoneLocation,
        pos: ZonePosition,
        cause: Cause,
    ) -> Result<ObjectId, StateError> {
        self.move_object_with_departure(id, to, pos, cause, None)
    }

    /// A batch move supplies the battlefield information captured before any
    /// member of the simultaneous event left.
    #[allow(clippy::too_many_lines)] // one reset per field CR 400.7 clears
    pub(crate) fn move_object_with_departure(
        &mut self,
        id: ObjectId,
        to: ZoneLocation,
        pos: ZonePosition,
        cause: Cause,
        departure: Option<crate::damage_history::BattlefieldDeparture>,
    ) -> Result<ObjectId, StateError> {
        let (from_zone, from_player) = {
            let obj = self.object(id).ok_or(StateError::NoSuchObject(id))?;
            (obj.zone, obj.zone_owner.unwrap_or(obj.owner))
        };
        // CR 303.4i: an Aura forbidden from enchanting its intended host
        // does not enter at all. A spell goes to its owner's graveyard;
        // an Aura coming from another zone stays where it was.
        let blocked_aura = from_zone != Zone::Battlefield
            && to == ZoneLocation::Battlefield
            && self.object(id).is_some_and(|o| {
                o.characteristics()
                    .subtypes
                    .contains(baylee_core::generated::subtypes::enchantment::AURA)
                    && o.attached_to
                        .is_some_and(|host| !crate::eval::permits_enchantment(self, host, id))
            });
        let to = if blocked_aura {
            if let Some(o) = self.object_mut(id) {
                o.attached_to = None;
                o.kind = ObjectKind::Card;
            }
            if from_zone != Zone::Stack {
                return Ok(id);
            }
            ZoneLocation::Graveyard(self.object(id).expect("checked above").owner)
        } else {
            to
        };
        // CR 903.9b, applied here because here is the only place it can be
        // applied: the card must never reach the hand or the library it was
        // headed for. The answer was taken before the effect ran; all that
        // is left is to spend it.
        let to = self.flashback_destination(id, from_zone, to);
        let to = self.take_commander_redirect(id, to);
        let (to, exile_counter) = crate::replacement::graveyard_destination(self, id, to);
        let from_loc = ZoneLocation::of(from_zone, from_player);
        // A creature dies when it is put into a graveyard from the
        // battlefield (CR 700.4), and whether it was a creature is what it
        // was there: a land an effect animated dies as a creature, and the
        // card in the graveyard is a land. Asked before the move clears its
        // projection.
        let dies_as_a_creature = from_zone == Zone::Battlefield
            && to.zone() == Zone::Graveyard
            && self.object(id).is_some_and(|o| {
                o.characteristics()
                    .types
                    .contains(baylee_core::types::TypeSet::CREATURE)
            });
        // Record each exposed card before it leaves, including each draw of
        // a multi-card draw. Only the top was public, never the whole library.
        if from_zone == Zone::Library
            && self.library_top_revealed(from_player)
            && self.zones.list(from_loc).last() == Some(&id)
        {
            self.journal.record(GameEvent::Revealed {
                player: from_player,
                cards: vec![id],
            });
        }
        let departure = departure.or_else(|| self.departure_snapshot(id));
        self.remember_damage_source(id);
        let previous_source = self.source_identity(id);
        self.zones.remove(id, from_loc);
        self.timestamp += 1;
        let ts = self.timestamp;
        self.record_last_known(id, from_zone);
        if from_zone == Zone::Battlefield && to.zone() != Zone::Battlefield {
            // CR 400.7: a returning permanent is a new object even though
            // its arena handle is stable. Detach now, after departure
            // snapshots, so a blink cannot restore the old relationship
            // before the next SBA.
            // phasing: remember that a phased-out attachment must phase in
            // unattached if its original host left (CR 702.26i), even if
            // that handle has since returned. Do not affect it before then.
            for worn in self.zones.list(ZoneLocation::Battlefield).clone() {
                if let Some(attachment) = self.object_mut(worn)
                    && attachment.attached_to == Some(id)
                {
                    if attachment
                        .status
                        .contains(crate::object::Status::PHASED_OUT)
                    {
                        let marker = crate::object::Rider::AttachmentHostLeft;
                        if !attachment.riders.contains(&marker) {
                            attachment.riders.push(marker);
                        }
                    } else {
                        attachment.attached_to = None;
                    }
                }
            }
        }
        // Historical tallies remain available to that incarnation's pending
        // abilities. New incarnations have distinct keys (CR 400.7).
        let spell_version = self.object(id).map(|o| o.version);
        {
            let obj = self.object_mut(id).expect("checked above");
            obj.zone = to.zone();
            obj.zone_owner = to.player();
            obj.timestamp = ts;
            obj.controlled_since = ts;
            obj.version = obj.version.wrapping_add(1);
            // CR 400.7: it becomes a new object. The old projection must
            // not survive the move — the refresh pass only revisits the
            // battlefield and the stack, so a creature that died under an
            // anthem would otherwise sit in the graveyard still pumped,
            // and every filter reading its power would agree.
            obj.cache.clear();
            // And the rest of what the old object remembered, for the half
            // of it that only a *permanent* can have. Nothing cleared this,
            // so a permanent that left the battlefield carried its tapped
            // bit, its marked damage, its counters and what it was attached
            // to into the next zone and back out again: Ephemerate on a
            // tapped land returned it tapped, and a creature blinked with
            // three +1/+1 counters came back with them.
            //
            // `from_zone` and not "entering the battlefield", because these
            // fields are written *before* the arrival in more than one
            // place — `SearchDest::Battlefield` taps the land it fetched
            // and then moves it, so a reset on the way in would undo the
            // half of "put it onto the battlefield tapped" that does the
            // work.
            //
            // What stays: `riders` (a card exiled from the battlefield is
            // linked to whatever exiled it, which is the exception this
            // rule is written around, until it leaves exile: below, with
            // everything else it was in exile), and the spell-shaped fields
            // (`x_value`, `kicked`, `targets`), which a permanent resolving
            // off the stack still needs and which no permanent writes.
            if from_zone == Zone::Battlefield {
                obj.status = crate::object::Status::NONE;
                obj.damage = 0;
                obj.deathtouched = false;
                obj.regeneration_shields = 0;
                obj.attached_to = None;
                obj.riders
                    .retain(|r| *r != crate::object::Rider::AttachmentHostLeft);
                // A name chosen as it entered belongs to that permanent
                // (CR 400.7): a Pithing Needle bounced and cast again names
                // again, and nothing in between names anything.
                obj.chosen_name = None;
                obj.set_chosen_opponent(None);
                // A Room's designations are the permanent's (CR 709.5c), and
                // its face was the half they left showing. The card that
                // arrives is the card, left half first, and `original_base`
                // below brings its printed characteristics back.
                if obj.doors.is_room() {
                    obj.face_index = 0;
                }
                obj.doors = crate::object::Doors::NONE;
            }
            if matches!(from_zone, Zone::Battlefield | Zone::Exile) {
                obj.counters = crate::object::Counters::default();
            }
            // How a spell was cast belongs to the spell and to the permanent
            // it becomes, and to no later object (CR 400.7): a dashed
            // creature blinked or bounced and put back has had no dash cost
            // paid for it, and one that escaped and was blinked did not
            // escape. The cast writes the rider before the card moves to the
            // stack, so that move keeps it too.
            if to.zone() != Zone::Stack
                && !(from_zone == Zone::Stack && to.zone() == Zone::Battlefield)
            {
                // Evoke belongs to this casting too. Reanimating a card
                // that was evoked earlier must not sacrifice it again.
                obj.alt_cast = false;
                obj.riders.retain(|r| {
                    !matches!(
                        r,
                        crate::object::Rider::Dashed
                            | crate::object::Rider::Escaped
                            | crate::object::Rider::Masked
                    )
                });
            }
            // What the card was in exile lasts only as long as the exile. A
            // card that leaves exile any other way than the return its host
            // makes (cast, put into a hand, shuffled away) is a new object
            // with no relation to the exile it left: not "exiled with" its
            // host, not on an adventure, not suspended, not castable from
            // exile by anybody (`Rider::ends_as_it_leaves_exile`). The link
            // was kept, so a card cast out of Safe Haven's exile and later
            // hit by Swords to Plowshares came back when Safe Haven was
            // sacrificed. Not on a move from exile to exile, which is the one
            // move that is no leaving; `ExileLinked` and the other writers
            // put the rider on before they move the card.
            if from_zone == Zone::Exile && to.zone() != Zone::Exile {
                obj.riders.retain(|r| !r.ends_as_it_leaves_exile());
            }
            // What was paid is the spell's and no later object's (a flashback
            // is a new payment); nothing on the battlefield reads it yet, so
            // a resolved permanent spell gives it up too (`PaidRecord`).
            obj.paid = None;
            // The same rule for the other thing a copy replaced. Copiable
            // values are fixed while the copy exists (CR 707.2a) and the
            // new object has none of them: the card in the graveyard is
            // the card that was printed. Both halves go back together,
            // because a copy took both — `base` for the characteristics
            // and `own_abilities` for the rules text. A Glasspool Mimic
            // that copied a Wizard and then died is the case: left as a
            // copy it would be recast as one, and the enters-as-a-copy
            // ability it needs to be anything at all would be gone.
            //
            // Only for a card-backed object, because for every other kind
            // `own_abilities` *is* the object — an emblem (CR 114.2) and a
            // triggered ability on the stack (CR 113.7a, which is why it
            // captured its list) have no card to fall back to.
            if obj.card.is_some()
                && !((obj.prototyped || obj.status.contains(crate::object::Status::FACE_DOWN))
                    && from_zone == Zone::Stack
                    && to.zone() == Zone::Battlefield)
            {
                obj.prototyped = false;
                obj.status.remove(crate::object::Status::FACE_DOWN);
                if let Some(original) = obj.original_base.take() {
                    obj.base = original;
                }
                obj.drop_own_abilities();
                // And the flag that says when the field it just cleared was
                // due back, because it describes that copy and the copy ends
                // here. A Cursed Mirror that bounced and was recast without
                // copying anything would otherwise be swept at the next
                // cleanup as though it were still one — harmless for a turn
                // and a lie in the meantime.
                obj.own_abilities_until_eot = false;
            }
        }
        // CR 400.7a, the exception to the new object above: what a spell or
        // ability did to a permanent spell, it goes on doing to the
        // permanent (`EffectTable::follow_into_permanent`).
        if from_zone == Zone::Stack
            && to.zone() == Zone::Battlefield
            && let (Some(spell), Some(permanent)) =
                (spell_version, self.object(id).map(|o| o.version))
        {
            self.effects.follow_into_permanent(id, spell, permanent);
        }
        let projectable = self
            .object(id)
            .is_none_or(|o| o.kind != ObjectKind::AbilityOnStack);
        self.zones.insert(id, to, pos, projectable);
        // A card-less object that lands anywhere but the battlefield or the
        // stack is a cleanup candidate (CR 704.5d), and so is a copy of a
        // spell, which carries a card and would otherwise be invisible here
        // (CR 704.5e). The stack is excluded because a copy of a spell
        // legitimately lives there and must be allowed to resolve — the bug
        // this rule already caused once, recorded in `sba::run`.
        if !matches!(to.zone(), Zone::Battlefield | Zone::Stack)
            && self.object(id).is_some_and(|o| !o.is_card())
        {
            self.watch_token_cleanup(id);
        }
        // CR 506.4: a permanent that leaves the battlefield is removed from
        // combat, and this is the one place every departure passes through —
        // a death, a bounce, an exile, and the exile-and-return that is a
        // blink. The id is no help on its own: it is an arena handle and the
        // same one comes back, so a creature that was blinked mid-combat was
        // still declared as an attacker, still counted as blocked, and still
        // traded damage with a blocker it had never met.
        if from_zone == crate::zone::Zone::Battlefield {
            self.combat.remove_from_combat(id);
        }
        // Creature deaths this turn (Emeritus of Woe's re-prepare, Scavenging
        // Ghoul's corpse counters).
        if dies_as_a_creature {
            self.per_turn.creatures_died = self.per_turn.creatures_died.saturating_add(1);
        }
        // The projected set just changed, and the generation compare that
        // guards a refresh counts *effects* — so nothing would have
        // recomputed this object, or the ones that count it.
        //
        // Both directions, and both zones. An arrival keeps its cleared
        // cache, which reads as the printed card: a creature cast into a
        // board that already had an anthem on it stood there at its printed
        // power, and — the way this was found — a creature cast after a
        // Toxic Deluge resolved was the one creature on the battlefield the
        // Deluge did not shrink. A departure is the other half, because a
        // projection may count the board: `Modifier::ModifyPTPerCount` is
        // "+1/+1 for each artifact you control", so a permanent leaving
        // changes what a permanent that stayed projects to.
        //
        // The battlefield and the stack, which is exactly what the refresh
        // pass revisits. And the graveyards, because a permanent's
        // projection may count them: Pyrogoyf is as big as the card types
        // among cards in all graveyards (`PtCount::CardTypesInAllGraveyards`),
        // so a card milled or discarded grows a permanent that never moved.
        // And exile, for the same reason one zone over: Unlicensed Hearse is
        // as big as the cards exiled with it (`PtCount::ExiledWithThis`), and
        // one of them leaving exile shrinks it.
        //
        // And every zone at all while a cross-zone effect is registered
        // (Maskwood Nexus: "Creatures you control are every creature type.
        // The same is true for creature spells you control and creature
        // cards you own that aren't on the battlefield."). The pass that
        // honours one projects every object, but it runs only when the
        // generation moved, and a card drawn, tutored, wished for or put
        // back moves none: the move above cleared its cache, so a creature
        // card drawn under a Nexus read as its printed self in hand until
        // something else moved. `projected_cross_zone` says whether the last
        // pass was such a pass.
        if matches!(
            from_zone,
            Zone::Battlefield | Zone::Stack | Zone::Graveyard | Zone::Exile
        ) || matches!(
            to.zone(),
            Zone::Battlefield | Zone::Stack | Zone::Graveyard | Zone::Exile
        ) || self.projected_cross_zone
        {
            self.invalidate_projections();
        }
        // A card that defines its own power and toughness is projected in
        // every zone (CR 604.3), and the move just cleared its cache: a
        // drawn Ashaya would read its printed 0/0 until something else
        // moved.
        if self.printed_pt_cda.iter().any(|(card, _)| *card == id) {
            self.invalidate_projections();
        }
        if to.zone() == Zone::Battlefield {
            self.per_turn.entered_battlefield.push(id);
        }
        // "For as long as you control [this]" ends as its source leaves
        // (CR 611.2b), here rather than at the next pass over the effect
        // table: a blink is back before any pass runs, and what returns is a
        // new object (CR 400.7) that the effect never named.
        if from_zone == Zone::Battlefield {
            self.effects.remove_where(|fx| {
                matches!(
                    fx.duration,
                    baylee_cards_dsl::Duration::WhileYouControlSource
                        | baylee_cards_dsl::Duration::WhileSourceTapped
                ) && fx.source == Some(id)
            });
        }
        if to.zone() == Zone::Graveyard {
            self.per_turn.entered_graveyard.push(id);
        }
        // Read off `to` after the redirects above, so a commander that went
        // to the command zone instead names no place in a library.
        let place = match to {
            ZoneLocation::Library(_) => Some(library_place(pos, self.zones.list(to).len())),
            _ => None,
        };
        let departure = departure.map(|snapshot| {
            if to.zone() == Zone::Graveyard
                && let Some(mut death) = snapshot.damage
            {
                death.seq = self.journal.last_seq() + 1;
                self.damage_deaths.push(death);
            }
            snapshot.event
        });
        self.journal.record_departure(
            GameEvent::ZoneChanged {
                object: id,
                from: from_zone,
                to: to.zone(),
                cause,
                place,
            },
            departure,
        );
        if let Some(kind) = exile_counter {
            crate::replacement::put_counters(self, id, kind, 1);
        }
        // What this permanent held "until it leaves the battlefield" comes
        // back now, immediately after the event and before anything else
        // (CR 610.3).
        if from_zone == Zone::Battlefield {
            self.return_what_departed_hosts_held();
        }
        if let Some(previous) = previous_source {
            self.source_moved(previous);
        }
        Ok(id)
    }

    /// Whether CR 903.9b has anything to say about this move: is the object
    /// somebody's commander, and is it on its way to a hand or a library?
    ///
    /// Being a commander is [`Commander::object`], not a characteristic: the
    /// id survives the zone changes that make the card a new object
    /// (CR 400.7), which is the whole reason the marker is an id.
    #[must_use]
    pub fn commander_owner(&self, id: ObjectId, to: ZoneLocation) -> Option<PlayerId> {
        if !matches!(to, ZoneLocation::Hand(_) | ZoneLocation::Library(_)) {
            return None;
        }
        // Nothing here excludes a move from a library back into the same
        // library, because nothing has to. CR 400.7 makes that no zone
        // change at all, and it is the *callers* that enforce it: scry,
        // dig and every other library-internal reorder move their cards
        // without going near `ask_commander_replace`. A guard here would
        // have read as load-bearing while no call site could reach it.
        self.commanders
            .iter()
            .enumerate()
            .find(|(_, list)| list.iter().any(|c| c.object == id))
            .and_then(|(seat, _)| u8::try_from(seat).ok())
            .map(PlayerId::new)
    }

    /// Consumes the answer recorded for `id` and says where it really goes.
    ///
    /// A missing entry means nobody was asked — every non-commander move,
    /// and the sites listed in `docs/engine-internals.md` that this rule
    /// does not reach yet — so the destination stands unchanged.
    fn take_commander_redirect(&mut self, id: ObjectId, to: ZoneLocation) -> ZoneLocation {
        let Some(at) = self.commander_redirect.iter().position(|(o, _)| *o == id) else {
            return to;
        };
        let (_, home) = self.commander_redirect.remove(at);
        if !home {
            return to;
        }
        // The *owner's* command zone, for CR 903.9a's reason: a commander
        // stolen and then bounced goes home to whoever brought it.
        let Some(obj) = self.object_mut(id) else {
            return to;
        };
        let owner = obj.owner;
        // It stops being a permanent on the way, exactly as it would have
        // stopped being one on the way to a hand.
        obj.kind = ObjectKind::Card;
        ZoneLocation::Command(owner)
    }

    /// Shuffles a player's library (journaled).
    pub fn shuffle_library(&mut self, player: PlayerId) {
        let loc = ZoneLocation::Library(player);
        let rng = &mut self.rng;
        rng.shuffle(self.zones.list_mut(loc).as_mut_slice());
        self.journal.record(GameEvent::Shuffled {
            player,
            zone: Zone::Library,
        });
    }

    /// Shuffles a player's library with a stream other than the table's: a
    /// mulligan's, which is the seat's own ([`GameRng::for_seat`]).
    pub fn shuffle_library_with(&mut self, player: PlayerId, rng: &mut GameRng) {
        rng.shuffle(
            self.zones
                .list_mut(ZoneLocation::Library(player))
                .as_mut_slice(),
        );
        self.journal.record(GameEvent::Shuffled {
            player,
            zone: Zone::Library,
        });
    }

    /// `player` draws `n` cards: the one door every draw goes through.
    ///
    /// A draw Island Sanctuary may replace is not made here: it waits in
    /// [`GameState::draws_to_offer`] to be asked about, one card at a time,
    /// and so does every draw behind one already waiting, so that the order
    /// the instructions gave holds (CR 121.2c). Nothing is returned for a
    /// draw that waits.
    pub fn draw_cards(&mut self, player: PlayerId, n: usize) -> Vec<ObjectId> {
        if n > 0
            && (!self.draws_to_offer.is_empty() || self.draw_skip_source(player, &[]).is_some())
        {
            self.draws_to_offer
                .push_back((player, u32::try_from(n).unwrap_or(u32::MAX)));
            return Vec::new();
        }
        self.draw_now(player, n)
    }

    /// Moves the top `n` cards of a player's library to their hand.
    /// Drawing from an empty library flags [`Player::tried_empty_draw`] —
    /// the loss is a state-based action (CR 704.5b).
    pub(crate) fn draw_now(&mut self, player: PlayerId, n: usize) -> Vec<ObjectId> {
        let mut drawn = Vec::with_capacity(n);
        let first_of_turn = self
            .per_turn
            .draws
            .get(player.get() as usize)
            .copied()
            .unwrap_or(0)
            == 0;
        let already = self
            .per_turn
            .draws
            .get(player.get() as usize)
            .copied()
            .unwrap_or(0);
        let limit = self.draw_limit(player);
        for _ in 0..n {
            // CR 121.2b, and the loop is where it belongs: the effect
            // "applies to individual card draws", so "draw three cards"
            // under a limit of one is partially carried out rather than
            // refused. `already` is read once because `per_turn.draws` is
            // not written until the loop is over.
            if limit.is_some_and(|cap| already + drawn.len() as u32 >= cap) {
                break;
            }
            let Some(&top) = self.zones.list(ZoneLocation::Library(player)).last() else {
                if let Some(p) = self.players.get_mut(player.get() as usize) {
                    p.tried_empty_draw = true;
                }
                break;
            };
            if self
                .move_object(
                    top,
                    ZoneLocation::Hand(player),
                    ZonePosition::Top,
                    Cause::Effect,
                )
                .is_ok()
            {
                drawn.push(top);
                if let Some(version) = self.object(top).map(|o| o.version) {
                    self.per_turn.drawn.push((top, version));
                }
            }
        }
        // Miracle (CR 702.94): the first card drawn this turn may be
        // revealed and cast for its miracle cost — the engine offers it.
        if first_of_turn && let Some(&card) = drawn.first() {
            self.pending_miracle.push_back((player, card));
        }
        if !drawn.is_empty() {
            if let Some(v) = self.per_turn.draws.get_mut(player.get() as usize) {
                *v = v.saturating_add(drawn.len() as u32);
            }
            // "The first one they draw in each of their draw steps" is a
            // fact about this draw, so it is written down now. The turn's
            // count cannot stand in for it: a card drawn in the upkeep
            // would make the draw step's first card the turn's second.
            let in_own_draw_step =
                self.turn.active == player && self.turn.step == crate::turn::Step::Draw;
            let first_in_draw_step = in_own_draw_step && !self.per_turn.drew_in_draw_step;
            if in_own_draw_step {
                self.per_turn.drew_in_draw_step = true;
            }
            self.journal.record(crate::event::GameEvent::CardsDrawn {
                player,
                count: drawn.len() as u16,
                first_in_draw_step,
            });
        }
        drawn
    }
}
