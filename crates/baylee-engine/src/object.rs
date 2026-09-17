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
use std::sync::Arc;

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
    /// The card the ability belongs to, when there is one.
    ///
    /// `None` for a token, a token copy and an emblem (CR 111.1, CR 114.2):
    /// none of the three has a card, and the ability came off the object's
    /// own list rather than out of a printing. It used to be
    /// `CardIndex::new(0)` there, which is not a hole but a *claim* — index
    /// 0 is a real card, and this handle leaves the engine twice, as the
    /// stack entry a client draws and as the name a player's standing
    /// answer is filed under. Every token's ability answered to the same
    /// stranger's.
    ///
    /// The engine itself reads none of it: what an ability on the stack
    /// *does* is the list captured beside it (CR 608.2, `own_abilities`).
    pub card: Option<CardIndex>,
    /// Index into `CardDef::abilities`.
    pub index: u32,
    /// The permanent/spell that produced the ability (the source).
    pub source: ObjectId,
}

/// Copiable values (CR 707.2): printed or token-defined characteristics.
///
/// Computed/layered characteristics are a *projection* of this base plus
/// continuous effects (M2); for M1 the projection IS the base.
#[derive(Clone, PartialEq, Debug)]
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
    /// Whether one of this object's mana abilities makes *the chosen color*
    /// (Uncharted Haven, the Thriving cycle).
    ///
    /// A card-level fact beside two card-level fields, and it exists so that
    /// the object-level one stays honest: `GameObject::chosen_color` is set
    /// by an entry and says nothing about what the permanent does with it,
    /// so Reflecting Pool asks this before counting that colour as mana.
    pub produced_chosen: bool,
}

impl Characteristics {
    /// Builds the base characteristics from a card definition face.
    #[must_use]
    #[allow(clippy::too_many_lines)] // the color scan is one flat table
    pub fn from_face(def: &CardDef, face: usize, name: NameRef) -> Self {
        let f = &def.faces[face.min(def.faces.len() - 1)];
        let subtypes = SubtypeSet::from_slice(f.subtypes);
        let mut produced = ColorSet::EMPTY;
        let mut produced_colorless = false;
        let mut produced_chosen = false;
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
            // A conditional mana ability counts, and counts unconditionally.
            // CR 106.7 asks what an ability "would produce if the ability
            // were to resolve at that time", and the only thing it says to
            // ignore is whether the costs could be paid. An "activate only
            // if…" clause is not a cost — it is a restriction on
            // *beginning* the activation (CR 602.5) — so reading it as no
            // part of what 106.7 asks about is this engine's inference and
            // not a sentence of the rule. The alternative makes the answer
            // depend on the board, and the whole point of the field is that
            // it does not. So Bleachbone Verge could produce {W} whether or
            // not a Plains is on the table — and a Fellwar Stone across
            // from it was being offered {B} alone.
            let (baylee_cards_dsl::AbilityDef::Activated {
                mana_ability: true,
                effects,
                ..
            }
            | baylee_cards_dsl::AbilityDef::ActivatedConditional {
                mana_ability: true,
                effects,
                ..
            }) = ability
            else {
                continue;
            };
            let add_color = |produced: &mut ColorSet,
                             produced_colorless: &mut bool,
                             c: &baylee_core::mana::ManaColor| {
                match c {
                    baylee_core::mana::ManaColor::Colorless => *produced_colorless = true,
                    baylee_core::mana::ManaColor::White => {
                        *produced = produced.union(ColorSet::of(Color::White));
                    }
                    baylee_core::mana::ManaColor::Blue => {
                        *produced = produced.union(ColorSet::of(Color::Blue));
                    }
                    baylee_core::mana::ManaColor::Black => {
                        *produced = produced.union(ColorSet::of(Color::Black));
                    }
                    baylee_core::mana::ManaColor::Red => {
                        *produced = produced.union(ColorSet::of(Color::Red));
                    }
                    baylee_core::mana::ManaColor::Green => {
                        *produced = produced.union(ColorSet::of(Color::Green));
                    }
                }
            };
            for effect in *effects {
                let baylee_cards_dsl::Effect::AddMana { source, .. } = effect else {
                    continue;
                };
                match source {
                    baylee_cards_dsl::ManaSource::Fixed(c) => {
                        add_color(&mut produced, &mut produced_colorless, c);
                    }
                    baylee_cards_dsl::ManaSource::Choice(colors) => {
                        for c in *colors {
                            add_color(&mut produced, &mut produced_colorless, c);
                        }
                    }
                    // Command Tower: which colors depends on the command
                    // zone, so the printed card promises all five.
                    baylee_cards_dsl::ManaSource::CommanderIdentity => {
                        produced = produced.union(ColorSet::from_slice(&[
                            Color::White,
                            Color::Blue,
                            Color::Black,
                            Color::Red,
                            Color::Green,
                        ]));
                    }
                    // Reflecting Pool and Exotic Orchard produce nothing on
                    // their own: what they could produce is read *off* this
                    // very field of the other lands, so claiming all five
                    // here would let a lone Pool tap for any color and would
                    // make two Pools promise each other the rainbow.
                    baylee_cards_dsl::ManaSource::LandColor { .. } => {}
                    // The chosen colour is on the *object* and this is a
                    // reading of the card, so there is nothing to put here.
                    // Only Reflecting Pool and Exotic Orchard read this
                    // field, and they read it off an object they are holding
                    // — `resolve::mana` unions the chosen colour in there,
                    // where it is knowable.
                    // The chosen colour is on the *object* and this is a
                    // reading of the card, so there is no colour to put
                    // here — only the fact that this card reads one, which
                    // is what lets Reflecting Pool ask the object for it.
                    baylee_cards_dsl::ManaSource::Chosen => produced_chosen = true,
                    baylee_cards_dsl::ManaSource::ChosenOr(colors) => {
                        produced_chosen = true;
                        for c in *colors {
                            add_color(&mut produced, &mut produced_colorless, c);
                        }
                    }
                }
            }
        }
        Self {
            name,
            mana_cost: f.mana_cost,
            // CR 202.2e: a face with no mana cost takes its color from the
            // indicator printed on it. The union rather than an either/or,
            // because the two never coexist on a printed face and a card
            // that somehow stated both should be the sum, not one of them.
            colors: f.mana_cost.colors().union(f.color_indicator),
            types: f.types,
            supertypes: f.supertypes,
            subtypes,
            keywords: def.keywords_for_face(face),
            power: f.power,
            toughness: f.toughness,
            loyalty: f.loyalty,
            color_identity: def.color_identity,
            produced_colors: produced,
            produced_chosen,
            produced_colorless,
        }
    }
}

