//! Which printed sentence an ability came from.
//!
//! `docs/client.md` says the `//! Oracle:` header lines are "ordered to
//! match the ability list", and the stack-text feature rests on that being
//! true: an ability that knows its sentence can be shown in the player's
//! own language as what it *does*, instead of as "+1".
//!
//! This is the one reader of that claim, and it lives here because three
//! callers will need the same answer — codegen is to write the table and
//! `validate` to check it; `xtask ability-lines` measures it today. Two
//! readings that drifted would be a table nothing was holding.
//!
//! It maps **one face**. A card's abilities are per face by
//! `CardDef::abilities_for_face` — a back face never inherits the front's
//! — and a client renders one face's text at a time, so an index into a
//! string holding both faces' sentences would point past the end of the
//! one being read.
//!
//! It matches on **shape and content**, and the second half is not
//! optional. Shape alone cannot tell a walker's `+1` from its `−3` or a
//! `Dies` trigger from an `Enters` one, so a card whose abilities are in
//! each other's places walks in order and reports as aligned — an upper
//! bound wearing a count's clothes.

use baylee_cards_dsl::{AbilityDef, AltCondition, AlternativeCost, CostPart, StepKind, Trigger};

/// The sentences a printed oracle text is made of.
///
/// Re-exported rather than written here: the client resolves the index
/// this module hands out against a *localized* printed text and cannot
/// reach this crate, so the split belongs to `baylee-core`, which both
/// ends depend on.
pub use baylee_core::oracle::sentences;

mod grants;
use grants::granted_cost;

mod modes;
pub use modes::map_modes;

mod shape;
pub use shape::line_shape;

/// What shape an oracle sentence or an ability is, coarsely enough that the
/// two can be held against each other.
///
/// The point is not to understand the sentence — it is to say which of the
/// five *kinds* of line a card prints it is, because that is all a
/// per-ability line index needs in order to be checkable.
#[derive(Clone, Copy, PartialEq, Eq, Debug, PartialOrd, Ord)]
pub enum LineShape {
    /// `+1:`, `0:`, `−3:` — a planeswalker's loyalty ability.
    Loyalty,
    /// "When…", "Whenever…", "At…" — a triggered ability.
    Triggered,
    /// A cost, a colon, an effect — an activated ability.
    Activated,
    /// A saga chapter: "I —", "II, III —".
    Chapter,
    /// `{T}: Add {G}.` — a mana ability, on both sides.
    ///
    /// It was `Other`, which is the right answer to the question this
    /// module was first asked ("which sentence goes on the stack") and the
    /// wrong one to the question it is asked now. A mana ability never uses
    /// the stack (CR 605.1), so it is **not** counted in `FaceLines::stackable`
    /// and never will be — but it *is* drawn, on the ability sheet, where
    /// the owner asked to read the card's own sentence rather than a label
    /// this client composed. A shape of its own is what lets both be true:
    /// excluded from the denominator, and still placed.
    ///
    /// It has to be said on **both** sides or the exclusion is only half
    /// applied — Karakas prints two `{T}:` lines and only one of them is
    /// the mana, so its bounce ability fitted both and took whichever came
    /// first.
    Mana,
    /// `Lands you control have "{T}: Add one mana of any color."` — a
    /// sentence that grants an activated ability, which it prints in
    /// quotation marks (CR 113.10a), on both sides.
    ///
    /// Its own shape so a player can be told whose sentence a granted
    /// ability is (#212). The land under a Chromatic Lantern prints nothing
    /// about the `{T}` it was given; the Lantern does. As [`Self::Other`]
    /// the static had no line and the row read "Granted ability".
    ///
    /// Not every grant: one written inside an ability that already has a
    /// sentence of its own shape, such as Urza's Saga's chapters or Spawning
    /// Pool's `{1}{B}:`, is found under that shape. This is for abilities
    /// otherwise classified as [`Self::Other`]: statics, copy clauses
    /// (`…except it has "{T}: Add {U}."`) and spell effects.
    ///
    /// A static never uses the stack, so it is not counted in
    /// [`Self::stackable`].
    Grant,
    /// Anything else a card prints: a static ability, a keyword line, the
    /// body of an instant or sorcery.
    Other,
}

impl LineShape {
    /// Whether an ability of this shape can be a stack entry at all.
    ///
    /// The denominator of the whole feature, in one place instead of three
    /// `!= LineShape::Other` filters that would each have had to learn
    /// about [`LineShape::Mana`] separately.
    #[must_use]
    pub fn stackable(self) -> bool {
        !matches!(self, Self::Other | Self::Mana | Self::Grant)
    }
}

/// A printed line with its reminder text taken off.
///
/// It is `pub` because `xtask validate` wants the same thing over a whole
/// oracle text, and two implementations of "take the brackets out" are two
/// things that can disagree about a card.
///
/// Reminder text is the card explaining itself to a player, and it holds a
/// whole second sentence inside brackets — cycling prints
/// `Cycling {B} ({B}, Discard this card: Draw a card.)`, whose bracketed
/// half carries a colon and a second `{B}`. Reading the line whole made
/// forty cycling lands look like activated abilities with the wrong cost,
/// which is a fault in the reader and not a finding about the pool.
#[must_use]
pub fn without_reminder(line: &str) -> String {
    let mut out = String::new();
    let mut depth = 0usize;
    for c in line.chars() {
        match c {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            _ if depth == 0 => out.push(c),
            _ => {}
        }
    }
    out.trim().to_string()
}

