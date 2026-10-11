//! Prepared designations and their persistent copies in exile (CR 722.3c).
use crate::object::{CardRef, Characteristics, GameObject, ObjectKind, Rider, Status};
use crate::state::{CardLookup, GameState, PlayLifetime, PlayPermission};
use crate::zone::{Zone, ZoneLocation, ZonePosition};
use baylee_cards_dsl::AbilityDef;
use baylee_core::ids::{DamageSourceRef, ObjectId, PlayerId, PrintRef};

impl GameState {
    /// Gain the designation once, only if prepare characteristics exist.
    pub(crate) fn prepare(&mut self, source: ObjectId) {
        let eligible = self.object(source).is_some_and(|object| {
            object.zone == Zone::Battlefield
                && !object.status.contains(Status::PHASED_OUT)
                && !object.riders.contains(&Rider::Prepared)
        }) && self.prepare_card(source).is_some();
        if !eligible {
            return;
        }
        self.object_mut(source)
            .expect("eligible permanent")
            .riders
            .push(Rider::Prepared);
        self.create_prepared_copy(source);
    }

    fn prepare_card(&self, source: ObjectId) -> Option<(baylee_core::ids::CardIndex, u8)> {
        self.printed_ability_list(source)?
            .abilities
            .iter()
            .find_map(|ability| match ability {
                AbilityDef::Prepared { card, face } => Some((*card, *face)),
                _ => None,
            })
    }

    /// Read live presence, never last-known information or a replacement incarnation.
    pub(crate) fn prepared_controller(&self, source: DamageSourceRef) -> Option<PlayerId> {
        self.object(source.object)
            .filter(|object| {
                object.version == source.version
                    && object.zone == Zone::Battlefield
                    && !object.status.contains(Status::PHASED_OUT)
                    && object.riders.contains(&Rider::Prepared)
            })
            .map(|object| object.controller)
    }

    /// Becoming prepared and phasing in prepared are the only creation events.
    /// Changing copiable values later does not change an existing exiled copy.
    pub(crate) fn create_prepared_copy(&mut self, source: ObjectId) {
        let Some(identity) = self.source_identity(source) else {
            return;
        };
        let Some(player) = self.prepared_controller(identity) else {
            return;
        };
        let Some((card, face)) = self.prepare_card(source) else {
            return;
        };
        let Some(def) = self.bases.card(card) else {
            return;
        };
        let Some(spell) = def.faces.get(usize::from(face)) else {
            return;
        };
        let name = self.names.intern(spell.name);
        let base = Characteristics::from_face(def, usize::from(face), name);
        let timestamp = self.next_timestamp();
        let copy = self.arena.insert_with(|id| {
            let mut object = GameObject::new_bare(id, player, ObjectKind::Card, base);
            object.timestamp = timestamp;
            object.controlled_since = timestamp;
            object.zone = Zone::Exile;
            object.zone_owner = Some(player);
            object.card = Some(CardRef {
                index: card,
                print: PrintRef::UNKNOWN,
            });
            object.face_index = face;
            object.riders.push(Rider::SpellCopy);
            object.riders.push(Rider::PrepareSpellFace(face));
            object
        });
        self.zones
            .insert(copy, ZoneLocation::Exile(player), ZonePosition::Top, true);
        self.play_permissions.push(PlayPermission {
            player,
            card: copy,
            version: self.object(copy).expect("new copy").version,
            free: false,
            mana_any_type: false,
            cast_only: true,
            spell_types: None,
            window: PlayLifetime::Prepared(identity),
        });
        self.invalidate_projections();
    }

    /// Update control and expire copies without scanning the battlefield or arena.
    /// An expired entry is queued for the normal copy state-based action.
    pub(crate) fn sync_prepared_copies(&mut self) {
        let mut permissions = std::mem::take(&mut self.play_permissions);
        permissions.retain_mut(|permission| {
            let PlayLifetime::Prepared(source) = permission.window else {
                return true;
            };
            let controller = self.prepared_controller(source);
            let still_exiled = self.object(permission.card).is_some_and(|object| {
                object.zone == Zone::Exile && object.version == permission.version
            });
            if let Some(controller) = controller.filter(|_| still_exiled) {
                permission.player = controller;
                true
            } else {
                self.watch_token_cleanup(permission.card);
                false
            }
        });
        self.play_permissions = permissions;
    }

    /// The live exiled prepare spell associated with this permanent.
    #[must_use]
    pub fn prepared_copy(&self, source: ObjectId) -> Option<ObjectId> {
        self.play_permissions
            .iter()
            .find_map(|permission| match permission.window {
                PlayLifetime::Prepared(reference)
                    if reference.object == source
                        && self.prepared_controller(reference).is_some()
                        && self.object(permission.card).is_some_and(|object| {
                            object.zone == Zone::Exile && object.version == permission.version
                        }) =>
                {
                    Some(permission.card)
                }
                _ => None,
            })
    }

    /// The exception to the copy cleanup rule and the link consumed on casting.
    pub(crate) fn prepared_copy_source(&self, copy: ObjectId) -> Option<DamageSourceRef> {
        let object = self.object(copy)?;
        if object.zone != Zone::Exile {
            return None;
        }
        self.play_permissions.iter().find_map(|permission| {
            if permission.card != copy || permission.version != object.version {
                return None;
            }
            match permission.window {
                PlayLifetime::Prepared(source) if self.prepared_controller(source).is_some() => {
                    Some(source)
                }
                _ => None,
            }
        })
    }

    /// Called only after all costs are paid and the spell becomes cast.
    pub(crate) fn unprepare(&mut self, source: DamageSourceRef) {
        if let Some(object) = self.object_mut(source.object)
            && object.version == source.version
        {
            object.riders.retain(|rider| *rider != Rider::Prepared);
        }
    }
}
