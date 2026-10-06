use crate::counters::{CounterEntry, CounterKind};
use crate::log::LogZone;
use crate::player_view::WordChange;
use crate::status::ObjectStatus;
use baylee_core::color::ColorSet;
use baylee_core::ids::{AbilityRef, CardIndex, ObjectId, PlayerId, PrintRef, TargetRef};
use baylee_core::mana::ManaCost;
use baylee_core::types::{SubtypeSet, SupertypeSet, TypeSet};
use serde::{Deserialize, Serialize};

// ------------------------------------------------------------------- objects

/// An offered damage source, described as the exact rules incarnation that
/// was offered. Names and card identities obey the viewing seat's entitlement.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct DamageSourceView {
    /// Exact object and incarnation selected by the source-choice action.
    pub source: baylee_core::ids::DamageSourceRef,
    /// Projected or historical public name, including copy and token names.
    pub name: String,
    /// Entitled backing card identity; never a later hidden incarnation.
    pub card: Option<CardIdentity>,
    /// Entitled printed rules face, if this source has one.
    pub rules: Option<RulesFace>,
    /// Registry token identity, if applicable.
    pub token: Option<u16>,
    /// Controller of the source in the described incarnation.
    pub controller: PlayerId,
    /// Zone of the described incarnation, not a later card's present zone.
    pub zone: LogZone,
    /// True when this exact incarnation still exists in the game.
    pub is_current: bool,
    /// Stack objects referring to this exact incarnation.
    pub referenced_by: Vec<ObjectId>,
    /// Projected source colors.
    pub colors: ColorSet,
    /// Projected card types.
    pub types: TypeSet,
    /// Projected power, when defined.
    pub power: Option<i16>,
    /// Projected toughness, when defined.
    pub toughness: Option<i16>,
    /// Projected keyword bitset, as on public objects.
    pub keywords: u128,
}

/// Identity of the card backing an object, when the viewing seat may know it.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct CardIdentity {
    /// Rules identity (index into the compiled card registry).
    pub index: CardIndex,
    /// Print identity (index into [`GameStatic::prints`]).
    ///
    /// [`GameStatic::prints`]: crate::GameStatic::prints
    pub print: PrintRef,
    /// Which face is currently up (MDFC, transform, flip).
    pub face: u8,
}

/// The card and face whose printed ability list an object's abilities are.
///
/// For an ordinary card it is the card and the face it shows. It differs from
/// [`PublicObject::card`] exactly where a player would be misled by reading
/// that instead: a copy's abilities are the copied card's (CR 707.2), so a
/// Spark Double that became a Solemn Simulacrum offers Solemn's abilities and
/// its rows are Solemn's sentences — and an index into the Spark Double's own
/// text names nothing, or the wrong thing. A token copy has no card at all
/// and still has a face here.
///
/// A client draws every ability row and every stack entry out of this card's
/// text, and reads the ability an offered index names out of this card's
/// list, which is what makes a copy need no special treatment anywhere.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct RulesFace {
    /// The card the abilities are printed on.
    pub card: CardIndex,
    /// Which of its faces.
    pub face: u8,
}

/// A card name chosen for a permanent (CR 201.4): the card and the face whose
/// name it is. A card name, not a card: a client draws the name, and a copy
/// of the named card anywhere at the table answers to it.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct NamedFace {
    /// The card the name is printed on.
    pub card: CardIndex,
    /// Which of its faces, since a back face's name may be chosen too
    /// (CR 201.4d).
    pub face: u8,
}

impl From<CardIdentity> for RulesFace {
    /// The card itself and the face it shows — which is what a card that is
    /// not a copy has its abilities printed on.
    fn from(card: CardIdentity) -> Self {
        Self {
            card: card.index,
            face: card.face,
        }
    }
}