/// Where this line's **own** ability puts its colon, if it has one.
///
/// CR 113.10a: "An effect that adds an activated ability may include
/// activation instructions for that ability. These instructions become part
/// of the ability that's added to the object." A sentence that prints an
/// ability inside quotation marks is therefore quoting a cost and an effect
/// belonging to whatever it grants, never to the object printing them —
/// Chromatic Lantern's `Lands you control have "{T}: Add one mana of any
/// color."` is a static ability, and that colon is the lands'.
///
/// Read whole, that sentence answered [`LineShape::Mana`], and the
/// lantern's own mana ability then claimed it — one sentence *before* its
/// own, which is the direction that makes it invisible: the walk in [`map`]
/// goes forwards and never looked at the right line at all. Every card that
/// grants a quoted ability is a candidate, and thirteen of this pool's
/// printed sentences put a colon inside quotation marks.
///
/// The quotation mark is ASCII `"` and the toggle is symmetric because that
/// is what the pool prints: of its 52 oracle sentences carrying one, none
/// has an odd number of them and none prints a curly quote. A directional
/// pair needs a different state machine, and one is not invented here for a
/// spelling no printing uses.
fn ability_colon(line: &str) -> Option<usize> {
    let mut quoted = false;
    for (i, c) in line.char_indices() {
        match c {
            '"' => quoted = !quoted,
            ':' if !quoted => return Some(i),
            _ => {}
        }
    }
    None
}

/// Whether the line is "Crew N" (CR 702.122a) or "Saddle N" (CR 702.171a)
/// and nothing else.
///
/// Both are activated abilities whose cost is tapping creatures, so the line
/// carries no `{`, and a printing that drops the reminder leaves no colon for
/// [`reminder_spells_out_an_ability`] to find either: Unlicensed Hearse
/// prints "Crew 2" and not a word more, and its crew ability had no
/// sentence.
fn a_numbered_tapping_keyword(line: &str) -> bool {
    ["Crew ", "Saddle "].iter().any(|keyword| {
        line.strip_prefix(keyword)
            .is_some_and(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()))
    })
}

/// A type-changing effect may grant crew without quoting an activation.
fn granted_crew_number(line: &str) -> Option<u8> {
    let lower = line.to_ascii_lowercase();
    let (_, tail) = lower.split_once(" with crew ")?;
    tail.split(|c: char| !c.is_ascii_digit())
        .next()?
        .parse()
        .ok()
}

/// Whether a keyword line's own reminder text writes out an activated
/// ability.
///
/// The second half of the branch above, and the reason it is needed: a
/// keyword whose cost is mana prints that cost on the line — `Equip {2}`,
/// `Cycling {1}{B}` — and is found by the `{`. A keyword whose cost is
/// something else prints nothing but the word, and `Station` is the pool's
/// one example: `Station (Tap another creature you control: Put charge
/// counters equal to its power on this Spacecraft. …)`. Read as
/// [`LineShape::Other`] it was a card whose only activated ability had no
/// sentence at all, and the client drew the row as "Ability 1".
///
/// The colon is what separates the two kinds of keyword, and it is the
/// card's own punctuation rather than a guess: a keyword that *is* an
/// activated ability spells the ability out in its reminder, cost and colon
/// and all, while a keyword that is a bit describes a rule —
/// `Flying (This creature can't be blocked except by creatures with flying
/// or reach.)` has no colon anywhere in it.
fn reminder_spells_out_an_ability(printed: &str) -> bool {
    let Some((_, rest)) = printed.split_once('(') else {
        return false;
    };
    rest.split_once(')')
        .map_or(rest, |(inside, _)| inside)
        .contains(':')
}

/// The shape of one compiled ability.
///
/// A **mana ability is [`LineShape::Mana`]**, which is not the stack entry
/// this table was first built for (CR 605.1) and is still a sentence a
/// player reads. It used to be `Other`, and a mana ability therefore had no
/// printed text at all — the reason the ability sheet composed its own
/// label for one. A shape of its own places it while
/// [`LineShape::stackable`] keeps it out of the count.
///
/// A land with no printed mana line at all — a Bayou, whose type line
/// entitles it to mana by CR 305.6 and whose only text is the reminder —
/// has no `AbilityDef` here either, so there is nothing to place and this
/// never sees it.
pub fn ability_shape(ability: &baylee_cards_dsl::AbilityDef) -> LineShape {
    use baylee_cards_dsl::AbilityDef as A;
    match ability {
        A::Loyalty { .. } => LineShape::Loyalty,
        A::ModalActivated { .. } => {
            if ability.is_mana_ability() {
                LineShape::Mana
            } else {
                LineShape::Activated
            }
        }
        // The one trigger that is not a stack entry (CR 605.1b, 605.4a).
        A::Triggered { .. } if ability.is_triggered_mana_ability() => LineShape::Mana,
        A::Triggered { .. } | A::ModalTriggered { .. } | A::Echo { .. } | A::Ward { .. } => {
            LineShape::Triggered
        }
        A::Activated {
            cost,
            effects,
            targets,
            second_targets,
            mana_ability,
            ..
        }
        | A::ActivatedConditional {
            cost,
            effects,
            targets,
            second_targets,
            mana_ability,
            ..
        } => {
            if *mana_ability
                && baylee_cards_dsl::mana_rule::activated_mana_ability(
                    cost,
                    effects,
                    targets.is_some() || second_targets.is_some(),
                )
            {
                LineShape::Mana
            } else {
                LineShape::Activated
            }
        }
        A::SagaChapter { .. } => LineShape::Chapter,
        _ if granted_cost(ability).is_some() => LineShape::Grant,
        _ => LineShape::Other,
    }
}

