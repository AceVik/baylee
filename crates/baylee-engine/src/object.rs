//! Game objects and their characteristics.
//!
//! Identity model: an object's [`ObjectId`] is stable for its whole
//! lifetime; zone changes bump [`GameObject::version`] (CR 400.7 — "it
//! becomes a new object"). Effects and targets that must track identity
//! record `(ObjectId, version)`; blinked permanents naturally invalidate
//! old references.

use crate::zone::Zone;
use baylee_cards_dsl::{CardDef, KeywordSet};
use baylee_core::color::ColorSet;
use baylee_core::ids::{CardIndex, NameRef, ObjectId, PlayerId, PrintRef};
use baylee_core::mana::ManaCost;
use baylee_core::types::{SubtypeSet, SupertypeSet, TypeSet};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

/// What kind of object this is.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum ObjectKind {
    /// A card outside the battlefield (library, hand, graveyard, exile, command).
    Card,
    /// A card (or copy) on the stack.
    Spell,
    /// A permanent on the battlefield — card-backed or token.
    Permanent,
    /// An emblem in the command zone.
    Emblem,
    /// An activated/triggered ability on the stack.
    AbilityOnStack,
}

/// Rules identity + print identity of a card-backed object.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct CardRef {
    /// Rules identity.
    pub index: CardIndex,
    /// Print identity (presentation-only; the engine never reads it).
    pub print: PrintRef,
}

/// Which ability an `AbilityOnStack` object represents.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct AbilityLoc {
    /// The card the ability belongs to.
    pub card: CardIndex,
    /// Index into `CardDef::abilities`.
    pub index: u32,
    /// The permanent/spell that produced the ability (the source).
    pub source: ObjectId,
}

/// Copiable values (CR 707.2): printed or token-defined characteristics.
///
/// Computed/layered characteristics are a *projection* of this base plus
/// continuous effects (M2); for M1 the projection IS the base.
#[derive(Clone, Debug)]
pub struct Characteristics {
    /// Interned name.
    pub name: NameRef,
    /// Mana cost (`ManaCost::ZERO` if none).
    pub mana_cost: ManaCost,
    /// Colors.
    pub colors: ColorSet,
    /// Types.
    pub types: TypeSet,
    /// Supertypes.
    pub supertypes: SupertypeSet,
    /// Subtypes (512-bit bitmap; changeling = all).
    pub subtypes: SubtypeSet,
    /// Simple keywords.
    pub keywords: KeywordSet,
    /// Power (creatures).
    pub power: Option<i16>,
    /// Toughness (creatures).
    pub toughness: Option<i16>,
    /// Loyalty (planeswalkers).
    pub loyalty: Option<u16>,
}

impl Characteristics {
    /// Builds the base characteristics from a card definition face.
    #[must_use]
    pub fn from_face(def: &CardDef, face: usize, name: NameRef) -> Self {
        let f = &def.faces[face.min(def.faces.len() - 1)];
        Self {
            name,
            mana_cost: f.mana_cost,
            colors: f.mana_cost.colors(),
            types: f.types,
            supertypes: f.supertypes,
            subtypes: SubtypeSet::from_slice(f.subtypes),
            keywords: def.keywords,
            power: f.power,
            toughness: f.toughness,
            loyalty: f.loyalty,
        }
    }
}

/// Cached layered projection (M2); `generation` matches the effect
/// generation the value was computed at.
#[derive(Clone, Debug)]
pub struct CachedChar {
    /// Effect generation this cache was computed at; `u64::MAX` = stale.
    pub generation: u64,
    /// Cached value.
    pub value: Characteristics,
}

/// Counter kinds (objects and players).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum CounterKind {
    /// +1/+1.
    P1P1,
    /// −1/−1.
    M1M1,
    /// Loyalty.
    Loyalty,
    /// Lore (sagas).
    Lore,
    /// Time (suspend, vanishing).
    Time,
    /// Charge.
    Charge,
    /// Poison (players).
    Poison,
    /// Energy (players).
    Energy,
    /// Rad (players).
    Rad,
    /// Card-specific counters (generated names).
    Custom(u16),
}

/// Counters on an object.
#[derive(Clone, Debug, Default)]
pub struct Counters(SmallVec<[(CounterKind, u16); 4]>);

impl Counters {
    /// Amount of a counter kind.
    #[must_use]
    pub fn get(&self, kind: CounterKind) -> u16 {
        self.0
            .iter()
            .find(|(k, _)| *k == kind)
            .map_or(0, |(_, n)| *n)
    }

    /// Adds counters, returning the new total.
    pub fn add(&mut self, kind: CounterKind, n: u16) -> u16 {
        if let Some(entry) = self.0.iter_mut().find(|(k, _)| *k == kind) {
            entry.1 = entry.1.saturating_add(n);
            entry.1
        } else {
            self.0.push((kind, n));
            n
        }
    }

    /// Sets the amount directly.
    pub fn set(&mut self, kind: CounterKind, n: u16) {
        if n == 0 {
            self.0.retain(|(k, _)| *k != kind);
        } else if let Some(entry) = self.0.iter_mut().find(|(k, _)| *k == kind) {
            entry.1 = n;
        } else {
            self.0.push((kind, n));
        }
    }

    /// Whether no counters are present.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Iterates all counter entries.
    pub fn iter(&self) -> impl Iterator<Item = (CounterKind, u16)> + '_ {
        self.0.iter().copied()
    }
}