/// What a stack entry is, beyond the object carrying it.
///
/// A permanent's ability on the stack has no card of its own — it is a
/// separate object whose only identity is "ability *n* of card *c*, put
/// there by permanent *p*". Without this a client can only render an
/// anonymous entry: it knows a trigger is resolving but not whose, and
/// not which of the three abilities on that permanent it is.
///
/// The [`AbilityRef`] is the same handle a player's standing answer is
/// stored under, so "always yes for this" and "this is what is on the
/// stack" name the same thing. It is optional because a token, a token
/// copy and an emblem have no card to name (CR 111.1, CR 114.2): their
/// abilities are addressed by nothing, and a standing answer cannot be
/// filed against one. It used to be card index 0 — a real card, and the
/// same one for every such ability.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum StackItem {
    /// A spell: the card itself is on the stack, and [`PublicObject::card`]
    /// already identifies it.
    Spell,
    /// An activated or triggered ability.
    Ability {
        /// The permanent, spell or emblem the ability came from. It may
        /// already have left the battlefield — the ability on the stack is
        /// independent of its source (CR 113.7a) — so a client should fall
        /// back to the name below when it can no longer find the object.
        source: ObjectId,
        /// Which ability of which card, stable across games — `None` when
        /// the source has no card and there is no such handle.
        ability: Option<AbilityRef>,
        /// Where this ability's printed sentence is, when it is known.
        ///
        /// A separate field rather than more of [`AbilityRef`], because
        /// that handle is also what a player's standing answer is filed
        /// under: it names an ability across games and printings, and a
        /// sentence index is about one printing's text.
        ///
        /// An index into [`Self::Ability::rules`]'s card, which is not
        /// always the source's: a copy's ability is printed on the card it
        /// copied.
        text: Option<StackText>,
        /// The card this ability is printed on, when a card prints it —
        /// captured with the ability as it was put on the stack, like the
        /// ability itself (CR 113.7a), so it outlives the source and
        /// anything the source becomes. `None` for a token's ability and an
        /// emblem's.
        rules: Option<RulesFace>,
        /// Token definition and ability captured before its source disappeared.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        token: Option<TokenAbility>,
    },
}

/// A registry token's exact ability, independent of the source's lifetime.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct TokenAbility {
    /// Stable token registry id.
    pub token: u16,
    /// Index in that token definition's ability list.
    pub index: u32,
}

/// Where an ability's printed sentence is, so a client can draw a stack
/// entry as what the ability *does*.
///
/// The engine carries no card text and a client's text is whatever
/// printing and language that player chose, so neither end can be handed
/// the sentence itself. What travels is where to find it: which face, and
/// which sentence of that face — computed by `cargo xtask codegen` from
/// the **English** oracle text against the compiled ability list, which is
/// why [`StackText::of`] comes with it.
///
/// It is absent for an ability whose sentence is not known: one belonging
/// to a token, a token copy or an emblem (CR 111.1, CR 114.2), one a
/// continuous effect granted, and the handful of printed ones no sentence
/// fits (a keyword's own trigger has none of its own). A client draws
/// those as it drew every ability before this field existed.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct StackText {
    /// Which face of the card the ability came from.
    ///
    /// Not the face the source is showing now: an ability on the stack is
    /// independent of its source (CR 113.7a), which may have transformed
    /// or died since. A client splits *this* face's text, and borrows
    /// this face's picture, so the two agree with each other and with the
    /// ability that is actually resolving.
    pub face: u8,
    /// 0-based index into that face's sentences.
    pub line: u8,
    /// How many sentences the **English** text of that face has.
    ///
    /// The index was computed against English; a client resolves it
    /// against a localized printing, which may be pre-errata wording or a
    /// translation that joins two lines into one. An index merely out of
    /// range is caught by anyone — one that is *in* range and points a
    /// sentence off is shown to the player as precise text and is worse
    /// than no text at all. A client whose own split of the text it holds
    /// yields a different number must refuse the whole answer.
    pub of: u8,
}