/// What a sentence prints inside its first pair of quotation marks.
///
/// The ASCII `"` for [`ability_colon`]'s reason: it is what the pool prints.
fn quoted(line: &str) -> Option<&str> {
    let (_, rest) = line.split_once('"')?;
    rest.split_once('"').map(|(inside, _)| inside)
}

/// The loyalty cost a line prints, if it prints one.
///
/// The minus a card prints is U+2212 and the plus is written out, so `+1`,
/// `0` and `−3` are three different heads. This is the whole reason a
/// content check exists: all four of Jace, the Mind Sculptor's abilities
/// are `LineShape::Loyalty`, and only the number says which sentence is
/// which. A matcher that reads shape alone walks a swapped pair in order
/// and calls it aligned.
pub fn loyalty_head(line: &str) -> Option<i8> {
    let line = without_reminder(line);
    let (head, _) = line.split_once(':')?;
    head.trim().replace('\u{2212}', "-").parse().ok()
}

/// A roman chapter numeral, as a saga prints it.
pub fn roman(src: &str) -> Option<u8> {
    match src {
        "I" => Some(1),
        "II" => Some(2),
        "III" => Some(3),
        "IV" => Some(4),
        "V" => Some(5),
        _ => None,
    }
}

/// The chapters one saga line is the text of — several, because a saga
/// prints "I, II —" when two chapters do the same thing.
pub fn chapter_head(line: &str) -> Option<Vec<u8>> {
    let line = without_reminder(line);
    let (head, _) = line.split_once(" — ")?;
    head.split(',').map(|part| roman(part.trim())).collect()
}

/// The `{…}` symbols a string prints, in order.
pub fn braced(src: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = src;
    while let Some(open) = rest.find('{') {
        let Some(close) = rest[open..].find('}') else {
            break;
        };
        out.push(rest[open + 1..open + close].to_string());
        rest = &rest[open + close + 1..];
    }
    out
}

/// Does a compiled activation cost fit the head of a printed line?
///
/// Only the part of a cost that is written in symbols is compared — the
/// mana, the tap and the untap — because "Sacrifice a creature" is prose a
/// filter cannot be held against cheaply. That is enough for what this is
/// for: a card with two activated abilities almost always prints two
/// different costs, and `{T}` against `{2}{U}, {T}` separates them.
pub fn cost_fits(cost: &baylee_cards_dsl::Cost, line: &str) -> bool {
    use baylee_cards_dsl::CostPart;
    let line = without_reminder(line);
    // A keyword ability prints its cost with no colon after it — "Cycling
    // {B}" is a whole line — so a line without one is read as all cost.
    let head = line.split_once(':').map_or(line.as_str(), |(head, _)| head);
    let printed = braced(head);
    let taps = printed.iter().any(|s| s == "T");
    let untaps = printed.iter().any(|s| s == "Q");
    if taps != cost.parts.contains(&CostPart::TapSelf) {
        return false;
    }
    if untaps != cost.parts.contains(&CostPart::UntapSelf) {
        return false;
    }
    // Exert has no symbol, but must distinguish otherwise identical
    // activations so the client preserves this additional payment.
    if head.to_lowercase().contains("exert") != cost.parts.contains(&CostPart::ExertSelf) {
        return false;
    }
    // Energy symbols belong to nonmana payments, not the mana cost.
    let mut want = braced(&cost.mana.to_string());
    want.retain(|s| s != "0");
    want.sort();
    for alt_head in head.split(" or ") {
        let mut alt_printed = braced(alt_head);
        alt_printed.retain(|s| s != "T" && s != "Q" && s != "E" && s != "0");
        alt_printed.sort();
        if alt_printed == want {
            return true;
        }
    }
    false
}

