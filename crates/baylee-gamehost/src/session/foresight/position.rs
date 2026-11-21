//! World seeds for frozen house policies use their original position encoding.

use std::hash::{Hash, Hasher};

use baylee_ai::HeuristicAgent;
use baylee_core::preset::AIProfile;
use baylee_view::{CounterEntry, CounterKind, PlayerView, PublicObject};
use serde::ser::{Serialize, SerializeStruct, Serializer};

pub(in crate::session) fn world_position(view: &PlayerView, profile: AIProfile) -> u64 {
    if profile.plays(AIProfile::CLASS_UPGRADES) {
        return HeuristicAgent::position(view);
    }
    let seats: Vec<_> = view
        .seats
        .iter()
        .map(|seat| {
            (
                seat.player,
                seat.life,
                seat.poison,
                seat.energy,
                seat.hand_count,
                seat.library_count,
                seat.graveyard_count,
                &seat.mana_pool,
            )
        })
        .collect();
    let bytes = baylee_core::wire::encode(&(
        (view.turn, view.phase, view.step, view.active, view.seat),
        (view.priority_held, view.casting, view.announced_cast),
        &seats,
        &view.hand,
        LegacyObjects(&view.battlefield),
        LegacyObjects(&view.stack),
        LegacyZones(&view.graveyards),
        LegacyZones(&view.exile),
        LegacyZones(&view.command),
        &view.combat,
        &view.owed,
    ));
    let mut hasher = std::hash::DefaultHasher::new();
    bytes.hash(&mut hasher);
    hasher.finish()
}

struct LegacyObjects<'a>(&'a [PublicObject]);

impl Serialize for LegacyObjects<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_seq(self.0.iter().map(LegacyObject))
    }
}

struct LegacyZones<'a>(&'a [Vec<PublicObject>]);

impl Serialize for LegacyZones<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_seq(self.0.iter().map(|zone| LegacyObjects(zone)))
    }
}

/// The public object layout used to seed worlds before Class was a
/// designation. The older byte stream had one fewer field and encoded the
/// gained levels with a level counter. Keep that projection inside the seed
/// so the frozen profile receives the same visible options as any other.
struct LegacyObject<'a>(&'a PublicObject);

impl Serialize for LegacyObject<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let object = self.0;
        let mut fields = serializer.serialize_struct("PublicObject", 51)?;
        macro_rules! write_fields {
            ($($field:ident),* $(,)?) => {
                $(fields.serialize_field(stringify!($field), &object.$field)?;)*
            };
        }
        // Postcard writes the fields in this order, without their names.
        write_fields!(
            word_changes,
            id,
            card,
            rules,
            prepared_spell,
            ability_sources,
            name,
            controller,
            owner,
            battle_protector,
            commander,
            status,
            types,
            supertypes,
            subtypes,
            chosen_subtype,
            second_chosen_subtype,
            chosen_card_type,
            chosen_number,
            chosen_name,
            chosen_opponent,
            chosen_color,
            chosen_x,
            linked_exile,
            unlocked_doors,
            suspended,
            plotted,
            warped,
            suspected,
            token,
            colors,
            mana_value,
            keywords,
            power,
            toughness,
            base_power,
            base_toughness,
            loyalty,
            damage,
        );
        fields.serialize_field(
            "counters",
            &LegacyCounters(
                &object.counters,
                u16::from(object.class_level.unwrap_or(1).saturating_sub(1)),
            ),
        )?;
        write_fields!(
            attached_to,
            targets,
            stack_item,
            times_kicked,
            bought_back,
            summoning_sick,
            granted_mana,
            ability_mana_spending,
            board_mana,
            flashback,
            grants,
        );
        fields.end()
    }
}

/// The counter table retained insertion order. Keep the position of a real
/// level counter if present and otherwise append the former Class marker.
struct LegacyCounters<'a>(&'a [CounterEntry], u16);

impl Serialize for LegacyCounters<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let Self(counters, level) = *self;
        if level == 0 {
            return counters.serialize(serializer);
        }
        let added = (!counters
            .iter()
            .any(|entry| entry.kind == CounterKind::Level))
        .then_some(CounterEntry {
            kind: CounterKind::Level,
            count: level,
        });
        serializer.collect_seq(
            counters
                .iter()
                .copied()
                .map(|mut entry| {
                    if entry.kind == CounterKind::Level {
                        entry.count = entry.count.saturating_add(level);
                    }
                    entry
                })
                .chain(added),
        )
    }
}