/// An object a seat can see, with its characteristics already projected
/// through the layer system.
///
/// The projected fields are what a client renders. They are *not* the printed
/// values: a Mountain animated into a 4/4 arrives here as a creature with
/// power 4, and a clone of Serra Angel arrives with Serra Angel's name.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct PublicObject {
    /// Effective typed word substitutions, separate from the unchanged Oracle.
    #[serde(default)]
    pub word_changes: Vec<WordChange>,
    /// Engine object handle; stable while the object stays in its zone.
    pub id: ObjectId,
    /// Backing card, when the viewing seat is entitled to know it. `None` for
    /// tokens, emblems, and face-down permanents the seat may not look at.
    pub card: Option<CardIdentity>,
    /// The card this object's abilities are printed on ([`RulesFace`]).
    ///
    /// Equal to [`Self::card`]'s index and face for every object that is not
    /// a copy, and gated on the same entitlement: a face-down permanent the
    /// seat may not look at names no card here either. `None` for a registry
    /// token (which [`Self::token`] names) and an emblem.
    pub rules: Option<RulesFace>,
    /// Projected name. Present even when `card` is `None`, so tokens and
    /// face-down permanents still render a label ("Soldier", "Face-down").
    pub name: String,
    /// Controller.
    pub controller: PlayerId,
    /// Owner — differs from the controller under control-changing effects, and
    /// clients mark that difference because it decides where the card returns.
    pub owner: PlayerId,
    /// Whether this object is one of its owner's commanders (CR 903.3).
    ///
    /// Public information wherever the object itself is: a commander is
    /// designated openly at the start of the game, so knowing that *this*
    /// card is one reveals nothing the table did not already share.
    ///
    /// Carried on the object even though [`SeatView::commanders`] already
    /// names the same ids, because every renderer that draws a card has a
    /// `PublicObject` in hand and would otherwise have to carry the seat
    /// list down with it. The host asserts the two agree.
    ///
    /// [`SeatView::commanders`]: crate::SeatView::commanders
    pub commander: bool,
    /// Status bits.
    pub status: ObjectStatus,
    /// Projected types.
    pub types: TypeSet,
    /// Projected supertypes.
    pub supertypes: SupertypeSet,
    /// Projected subtypes.
    ///
    /// Carried for the same reason as [`Self::types`]: a client that builds a
    /// type line cannot derive it from the printed card. An animated land is
    /// genuinely a `Creature — Elemental`, and a card that gained a type keeps
    /// its printed ones — only the projection knows the answer.
    pub subtypes: SubtypeSet,
    /// Creature type named for this permanent (Reflections of Littjara, Cavern of Souls).
    #[serde(default)]
    pub chosen_subtype: Option<baylee_core::ids::SubtypeId>,
    /// Card name chosen for this permanent as it entered (Pithing Needle).
    /// Public: the choice is announced as it is made.
    #[serde(default)]
    pub chosen_name: Option<NamedFace>,
    /// Opponent publicly chosen as this permanent entered (Black Vise).
    #[serde(default)]
    pub chosen_opponent: Option<PlayerId>,
    /// A Room's doors (CR 709.5c): `[left, right]`, each `true` while that
    /// half is unlocked; `None` for anything that is not a Room on the
    /// battlefield. Public, as the designations are. A locked half has no
    /// name, mana cost or rules text on the battlefield (CR 709.5), which
    /// the other fields already say; this says which half is which.
    #[serde(default)]
    pub unlocked_doors: Option<[bool; 2]>,
    /// Whether this face-up exiled card is suspended. Its time counters are public.
    #[serde(default)]
    pub suspended: bool,
    /// Which token this is, for permanents with no card behind them.
    ///
    /// The index into `baylee_cards::tokens::ALL`. A token has no printing
    /// and therefore no [`Self::card`], which left a client with nothing to
    /// draw but a coloured rectangle; this is the handle it keys token art
    /// on, and the one thing that distinguishes a Treasure from a Clue when
    /// both project to "an artifact named something". `None` for cards and
    /// for the tokens a copy effect makes, which are copies of a card rather
    /// than of a registry token.
    pub token: Option<u16>,
    /// Projected colors.
    pub colors: ColorSet,
    /// Projected mana value.
    ///
    /// The stack shows what a spell actually cost to cast rather than what
    /// its card prints, and a graveyard or exile card is often the one whose
    /// cost decides whether it can be played from there.
    pub mana_value: u32,
    /// Projected keyword bitset (`baylee_cards_dsl::KeywordSet` bits).
    pub keywords: u128,
    /// Projected power, for creatures.
    pub power: Option<i16>,
    /// Projected toughness, for creatures.
    pub toughness: Option<i16>,
    /// The power the card itself prints, before any continuous effect.
    ///
    /// Carried beside the projection rather than derived from it, because a
    /// client cannot run the layer system and the two numbers differ for
    /// four unrelated reasons — an anthem, a counter, a pump spell, a copy
    /// effect. Only the engine can say which base a permanent has: a Clone's
    /// base is the *copied* card's printed body (CR 706.2), not the Clone's
    /// own, and a token's is whatever minted it.
    ///
    /// The client draws it under the corner plate, which sits exactly where
    /// a real card prints its power and toughness and therefore hides them.
    /// `None` for anything that is not a creature, and for a creature whose
    /// base the engine has no number for.
    pub base_power: Option<i16>,
    /// The toughness the card itself prints. See [`Self::base_power`].
    pub base_toughness: Option<i16>,
    /// Loyalty, for planeswalkers.
    pub loyalty: Option<u16>,
    /// Damage marked this turn.
    pub damage: u16,
    /// Counters on the object.
    pub counters: Vec<CounterEntry>,
    /// What this is attached to (auras, equipment, fortifications).
    pub attached_to: Option<ObjectId>,
    /// Targets, for objects on the stack.
    pub targets: Vec<TargetRef>,
    /// What this is, for objects on the stack; `None` everywhere else.
    pub stack_item: Option<StackItem>,
    /// Whether this is a creature that has *not* been under its controller's
    /// control continuously since their most recent turn began, and has no
    /// haste (CR 302.6) — so it cannot attack, and cannot pay `{T}` or `{Q}`
    /// for an ability of its own.
    ///
    /// Creatures only, which is narrower than it reads: the field once
    /// answered "did this permanent enter this turn", and a land played this
    /// turn came back `true` for a question the rules never ask about lands.
    /// It also holds through an opponent's turn, because the clock it is
    /// measured against is the controller's own.
    pub summoning_sick: bool,
    /// Mana this permanent can make through an ability it does not print.
    ///
    /// A projected *characteristic* like the ones above, and carried for the
    /// same reason: a land under a Chromatic Lantern taps for any colour, and
    /// there is no card anywhere a client could read that off — the ability
    /// exists only in the effect table. Without this a client's mana planner
    /// counts such a land for nothing and the player taps it by hand.
    ///
    /// `None` for everything that has no such ability, and for a granted
    /// ability too complicated to reduce to "n mana of these colours".
    pub granted_mana: Option<GrantedMana>,
    /// Which colours a **printed** mana ability of this permanent makes, when
    /// the printing does not say.
    ///
    /// The field beside it covers an ability that is on no card; this one
    /// covers an ability that is, and still cannot be read alone. Reflecting
    /// Pool, Exotic Orchard and Fellwar Stone are "one mana of any type that
    /// a land on *that* side of the table could produce", and Command Tower,
    /// Arcane Signet, Commander's Sphere and Path of Ancestry are "any colour
    /// in your commander's identity" — the words are printed, the answer is
    /// the board's.
    ///
    /// It is the first that needs a board and no other, which is the same
    /// bargain [`GrantedMana::slot`] strikes: no card in the pool prints two,
    /// and a permanent that did would have its second refused rather than
    /// guessed at — a refused tap costs a player one manual tap, and a wrong
    /// one strands a mana run with the permanent already tapped.
    ///
    /// **Colours only, and no amount.** The ability is printed, so how much
    /// it makes is on the card and the client reads it there; the colours are
    /// the one thing that is not. The union a Reflecting Pool reads is over
    /// `produced_colors`, which is a *projected* characteristic — an animated
    /// land, a land that has lost its abilities and a Chromatic Lantern's
    /// grant are all in it and none of them is in the registry — so a client
    /// re-deriving it would over-count, and over-counting is the direction
    /// that leaves a board half tapped when the engine refuses the colour.
    ///
    /// `None` means there is nothing here to plan with: no such ability, or
    /// one that makes nothing right now. A lone Reflecting Pool contributes
    /// nothing to the union it reads, so it taps for no colour at all.
    pub board_mana: Option<BoardMana>,
    /// What the viewing seat may pay to cast this card from its graveyard:
    /// the cost of a flashback it has right now (CR 702.34a).
    ///
    /// A projected characteristic like [`Self::granted_mana`], and carried
    /// for the same reason: the grant exists only in the effect table. The
    /// engine offers a graveyard spell in `LegalActions::castable` only once
    /// its cost is already floating, so a planner that could not see the
    /// card was castable never tapped for it — Snapcaster Mage's Opt ended
    /// the turn in the graveyard beside an untapped Island (#242).
    ///
    /// A printed flashback is priced at what the card prints (Memory Deluge's
    /// `{5}{U}{U}`); a granted one at the card's own mana cost, and so is a
    /// permanent card a graveyard permission lets the seat cast (Muldrotha,
    /// Wrenn and Realmbreaker's emblem), which is no flashback but is the
    /// same question for the planner. A card with escape is priced at its
    /// escape mana once its owner's graveyard holds the other cards it
    /// exiles (Uro, Titan of Nature's Wrath), and at nothing before.
    ///
    /// Graveyard only, and per viewer: `None` unless this seat may cast the
    /// card, which is only ever from its own graveyard.
    pub flashback: Option<ManaCost>,
    /// Who granted each of this permanent's granted activated abilities, in
    /// slot order: entry `n` is the ability offered as `granted_ability(n)`
    /// (#212).
    ///
    /// One entry per grant the engine offers, the same walk and the same
    /// cap (`GRANTED_SLOTS`), so a slot and its entry cannot drift apart.
    /// Empty for anything that is not a permanent, and for a permanent
    /// granted nothing.
    ///
    /// Per viewer: a grantor this seat may not see — gone to a hand or a
    /// library since, or face down — is an entry with every field `None`.
    /// A nontoken card keeps its handle across zones (#240), so naming it
    /// would say which card in a hidden zone it is.
    pub grants: Vec<GrantSource>,
}