/// Does what a mana ability *makes* fit the body of a printed line?
///
/// [`cost_fits`] is not enough on its own here, and Yavimaya Coast is why:
/// it prints `{T}: Add {C}.` and `{T}: Add {G} or {U}. …`, two lines whose
/// costs are the same symbol. Reading the colours after "Add" separates
/// them, which is the same job [`loyalty_head`] does for a walker.
///
/// It compares **sets**, because the amount is written in words as often as
/// in symbols ("Add two mana", "Add {C}{C}"), and it **accepts a body with
/// no symbols in it at all** — "Add one mana of any color", "Add X mana of
/// any one color" — where there is nothing to compare and the cost is the
/// only handle left. A card printing two such lines would be `ambiguous`,
/// which is the number that says so rather than a silent coin toss.
fn mana_fits(effects: &[baylee_cards_dsl::Effect], line: &str) -> bool {
    use baylee_cards_dsl::effect::{Effect, ManaSource};
    let line = without_reminder(line);
    let Some((_, body)) = line.split_once(':') else {
        return true;
    };
    let mut printed: Vec<String> = braced(body);
    if printed.is_empty() {
        return true;
    }
    printed.sort();
    printed.dedup();
    let mut made: Vec<String> = Vec::new();
    for effect in effects {
        let source = match effect {
            Effect::AddMana { source, .. } => *source,
            Effect::AddManaFor { color, .. } => ManaSource::Fixed(*color),
            _ => continue,
        };
        match source {
            ManaSource::Fixed(color) => made.push(symbol(color).to_string()),
            ManaSource::Choice(colors) => {
                made.extend(colors.iter().map(|c| symbol(*c).to_string()));
            }
            // None of these names a colour the card could print, so none
            // has anything to be held against — `{T}: Add one mana of any
            // color in your commander's color identity` carries no symbol
            // either, and neither does "one mana of the chosen color".
            // `ChosenOr` prints the {W} beside it, but the line as a whole
            // is still half unprintable, so it is let through with them.
            ManaSource::IntrinsicBasicLandTypes
            | ManaSource::CommanderIdentity
            | ManaSource::ColorsAmong { .. }
            | ManaSource::CircledColors
            | ManaSource::ExiledWithSource
            | ManaSource::LandColor { .. }
            | ManaSource::Chosen
            | ManaSource::ChosenOr(_) => return true,
        }
    }
    if made.is_empty() {
        return true;
    }
    made.sort();
    made.dedup();
    // "{T}: Add {U}. If you played a land this turn, add {B} instead." is
    // one sentence printing two outputs of which the ability makes one at a
    // time, so what it makes has to be *among* what is printed rather than
    // all of it. River of Tears found it: its ability makes {U}, and a
    // reader wanting both symbols left that ability with no sentence.
    if line.to_lowercase().contains(" instead") {
        return made.iter().all(|m| printed.contains(m));
    }
    if printed == made {
        return true;
    }
    // "{T}: Add {U} or {C}{U}." prints two outputs of different symbol counts
    // in one clause, which compiles to one mana ability per arm rather than a
    // single `ManaSource::Choice` (whose arms all share one `Amount`).
    let first_sentence = body.split('.').next().unwrap_or(body);
    let arms: Vec<Vec<String>> = first_sentence
        .split(',')
        .flat_map(|part| part.split(" or "))
        .map(braced)
        .filter(|arm| !arm.is_empty())
        .collect();
    if arms.len() > 1 && arms.iter().any(|arm| arm.len() != arms[0].len()) {
        return arms.into_iter().any(|mut arm| {
            arm.sort();
            arm.dedup();
            arm == made
        });
    }
    false
}

/// The letter a colour is printed as inside braces.
fn symbol(color: baylee_core::mana::ManaColor) -> &'static str {
    use baylee_core::mana::ManaColor as C;
    match color {
        C::White => "W",
        C::Blue => "U",
        C::Black => "B",
        C::Red => "R",
        C::Green => "G",
        C::Colorless => "C",
    }
}

