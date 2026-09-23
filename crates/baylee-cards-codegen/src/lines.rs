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

use baylee_cards_dsl::{
    AbilityDef, AltCondition, AlternativeCost, CostPart, SpellMode, StepKind, Trigger,
};

/// The sentences a printed oracle text is made of.
///
/// Re-exported rather than written here: the client resolves the index
/// this module hands out against a *localized* printed text and cannot
/// reach this crate, so the split belongs to `baylee-core`, which both
/// ends depend on.
pub use baylee_core::oracle::sentences;

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
        !matches!(self, Self::Other | Self::Mana)
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

/// The shape of one printed line.
pub fn line_shape(line: &str) -> LineShape {
    let printed = line;
    let line = without_reminder(line);
    let line = line.as_str();
    // The trigger word is read **before** the colon, and putting it after
    // was this function's own first bug: most triggered abilities print no
    // colon at all ("When this creature enters, draw a card"), so a reader
    // that split on one first sent all of them to `Other` and reported 125
    // cards as misaligned. The pool was right and the heuristic was wrong,
    // which is the failure mode a measurement like this exists to have
    // early rather than after a table is built on it.
    let lower = line.to_lowercase();
    // "As this creature enters" is a replacement rather than a trigger by
    // the rules (CR 614), and every card that prints it writes it as
    // `triggered!` here — so for the purpose of finding which sentence an
    // ability came from it is a third trigger word, not a fourth shape.
    //
    // The word is not only that sentence's, and that is a known boundary
    // rather than an oversight: "As long as …" opens a *static* ability and
    // lands here too, which is how The World Tree and Riftstone Portal
    // both read as `Triggered`. Neither card has a triggered ability to be
    // given the wrong sentence, so it is latent today — and it is the next
    // hole of the kind [`ability_colon`] closed, not a second instance of
    // that one.
    if lower.starts_with("when") || lower.starts_with("at ") || lower.starts_with("as ") {
        return LineShape::Triggered;
    }
    // A chapter's separator is an em dash, not a colon.
    if line
        .split_once(" — ")
        .is_some_and(|(head, _)| !head.is_empty() && head.chars().all(|c| "IVX, ".contains(c)))
    {
        return LineShape::Chapter;
    }
    let Some(colon) = ability_colon(line) else {
        // Every colon this line prints stands inside quotation marks, so
        // the line defines no ability of its own and no ability may claim
        // it — see [`ability_colon`]. Refused here rather than left to fall
        // through, because the branch below reads a *keyword* line and a
        // short grant fits it: `Lands have "{T}: Add {C}."` is 26
        // characters, ends in a quotation mark rather than a full stop, and
        // carries a `{`. The pool's shortest grant is 46 characters and
        // only that length stood between this and the same defect one
        // branch over.
        if line.contains(':') {
            return LineShape::Other;
        }
        // A keyword ability with a cost is a printed sentence too, and it
        // is the *right* sentence for the ability it compiles to — cycling
        // and equip are `AbilityDef::Activated` here, and "Cycling {B}" is
        // what a player reads when one of them goes on the stack. It has no
        // colon because the card put the cost and the effect in a keyword
        // instead of side by side.
        return if line.len() <= 40
            && !line.ends_with('.')
            && (line.contains('{') || reminder_spells_out_an_ability(printed))
        {
            LineShape::Activated
        } else {
            LineShape::Other
        };
    };
    let (head, body) = line.split_at(colon);
    let body = body[':'.len_utf8()..].trim();
    // A line that makes mana is its own shape, and the reasons are on
    // [`LineShape::Mana`] — including why it has to be said here as well as
    // in `ability_shape`.
    let lower_body = body.to_lowercase();
    // The "add" may open the body's *second* sentence: Three Tree City's
    // "{2}, {T}: Choose a color. Add an amount of mana of that color …" is a
    // mana ability (CR 605.1a — it could add mana and targets nothing), and
    // read on the first sentence alone it was `Activated`, so the card's
    // mana ability had no sentence. That later "add" is believed only in a
    // body that never says "target", because a targeting ability is not a
    // mana ability whatever it adds; a body *opening* on "add" keeps the
    // reading it always had, since Primal Wellspring's "Add one mana of any
    // color. When that mana is spent …, you may choose new targets for the
    // copy" is a mana ability whose "target" belongs to a delayed trigger.
    let makes = |s: &str| s.starts_with("add ") && (s.contains('{') || s.contains("mana"));
    let adds_first = makes(&lower_body);
    let adds_later = !lower_body.contains("target") && lower_body.split(". ").skip(1).any(makes);
    if adds_first || adds_later {
        return LineShape::Mana;
    }
    // A loyalty cost is the whole of what precedes the colon, and the minus
    // a card prints is U+2212, not a hyphen.
    if !head.is_empty()
        && head
            .chars()
            .all(|c| c.is_ascii_digit() || c == '+' || c == '-' || c == '\u{2212}' || c == 'X')
    {
        return LineShape::Loyalty;
    }
    // An activation cost is mana, a tap, a sacrifice — never a sentence, so
    // the head is length-bounded rather than merely non-empty. The bound is
    // 120 and not 60 because a cost may be several clauses of prose:
    // Recurring Nightmare pays "Sacrifice a creature, Return this
    // enchantment to its owner's hand", which is 76 characters and was
    // being read as a card with no activated ability at all.
    if head.len() <= 120 {
        LineShape::Activated
    } else {
        LineShape::Other
    }
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
        A::Triggered { .. } | A::ModalTriggered { .. } | A::Echo { .. } => LineShape::Triggered,
        A::Activated { mana_ability, .. } | A::ActivatedConditional { mana_ability, .. } => {
            if *mana_ability {
                LineShape::Mana
            } else {
                LineShape::Activated
            }
        }
        A::SagaChapter { .. } => LineShape::Chapter,
        _ => LineShape::Other,
    }
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
    let mut printed = braced(head);
    let taps = printed.iter().any(|s| s == "T");
    let untaps = printed.iter().any(|s| s == "Q");
    if taps != cost.parts.contains(&CostPart::TapSelf) {
        return false;
    }
    if untaps != cost.parts.contains(&CostPart::UntapSelf) {
        return false;
    }
    printed.retain(|s| s != "T" && s != "Q");
    let mut want = braced(&cost.mana.to_string());
    // `{0}` and no mana at all are the same cost written two ways, and
    // equip is where they meet: Lightning Greaves prints "Equip {0}" and
    // compiles to `ManaCost::ZERO`, which renders as nothing.
    printed.retain(|s| s != "0");
    want.retain(|s| s != "0");
    printed.sort();
    want.sort();
    printed == want
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
        let Effect::AddMana { source, .. } = effect else {
            continue;
        };
        match source {
            ManaSource::Fixed(color) => made.push(symbol(*color).to_string()),
            ManaSource::Choice(colors) => {
                made.extend(colors.iter().map(|c| symbol(*c).to_string()));
            }
            // None of these names a colour the card could print, so none
            // has anything to be held against — `{T}: Add one mana of any
            // color in your commander's color identity` carries no symbol
            // either, and neither does "one mana of the chosen color".
            // `ChosenOr` prints the {W} beside it, but the line as a whole
            // is still half unprintable, so it is let through with them.
            ManaSource::CommanderIdentity
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
    printed == made
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
        T::LeavesBattlefield(_) => &["leave"],
        T::Dies(_) => &["die", "put into a graveyard"],
        T::SpellCast(_) | T::NthSpellCast { .. } | T::FirstNoncreatureSpellCast(_) => &["cast"],
        T::BecomesTarget => &["becomes the target"],
        T::ExiledFromBattlefield(_) => &["exiled"],
        T::DealsCombatDamageToPlayer(_) => &["damage"],
        T::BecomesTapped(_) => &["tap"],
        T::Draws(_) | T::DrawsExceptFirst(_) => &["draw"],
        T::Attacks(_) => &["attack"],
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
        T::StepBegin { step, .. } => match step {
            StepKind::Upkeep => &["upkeep"],
            StepKind::Draw => &["draw step"],
            StepKind::CombatBegin => &["beginning of combat"],
            StepKind::End => &["end step"],
        },
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
        | Trigger::FirstNoncreatureSpellCast(rel)
        | Trigger::StepBegin { whose: rel, .. } => match rel {
            PlayerRel::You => !lower.contains("opponent"),
            PlayerRel::Opponent | PlayerRel::EachOpponent => lower.contains("opponent"),
            _ => true,
        },
        Trigger::EntersBattlefield(filter)
        | Trigger::LeavesBattlefield(filter)
        | Trigger::Dies(filter)
        | Trigger::Attacks(filter)
        | Trigger::BecomesTapped(filter)
        | Trigger::ExiledFromBattlefield(filter)
        | Trigger::DealsCombatDamageToPlayer(filter)
        | Trigger::SpellCast(filter) => {
            if matches!(filter, baylee_cards_dsl::Filter::This) {
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
        A::Activated { cost, .. } | A::ActivatedConditional { cost, .. } => cost_fits(cost, line),
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

/// The bullet a modal card lists its modes under.
const BULLET: char = '\u{2022}';

/// Which printed sentence each mode of a modal spell came from.
///
/// `CastModeKind::Mode(i)` is the only thing a client is told about a mode,
/// and a chooser that can only say "Mode 2" asks a player to pick between
/// two numbers. The sentence is what turns that into the card — but only
/// where it is *known*: a label pointing one sentence off is drawn as the
/// card's own words and is worse than the number it replaced, so anything
/// this cannot read whole answers `None` for every mode of the face.
///
/// Two printings, and the text says which. A card that prints `Choose one —`
/// — or `choose up to one —`, where one mode does nothing and is how a
/// player declines — lists its modes as bullets, one per mode that does
/// something, in order. The other shape is **overload**, which prints no
/// bullet at all: the card's body, and a keyword line naming the other
/// cost. There, a mode with a `cost_override` finds the keyword line
/// printing that cost and a mode without one is the body, and both halves
/// have to come out to exactly the right count or the card is refused.
///
/// A modal **trigger** is read by the same two, and most of the pool's are
/// bulleted (Charming Prince, Aether Channeler, Ertai Resurrected,
/// Primaris Eliminator). Two are neither, and refuse: Derevi and Inspirit,
/// Flagship Vessel print their choice *inside* one sentence — "you may tap
/// or untap target permanent", "your choice of a +1/+1 counter or two
/// charge counters" — which is a mode with no sentence of its own, and a
/// number is the better label for it.
///
/// It reads modes and not abilities, so it is not part of [`map`]: an
/// `AbilityDef::ModalSpell` is [`LineShape::Other`] there, which is the
/// right answer for the ability — the whole card is its text — and no
/// answer at all for the modes inside it.
#[must_use]
pub fn map_modes(modes: &[SpellMode], oracle: &str) -> Vec<Option<u8>> {
    let lines: Vec<&str> = sentences(oracle).collect();
    let refuse = || vec![None; modes.len()];
    let bullets: Vec<usize> = lines
        .iter()
        .enumerate()
        .filter(|(_, line)| line.trim_start().starts_with(BULLET))
        .map(|(at, _)| at)
        .collect();
    if !bullets.is_empty() {
        // A mode that does nothing is the one a player declines with, and
        // it is printed nowhere: "choose up to one —" states the
        // permission in the header and then lists only the things there
        // are to choose. Ertai Resurrected is the pool's card, its third
        // mode is `mode!(&[])`, and read as a bullet's owner it cost the
        // other two theirs — three `None`s for a card that prints two
        // perfectly good sentences.
        //
        // Both halves are asked for, because either alone is a guess. The
        // printing has to say "choose up to", and the modes have to hold
        // exactly one that does nothing; a bullet count one short on its
        // own is the ordinary disagreement below, where the honest answer
        // is still to refuse the card whole.
        let declining = |m: &SpellMode| {
            m.effects.is_empty() && m.targets.is_none() && m.cost_override.is_none()
        };
        let spoken: Vec<usize> = modes
            .iter()
            .enumerate()
            .filter(|(_, m)| !declining(m))
            .map(|(at, _)| at)
            .collect();
        let declines = modes.len() - spoken.len();
        let optional = oracle.to_lowercase().contains("choose up to");
        // A bullet count that disagrees with the mode count is the printing
        // and the code describing different cards, which is exactly the
        // case where walking them in step is confidently wrong.
        if bullets.len() != spoken.len() || (declines > 0 && !(optional && declines == 1)) {
            return refuse();
        }
        let mut found = vec![None; modes.len()];
        for (at, line) in spoken.iter().zip(&bullets) {
            found[*at] = u8::try_from(*line).ok();
        }
        return found;
    }
    let mut found = vec![None; modes.len()];
    let mut spoken: Vec<usize> = Vec::new();
    for (at, mode) in modes.iter().enumerate() {
        let Some(mana) = mode.cost_override else {
            continue;
        };
        let cost = baylee_cards_dsl::Cost { mana, parts: &[] };
        let hits: Vec<usize> = lines
            .iter()
            .enumerate()
            .filter(|(_, line)| line_shape(line) == LineShape::Activated && cost_fits(&cost, line))
            .map(|(at, _)| at)
            .collect();
        // Two lines printing the same cost is the reader unable to tell
        // them apart, and whichever came first would be a coin toss drawn
        // to the player as the card's own sentence.
        let [only] = hits[..] else {
            return refuse();
        };
        found[at] = u8::try_from(only).ok();
        spoken.push(only);
    }
    // What is left is the card's body, and the counts have to meet exactly.
    // A modal card that also prints a keyword line — "Flying" over a body
    // over an overload cost — leaves two candidates for one mode and is
    // refused here rather than given the first of them.
    let body: Vec<usize> = lines
        .iter()
        .enumerate()
        .filter(|(at, line)| !spoken.contains(at) && line_shape(line) == LineShape::Other)
        .map(|(at, _)| at)
        .collect();
    let plain: Vec<usize> = (0..modes.len())
        .filter(|at| modes[*at].cost_override.is_none())
        .collect();
    if body.len() != plain.len() {
        return refuse();
    }
    for (at, line) in plain.iter().zip(body) {
        found[*at] = u8::try_from(line).ok();
    }
    found
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
        _ => false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_cards_dsl::effect::{Amount, Effect};
    use baylee_cards_dsl::filter::Filter;
    use baylee_cards_dsl::loyalty;

    /// A mana ability whose "add" opens the body's second sentence is still
    /// a mana line — and one that targets is not, whatever it adds.
    #[test]
    fn a_mana_line_may_choose_before_it_adds() {
        assert_eq!(
            line_shape(
                "{2}, {T}: Choose a color. Add an amount of mana of that color \
                 equal to the number of creatures you control of the chosen type."
            ),
            LineShape::Mana,
            "Three Tree City's second ability"
        );
        assert_eq!(
            line_shape("{T}: Target player loses 1 life. Add {B}."),
            LineShape::Activated,
            "a targeting ability is not a mana ability (CR 605.1a)"
        );
        assert_eq!(
            line_shape(
                "{T}: Add one mana of any color. When that mana is spent to cast an \
                 instant or sorcery spell, copy that spell and you may choose new \
                 targets for the copy."
            ),
            LineShape::Mana,
            "Primal Wellspring: the \"target\" is the delayed trigger's"
        );
    }

    /// "… add {B} instead" prints two outputs and the ability makes one;
    /// without "instead", every printed symbol still has to be made.
    #[test]
    fn an_instead_sentence_fits_either_of_its_outputs() {
        use baylee_core::mana::ManaColor;
        let blue = [Effect::mana(ManaColor::Blue, 1)];
        assert!(mana_fits(
            &blue,
            "{T}: Add {U}. If you played a land this turn, add {B} instead."
        ));
        assert!(
            !mana_fits(&blue, "{T}: Add {U} or {B}."),
            "a choice between two is not an ability that makes one of them"
        );
        let red = [Effect::mana(ManaColor::Red, 1)];
        assert!(
            !mana_fits(
                &red,
                "{T}: Add {U}. If you played a land this turn, add {B} instead."
            ),
            "\"instead\" widens the printed pair, not the whole wheel"
        );
    }

    /// Two "At the beginning" sentences are told apart by the step, not by
    /// the words they share.
    ///
    /// Mana Vault is the card that found it. It prints an upkeep sentence
    /// and a draw-step one, `trigger_words` answered `["beginning"]` for
    /// both, and `whose_trigger_fits` said nothing about a step at all — so
    /// its draw-step trigger fitted either, took the first, and the table
    /// pointed at an ability the card does not have. The reader knew it was
    /// a coin toss and said so in `Mapping::ambiguous`; nothing read that
    /// number.
    ///
    /// The step is what a card cannot phrase two ways: "at the beginning of
    /// your upkeep", "… of your draw step". Reading it turns the toss into
    /// a decision, which is what `ambiguous == 0` here asserts — and the
    /// `swapped` half is the other direction, so a reader that merely
    /// preferred the later sentence would not pass.
    #[test]
    fn two_step_triggers_are_told_apart_by_which_step_it_is() {
        use baylee_cards_dsl::effect::PlayerRel;
        use baylee_cards_dsl::{StepKind, Trigger, triggered};
        let text = "This artifact doesn't untap during your untap step.\n\
                    At the beginning of your upkeep, you may pay {4}. If you \
                    do, untap this artifact.\n\
                    At the beginning of your draw step, if this artifact is \
                    tapped, it deals 1 damage to you.\n\
                    {T}: Add {C}{C}{C}.";
        let step = |step| {
            triggered!(
                Trigger::StepBegin {
                    step,
                    whose: PlayerRel::You,
                },
                draw(1)
            )
        };
        let abilities = [step(StepKind::Draw)];
        let found = map(&abilities, text);
        assert_eq!(
            found.lines,
            vec![Some(2)],
            "the draw-step trigger is the draw-step sentence",
        );
        assert_eq!(
            found.ambiguous, 0,
            "the step decides it, so nothing is tossed"
        );

        let abilities = [step(StepKind::Upkeep), step(StepKind::Draw)];
        let found = map(&abilities, text);
        assert_eq!(found.lines, vec![Some(1), Some(2)]);

        let swapped = [step(StepKind::Draw), step(StepKind::Upkeep)];
        let found = map(&swapped, text);
        assert_eq!(found.lines, vec![Some(2), Some(1)]);
        assert!(!found.in_order, "the card prints them the other way round");
    }

    /// A quoted ability belongs to whatever the sentence grants, and the
    /// sentence itself has no shape an ability may claim.
    ///
    /// CR 113.10a. Chromatic Lantern is the card that found it: its own
    /// mana ability claimed `Lands you control have "{T}: Add one mana of
    /// any color."` — the grant, one sentence before the identical line the
    /// lantern actually prints for itself — and the table said `Some(0)`
    /// where the card says `Some(1)`.
    ///
    /// The short grant is the second half and is the case the pool does not
    /// print yet. Length is what kept it out of the keyword branch, not
    /// punctuation: a grant ends in a quotation mark, so `!ends_with('.')`
    /// is true of it, and it carries a `{` like any cost. At 26 characters
    /// it would have been read as `Activated` — the same defect one branch
    /// over, which is why the refusal is stated and not left to fall
    /// through.
    #[test]
    fn a_colon_inside_quotation_marks_belongs_to_the_ability_being_granted() {
        assert_eq!(
            line_shape(r#"Lands you control have "{T}: Add one mana of any color.""#),
            LineShape::Other,
            "the lantern's grant is a static ability, not the mana it hands out",
        );
        assert_eq!(
            line_shape(r#"Lands have "{T}: Add {C}.""#),
            LineShape::Other,
            "a grant short enough for the keyword branch is still a grant",
        );
        // The other direction, or the assertions above are satisfied by a
        // reader that refuses every sentence carrying a quotation mark: the
        // colon a card prints outside one is still its own.
        assert_eq!(
            line_shape("{T}: Add one mana of any color."),
            LineShape::Mana,
            "the lantern's own line is unchanged",
        );
        assert_eq!(
            line_shape(r#"{2}, {T}: Target creature gains "flying" until end of turn."#),
            LineShape::Activated,
            "a quotation mark later in the line does not hide the cost's colon",
        );
    }

    /// A keyword line is a sentence too, and which *kind* of keyword it is
    /// the card says with its own punctuation.
    ///
    /// Station is the case that found this. `Station (Tap another creature
    /// you control: Put charge counters equal to its power on this
    /// Spacecraft. …)` is an **activated ability** spelled out in reminder
    /// text — it has a cost, a colon and an effect — while `Flying (This
    /// creature can't be blocked except by creatures with flying or reach.)`
    /// describes a rule and has no colon anywhere in it. Read as
    /// [`LineShape::Other`], Station was a card whose only activated ability
    /// had no sentence at all, and the sheet drew the row as "Ability 1".
    ///
    /// The colon is read from the **printed** line and not from the line with
    /// its reminder stripped, which is the whole trick: strip the reminder
    /// first and both of these are one bare word.
    #[test]
    fn a_keyword_whose_reminder_spells_out_an_ability_is_an_activated_one() {
        let station = "Station (Tap another creature you control: Put charge \
                       counters equal to its power on this Spacecraft. \
                       Station only as a sorcery.)";
        assert_eq!(
            line_shape(station),
            LineShape::Activated,
            "a cost and a colon in the reminder is an activated ability",
        );

        let flying = "Flying (This creature can't be blocked except by \
                      creatures with flying or reach.)";
        assert_eq!(
            line_shape(flying),
            LineShape::Other,
            "a keyword that is a bit describes a rule and names no cost",
        );

        // The counter-test for the reading itself: it is the colon that
        // decides, and it has to be found inside the brackets. A colon in the
        // prose *outside* a reminder is a line this already handled.
        assert!(reminder_spells_out_an_ability(station));
        assert!(!reminder_spells_out_an_ability(flying));
        assert!(
            !reminder_spells_out_an_ability("Trample"),
            "a keyword printed with no reminder at all spells out nothing",
        );
    }

    /// Three loyalty abilities, printed the way a walker prints them.
    const WALKER: &str = "+2: Look at the top card of target player's library.\n\
                          0: Draw three cards, then put two cards back.\n\
                          −1: Return target creature to its owner's hand.";

    fn draw(n: u32) -> &'static [Effect] {
        Box::leak(Box::new([Effect::DrawCards {
            amount: Amount::Fixed(n),
        }]))
    }

    /// The counter-test the whole content check exists for.
    ///
    /// All three of these abilities are `LineShape::Loyalty`, so a matcher
    /// reading shape alone walks the swapped pair in printed order, finds a
    /// loyalty line for each, and reports the card as aligned — while the
    /// table it produces points the `+2` at the `0`'s sentence. Only the
    /// cost printed before the colon can tell them apart.
    #[test]
    fn two_loyalty_abilities_in_each_others_places_are_caught_by_their_cost() {
        let right = [
            loyalty!(2, draw(1)),
            loyalty!(0, draw(3)),
            loyalty!(-1, draw(1)),
        ];
        let found = map(&right, WALKER);
        assert_eq!(found.lines, vec![Some(0), Some(1), Some(2)]);
        assert!(found.in_order);

        let swapped = [
            loyalty!(0, draw(3)),
            loyalty!(2, draw(1)),
            loyalty!(-1, draw(1)),
        ];
        let found = map(&swapped, WALKER);
        assert_eq!(
            found.lines,
            vec![Some(1), Some(0), Some(2)],
            "each ability still finds its own sentence — backwards"
        );
        assert!(
            !found.in_order,
            "and the walk says so, which is what the report counts"
        );
    }

    /// One printed sentence, two compiled triggers — Sun Titan's shape.
    ///
    /// Both abilities belong to line 1, so a bijection invariant would
    /// report the DSL being more precise than the printing as a defect.
    #[test]
    fn one_sentence_may_be_claimed_by_two_triggers() {
        use baylee_cards_dsl::{Trigger, triggered};
        let text = "Vigilance\n\
                    Whenever this creature enters or attacks, you may return \
                    target permanent card from your graveyard to the battlefield.";
        let abilities = [
            triggered!(Trigger::ETB, draw(1)),
            triggered!(Trigger::Attacks(&Filter::This), draw(1)),
        ];
        assert_eq!(map(&abilities, text).lines, vec![Some(1), Some(1)]);
    }

    /// Evoke's sacrifice is printed as a keyword line, not as a sentence.
    ///
    /// Mulldrifter prints two lines that both say "enters" and only one of
    /// them is a sentence; a trigger that accepted either took the draw and
    /// left the card looking complete with its table wrong twice over.
    #[test]
    fn an_ability_printed_as_a_keyword_is_honestly_lineless() {
        use baylee_cards_dsl::{Trigger, triggered};
        let text = "Flying\n\
                    When this creature enters, draw two cards.\n\
                    Evoke {2}{U}";
        let abilities = [
            triggered!(Trigger::ETB, draw(2)),
            triggered!(Trigger::EntersBattlefieldEvoked, &[]),
        ];
        assert_eq!(map(&abilities, text).lines, vec![Some(1), None]);
    }

    /// Two triggers with the same verb, told apart by their subject.
    ///
    /// "you draw" and "an opponent draws" fit each other's sentences on the
    /// word alone. Whichever the walk reached first was a coin toss the
    /// report printed as a hit, which is what `Mapping::ambiguous` counts.
    #[test]
    fn two_triggers_sharing_a_verb_are_told_apart_by_whose_event_it_is() {
        use baylee_cards_dsl::effect::PlayerRel;
        use baylee_cards_dsl::{Trigger, triggered};
        let text = "Deathtouch\n\
                    Whenever you draw a card, you gain 2 life.\n\
                    Whenever an opponent draws a card, they lose 2 life.";
        let abilities = [
            triggered!(Trigger::Draws(PlayerRel::You), draw(1)),
            triggered!(Trigger::Draws(PlayerRel::Opponent), draw(1)),
        ];
        let found = map(&abilities, text);
        assert_eq!(found.lines, vec![Some(1), Some(2)]);
        assert_eq!(found.ambiguous, 0, "neither fits the other's sentence");

        let swapped = [
            triggered!(Trigger::Draws(PlayerRel::Opponent), draw(1)),
            triggered!(Trigger::Draws(PlayerRel::You), draw(1)),
        ];
        let found = map(&swapped, text);
        assert_eq!(found.lines, vec![Some(2), Some(1)]);
        assert!(!found.in_order);
    }

    /// One sentence naming two subjects belongs to both of them.
    ///
    /// Orcish Bowmasters prints "When this creature enters **and** whenever
    /// an opponent draws a card…", and a subject test that refused either
    /// half sent a correctly written card to the misses. The subject is
    /// also read from the clause before the first comma only — the rest of
    /// the sentence routinely names the other player without the trigger
    /// being theirs.
    #[test]
    fn a_clause_naming_both_subjects_is_refused_to_neither() {
        use baylee_cards_dsl::effect::PlayerRel;
        use baylee_cards_dsl::{Trigger, triggered};
        let text = "Flash\n\
                    When this creature enters and whenever an opponent draws a card \
                    except the first one they draw in each of their draw steps, this \
                    creature deals 1 damage to any target.";
        let abilities = [
            triggered!(Trigger::ETB, draw(1)),
            triggered!(Trigger::DrawsExceptFirst(PlayerRel::Opponent), draw(1)),
        ];
        let found = map(&abilities, text);
        assert_eq!(found.lines, vec![Some(1), Some(1)]);
        assert!(found.in_order, "a shared sentence is not disorder");
    }

    /// One mode, one bullet — the shape three of the pool's four modal
    /// cards print.
    #[test]
    fn a_bulleted_card_gives_each_mode_its_own_bullet() {
        let text = "Choose one —\n\
                    • Each opponent sacrifices a nontoken creature of their choice.\n\
                    • Each opponent sacrifices a creature token of their choice.\n\
                    • Each opponent sacrifices a planeswalker of their choice.";
        let modes = [mode(None), mode(None), mode(None)];
        assert_eq!(
            map_modes(&modes, text),
            vec![Some(1), Some(2), Some(3)],
            "the header is a sentence too, and no mode is it"
        );
    }

    /// A bullet count that disagrees with the mode count is refused whole.
    ///
    /// The counter-test the honesty rule exists for: walking them in step
    /// places two of the three and is confidently wrong about which, and
    /// the row it draws is the card's own words on the wrong mode — worse
    /// than the "Mode 2" it replaced.
    #[test]
    fn a_printing_with_a_bullet_too_few_is_refused_whole() {
        let text = "Choose one —\n\
                    • Destroy target artifact.\n\
                    • Destroy target enchantment.";
        let modes = [mode(None), mode(None), mode(None)];
        assert_eq!(map_modes(&modes, text), vec![None, None, None]);
    }

    /// "Choose up to one" prints one bullet fewer than it has modes.
    ///
    /// Ertai Resurrected's shape: two things to choose and a third mode
    /// that does nothing, which is how a player declines. Read as an
    /// ordinary bullet count it is one short, and refusing it cost the two
    /// printed sentences their rows as well.
    #[test]
    fn a_mode_a_player_declines_with_is_printed_nowhere_and_takes_no_bullet() {
        let text = "Flash\n\
                    When this creature enters, choose up to one —\n\
                    • Counter target spell, activated ability, or triggered \
                    ability. Its controller draws a card.\n\
                    • Destroy another target creature or planeswalker. Its \
                    controller draws a card.";
        let modes = [mode(None), mode(None), decline()];
        assert_eq!(
            map_modes(&modes, text),
            vec![Some(2), Some(3), None],
            "the decline is not a bullet, and the two that are keep theirs"
        );
    }

    /// The counter-test for the clause above, in both directions.
    ///
    /// The reading needs the printing to say "choose up to" *and* exactly
    /// one mode that does nothing. A card one bullet short without the
    /// words is the ordinary disagreement and is refused whole; a card
    /// that says the words while every mode does something is read
    /// bullet-for-mode as any other, with nothing to spare.
    #[test]
    fn a_short_bullet_list_without_the_words_is_still_refused() {
        let silent = "Choose one —\n\
                      • Destroy target artifact.\n\
                      • Destroy target enchantment.";
        assert_eq!(
            map_modes(&[mode(None), mode(None), decline()], silent),
            vec![None, None, None],
            "nothing here says a mode may be declined"
        );

        let spoken = "Choose up to one —\n\
                      • Destroy target artifact.\n\
                      • Destroy target enchantment.";
        assert_eq!(
            map_modes(&[mode(None), mode(None)], spoken),
            vec![Some(1), Some(2)],
            "the words alone invent no decline"
        );
    }

    /// Overload prints no bullet: a body and a keyword line.
    #[test]
    fn an_overloaded_card_finds_its_body_and_its_keyword_line() {
        let text = "Return target nonland permanent you don't control to its \
                    owner's hand.\n\
                    Overload {6}{U} (You may cast this spell for its overload \
                    cost. If you do, change \"target\" in its text to \"each.\")";
        let modes = [mode(None), mode(Some(baylee_core::mana!("{6}{U}")))];
        assert_eq!(map_modes(&modes, text), vec![Some(0), Some(1)]);
    }

    /// A second brace-free sentence leaves the plain mode two candidates.
    ///
    /// "Flying" over a body over an overload cost is a card nobody prints
    /// today, and the point is exactly that: the reader says it cannot tell
    /// which of the two is the mode instead of taking the first.
    #[test]
    fn a_keyword_line_beside_the_body_refuses_the_plain_mode() {
        let text = "Flying and vigilance, and this creature attacks each combat \
                    if able.\n\
                    Return target nonland permanent you don't control to its \
                    owner's hand.\n\
                    Overload {6}{U}";
        let modes = [mode(None), mode(Some(baylee_core::mana!("{6}{U}")))];
        assert_eq!(map_modes(&modes, text), vec![None, None]);
    }

    /// The two ways a card prints an alternative cost, and a sentence that
    /// is neither.
    #[test]
    fn an_alternative_cost_is_found_as_a_sentence_or_as_a_keyword_line() {
        let pitch = "You may pay 1 life and exile a blue card from your hand \
                     rather than pay this spell's mana cost.\n\
                     Counter target spell.";
        assert_eq!(
            map_alternatives(&[alt(life_and_pitch(), AltCondition::Always)], pitch),
            vec![Some(0)]
        );
        let evoke = "Flash\n\
                     Lifelink\n\
                     When this creature enters, exile up to one other target \
                     creature. That creature's controller gains life equal to \
                     its power.\n\
                     Evoke—Exile a white card from your hand.";
        assert_eq!(
            map_alternatives(&[alt(pitch_only(), AltCondition::Always)], evoke),
            vec![Some(3)],
            "the trigger names an exile too, and says nothing about a hand"
        );
    }

    /// "Without paying its mana cost" is the cost of nothing, and "rather
    /// than pay" is a cost that was substituted.
    ///
    /// Which is what tells a free alternative from a pitch one without
    /// reading either sentence, and it is a claim about the templating that
    /// a card printing the wrong one of the two would break loudly.
    #[test]
    fn the_two_templates_separate_a_free_cost_from_a_substituted_one() {
        let free = "If you control a commander, you may cast this spell \
                    without paying its mana cost.\n\
                    Counter target noncreature spell.";
        assert_eq!(
            map_alternatives(
                &[alt(
                    baylee_cards_dsl::Cost::FREE,
                    AltCondition::CommanderControlled
                )],
                free
            ),
            vec![Some(0)]
        );
        assert_eq!(
            map_alternatives(
                &[alt(pitch_only(), AltCondition::CommanderControlled)],
                free
            ),
            vec![None],
            "a cost that pays something cannot be the sentence that pays nothing"
        );
    }

    /// The condition is read, so the two free commander instants cannot
    /// take Force of Negation's sentence and the other way round.
    #[test]
    fn a_condition_the_sentence_does_not_state_is_refused() {
        let text = "If it's not your turn, you may exile a blue card from your \
                    hand rather than pay this spell's mana cost.\n\
                    Counter target noncreature spell.";
        assert_eq!(
            map_alternatives(&[alt(pitch_only(), AltCondition::NotYourTurn)], text),
            vec![Some(0)]
        );
        assert_eq!(
            map_alternatives(&[alt(pitch_only(), AltCondition::Always)], text),
            vec![None],
            "the sentence opens with a condition and the cost carries none"
        );
    }

    /// A sentence that fits two alternative costs names neither of them.
    ///
    /// No card in the pool prints two, so this is the rule stated rather
    /// than a case observed — and it is read out of a snapshot, because
    /// striking the first one through in place would leave the second
    /// holding the sentence alone and looking like a single hit.
    #[test]
    fn one_sentence_that_fits_two_alternatives_names_neither() {
        let text = "You may exile a blue card from your hand rather than pay \
                    this spell's mana cost.\n\
                    Counter target spell.";
        assert_eq!(
            map_alternatives(
                &[
                    alt(pitch_only(), AltCondition::Always),
                    alt(pitch_only(), AltCondition::Always),
                ],
                text
            ),
            vec![None, None],
            "the second one must not keep the sentence the first gave up"
        );
    }

    /// One mode with no `cost_override` and no sentence at all.
    ///
    /// It carries an effect, and that is not decoration: [`map_modes`]
    /// reads an **effect-less** mode as the one a player declines with,
    /// so a stand-in that did nothing would be every test's card printing
    /// "choose up to". [`decline`] is the mode that means it.
    fn mode(cost_override: Option<baylee_core::mana::ManaCost>) -> SpellMode {
        SpellMode {
            effects: draw(1),
            targets: None,
            cost_override,
        }
    }

    /// The mode "choose up to one" gives a player to decline with: no
    /// effects, nothing to target, nothing to pay (Ertai Resurrected).
    fn decline() -> SpellMode {
        SpellMode {
            effects: &[],
            targets: None,
            cost_override: None,
        }
    }

    /// One alternative cost.
    fn alt(cost: baylee_cards_dsl::Cost, condition: AltCondition) -> AlternativeCost {
        AlternativeCost { cost, condition }
    }

    /// Force of Will's: a life payment and a pitched card.
    fn life_and_pitch() -> baylee_cards_dsl::Cost {
        baylee_cards_dsl::Cost {
            mana: baylee_core::mana::ManaCost::ZERO,
            parts: &[
                CostPart::PayLife(1),
                CostPart::ExileFromHand(&baylee_cards_dsl::Filter::Any),
            ],
        }
    }

    /// Misdirection's and evoke's: a pitched card and nothing else.
    fn pitch_only() -> baylee_cards_dsl::Cost {
        baylee_cards_dsl::Cost {
            mana: baylee_core::mana::ManaCost::ZERO,
            parts: &[CostPart::ExileFromHand(&baylee_cards_dsl::Filter::Any)],
        }
    }
}