/// Where a granted ability comes from, as far as the viewing seat may know.
///
/// The ability is printed on no card the permanent has: a land under a
/// Chromatic Lantern prints nothing about the `{T}` it was given. The
/// grantor prints it, in a sentence saying the permanent "has" it
/// (CR 113.10), and this says which grantor and which sentence.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GrantSource {
    /// The object that granted it, when this seat may see that object.
    pub source: Option<ObjectId>,
    /// The card and face the grant's sentence is printed on.
    ///
    /// Its own field and not the grantor's [`PublicObject::rules`], because
    /// the two differ for a copy: a Machine God's Effigy copying an
    /// artifact has the copied card's abilities, and still prints the
    /// clause that grants it `{T}: Add {U}`.
    pub rules: Option<RulesFace>,
    /// Which sentence of [`Self::rules`] granted it, for a client drawing it
    /// in the player's own language.
    ///
    /// `None` when the grantor is hidden, when no ability of its card wrote
    /// this grant (found by value; there is no nearest match), and when the
    /// card prints no line for the ability that did. A client then draws
    /// its own label, as it did before this field existed.
    pub text: Option<StackText>,
}

/// Mana a granted ability makes, as much of it as a planner can use.
///
/// Deliberately not an ability: the client already knows the handle
/// (`choice::GRANTED_ABILITY`) and the engine already decided whether it may
/// be activated. What it cannot know is what comes out.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GrantedMana {
    /// Which of the permanent's granted abilities this is, counting from 0.
    ///
    /// Carried because it is not always the first: a permanent may be granted
    /// several, and the one that makes mana is not necessarily the one at the
    /// front. Without it a client would tap the slot next to the one it was
    /// told about. A plain ordinal rather than the engine's synthetic index,
    /// because this crate does not depend on the rules kernel and the
    /// encoding is the kernel's (`choice::granted_ability`).
    pub slot: u32,
    /// The colours it may make. More than one means the ability asks.
    pub colors: Vec<baylee_core::mana::ManaColor>,
    /// How much, of whichever colour is chosen.
    pub amount: u8,
}