/// Words the printed sentence must carry for a trigger to have come from it.
///
/// Coarse on purpose. It is not parsing the sentence, it is refusing the
/// swap: two triggered abilities on one card print two sentences, and
/// "enter" against "dies" is all it takes to say which is which. An empty
/// slice would be an honest "this variant makes no claim"; none of them
/// needs one yet.
///
/// The words are **stems**, because a card names its own subject and the
/// verb agrees with it: Sun Titan prints "Whenever this creature enters or
/// attacks" and Aang and Katara print "Whenever Aang and Katara enter or
/// attack". Matching "attacks" reported the second of those as an ability
/// with no sentence, which is the reader being wrong about the pool.
///
/// `EntersBattlefieldEvoked` is the one that must **not** say "enter".
/// Evoke's sacrifice is printed as the keyword line "Evoke {2}{U}" and not
/// as a sentence at all, so a trigger that accepted an "enters" line took
/// Mulldrifter's *other* ETB — the draw — and the card was reported as
/// aligned while its table pointed two abilities at one sentence and left
/// the real one unclaimed. Reveillark is what showed it: its two triggers
/// are "leaves" and evoke, nothing says "enters", and it was the only one
/// of the three evoke cards the old reading could not quietly mismatch.
pub fn trigger_words(trigger: &baylee_cards_dsl::Trigger) -> &'static [&'static str] {
    use baylee_cards_dsl::Trigger as T;
    match trigger {
        T::EntersBattlefield(_) => &["enter"],
        T::EntersBattlefieldEvoked => &["evoke"],
        T::CycledThis => &["cycle"],
        T::EnteredThisFrom(_) => &["enters"],
        T::Discarded { .. } | T::DiscardedThisByOpponent => &["discard"],
        T::LeavesBattlefield(_) | T::LinkedCreatureLeaves => &["leave"],
        T::Dies(_) | T::DiesNotSacrificed(_) | T::DiesAfterDamageByThis(_) => {
            &["die", "put into a graveyard"]
        }
        T::CastThisWithMana(_)
        | T::SpellCast(_)
        | T::SpellCastUsingSourceMana(_)
        | T::SpellCastOrCopied(_)
        | T::InstantCastAfterFirst { .. }
        | T::NthSpellCast { .. }
        | T::FirstNoncreatureSpellCast(_) => &["cast"],
        T::BecomesTarget | T::EnchantedBecomesTarget | T::TargetedByOpponent { .. } => {
            &["becomes the target"]
        }
        T::TurnedFaceUp => &["turned face up"],
        T::TransformsIntoThis => &["transforms into"],
        T::Mutates => &["mutates"],
        T::UnlockThisDoor(_) => &["unlock this door"],
        T::Ward => &["ward"],
        T::ExiledFromBattlefield(_) => &["exiled"],
        T::DealsCombatDamageToCreature(_)
        | T::DealsCombatDamageToPlayer(_)
        | T::DealsCombatDamageToPlayerOrPlaneswalker(_)
        | T::DealsCombatDamage(_)
        | T::DealsCombatDamageToYou(_)
        | T::OneOrMoreDealCombatDamageToPlayer(_)
        | T::DealsCombatDamageToOpponent(_)
        | T::DealsDamageToPlayer { .. }
        | T::DealsDamageToOpponent(_)
        | T::DealsDamage(_)
        | T::DealtDamage(_)
        | T::DealtCombatDamage(_) => &["damage"],
        // Living Artifact and Lich, "Whenever you're dealt damage".
        T::PlayerDealtDamage(_) => &["dealt damage"],
        T::BecomesTapped(_) => &["tap"],
        T::BecomesUntapped(_) => &["untap"],
        T::AbilityActivated { .. } => &["activates"],
        // Badgermole Cub, "Whenever you tap a creature for mana".
        T::TappedForMana { .. } => &["for mana"],
        // Druid Class, "When this Class becomes level 3".
        T::CountersPlaced { .. } => &["counter"],
        T::CountersReach { .. } => &["becomes level"],
        // Divine Intervention, "When you remove the last intervention counter".
        T::LastCounterRemoved { .. } => &["remove the last"],
        T::Draws(_) | T::DrawsExceptFirst(_) | T::NthCardDrawnInTurn { .. } => &["draw"],
        T::SearchesLibrary(_) => &["search"],
        T::GainsLife(_) => &["gain life", "gains life"],
        T::LosesLife(_) => &["lose life", "loses life"],
        // Fastbond, "Whenever you play a land".
        T::PlaysLand(_) => &["play a land"],
        T::PlaysAnotherLand => &["play another land"],
        T::Attacks(_)
        | T::OneOrMoreAttack(_)
        | T::AttackWithAtLeast(_)
        | T::AttackWithAtLeastMatching { .. } => &["attack"],
        T::AttacksAlone(_) => &["exalted", "attacks alone"],
        // Cockatrice, "Whenever this creature blocks or becomes blocked by
        // a non-Wall creature". Its filter is the *other* creature, so it
        // stays out of `whose_trigger_fits`, where a filter is the subject.
        T::Rampage => &["rampage"],
        T::BlocksOrBecomesBlockedBy(_)
        | T::BlocksOrBecomesBlockedByOneOrMore(_)
        | T::AttachedBlocksOrBecomesBlockedBy(_)
        | T::BecomesBlockedBy(_)
        | T::BecomesBlocked
        | T::BlocksCreature(_) => &["block"],
        // Ydwen Efreet, "Whenever this creature blocks".
        T::Blocks(_) => &["blocks"],
        // Merchant Ship, "Whenever this creature attacks and isn't blocked".
        T::AttacksAndIsntBlocked(_) => &["isn't blocked"],
        // The step, not the word "beginning" — every one of these sentences
        // opens with it, so on its own it says nothing and a card printing
        // two of them was a coin toss. Mana Vault prints an upkeep sentence
        // and a draw-step one, and its draw-step trigger claimed the upkeep
        // line: an ability whose effect the card prints two sentences below
        // the one the table pointed at.
        //
        // What each one prints is the template, and the words are the part
        // of it that cannot be phrased otherwise: "At the beginning of your
        // upkeep", "… of your draw step", "… of your end step", "At the
        // beginning of combat on your turn".
        T::StepBegin { step, .. } | T::StepBeginChosenOpponent { step } => match step {
            StepKind::Upkeep => &["upkeep"],
            StepKind::Draw => &["draw step"],
            StepKind::CombatBegin => &["beginning of combat"],
            StepKind::DeclareAttackers => &["declare attackers step"],
            StepKind::DeclareBlockers => &["declare blockers step"],
            StepKind::CombatDamage => &["combat damage step"],
            StepKind::CombatEnd => &["end of combat"],
            StepKind::End => &["end step"],
            StepKind::FirstMain => &["first main phase"],
        },
        // A state trigger (CR 603.8) prints the state it waits for: Sea
        // Serpent's "When you control no Islands".
        T::State(_) => &[
            "when you control no",
            "when there are no",
            "when you have no",
        ],
    }
}

