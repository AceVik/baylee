//! How much an effect does: a number, or a count read off the game.

use super::{CounterKind, Filter, ZoneSel};

/// A computed number (CR 107.1).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Amount {
    /// A fixed value.
    Fixed(u32),
    /// Total life opponents have lost this turn; later gains do not subtract.
    OpponentsLifeLostThisTurn,
    /// Creature arrivals under your control this turn, including departed entrants.
    CreaturesEnteredUnderYourControlThisTurn,
    /// The value of X chosen at cast time.
    X,
    /// X if this spell was foretold, otherwise the stated amount. The rider
    /// is on the spell, not on a later permanent incarnation (CR 702.143c).
    XIfSourceWasForetold {
        /// Amount for an ordinary cast or another non-foretold source.
        otherwise: u32,
    },
    /// The negated value of X (Toxic Deluge's `-X/-X`; evaluated as a
    /// negative at use sites).
    NegX,
    /// The value of X plus the controller's commander-cast count
    /// (Commander's Insight).
    XPlusCommanderCasts,
    /// Twice X (Heliod's Intervention).
    DoubleX,
    /// Number of distinct colors among battlefield objects matching the
    /// filter (General Tazri).
    DistinctColorsAmong(&'static Filter),
    /// Number of distinct **basic land types** among battlefield objects
    /// matching the filter — the count every "domain" card spells out.
    ///
    /// Domain is an ability word and so has no rules meaning of its own
    /// (CR 207.2c): what the card actually says is "for each basic land
    /// type among lands you control", which is why this takes a filter
    /// rather than being a bare `Domain` variant with "lands you control"
    /// hidden inside the engine.
    ///
    /// It is not [`Amount::CountOf`] and not [`Amount::DistinctColorsAmong`],
    /// which is what left three cards in the pool unwritable: `CountOf`
    /// counts *objects*, so one Tundra answers 1 where the card wants 2 and
    /// two Forests answer 2 where the card wants 1; and a land is
    /// colourless, so the colour count answers 0 for any of them. The five
    /// types are CR 305.6's, and no other land type counts.
    BasicLandTypesAmong(&'static Filter),
    /// A fixed negative value (-N at use sites).
    NegXFixed(u32),
    /// The power of the first target (last known characteristics).
    TargetPower,
    /// The toughness of the first target (last known characteristics).
    TargetToughness,
    /// The power of the ability's own source (Esper Sentinel's tax).
    ///
    /// Read off the *projected* characteristics, so an anthem or a counter
    /// raises it — which is the whole reason the card prints `{X}` instead
    /// of `{1}`. A source that is not a creature, or that has left the
    /// battlefield, counts as zero rather than as its printed number: an
    /// ability whose amount comes off a permanent has nothing to read when
    /// the permanent is gone.
    SourcePower,
    /// Creatures currently blocking the source beyond the first (CR 702.23b).
    BlockersBeyondFirst,
    /// How many counters of a kind are on the ability's own source — Aether
    /// Vial's "the number of charge counters on this artifact", cumulative
    /// upkeep's "for each age counter on it" (CR 702.24a).
    ///
    /// Resolution reads the source's current incarnation or its captured last
    /// battlefield information if it left (CR 608.2h), including a source
    /// sacrificed to pay the ability's cost. Outside resolution this is a live
    /// count only. Earlier instructions' counter changes are visible.
    CountersOnSource(CounterKind),
    /// Number of counters of this kind held by the player evaluating the effect or cost.
    CountersOnYou(CounterKind),
    /// Devotion to the color produced by an `AddMana` instruction (CR 700.5).
    /// Evaluated per available color by mana resolution; zero without that context.
    DevotionToManaColor,
    /// The mana value of the first target (Reanimate's life loss).
    TargetCmc,
    /// "That much": the amount of damage the triggering event dealt
    /// (Questing Beast's redirect). Carried from the event onto the
    /// triggered ability as it goes on the stack; 0 anywhere else.
    EventAmount,
    /// Last battlefield toughness of the event permanent, captured on its
    /// triggered ability. Negative values count as zero.
    EventLastToughness,
    /// Power of the event permanent (or its last battlefield power if it left,
    /// CR 608.2h). Negative values count as zero.
    EventPower,
    /// "The sacrificed creature's mana value": the mana value, as it last
    /// existed on the battlefield (CR 608.2h), of the permanent sacrificed
    /// to pay the cost of the spell or ability that is resolving (Eldritch
    /// Evolution, Neoform, Birthing Pod).
    ///
    /// Read off the stack object, which is where the payment wrote it down:
    /// a spell's `Sacrifice` additional cost in the cast wizard, an
    /// activation's in `pay_cost`. Nothing sacrificed reads 0.
    SacrificedManaValue,
    /// "The sacrificed creature's power" (Kazuul's Fury, Life Chisel's
    /// sibling Miren): as [`Self::SacrificedManaValue`], the power it had
    /// as it last existed on the battlefield (CR 608.2h). Never negative.
    SacrificedPower,
    /// "The sacrificed creature's toughness" (Diamond Valley, Miren, the
    /// Moaning Well): likewise, its toughness. Never negative.
    SacrificedToughness,
    /// "The total power of the creatures sacrificed this way" (Sword of the
    /// Ages, Angelic Aberration, Disciple of Bolas): the sum of the powers the
    /// permanents sacrificed during this resolution or via a `CostPart::Sacrifice*`
    /// payment had as they last existed on the battlefield (CR 608.2h), floored
    /// at 0 (CR 107.1b).
    SacrificedTotalPower,
    /// "The amount of mana spent to cast this spell" (Memory Deluge): what
    /// the cast paid in mana (CR 601.2h), read off the stack object where
    /// the payment wrote it, as [`Self::SacrificedManaValue`] is. A free
    /// cast spent none; a flashback spent its flashback cost.
    ManaSpentToCast,
    /// Actual units of this mana type assigned to the printed X payment,
    /// excluding fixed, additional and increased costs. No payment means zero.
    ManaTypeSpentOnX(baylee_core::mana::ManaColor),
    /// "The tapped creature's power": the power of the permanent a
    /// `CostPart::TapOther` tapped to pay the cost of the ability that is
    /// resolving — station's "put a number of charge counters on this
    /// permanent equal to the tapped creature's power" (CR 702.184a). Its
    /// power as the effect applies while it is still on the battlefield as
    /// the same object, and as it last existed there otherwise (CR 608.2h).
    ///
    /// The creature is not a target (station targets nothing), so hexproof
    /// does not stop it. Read off the stack object, where `pay_cost` wrote
    /// which creature it tapped; nothing tapped reads 0.
    TappedPower,
    /// "For each creature that died this turn" (Scavenging Ghoul): the
    /// creatures put into a graveyard from the battlefield this turn
    /// (CR 700.4), every player's, tokens included. A permanent counts as
    /// a creature if it was one as it left, not if the card in the graveyard
    /// is one. It is counted once, as the effect applies (CR 608.2h).
    CreaturesDiedThisTurn,
    /// Untapped lands the active player controlled as this turn began,
    /// before the untap step or phasing. This historical count is retained
    /// even if the ability's source was not on the battlefield then.
    UntappedLandsAtTurnStart,
    /// "The damage dealt to you this turn" (Simulacrum): every point of
    /// damage dealt to the ability's controller since the turn began,
    /// combat and not, from any source. Damage dealt, not life lost: a
    /// player whose life can't change is still dealt it, and life gained
    /// back since takes none of it away. Prevented damage was never dealt
    /// (CR 615.1). Read as the effect applies (CR 608.2h).
    DamageDealtToYouThisTurn,
    /// "The number of Mountains put into a graveyard this way" (Volcanic
    /// Eruption): how many of the resolving ability's targets a graveyard
    /// now holds as new objects, read after the effect that moved them. A
    /// target that was regenerated, went somewhere else instead, or was
    /// dropped as illegal (CR 608.2b) is not one. Outside a resolution, 0.
    TargetsPutIntoGraveyard,
    /// Number of objects matching a filter in a zone.
    CountOf {
        /// What to count.
        filter: &'static Filter,
        /// Where to count.
        zone: ZoneSel,
    },
    /// Number of cards in the named player's library, read on resolution.
    LibrarySize(super::PlayerRel),
    /// Number of cards in the named player's hand.
    HandSize(super::PlayerRel),
    /// The named player's current life total, floored at zero for a magnitude.
    LifeTotal(super::PlayerRel),
    /// The named player's starting life total (CR 103.4).
    StartingLifeTotal(super::PlayerRel),
    /// Total life the named player gained this turn; later losses do not subtract.
    LifeGainedThisTurn(super::PlayerRel),
    /// Number of matching permanents the named player controls.
    ControlledCount {
        /// Controller relative to the resolving ability.
        who: super::PlayerRel,
        /// Permanent characteristics to count.
        filter: &'static Filter,
    },
    /// Greatest mana value among your designated commanders, in any zone.
    GreatestCommanderManaValue,
    /// Half a magnitude, with the printed rounding direction.
    Half {
        /// Magnitude to halve.
        base: &'static Amount,
        /// Round odd values up instead of down.
        round_up: bool,
    },
    /// Divide a magnitude by a positive integer with the printed rounding.
    Divided {
        /// Magnitude to divide.
        base: &'static Amount,
        /// A zero divisor is unrepresentable.
        divisor: core::num::NonZeroU32,
        /// Round a fractional result up instead of down.
        round_up: bool,
    },
    /// Multiply a magnitude by a nonnegative integer, saturating on overflow.
    Times {
        /// Magnitude to multiply.
        base: &'static Amount,
        /// Nonnegative multiplier.
        factor: u32,
    },
    /// Add two magnitudes, saturating only if the result exceeds the engine's numeric range.
    Sum {
        /// First magnitude.
        left: &'static Amount,
        /// Second magnitude.
        right: &'static Amount,
    },
    /// Subtract one magnitude from another, flooring the result at zero.
    Difference {
        /// Minuend.
        left: &'static Amount,
        /// Subtrahend.
        right: &'static Amount,
    },
    /// Number of players still in the game matching this relation.
    Players(super::PlayerRel),
    /// Another amount with a constant added to it: Muscle Burst's "3 plus
    /// the number of cards named Muscle Burst in all graveyards".
    ///
    /// A wrapper for the same reason [`Self::Negated`] is one — the offset
    /// is said once instead of doubling every counting amount there is — and
    /// it saturates rather than wraps, because a count is a count.
    ///
    /// The base is a **magnitude**: [`Self::is_negative`] reads a `Plus` as
    /// positive, because that is what the addition computes, and
    /// `amount_sign_tests::no_amount_in_the_pool_offsets_a_negative` is what
    /// keeps a negative base out rather than letting it resolve upwards.
    Plus {
        /// The amount being offset.
        base: &'static Amount,
        /// What is added to it.
        offset: u32,
    },
    /// The negation of another amount: "-1/-1 for each artifact you control"
    /// (Irradiate).
    ///
    /// [`Amount::NegX`] and [`Amount::NegXFixed`] are the two negatives the
    /// pool had before this, and they are the two whose magnitude is already
    /// a variant of its own. A *counted* quantity has no such twin — there is
    /// one `CountOf`, and a card wanting its negative had nothing to write —
    /// so this says the negation rather than doubling every amount there is.
    ///
    /// The sign is not in the evaluated number: the engine's `eval::amount`
    /// answers a magnitude, and every reader asks [`Amount::is_negative`] for
    /// the sign. Ask that function and never `matches!` on the variants: the
    /// three places that spelled the question out by hand would each have
    /// read this one as positive, which is a card that prints `-X/-X` and
    /// hands out `+X/+X`.
    Negated(&'static Amount),
    /// Subtract a constant from a nonnegative amount, with zero as the floor.
    SaturatingSub {
        /// The amount being reduced.
        base: &'static Amount,
        /// The constant subtracted.
        subtract: u32,
    },
    /// A constant minus a nonnegative amount, with zero as the floor: the
    /// reverse of [`Self::SaturatingSub`] (The Rack and Storm World: "3 minus
    /// the number of cards in their hand").
    ConstantMinus {
        /// The constant.
        constant: u32,
        /// The amount subtracted from it.
        subtract: &'static Amount,
    },
    /// Mana value of the captured triggering spell, including its chosen X.
    EventManaValue,
    /// "… this way" and "that many" (CR 608.2c): what the resolving spell
    /// or ability has already done, counted as this instruction runs — never
    /// what anything else did, and never what a later instruction of the
    /// same resolution will do. Hellfire's "the number of creatures that
    /// died this way", Syphon Soul's "the damage dealt this way", Recall's
    /// "for each card discarded this way".
    ///
    /// Read only where a spell or ability resolves off the stack; anywhere
    /// else (a static ability, a mana ability, a cost) it is 0. It counts
    /// every player's part; [`Self::ThisWayBy`] counts one player's.
    ThisWay(ThisWay),
    /// [`Self::ThisWay`], counting only the part of the players `who`
    /// names: whose cards were discarded or put into a library, whose
    /// permanents died or were returned (as their controller as they left),
    /// or to whom damage was dealt. Winds of Change's "each player … then
    /// draws that many cards" is
    /// `ThisWayBy { what: PutIntoLibraryFromHand, who: PlayerRel::Recipient }`
    /// inside `Effect::DrawCardsFor { who: EachPlayer, .. }`: each player
    /// draws the number they shuffled in.
    ThisWayBy {
        /// What is counted.
        what: ThisWay,
        /// Whose part.
        who: super::PlayerRel,
    },
    /// Damage dealt this turn (after prevention, CR 615.1) to `to` by
    /// sources the filter matches **as they were when they dealt it**, read
    /// with the ability's controller as "you" and its source as "this":
    /// Reverse Polarity's "the damage dealt to you so far this turn by
    /// artifacts", Blazing Effigy's "damage dealt to it this turn by other
    /// sources named Blazing Effigy", Temple of Power's "red sources you
    /// controlled dealt 4 or more noncombat damage this turn". `combat`
    /// narrows it to combat (`Some(true)`) or noncombat (`Some(false)`)
    /// damage.
    DamageThisTurn {
        /// Who or what was dealt it.
        to: DamageTo,
        /// The sources counted.
        by: &'static Filter,
        /// Only combat, only noncombat, or both.
        combat: Option<bool>,
    },
    /// How many times the spell was kicked (CR 702.33d): "for each time it
    /// was kicked", 0 for a spell or permanent that was not.
    ///
    /// Read off the object itself, which is what lets one word serve the
    /// spell's own effect and the permanent it became (CR 400.7d): an entry
    /// replacement and an enters trigger of a multikicked creature count the
    /// payments of the spell it was, and a permanent that arrived any other
    /// way counts none, because arriving from anywhere but the stack clears
    /// the count with the X (`progress::apply_enter_modifiers`).
    TimesKicked,
    /// Result of the most recent [`Effect::RollDice`](super::Effect::RollDice)
    /// executed earlier in this resolution (CR 706.2).
    LastDieRoll,
    /// Number of coin flips won by the controller so far in this resolution
    /// (CR 705.2).
    CoinsWonThisResolution,
    /// Number of distinct opponents you declared at least one attacking
    /// creature against during this combat (CR 702.121a: melee, Adriana).
    OpponentsYouAttackedThisCombat,
    /// Total damage dealt to your opponents this turn (Bloodthirst X,
    /// CR 702.54b — strictly damage dealt, not non-damage life loss).
    /// Appended to preserve replay identities.
    DamageDealtToOpponentsThisTurn,
    /// Amount of snow mana (mana produced by snow sources, CR 107.4h) spent to
    /// cast this spell or the spell that became this permanent (or the source
    /// of this triggered ability, CR 400.7d, CR 608.2h). Zero when not cast or
    /// when copied. Appended to preserve replay identities.
    SnowManaSpentToCast,
    /// Number of permanents sacrificed as this permanent entered the
    /// battlefield via [`crate::EnterModifier::Devour`] or
    /// [`crate::EnterModifier::SacrificeForPowerToughness`] ("each creature it
    /// devoured", CR 702.82b, CR 608.2h). Appended to preserve replay
    /// identities.
    EntrySacrifices,
    /// Number of spells warped by players matching the relation this turn
    /// (CR 702.185c). Appended to preserve replay identities.
    SpellsWarpedThisTurn(super::PlayerRel),
    /// Cumulative mana spent to cast spells this turn by players matching the
    /// relation (expend, CR 700.14). Appended to preserve replay identities.
    ManaExpendedOnSpellsThisTurn(super::PlayerRel),
    /// Number of distinct opponents currently being attacked directly by
    /// creatures you control (CR 506.3–4, CR 508.4).
    OpponentsYouAreAttacking,
    /// Evaluates `then` when `condition` holds, otherwise `otherwise`
    /// (`"deals 2 damage … if this spell's additional cost was paid, it deals 4
    /// damage instead"`). Appended to preserve replay identities.
    IfCondition {
        /// Condition tested when the amount is evaluated.
        condition: &'static crate::Condition,
        /// Amount evaluated when `condition` holds.
        then: &'static Amount,
        /// Amount evaluated when `condition` does not hold.
        otherwise: &'static Amount,
    },
    /// Greatest power among objects matching `filter` (`0` if none match or
    /// their greatest power is negative, `CR 107.1b`; scans the battlefield by
    /// default or all zones when `filter` reaches non-battlefield zones;
    /// *Bouncer's Beatdown*, *Selvala, Heart of the Wilds*). Appended to
    /// preserve replay identities.
    GreatestPowerAmong(&'static Filter),
    /// Greatest mana value among objects matching `filter` (`0` if none match;
    /// scans the battlefield by default or all zones when `filter` reaches
    /// non-battlefield zones; *Shadow of Mortality*, *Yuriko*-likes). Appended
    /// to preserve replay identities.
    GreatestManaValueAmong(&'static Filter),
    /// Devotion of player(s) matching `who` to `colors` (`CR 700.5`): number of
    /// mana symbols in the mana costs of permanents controlled by `who` that
    /// include at least one color in `colors` (each symbol counted once even
    /// when a hybrid symbol matches two colors in `colors`; nonmodal back
    /// faces / tokens without copied mana costs contribute 0; e.g. *Daybreak
    /// Chimera*). Appended to preserve replay identities.
    Devotion {
        /// Whose permanents are counted.
        who: super::PlayerRel,
        /// Color(s) included in the devotion count.
        colors: baylee_core::color::ColorSet,
    },
    /// Number of creatures in the party of `who` (up to one each of Cleric,
    /// Rogue, Warrior, and Wizard, `0..=4`, `CR 700.8a–c`; e.g. *Deadly
    /// Alliance*). Appended to preserve replay identities.
    PartySize(super::PlayerRel),
    /// Total spells cast this turn by players matching `who` (e.g.
    /// `PlayerRel::Opponent` / `PlayerRel::EachOpponent` for *Delightful
    /// Discovery*). Appended to preserve replay identities.
    SpellsCastThisTurn(super::PlayerRel),
    /// Total cards drawn this turn by players matching `who` (e.g.
    /// `PlayerRel::You` for *Deem Inferior*). Appended to preserve replay
    /// identities.
    CardsDrawnThisTurn(super::PlayerRel),
    /// Sum of projected powers of permanents on the battlefield matching
    /// `filter`, floored at `0` (`CR 107.1b`; negative powers reduce the sum
    /// before flooring at `0`, e.g. *Ghalta, Primal Hunger*). Appended to
    /// preserve replay identities.
    TotalPowerAmong(&'static Filter),
    /// Total toughness of permanents sacrificed during this resolution or as
    /// part of paying this spell's or ability's cost, as they last existed on
    /// the battlefield (CR 608.2h), floored at 0 (CR 107.1b). Appended to
    /// preserve replay identities.
    SacrificedTotalToughness,
    /// Total mana value of permanents sacrificed during this resolution or as
    /// part of paying this spell's or ability's cost, as they last existed on
    /// the battlefield (CR 608.2h). Appended to preserve replay identities.
    SacrificedTotalManaValue,
    /// Number of distinct colors among permanents sacrificed during this
    /// resolution or as part of paying this spell's or ability's cost, as they
    /// last existed on the battlefield (CR 608.2h; *Bound*). Appended to
    /// preserve replay identities.
    SacrificedDistinctColors,
    /// Number most recently chosen (and paid, when `pay_cost.is_some()`) by
    /// [`super::Effect::ChooseNumberEx`] earlier in the current resolution
    /// (`CR 107.1`, `CR 608.2d`, `CR 608.2h`; *Galvanic Discharge*, *Aether Spike*,
    /// *Aether Refinery*, *Die Young*, *By Invitation Only*, *Expel the
    /// Interlopers*, *Choice of Damnations*). Evaluates to `0` outside an active
    /// resolution or before a number is chosen. Appended to preserve replay
    /// identities.
    ResolutionChosenNumber,
    /// Objects that moved into zone `to` this turn (from zone `from`, or from
    /// anywhere including token creation when `None`), each counted once per
    /// move and matched against `filter` **as it was** (CR 400.7, CR 608.2h):
    /// an arrival on the battlefield as it entered, with every co-arrival
    /// present (CR 603.6a), and a departure from the battlefield as it last
    /// existed there. Departed objects still count.
    ///
    /// `owner_or_ctrl` names whose moves count: the player the object
    /// entered under for a battlefield arrival, and the zone's owner (the
    /// card's owner) for every other destination — celebration's "two or
    /// more nonland permanents entered the battlefield under your control"
    /// (Armory Mice), Asmira's "each creature put into your graveyard from
    /// the battlefield this turn". `exclude_source` leaves out the ability's
    /// own source as the object it is now ("another"). Appended to preserve
    /// replay identities.
    EnteredThisTurn {
        /// What the object was as it moved.
        filter: &'static Filter,
        /// The zone it left, or `None` for any (including being created).
        from: Option<crate::ZoneRef>,
        /// The zone it entered.
        to: crate::ZoneRef,
        /// Controller (battlefield) or owner (other zones) relative to "you".
        owner_or_ctrl: super::PlayerRel,
        /// Leave out the ability's source as the object it is now.
        exclude_source: bool,
    },
    /// Spells cast this turn by players matching `caster` that matched
    /// `filter` as they were cast (CR 601.2i), countered or not: Arclight
    /// Phoenix's "three or more instant and sorcery spells this turn".
    /// Copies are not cast (CR 707.10). Appended to preserve replay
    /// identities.
    SpellsCastMatchingThisTurn {
        /// What the spell was as it became cast.
        filter: &'static Filter,
        /// Who cast it, relative to "you".
        caster: super::PlayerRel,
    },
    /// Distinct card types (CR 300.1, CR 205.2a) among spells players
    /// matching the relation cast this turn, as they were cast (April
    /// O'Neil). Supertypes and subtypes are not card types. Appended to
    /// preserve replay identities.
    DistinctCardTypesCastThisTurn(super::PlayerRel),
    /// Distinct creatures declared as attackers this turn (CR 508.1, so a
    /// creature put onto the battlefield attacking is not one, CR 508.4) by
    /// players matching `controller`, matched against `filter` as each is
    /// now, or as it last existed on the battlefield if it has left
    /// (CR 608.2h). Appended to preserve replay identities.
    CreaturesAttackedThisTurn {
        /// What the attacker is (or last was).
        filter: &'static Filter,
        /// The attacking player, relative to "you".
        controller: super::PlayerRel,
    },
    /// Number of different mana values (CR 202.3) among objects in the zone
    /// matching the filter ("for each different mana value among cards in
    /// your graveyard"). Zero is a mana value like any other. Appended to
    /// preserve replay identities.
    DistinctManaValuesInZone {
        /// Where to look.
        zone: ZoneSel,
        /// What to count.
        filter: &'static Filter,
    },
    /// Number of different kinds of counters on the ability's own source
    /// (CR 122.1; "for each kind of counter on it"), read as
    /// [`Self::CountersOnSource`] reads its source. Appended to preserve
    /// replay identities.
    DistinctCounterKindsOnSource,
    /// The greatest number of cards in hand among "your" opponents still in
    /// the game (CR 102.2, CR 800.4a), zero without one. Appended to preserve
    /// replay identities.
    HighestOpponentHandSize,
    /// Number of objects remembered by the innermost enclosing
    /// [`super::Effect::CaptureAffected`] ("that many": Arjun, the Shifting
    /// Flame draws as many cards as it put on the bottom). Counts objects that
    /// have since ceased to exist (a sacrificed token was still sacrificed).
    /// `0` outside a capture's `then`. Appended to preserve replay identities.
    RememberedCount,
    /// Total mana value of the remembered objects, each as it last existed
    /// before the capturing instruction moved it (`CR 608.2h`). Appended to
    /// preserve replay identities.
    RememberedManaValue,
    /// Total power of the remembered objects as they last existed
    /// (`CR 608.2h`), floored at 0 (`CR 107.1b`). Appended to preserve replay
    /// identities.
    RememberedPower,
    /// Total toughness of the remembered objects as they last existed
    /// (`CR 608.2h`), floored at 0 (`CR 107.1b`). Appended to preserve replay
    /// identities.
    RememberedToughness,
    /// Power of the permanent beheld or card revealed from hand to pay this
    /// spell's or ability's cost (`CR 608.2h`, `CR 107.1b`; *Dragon's Fire*,
    /// *Monstrous Emergence*, *Charge of the Forever-Beast*). Appended to
    /// preserve replay identities.
    BeheldOrRevealedPower,
    /// Mana value of the permanent beheld or card revealed from hand to pay this
    /// spell's or ability's cost (`CR 608.2h`, `CR 202.3`; *Disaster Radius*,
    /// *Induce Despair*, *Living Destiny*). Appended to preserve replay
    /// identities.
    BeheldOrRevealedManaValue,
}

/// The recipient [`Amount::DamageThisTurn`] counts.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum DamageTo {
    /// The ability's controller.
    You,
    /// The ability's source, as the permanent it last was on the
    /// battlefield (a dies trigger counts the damage the creature took
    /// before it died).
    Source,
    /// Any player or permanent.
    Anything,
}

/// What a resolution has done, as [`Amount::ThisWay`] counts it.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum ThisWay {
    /// Damage the resolving spell's or ability's source dealt, after
    /// prevention (damage prevented was never dealt, CR 615.1): Syphon
    /// Soul's "you gain life equal to the damage dealt this way".
    DamageDealt,
    /// The part of that damage dealt to the ability's controller: Mishra's
    /// War Machine's "if it deals damage to you this way".
    DamageDealtToYou,
    /// Permanents put into a graveyard from the battlefield (CR 700.4,
    /// "dies") that matched the filter as they last existed there
    /// (CR 608.2h): Hellfire's "creatures that died this way". A creature
    /// that was regenerated, or went to exile instead, did not die.
    Died(&'static Filter),
    /// Permanents destroyed by this resolution that matched the filter as they
    /// last existed on the battlefield. A replacement changing the destination
    /// (including exile) still counts; regeneration and indestructibility do not.
    /// Sacrifice and direct exile are not destruction.
    Destroyed(&'static Filter),
    /// Permanents exiled from the battlefield that matched the filter as they
    /// last existed there (CR 608.2h): Martyr's Cry's "for each creature
    /// exiled this way, its controller draws a card".
    ExiledFromBattlefield(&'static Filter),
    /// Permanents returned from the battlefield to a hand that matched the
    /// filter as they last existed there: Arid Archway's "if another Desert
    /// was returned this way" (`Filter::Another` excludes the source).
    ReturnedToHand(&'static Filter),
    /// Cards discarded (CR 701.9): Recall's "for each card discarded this
    /// way".
    Discarded,
    /// Cards discarded this way that match the filter, read in the graveyard.
    DiscardedMatching(&'static crate::Filter),
    /// Cards moved from a library to exile earlier in this resolution that
    /// match the filter as they exist in exile.
    ExiledFromLibrary(&'static crate::Filter),
    /// Counters of this kind removed from the resolving source earlier in
    /// this resolution, including only actual reductions.
    CountersRemoved(crate::CounterKind),
    /// Counters of every kind actually removed from this resolution's object
    /// or player targets, including counters held by players.
    CountersRemovedFromTargets,
    /// Cards milled (CR 701.17a: put from a library into a graveyard) that
    /// match the filter as they are in the graveyard: Grist, the Hunger
    /// Tide's "if an Insect card was milled this way".
    Milled(&'static Filter),
    /// Cards put from a hand into a library: Winds of Change's shuffle,
    /// Valakut Awakening's "put any number of cards from your hand on the
    /// bottom of your library, then draw that many cards plus one". A card
    /// a replacement put somewhere else (a commander to the command zone,
    /// CR 903.9b) was not put there.
    PutIntoLibraryFromHand,
    /// Permanents whose controller changed (Gauntlets of Chaos: "if those
    /// permanents are exchanged this way" is two).
    ControlChanged,
    /// Cards revealed (CR 701.20) that match the filter: Wand of Ith's
    /// "if it's a land card".
    Revealed(&'static Filter),
    /// Total mana value of cards revealed this way: Wand of Ith's
    /// "unless they pay life equal to its mana value".
    RevealedManaValue,
    /// Total life actually lost by players earlier in this resolution
    /// (extort's "you gain life equal to the total life lost this way",
    /// CR 702.101a). Appended to preserve replay identities.
    LifeLost,
    /// Cards moved from a graveyard to exile during this resolution that
    /// match the filter in exile.
    ExiledFromGraveyard(&'static Filter),
    /// Cards or permanents moved from any zone to exile during this resolution
    /// that match the filter (as they last existed on the battlefield when
    /// exiled from the battlefield, CR 608.2h, or as they exist in exile when
    /// exiled from any other zone). Appended to preserve replay identities.
    Exiled(&'static Filter),
    /// Number of votes cast for option `index` in the most recent
    /// [`crate::Effect::Vote`] of this resolution (Council's dilemma,
    /// CR 701.38). Appended to preserve replay identities.
    VoteCount(u8),
    /// Excess damage dealt to permanents (creatures, planeswalkers, and/or
    /// battles) earlier in this resolution (CR 120.10), summed across all
    /// excess-damage events caused by this resolution's source or instructed
    /// damage sources. Appended to preserve replay identities.
    ExcessDamage,
    /// Total damage prevented or replaced by the enclosing damage prevention /
    /// replacement-with-rider shield or static modifier (CR 615.5, CR 614.1a;
    /// *Awe Strike*, *Gloom Surgeon*, *Force Bubble*). Bound inside the `then`
    /// slice of [`crate::Effect::PreventNextDamageWithThen`] and
    /// [`crate::Modifier::ReplaceOrPreventDamageWithThen`]. Appended to
    /// preserve replay identities.
    DamagePrevented,
    /// Number of permanents sacrificed earlier in this resolution (CR 701.21a),
    /// including permanents sent elsewhere by a replacement effect (such as
    /// exile under Rest in Peace). Appended to preserve replay identities.
    SacrificedCount,
    /// Power of the permanent chosen or card revealed from hand to pay a
    /// [`crate::CostPart::Behold`] or [`crate::CostPart::RevealFromHand`] cost
    /// of this spell or ability (`CR 608.2h`; *Dragon's Fire*). Uses current
    /// projected power if a chosen permanent is still that same incarnation on
    /// the battlefield, otherwise its LKI/card power when chosen or revealed.
    /// Appended to preserve replay identities.
    BeheldOrRevealedPower,
}

impl Amount {
    /// Construct a division, rejecting an invalid divisor before evaluation.
    ///
    /// # Panics
    /// Panics if `divisor` is zero. Static card definitions validate at compile time.
    #[must_use]
    pub const fn divided(base: &'static Self, divisor: u32, round_up: bool) -> Self {
        Self::Divided {
            base,
            divisor: core::num::NonZeroU32::new(divisor).expect("amount divisor must be nonzero"),
            round_up,
        }
    }

    /// Whether this amount counts **downwards**.
    ///
    /// The one place the question is answered, because it was four places
    /// before: `resolve::counters` spelled `matches!(a, Amount::NegX |
    /// Amount::NegXFixed(_))` in three separate closures and `baylee-ai`'s
    /// `tactics` had a fourth arm of its own. Four positive lists over an
    /// enum is four chances for the next negative amount to be read as a
    /// bonus, and nothing in a test suite reads a sign as a bug: the card
    /// resolves, the creature changes size, and only the direction is wrong.
    ///
    /// Nesting is answered by parity rather than refused, because that is
    /// what the word means. No card prints a double negative.
    #[must_use]
    pub const fn is_negative(&self) -> bool {
        match self {
            Self::NegX | Self::NegXFixed(_) => true,
            Self::Negated(inner) => !inner.is_negative(),
            Self::IfCondition {
                then, otherwise, ..
            } => then.is_negative() && otherwise.is_negative(),
            _ => false,
        }
    }
}
