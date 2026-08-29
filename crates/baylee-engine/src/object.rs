//! Game objects and their characteristics.
//!
//! Identity model: an object's [`ObjectId`] is stable for its whole
//! lifetime; zone changes bump [`GameObject::version`] (CR 400.7 — "it
//! becomes a new object"). Effects and targets that must track identity
//! record `(ObjectId, version)`; blinked permanents naturally invalidate
//! old references.

use crate::zone::Zone;
use baylee_cards_dsl::{CardDef, KeywordSet};
use baylee_core::color::{Color, ColorSet};
use baylee_core::ids::{CardIndex, NameRef, ObjectId, PlayerId, PrintRef};
use baylee_core::mana::ManaCost;
use baylee_core::types::{SubtypeSet, SupertypeSet, TypeSet};
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
    /// Color identity (commander rules: mana symbols in cost + rules text).
    /// Not a characteristic in the CR sense — never layer-modified.
    pub color_identity: ColorSet,
    /// Mana colors this object's abilities could produce (Exotic Orchard,
    /// Reflecting Pool). Includes basic-land-type derivation.
    pub produced_colors: ColorSet,
    /// Whether any ability could produce colorless mana.
    pub produced_colorless: bool,
}

impl Characteristics {
    /// Builds the base characteristics from a card definition face.
    #[must_use]
    pub fn from_face(def: &CardDef, face: usize, name: NameRef) -> Self {
        let f = &def.faces[face.min(def.faces.len() - 1)];
        let subtypes = SubtypeSet::from_slice(f.subtypes);
        let mut produced = ColorSet::EMPTY;
        let mut produced_colorless = false;
        // Basic-land-type derivation (CR 305.6).
        let land_types = [
            (baylee_core::generated::subtypes::land::PLAINS, Color::White),
            (baylee_core::generated::subtypes::land::ISLAND, Color::Blue),
            (baylee_core::generated::subtypes::land::SWAMP, Color::Black),
            (baylee_core::generated::subtypes::land::MOUNTAIN, Color::Red),
            (baylee_core::generated::subtypes::land::FOREST, Color::Green),
        ];
        for (subtype, color) in land_types {
            if subtypes.contains(subtype) {
                produced = produced.union(ColorSet::of(color));
            }
        }
        // Mana abilities on the card.
        let all_abilities = def
            .abilities
            .iter()
            .chain(def.faces.iter().flat_map(|f| f.abilities.iter()));
        for ability in all_abilities {
            let baylee_cards_dsl::AbilityDef::Activated {
                mana_ability: true,
                effects,
                ..
            } = ability
            else {
                continue;
            };
            for effect in *effects {
                match effect {
                    baylee_cards_dsl::Effect::AddMana { color, .. } => match color {
                        baylee_core::mana::ManaColor::Colorless => produced_colorless = true,
                        c => {
                            let col = match c {
                                baylee_core::mana::ManaColor::White => Color::White,
                                baylee_core::mana::ManaColor::Blue => Color::Blue,
                                baylee_core::mana::ManaColor::Black => Color::Black,
                                baylee_core::mana::ManaColor::Red => Color::Red,
                                baylee_core::mana::ManaColor::Green => Color::Green,
                                baylee_core::mana::ManaColor::Colorless => unreachable!(),
                            };
                            produced = produced.union(ColorSet::of(col));
                        }
                    },
                    baylee_cards_dsl::Effect::AddManaChoice { colors, .. } => {
                        for c in *colors {
                            match c {
                                baylee_core::mana::ManaColor::Colorless => {
                                    produced_colorless = true;
                                }
                                baylee_core::mana::ManaColor::White => {
                                    produced = produced.union(ColorSet::of(Color::White));
                                }
                                baylee_core::mana::ManaColor::Blue => {
                                    produced = produced.union(ColorSet::of(Color::Blue));
                                }
                                baylee_core::mana::ManaColor::Black => {
                                    produced = produced.union(ColorSet::of(Color::Black));
                                }
                                baylee_core::mana::ManaColor::Red => {
                                    produced = produced.union(ColorSet::of(Color::Red));
                                }
                                baylee_core::mana::ManaColor::Green => {
                                    produced = produced.union(ColorSet::of(Color::Green));
                                }
                            }
                        }
                    }
                    // "Any color" families (Command Tower, orchard/pool).
                    baylee_cards_dsl::Effect::AddManaCommanderIdentity
                    | baylee_cards_dsl::Effect::AddManaLandColor { .. } => {
                        produced = produced.union(ColorSet::from_slice(&[
                            Color::White,
                            Color::Blue,
                            Color::Black,
                            Color::Red,
                            Color::Green,
                        ]));
                    }
                    _ => {}
                }
            }
        }
        Self {
            name,
            mana_cost: f.mana_cost,
            colors: f.mana_cost.colors(),
            types: f.types,
            supertypes: f.supertypes,
            subtypes,
            keywords: def.keywords,
            power: f.power,
            toughness: f.toughness,
            loyalty: f.loyalty,
            color_identity: def.color_identity,
            produced_colors: produced,
            produced_colorless,
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

pub use baylee_cards_dsl::CounterKind;

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
    /// Suspended (time counters tick down at upkeep; cast for free at zero).
    Suspend,
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
    /// Original base before a temporary copy (Cursed Mirror); reverted at
    /// cleanup.
    pub original_base: Option<Box<Characteristics>>,
    /// Which ability this is (`AbilityOnStack` objects only).
    pub ability: Option<AbilityLoc>,
    /// The value of X chosen at cast time (spells).
    pub x_value: u32,
    /// Whether the kicker/additional cost was paid (spells).
    pub kicked: bool,
    /// Whether this spell was cast for an alternative cost (evoke checks).
    pub alt_cast: bool,
    /// A chosen target player (player-targeting spells).
    pub chosen_player: Option<PlayerId>,
    /// The chosen spell mode (modal spells / overload).
    pub mode_index: Option<u8>,
    /// The creature type chosen as this entered ("the chosen type" —
    /// Roaming Throne, Reflections of Littjara, Cavern of Souls).
    pub chosen_subtype: Option<baylee_core::ids::SubtypeId>,
    /// Which face of the card is active (MDFC/split; 0 = front).
    pub face_index: u8,
    /// The object a triggering event was about (event-driven triggers).
    pub event_object: Option<ObjectId>,
    /// Whether the spell was cast from the hand (rebound condition).
    pub cast_from_hand: bool,
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
            original_base: None,
            event_object: None,
            ability: None,
            x_value: 0,
            kicked: false,
            alt_cast: false,
            chosen_player: None,
            mode_index: None,
            chosen_subtype: None,
            face_index: 0,
            cast_from_hand: true,
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
                color_identity: ColorSet::EMPTY,
                produced_colors: ColorSet::EMPTY,
                produced_colorless: false,
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
    /// Returns the layer-projected cache when it has been computed (the
    /// engine refreshes caches after every effect-set change), otherwise
    /// the copiable base.
    #[must_use]
    pub fn characteristics(&self) -> &Characteristics {
        if self.cache.generation == u64::MAX {
            &self.base
        } else {
            &self.cache.value
        }
    }
}