/// Does the *subject* of a trigger fit the sentence?
///
/// A card whose two triggers use the same verb is where the verb alone runs
/// out, and the pool has a dozen of them: Sheoldred's "whenever **you**
/// draw" beside "whenever **an opponent** draws", Earth King's Lieutenant's
/// "when **this creature** enters" beside "whenever **another Ally** you
/// control enters". Both of each pair fit on the word, and whichever the
/// walk reached first was a coin toss printed as a hit.
///
/// The rules here **refuse** rather than require, which is the half that
/// makes them safe. Requiring a `Filter::This` trigger to say "this" would
/// break every legendary creature, which prints its own name instead
/// ("Whenever Sheoldred enters"); refusing it the sentence that says
/// "another" costs nothing and separates the pair just as well.
fn whose_trigger_fits(trigger: &Trigger, line: &str) -> bool {
    use baylee_cards_dsl::effect::PlayerRel;
    // Only the **subject clause** — everything before the first comma — is
    // read, because that is the half that says whose event this is; the
    // rest is what happens afterwards and routinely names the other player.
    // Abraded Bluffs triggers on *itself* entering and then "deals 1 damage
    // to target opponent"; reading the whole sentence refused it its own
    // line, and eight lands went with it.
    let lower = line.split_once(',').map_or(line, |(clause, _)| clause);
    let others = ["another", "opponent"];
    let itself = [
        "this creature",
        "this land",
        "this permanent",
        "this artifact",
        "this enchantment",
    ];
    // The two tests are mirrors, and both are "…and nobody else", because
    // one sentence can carry two subjects. Hagra Diabolist triggers on
    // "this creature **or** another Ally you control"; Orcish Bowmasters on
    // "when this creature enters **and** whenever an opponent draws". A
    // clause naming both belongs to both abilities, so neither test may
    // refuse it.
    let names_self = || itself.iter().any(|w| lower.contains(w));
    let names_other = || others.iter().any(|w| lower.contains(w));
    let about_someone_else = || names_other() && !names_self();
    let about_itself = || names_self() && !names_other();
    match trigger {
        // A player relation is the plainest case: the sentence names whose
        // draw or cast it is, and one of the two words is always present.
        // A step is the same case with the same words: a card prints "your
        // upkeep" or "each opponent's upkeep", never neither. Fourteen of
        // the pool's `StepBegin` triggers are `You` and one is `Opponent`,
        // so it separates nothing here today — it is written because the
        // step word beside it does, and reading only half of a trigger is
        // what let Mana Vault's draw-step ability sit on the upkeep
        // sentence.
        Trigger::Draws(rel)
        | Trigger::DrawsExceptFirst(rel)
        | Trigger::NthCardDrawnInTurn { who: rel, .. }
        | Trigger::SearchesLibrary(rel)
        | Trigger::GainsLife(rel)
        | Trigger::LosesLife(rel)
        | Trigger::PlaysLand(rel)
        | Trigger::InstantCastAfterFirst { who: rel }
        | Trigger::FirstNoncreatureSpellCast(rel)
        | Trigger::PlayerDealtDamage(rel)
        | Trigger::StepBegin { whose: rel, .. } => match rel {
            PlayerRel::You => !lower.contains("opponent"),
            PlayerRel::Opponent | PlayerRel::EachOpponent => lower.contains("opponent"),
            _ => true,
        },
        Trigger::EnteredThisFrom(_) => !about_someone_else(),
        Trigger::EntersBattlefield(filter)
        | Trigger::LeavesBattlefield(filter)
        | Trigger::Dies(filter)
        | Trigger::DiesNotSacrificed(filter)
        | Trigger::Attacks(filter)
        | Trigger::AttacksAlone(filter)
        | Trigger::OneOrMoreAttack(filter)
        | Trigger::Blocks(filter)
        | Trigger::AttacksAndIsntBlocked(filter)
        | Trigger::AbilityActivated { filter, .. }
        | Trigger::BecomesTapped(filter)
        | Trigger::TappedForMana { filter, .. }
        | Trigger::ExiledFromBattlefield(filter)
        | Trigger::DealsCombatDamageToCreature(filter)
        | Trigger::DealsCombatDamageToPlayer(filter)
        | Trigger::DealsCombatDamageToPlayerOrPlaneswalker(filter)
        | Trigger::DealsCombatDamage(filter)
        | Trigger::OneOrMoreDealCombatDamageToPlayer(filter)
        | Trigger::DealsCombatDamageToOpponent(filter)
        | Trigger::DealsDamageToPlayer { source: filter, .. }
        | Trigger::DealsDamageToOpponent(filter)
        | Trigger::DealsDamage(filter)
        | Trigger::DealtDamage(filter)
        | Trigger::DealtCombatDamage(filter)
        | Trigger::SpellCast(filter)
        | Trigger::SpellCastUsingSourceMana(filter)
        | Trigger::SpellCastOrCopied(filter) => {
            // "This" narrowed by a condition is still this object: Bogardan
            // Phoenix's two dies triggers are `This` and with and without a
            // death counter, and both are "when this creature dies".
            let names_this =
                |f: &baylee_cards_dsl::Filter| matches!(f, baylee_cards_dsl::Filter::This);
            let this = match filter {
                baylee_cards_dsl::Filter::And(parts) => parts.iter().any(names_this),
                other => names_this(other),
            };
            if this {
                !about_someone_else()
            } else {
                !about_itself()
            }
        }
        _ => true,
    }
}

/// Could this printed line be the text of this compiled ability?
///
/// Shape says a line is *a* loyalty ability; this says it is *the* `+1`.
/// Without it the measurement is an upper bound rather than a count, and a
/// table built on the upper bound points a card's abilities at each
/// other's sentences while every number in the report stays green.
pub fn content_fits(ability: &baylee_cards_dsl::AbilityDef, line: &str) -> bool {
    use baylee_cards_dsl::AbilityDef as A;
    match ability {
        A::Loyalty { cost, .. } => loyalty_head(line) == Some(*cost),
        A::Activated {
            cost,
            effects,
            mana_ability: true,
            ..
        }
        | A::ActivatedConditional {
            cost,
            effects,
            mana_ability: true,
            ..
        } => cost_fits(cost, line) && mana_fits(effects, line),
        A::Activated { cost, .. }
        | A::ActivatedConditional { cost, .. }
        | A::ModalActivated { cost, .. } => cost_fits(cost, line),
        A::SagaChapter { chapter, .. } => {
            chapter_head(line).is_some_and(|chapters| chapters.contains(chapter))
        }
        A::Triggered { trigger, .. } => {
            let lower = without_reminder(line).to_lowercase();
            let words = trigger_words(trigger);
            (words.is_empty() || words.iter().any(|w| lower.contains(w)))
                && whose_trigger_fits(trigger, &lower)
        }
        // Echo prints as the keyword line `Echo {3}{W}{W}`, which is not a
        // sentence and is not shaped like one. Saying so is what stops it
        // taking the neighbouring "When this creature enters" — which is
        // exactly what Karmic Guide's table did before this line existed.
        A::Echo { .. } => line.to_lowercase().contains("echo"),
        A::Ward { .. } => line.to_lowercase().starts_with("ward"),
        // A grant's cost is the one inside the quotation marks, which is the
        // granted ability's and printed as its head (CR 113.10a).
        A::Static(_) | A::CopyOnEnter { .. } | A::CopyOnEnterUntilEot { .. } | A::Spell { .. } => {
            match granted_cost(ability) {
                Some(cost) => {
                    quoted(line).is_some_and(|inside| cost_fits(&cost, inside))
                        || granted_crew_number(line).is_some_and(|n| {
                            cost.mana == baylee_core::mana::ManaCost::ZERO
                                && cost.parts == [CostPart::Crew(n)]
                        })
                }
                None => true,
            }
        }
        // A modal trigger's text is its modes; it carries no handle the
        // code can be held against, so it makes no claim.
        _ => true,
    }
}