/// Status bits of an object (CR 110.5; phasing per CR 702.26).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default, Debug)]
pub struct Status(u8);

impl Status {
    /// No status.
    pub const NONE: Self = Self(0);
    /// Tapped.
    pub const TAPPED: Self = Self(1);
    /// Face down (morph, manifest, …).
    pub const FACE_DOWN: Self = Self(2);
    /// Phased out (treated as though it doesn't exist, CR 702.26).
    pub const PHASED_OUT: Self = Self(4);
    /// Flipped (flip cards).
    pub const FLIPPED: Self = Self(8);

    /// Whether all bits of `other` are set.
    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// Sets bits.
    pub const fn insert(&mut self, other: Self) {
        self.0 |= other.0;
    }

    /// Clears bits.
    pub const fn remove(&mut self, other: Self) {
        self.0 &= !other.0;
    }

    /// Raw bits.
    #[must_use]
    pub const fn bits(self) -> u8 {
        self.0
    }
}

/// Typed payload attached to cards in exile (or similar) by effects.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Rider {
    /// Exiled by another object ("until ~ leaves the battlefield", imprint).
    Linked {
        /// The host object this card is linked to.
        host: ObjectId,
    },
    /// Rebound: cast from hand, may be cast again at the next upkeep.
    Rebound,
    /// On an adventure (may cast the permanent later).
    Adventure,
    /// Foretold (castable for its foretell cost).
    Foretold,
    /// Plotted (castable without paying as a sorcery).
    Plotted,
}

/// Riders attached to an object.
pub type RiderSet = SmallVec<[Rider; 2]>;

/// A game object.
#[derive(Clone, Debug)]
pub struct GameObject {
    /// Arena handle (stable for the object's lifetime).
    pub id: ObjectId,
    /// Owning player.
    pub owner: PlayerId,
    /// Controlling player.
    pub controller: PlayerId,
    /// Current zone.
    pub zone: Zone,
    /// Which player's zone list contains this object (`None` on
    /// battlefield/stack).
    pub zone_owner: Option<PlayerId>,
    /// Object kind.
    pub kind: ObjectKind,
    /// Backing card, if any (tokens/emblems have none).
    pub card: Option<CardRef>,
    /// Copiable base characteristics.
    pub base: Characteristics,
    /// Layered projection cache (M2).
    pub cache: CachedChar,
    /// Counters.
    pub counters: Counters,
    /// Marked damage this turn.
    pub damage: u16,
    /// Status bits.
    pub status: Status,
    /// What this object is attached to (auras/equipment).
    pub attached_to: Option<ObjectId>,
    /// Entered-the-current-zone timestamp (effects ordering, summoning
    /// sickness evaluation).
    pub timestamp: u64,
    /// Identity version; bumped on every zone change (CR 400.7).
    pub version: u32,
    /// Exile riders.
    pub riders: RiderSet,
    /// Chosen targets (spells/abilities on the stack).
    pub targets: SmallVec<[ObjectId; 2]>,
    /// Which ability this is (`AbilityOnStack` objects only).
    pub ability: Option<AbilityLoc>,
}

impl GameObject {
    /// Creates a card-backed object.
    #[must_use]
    pub fn new_card(id: ObjectId, owner: PlayerId, card: CardRef, base: Characteristics) -> Self {
        Self {
            id,
            owner,
            controller: owner,
            zone: Zone::Library,
            zone_owner: None,
            kind: ObjectKind::Card,
            card: Some(card),
            cache: CachedChar {
                generation: u64::MAX,
                value: base.clone(),
            },
            base,
            counters: Counters::default(),
            damage: 0,
            status: Status::NONE,
            attached_to: None,
            timestamp: 0,
            version: 0,
            riders: RiderSet::new(),
            targets: SmallVec::new(),
            ability: None,
        }
    }

    /// Creates an ability object on the stack.
    #[must_use]
    pub fn new_ability_on_stack(
        id: ObjectId,
        controller: PlayerId,
        ability: AbilityLoc,
        targets: SmallVec<[ObjectId; 2]>,
        name: NameRef,
    ) -> Self {
        let mut obj = Self::new_bare(
            id,
            controller,
            ObjectKind::AbilityOnStack,
            Characteristics {
                name,
                mana_cost: ManaCost::ZERO,
                colors: ColorSet::EMPTY,
                types: TypeSet::EMPTY,
                supertypes: SupertypeSet::EMPTY,
                subtypes: SubtypeSet::EMPTY,
                keywords: KeywordSet::EMPTY,
                power: None,
                toughness: None,
                loyalty: None,
            },
        );
        obj.ability = Some(ability);
        obj.targets = targets;
        obj.zone = Zone::Stack;
        obj
    }

    /// Creates a card-less object (token, emblem).
    #[must_use]
    pub fn new_bare(
        id: ObjectId,
        owner: PlayerId,
        kind: ObjectKind,
        base: Characteristics,
    ) -> Self {
        let mut obj = Self::new_card(
            id,
            owner,
            CardRef {
                index: baylee_core::ids::CardIndex::new(0),
                print: baylee_core::ids::PrintRef::new(0),
            },
            base,
        );
        obj.card = None;
        obj.kind = kind;
        obj
    }

    /// Current characteristics.
    ///
    /// M1: always the base. M2 will project through the layer system when
    /// `cache.generation != game.effect_generation`.
    #[must_use]
    pub fn characteristics(&self) -> &Characteristics {
        &self.base
    }
}