/// Which colours a printed mana ability makes, once a board has been read.
///
/// The counterpart of [`GrantedMana`] for an ability that *is* printed —
/// see [`PublicObject::board_mana`] for which cards these are and why the
/// answer cannot be worked out from the card.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BoardMana {
    /// Which of the permanent's printed abilities this is, counting from 0 on
    /// the face it is showing.
    ///
    /// The same numbering `AbilityRef` uses and the same one a client indexes
    /// the registry with, so it addresses the ability directly instead of
    /// being matched by shape. Carried for the reason [`GrantedMana::slot`]
    /// is: a permanent whose mana ability is not its first — Commander's
    /// Sphere prints a sacrifice ability beside it — would otherwise have the
    /// colours read onto the wrong row.
    pub index: u32,
    /// The colours it may make. More than one means the ability asks.
    ///
    /// Never empty: an ability that makes nothing right now is reported as no
    /// [`PublicObject::board_mana`] at all.
    pub colors: Vec<baylee_core::mana::ManaColor>,
}

impl PublicObject {
    /// Effective toughness minus marked damage; `None` for non-creatures.
    ///
    /// Clients show this as the "remaining" number so a player can see lethal
    /// without doing arithmetic under time pressure.
    #[must_use]
    pub fn remaining_toughness(&self) -> Option<i16> {
        self.toughness.map(|t| t - self.damage as i16)
    }