/// Cached layered projection (CR 613).
///
/// The projection is stored **only when it differs from the base**. Every
/// card in a library, hand or graveyard, and every permanent on a board
/// with no continuous effect touching it, stores nothing at all — which is
/// the difference between a `GameObject` carrying one set of
/// characteristics and carrying two. At 256 bytes per [`Characteristics`]
/// that is a third of the object, on every object, paid again on every
/// state clone the AI lookahead does.
#[derive(Clone, Debug, Default)]
pub struct CachedChar {
    /// Effect generation the value was computed at; `u64::MAX` = never.
    generation: u64,
    /// The projection, when it differs from the base.
    value: Option<Box<Characteristics>>,
}

impl CachedChar {
    /// The effect generation this cache was computed at.
    #[must_use]
    pub const fn generation(&self) -> u64 {
        self.generation
    }

    /// The cached projection, if one is stored.
    #[must_use]
    pub fn value(&self) -> Option<&Characteristics> {
        self.value.as_deref()
    }

    /// Drops the projection: the base is authoritative again.
    ///
    /// Called for objects the layer pass found nothing to change, and on
    /// every zone change — a permanent that dies is a new object (CR
    /// 400.7) and must not carry the anthem it was standing under into
    /// the graveyard.
    pub fn clear(&mut self) {
        self.generation = u64::MAX;
        self.value = None;
    }