/// Which sentence each ability came from.
///
/// One entry per ability in `abilities`, in step with it — `None` where an
/// ability has no sentence of its own, which is an honest answer and not a
/// failure. A static is many-to-one with the printing by design, and evoke,
/// echo and station are *printed* as keyword lines rather than as
/// sentences, so the client renders those from the registry the way
/// `abilities.rs` already renders "Equip {0}".
///
/// A **mana ability is placed**, though it is not a stack entry: only
/// [`LineShape::Other`] short-circuits here, and [`LineShape::stackable`]
/// is the separate question of whether it counts. See [`LineShape::Mana`].
///
/// A sentence may be claimed **twice**, and that is not a defect: "enters
/// or attacks" is one printed sentence and two `Trigger`s, so both of Sun
/// Titan's triggers belong to the same line. An invariant forbidding it
/// would report the DSL being *more precise* than the printing as a fault.
#[must_use]
pub fn map(abilities: &[AbilityDef], oracle: &str) -> Mapping {
    let lines: Vec<&str> = sentences(oracle).collect();
    let mut at = 0usize;
    let mut in_order = true;
    let mut ambiguous = 0usize;
    let mut found = Vec::with_capacity(abilities.len());
    for ability in abilities {
        let want = ability_shape(ability);
        if want == LineShape::Other {
            found.push(None);
            continue;
        }
        let fits = |line: &str| line_shape(line) == want && content_fits(ability, line);
        // Two sentences this ability fits equally well means the content
        // check cannot tell them apart, and whichever the walk reaches
        // first is a coin toss the report would print as a hit. Counting
        // them is how the next discriminator gets found instead of guessed
        // at — `cost_fits` reads mana, `{T}` and `{Q}`, so two `{2}:`
        // abilities that neither tap are exactly this case.
        if lines.iter().filter(|l| fits(l)).count() > 1 {
            ambiguous += 1;
        }
        if let Some(offset) = lines[at..].iter().position(|l| fits(l)) {
            at += offset + 1;
            found.push(u8::try_from(at - 1).ok());
            continue;
        }
        // Looking backwards is what a table is allowed to do and a
        // bijection is not: the ability is still *somewhere*, and only the
        // order the code happens to list them in was different.
        let back = lines.iter().position(|l| fits(l));
        // Landing on the sentence just claimed is **not** disorder — it is
        // the shared sentence this module exists to allow. Sun Titan's
        // "enters or attacks" is one line and two triggers, and calling
        // that out of order left `in_order` unable to mean anything: three
        // of the pool's cards tripped it for being written correctly, so a
        // fourth that was genuinely scrambled would have looked the same.
        //
        // An ability with no sentence anywhere leaves `in_order` alone too.
        // It is not a walk that went backwards, it is a walk that found
        // nothing, and the `None` below is where that is already said — so
        // a caller must read the two together and not take `in_order` for
        // "every ability was placed".
        if let Some(j) = back
            && Some(j) != at.checked_sub(1)
        {
            in_order = false;
        }
        found.push(back.and_then(|j| u8::try_from(j).ok()));
    }
    Mapping {
        lines: found,
        in_order,
        ambiguous,
    }
}

/// What [`map`] found.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Mapping {
    /// The sentence each ability came from, in step with the ability list.
    pub lines: Vec<Option<u8>>,
    /// Whether the walk found every ability its sentence without looking
    /// backwards at anything but the sentence it had just claimed. A table
    /// does not need this to be true — it records a mapping, not a march —
    /// so it is reported rather than required.
    pub in_order: bool,
    /// How many abilities fit more than one sentence equally well.
    ///
    /// A hit that could have gone either way is not a hit, and this is the
    /// only number here that says the *reader* needs work rather than the
    /// pool. It stays visible so the next discriminator is chosen against
    /// a count instead of a hunch.
    pub ambiguous: usize,
}