    /// Whether marked damage is lethal (CR 704.5g), ignoring indestructible —
    /// a client uses it to tint the damage badge, not to decide the rules.
    #[must_use]
    pub fn is_lethally_damaged(&self) -> bool {
        self.remaining_toughness().is_some_and(|r| r <= 0)
    }

    /// How many counters of a given kind sit on the object.
    #[must_use]
    pub fn counter_count(&self, kind: CounterKind) -> u16 {
        self.counters
            .iter()
            .find(|c| c.kind == kind)
            .map_or(0, |c| c.count)
    }

    /// A stable grouping key for identical objects.
    ///
    /// Token-heavy boards are unreadable one card at a time; a client collapses
    /// objects that share this key into a single stack with a count. Two
    /// objects group only when every property a decision reads matches —
    /// drawn or not, since the merged card shows one member's — so collapsing
    /// can never hide a difference that matters to a decision. Left out:
    /// `id`; `targets`, `stack_item` and `flashback`, which a permanent never
    /// has; and
    /// `rules` and `mana_value`, which `card`, `name` and `status` already
    /// decide on the battlefield. `attached_to` is in only as a yes or no.
    #[must_use]
    pub fn summary_key(&self) -> ObjectSummaryKey {
        let mut counters: Vec<CounterEntry> = self.counters.clone();
        counters.sort_by_key(|c| (format!("{:?}", c.kind), c.count));
        ObjectSummaryKey {
            card: self.card.map(|c| (c.index, c.face)),
            token: self.token,
            name: self.name.clone(),
            controller: self.controller,
            owner: self.owner,
            commander: self.commander,
            status: self.status,
            types: self.types,
            supertypes: self.supertypes,
            subtypes: self.subtypes,
            chosen_subtype: self.chosen_subtype,
            chosen_name: self.chosen_name,
            chosen_opponent: self.chosen_opponent,
            unlocked_doors: self.unlocked_doors,
            suspended: self.suspended,
            colors: self.colors,
            keywords: self.keywords,
            power: self.power,
            toughness: self.toughness,
            base_power: self.base_power,
            base_toughness: self.base_toughness,
            damage: self.damage,
            loyalty: self.loyalty,
            counters,
            attached: self.attached_to.is_some(),
            summoning_sick: self.summoning_sick,
            granted_mana: self.granted_mana.clone(),
            board_mana: self.board_mana.clone(),
        }
    }
}