    /// Stores a projection, reusing the allocation when one is already
    /// held and releasing it when the projection collapsed onto the base.
    ///
    /// The projected *controller* is not cached: the refresh writes it
    /// straight to [`GameObject::controller`], so there is one answer to
    /// "who controls this" rather than a cached second one.
    pub fn store(
        &mut self,
        generation: u64,
        characteristics: Characteristics,
        base: &Characteristics,
    ) {
        self.generation = generation;
        if characteristics == *base {
            self.value = None;
            return;
        }
        match &mut self.value {
            Some(slot) => **slot = characteristics,
            None => self.value = Some(Box::new(characteristics)),
        }
    }
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
    /// Cast via flashback: exile instead of the graveyard on resolution
    /// (CR 702.34).
    Flashback,
    /// Can't be countered (Cavern of Souls mana rider).
    Uncounterable,
    /// The permanent has the prepared marker (Emeritus of Woe & co.).
    Prepared,
    /// May be played from exile by the given player, spending mana of
    /// any color (Opposition Agent's search takeover).
    PlayableFromExileFor(PlayerId),
    /// This object is a copy of a spell, and ceases to exist as soon as it
    /// is anywhere but the stack or the battlefield (CR 704.5e).
    ///
    /// It cannot be read off the object itself. A copy carries the copied
    /// card so that [`crate::engine::Engine::resolve_stack_top`] can find
    /// the spell's effects, which makes it indistinguishable from the card
    /// it was copied from — and a token is the opposite thing, card-less
    /// and swept up by CR 704.5d.
    SpellCopy,
}

/// Riders attached to an object.
pub type RiderSet = SmallVec<[Rider; 2]>;