/// Which printed sentence each alternative cost came from.
///
/// The twin of [`map_modes`] for `CastModeKind::Alternative(i)`, and the
/// owner's own acceptance line for the cast chooser: a row reading
/// "Evoke—Exile a white card from your hand" is the card, and
/// "Alternative cost" is a category.
///
/// An alternative is printed in one of two ways. A card may write it out as
/// a sentence — "You may exile a blue card from your hand **rather than
/// pay** this spell's mana cost", "you may cast this spell **without paying
/// its mana cost**" — where the two templates are themselves the
/// discriminator: the second is the cost of nothing and the first is a cost
/// that was substituted. Or it prints it as a **keyword line**, `Evoke
/// {2}{U}` and `Evoke—Exile a white card from your hand.`, where the cost
/// stands alone behind one word.
///
/// Every part of the cost has to be found in the sentence ([`parts_fit`]),
/// and a sentence that fits two alternatives — or two sentences that fit
/// one — is refused rather than guessed at.
#[must_use]
pub fn map_alternatives(alts: &[AlternativeCost], oracle: &str) -> Vec<Option<u8>> {
    let lines: Vec<&str> = sentences(oracle).collect();
    let mut found: Vec<Option<u8>> = alts
        .iter()
        .map(|alt| {
            let hits: Vec<usize> = lines
                .iter()
                .enumerate()
                .filter(|(_, line)| alternative_fits(alt, line))
                .map(|(at, _)| at)
                .collect();
            match hits[..] {
                [only] => u8::try_from(only).ok(),
                _ => None,
            }
        })
        .collect();
    // One sentence cannot be two alternative costs. Where it looks like
    // both, neither is known — the same refusal the single-hit rule above
    // makes, seen from the sentence's side. The pair is read out of a
    // snapshot rather than out of the list being struck through: clearing
    // the first in place would leave the second one alone and hand it the
    // sentence both of them fit.
    let taken = found.clone();
    for (at, line) in found.iter_mut().enumerate() {
        if line.is_some() && taken.iter().filter(|other| *other == &taken[at]).count() > 1 {
            *line = None;
        }
    }
    found
}

/// Could this printed line be the alternative cost this card compiled?
fn alternative_fits(alt: &AlternativeCost, line: &str) -> bool {
    let text = without_reminder(line);
    let lower = text.to_lowercase();
    let free = lower.contains("without paying its mana cost");
    let instead = lower.contains("rather than pay");
    if free || instead {
        // The two templates are a fact about the cost and not only about
        // the words: "without paying its mana cost" is printed for a cost
        // of nothing, and "rather than pay" for one that was substituted.
        // A card printing both sentences is separated by exactly this.
        if free != (alt.cost.mana.is_empty() && alt.cost.parts.is_empty()) {
            return false;
        }
        return condition_fits(alt.condition, &lower) && parts_fit(alt.cost.parts, &lower);
    }
    // The keyword form. `cost_fits` reads the mana; the words after an em
    // dash carry a cost that has no symbols, and both are held to the
    // single word in front — without which "When this creature enters,
    // exile up to one other target creature" is a line with no braces
    // fitting a cost with no mana, which is nothing compared against
    // nothing.
    let Some((_, rest)) = keyword_cost(&text) else {
        return false;
    };
    matches!(alt.condition, AltCondition::Always)
        && cost_fits(&alt.cost, &text)
        && parts_fit(alt.cost.parts, &rest.to_lowercase())
}

/// A keyword line that prints a cost, split into the keyword and the cost.
///
/// `Evoke {2}{U}` and `Evoke—Exile a white card from your hand.` are the
/// two shapes, and both begin with a single word. That is the whole of what
/// keeps an ordinary sentence out, so it is a word and not merely a short
/// head: every English sentence on a card starts with a word and a space.
fn keyword_cost(line: &str) -> Option<(&str, &str)> {
    if let Some((head, rest)) = line.split_once('\u{2014}') {
        return one_word(head).then_some((head, rest.trim()));
    }
    let (head, rest) = line.split_once(' ')?;
    (one_word(head) && rest.starts_with('{')).then_some((head, rest))
}

/// Is this the whole of one printed word?
fn one_word(head: &str) -> bool {
    !head.is_empty() && head.chars().all(char::is_alphabetic)
}

/// Does the sentence state the condition the alternative cost carries?
fn condition_fits(condition: AltCondition, lower: &str) -> bool {
    match condition {
        // An unconditional alternative states no condition, and both
        // conditional ones print theirs as the clause the sentence opens
        // with — which is what separates Force of Will's "You may…" from
        // Force of Negation's "If it's not your turn, you may…".
        AltCondition::Always => !lower.starts_with("if "),
        AltCondition::NotYourTurn => lower.contains("not your turn"),
        AltCondition::CommanderControlled => lower.contains("control a commander"),
    }
}

/// Is every non-mana part of a cost named in the printed words?
///
/// The honesty rule, in one function. A part this cannot read refuses the
/// whole sentence rather than letting the others carry it: a row labelled
/// with a cost that leaves out half of what it charges is worse than one
/// labelled "Alternative cost", because the player reads it as the card.
fn parts_fit(parts: &[CostPart], lower: &str) -> bool {
    parts.iter().all(|part| match part {
        // Printed in symbols, and read by `cost_fits` there; the prose says
        // nothing about either.
        CostPart::TapSelf | CostPart::UntapSelf => true,
        CostPart::ExertSelf => lower.contains("exert"),
        CostPart::PayLife(n) => lower.contains(&format!("pay {n} life")),
        CostPart::ExileFromHand(_) => lower.contains("exile") && lower.contains("from your hand"),
        CostPart::Discard(_) => lower.contains("discard"),
        CostPart::Sacrifice(_) | CostPart::SacrificeSelf => lower.contains("sacrifice"),
        CostPart::ExileSelf => lower.contains("exile"),
        // As loose as the two above it, and for the same reason: the word is
        // what the card prints, and a sentence whose *effect* also returns
        // something ("Return a Forest you control …: Untap target creature"
        // does not, but Flooded Shoreline's does) is still a sentence that
        // names this cost.
        CostPart::ReturnToHand(_) => lower.contains("return"),
        CostPart::ExileFromGraveyard(_) => {
            lower.contains("exile") && lower.contains("from your graveyard")
        }
        _ => false,
    })
}

#[cfg(test)]
mod tests;