/// Grouping key produced by [`PublicObject::summary_key`].
#[derive(Clone, PartialEq, Eq, Debug)]
#[allow(clippy::struct_excessive_bools)] // independent public facts, not states of one machine
pub struct ObjectSummaryKey {
    card: Option<(CardIndex, u8)>,
    /// A token has no `card`, so without this a Soldier with lifelink and
    /// the plain Soldier another set prints group by name alone — and every
    /// token merges, however roomy its row.
    token: Option<u16>,
    name: String,
    controller: PlayerId,
    /// Not drawn, but read: "a permanent you own" is a target filter, and a
    /// stolen token answers it differently from its twin.
    owner: PlayerId,
    /// A commander is drawn with a marker on it, so it must not group with an
    /// ordinary copy of the same card — the token a clone effect makes is
    /// identical in every other field.
    commander: bool,
    status: ObjectStatus,
    /// The projection, all of it: an effect that makes one of two twins a
    /// Zombie, blue or a flier makes them two different cards to decide
    /// about, and the merged card shows only one of them.
    types: TypeSet,
    supertypes: SupertypeSet,
    subtypes: SubtypeSet,
    chosen_subtype: Option<baylee_core::ids::SubtypeId>,
    /// Two Needles naming different cards are two different cards to look
    /// at, and a pile shows one label.
    chosen_name: Option<NamedFace>,
    chosen_opponent: Option<PlayerId>,
    /// Two Rooms with different doors open are different cards to act on:
    /// one has a door left to unlock.
    unlocked_doors: Option<[bool; 2]>,
    suspended: bool,
    colors: ColorSet,
    keywords: u128,
    power: Option<i16>,
    toughness: Option<i16>,
    /// In the key because it is drawn. A printed 3/3 and a 2/2 under an
    /// anthem are both projected 3/3 and their corners say different things,
    /// so grouping them would put one card's appendage on the other's pile.
    /// The same argument loyalty was added on.
    base_power: Option<i16>,
    base_toughness: Option<i16>,
    damage: u16,
    loyalty: Option<u16>,
    counters: Vec<CounterEntry>,
    attached: bool,
    summoning_sick: bool,
    /// What the permanent taps for when its card does not say: a land a
    /// spell made tap for any colour is not the Forest beside it.
    granted_mana: Option<GrantedMana>,
    board_mana: Option<BoardMana>,
}

impl core::hash::Hash for ObjectSummaryKey {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.card.hash(state);
        self.token.hash(state);
        self.name.hash(state);
        self.controller.hash(state);
        self.commander.hash(state);
        self.status.hash(state);
        self.types.hash(state);
        self.chosen_subtype.hash(state);
        self.chosen_name.hash(state);
        self.chosen_opponent.hash(state);
        self.unlocked_doors.hash(state);
        self.suspended.hash(state);
        self.power.hash(state);
        self.toughness.hash(state);
        self.damage.hash(state);
        self.loyalty.hash(state);
        for c in &self.counters {
            c.kind.hash(state);
            c.count.hash(state);
        }
        self.attached.hash(state);
        self.summoning_sick.hash(state);
    }
}

/// A card in the viewing seat's own hand, or in a teammate's it is shown
/// ([`SharedHand`]).
///
/// Separate from [`PublicObject`] because a card in hand has no board state and
/// carrying the permanent-only fields would invite a client to render them.
///
/// [`SharedHand`]: crate::SharedHand
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct HandObject {
    /// Engine object handle.
    pub id: ObjectId,
    /// Card identity — always known: it is the seat's own hand, or one its
    /// owner is showing this seat.
    pub card: CardIdentity,
    /// Printed name of the active face.
    pub name: String,
    /// Converted mana cost, for sorting the hand.
    pub mana_value: u32,
    /// Colors, for the hand's color grouping.
    pub colors: ColorSet,
    /// Types, so a client can badge lands and instants.
    pub types: TypeSet,
    /// Whether this is one of the seat's commanders (CR 903.3).
    ///
    /// A commander reaches a hand by declining CR 903.9b's replacement, and
    /// there it is an ordinary card that happens to recast for its printed
    /// cost — CR 903.8 taxes only the command zone. Worth marking for
    /// exactly that reason: it is the one card in the hand whose price goes
    /// up if it is played and then dies.
    pub commander: bool,
}