/// A game object.
///
/// The independent flags (`kicked`, `alt_cast`, `cast_from_hand`,
/// `deathtouched`) genuinely are unrelated one-bit facts about one object,
/// not a state machine waiting to be an enum.
#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Debug)]
pub struct GameObject {
    /// Arena handle (stable for the object's lifetime).
    pub id: ObjectId,
    /// Owning player.
    pub owner: PlayerId,
    /// Controlling player *right now*, layer 2 included (CR 613.1b).
    ///
    /// Read this everywhere; it is the answer to "who controls this?".
    /// The layer refresh writes it, so a "gain control until end of turn"
    /// effect is visible to every rule that asks — combat, targeting,
    /// priority, "creatures you control" — without any of them knowing
    /// that layers exist.
    pub controller: PlayerId,
    /// Controller before any layer-2 effect: who this permanent goes back
    /// to when the control effect ends.
    ///
    /// Only a *permanent* control change writes it (a permanent entering,
    /// a resolved Gilded Drake exchange, Homeward Path); use
    /// [`GameObject::set_controller`] so the two can never drift apart.
    pub base_controller: PlayerId,
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
    ///
    /// Shared rather than owned, for two reasons that pull the same way.
    /// Inline it is 256 bytes — half of a `GameObject` — so every object in
    /// the arena paid for a full characteristic set whether or not anything
    /// ever looked at it. And `GameState::clone` is the AI's lookahead
    /// primitive: with the set inline, every ply deep-copied all of them;
    /// behind a handle a clone is a refcount bump.
    ///
    /// Nothing here is copy-on-write by accident — the three places that
    /// rewrite a base (a clone effect, Cursed Mirror, its cleanup) go
    /// through [`GameObject::base_mut`], which is `Arc::make_mut` and so
    /// splits the sharing exactly when someone writes.
    pub base: Arc<Characteristics>,
    /// Layered projection cache (M2).
    pub cache: CachedChar,
    /// Counters.
    pub counters: Counters,
    /// Marked damage this turn.
    pub damage: u16,
    /// Dealt damage by a source with deathtouch since the last
    /// state-based-action check (CR 704.5h).
    ///
    /// Deathtouch is not "lethal damage" — a 1/1 deathtoucher marks one
    /// damage on a 6/6 and the 6/6 dies with five toughness to spare, so
    /// the marked total can never carry this. The SBA pass that reads the
    /// flag also clears it, which is exactly the "since the last time
    /// state-based actions were checked" window the rule is written in:
    /// a creature that survived because it was indestructible does not
    /// die later in the turn when the indestructibility wears off.
    pub deathtouched: bool,
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
    /// What this spell is allowed to target, kept so a copy can be retargeted
    /// without a card lookup (CR 707.10c). `None` for anything that does not
    /// target, and for objects that never went through the cast wizard.
    pub target_req: Option<baylee_cards_dsl::TargetReq>,
    /// What this object's base was before it became a copy; restored when it
    /// changes zones (CR 400.7), because the new object is not a copy.
    pub original_base: Option<Arc<Characteristics>>,
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
    /// Players chosen as targets, beside `targets` ("any target").
    pub target_players: baylee_core::ids::SeatSet,
    /// The chosen spell mode (modal spells / overload).
    pub mode_index: Option<u8>,
    /// The creature type chosen as this entered ("the chosen type" —
    /// Roaming Throne, Reflections of Littjara, Cavern of Souls).
    pub chosen_subtype: Option<baylee_core::ids::SubtypeId>,
    /// The color chosen as this entered ("the chosen color" — Uncharted
    /// Haven, the Thriving cycle, the Gates).
    ///
    /// On the object and not on the card, because that is where the answer
    /// differs: two Thriving Moors on one battlefield are two colours, and
    /// the card they share can say neither.
    pub chosen_color: Option<baylee_core::mana::ManaColor>,
    /// Which face of the card is active (MDFC/split; 0 = front).
    pub face_index: u8,
    /// Abilities this object carries itself, instead of reading them off a
    /// card face.
    ///
    /// Three kinds of object need it and they need the same thing. An
    /// **emblem** is not card-backed at all (CR 114.2), so its abilities have
    /// nowhere else to live. A **copy** has a card underneath it and must not
    /// answer with it: abilities are copiable values (CR 707.2), `base`
    /// carries only characteristics, and a clone that replaced its base alone
    /// arrived with the right name, types and power and no rules text.
    /// Glasspool Mimic copying Earth King's Lieutenant is the case that found
    /// it — the board saw an Ally enter and the Lieutenant beside it grew,
    /// while the copy's own enters-trigger never fired. An **ability on the
    /// stack** captures its source's list as it is put there, because it
    /// exists independently of that source afterwards (CR 113.7a) and the
    /// source may already have stopped being a copy by the time it resolves.
    ///
    /// Stored rather than derived because the copiable values are fixed as
    /// the copy is made (CR 707.2b) and the original may leave; one field
    /// rather than two because an emblem is never a copy and the extra
    /// `Option<&[_]>` is 16 bytes on every object in every AI ply.
    ///
    /// A card-backed object gives it up at the next zone change, with
    /// [`GameObject::original_base`] and for the same reason — and at the
    /// cleanup step as well when [`GameObject::own_abilities_until_eot`]
    /// says the copy that wrote it was a temporary one.
    pub own_abilities: Option<&'static [baylee_cards_dsl::AbilityDef]>,
    /// Whether [`GameObject::own_abilities`] was written by a copy that
    /// ends with the turn (Cursed Mirror), and so has to be given back at
    /// the cleanup step.
    ///
    /// A flag rather than a second list because there is exactly one
    /// question — does this object stop being a copy tonight — and the
    /// characteristics half of the same copy is already answered that way:
    /// it is a `Layer::Copy` effect with `Duration::UntilEndOfTurn`, and
    /// expiring is the whole of its revert. Abilities are not layer-
    /// projected (`Characteristics` has no field for them), so the ability
    /// half cannot expire with it and needs somewhere to say when it ends.
    ///
    /// It is not a sweep over every object with an `own_abilities` at
    /// cleanup, for the reason recorded in [`Engine::cleanup_step`]: a
    /// permanent copy has written that field too, and reverting one of
    /// those would turn a Glasspool Mimic back into a 0/0 on the turn it
    /// was cast.
    ///
    /// [`Engine::cleanup_step`]: crate::engine::Engine
    pub own_abilities_until_eot: bool,
    /// What this object could do as it *left* the battlefield is not here but
    /// on the state, in [`GameState::ltb_abilities`], for the reason the
    /// paragraph above gives about a second `Option<&[_]>`: it is `Some` for
    /// the handful of copies that have died and would cost sixteen bytes on
    /// every object in every ply of the AI's search.
    ///
    /// [`GameState::ltb_abilities`]: crate::state::GameState::ltb_abilities
    /// The definition this object was created from, if a token created it.
    ///
    /// The only thing that says *which* token a card-less permanent is, and
    /// the engine needs it for one reason: a token's abilities live here, so
    /// a Treasure can be cracked at all. The definition is a `'static` from
    /// the registry — the same shape [`GameObject::own_abilities`]
    /// already had — because the rules kernel does not depend on the card
    /// registry and cannot resolve an index into it. The client's art key is
    /// the token's position in that registry, which the view builder derives
    /// from this on the way out.
    pub token: Option<&'static baylee_cards_dsl::TokenDef>,
    /// A face switch queued by a resolution (transform); the engine
    /// applies it after the resolution completes.
    pub pending_face_change: Option<u8>,
    /// The object a triggering event was about (event-driven triggers).
    pub event_object: Option<ObjectId>,
    /// Whether the spell was cast from the hand (rebound condition).
    pub cast_from_hand: bool,
}

impl GameObject {
    /// Creates a card-backed object.
    #[must_use]
    pub fn new_card(
        id: ObjectId,
        owner: PlayerId,
        card: CardRef,
        base: impl Into<Arc<Characteristics>>,
    ) -> Self {
        Self {
            id,
            owner,
            controller: owner,
            base_controller: owner,
            zone: Zone::Library,
            zone_owner: None,
            kind: ObjectKind::Card,
            card: Some(card),
            cache: CachedChar::default(),
            base: base.into(),
            counters: Counters::default(),
            damage: 0,
            deathtouched: false,
            status: Status::NONE,
            attached_to: None,
            timestamp: 0,
            version: 0,
            riders: RiderSet::new(),
            targets: SmallVec::new(),
            target_req: None,
            original_base: None,
            event_object: None,
            ability: None,
            x_value: 0,
            kicked: false,
            alt_cast: false,
            chosen_player: None,
            target_players: baylee_core::ids::SeatSet::new(),
            mode_index: None,
            chosen_subtype: None,
            chosen_color: None,
            face_index: 0,
            own_abilities: None,
            own_abilities_until_eot: false,
            token: None,
            pending_face_change: None,
            cast_from_hand: true,
        }
    }

    /// Creates an ability object on the stack.
    ///
    /// `base` is the blank face from [`GameState::bare_base`], not one built
    /// here: an Ally deck puts six figures of these on the stack, and a face
    /// of its own for each would be six figures of allocations for a name.
    ///
    /// [`GameState::bare_base`]: crate::state::GameState::bare_base
    #[must_use]
    pub fn new_ability_on_stack(
        id: ObjectId,
        controller: PlayerId,
        ability: AbilityLoc,
        targets: SmallVec<[ObjectId; 2]>,
        base: Arc<Characteristics>,
    ) -> Self {
        let mut obj = Self::new_bare(id, controller, ObjectKind::AbilityOnStack, base);
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
        base: impl Into<Arc<Characteristics>>,
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

    /// The abilities this object actually has, whatever it is.
    ///
    /// Three sources answer to the same question and used to be asked
    /// separately at two dozen call sites: a card's active face, an emblem's
    /// stored list, and — since tokens gained rules of their own — the
    /// central token definition. Anything that wants to know what a permanent
    /// can do goes through here, so a new kind of card-less object is one
    /// arm of this match rather than a sweep through the engine.
    #[must_use]
    pub fn abilities(
        &self,
        lookup: &impl crate::state::CardLookup,
    ) -> &'static [baylee_cards_dsl::AbilityDef] {
        // An emblem has no card, and a copy has one it must not answer with.
        if let Some(abilities) = self.own_abilities {
            return abilities;
        }
        if let Some(card) = self.card {
            return lookup
                .card(card.index)
                .map_or(&[], |def| def.abilities_for_face(self.face_index as usize));
        }
        self.token.map_or(&[], |token| token.abilities)
    }

    /// Mutable access to the base, splitting the sharing if anyone else
    /// holds it.
    ///
    /// The only way to write a base. Going through `Arc::make_mut` is what
    /// makes sharing safe: a clone of the game state, or a token that took
    /// its base from the permanent it copies, keeps reading the old values
    /// while the writer gets its own copy.
    pub fn base_mut(&mut self) -> &mut Characteristics {
        Arc::make_mut(&mut self.base)
    }

    /// Current characteristics.
    ///
    /// Returns the layer-projected cache when it has been computed (the
    /// engine refreshes caches after every effect-set change), otherwise
    /// the copiable base.
    #[must_use]
    pub fn characteristics(&self) -> &Characteristics {
        self.cache.value().unwrap_or(&self.base)
    }

    /// Sets the controller permanently: both the value everyone reads and
    /// the one a layer-2 effect falls back to when it ends.
    ///
    /// The one way to hand a permanent over for good. Assigning
    /// [`GameObject::controller`] directly would be undone by the next
    /// layer refresh.
    pub const fn set_controller(&mut self, player: PlayerId) {
        self.controller = player;
        self.base_controller = player;
    }
}
