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

use baylee_cards_dsl::{AbilityDef, AltCondition, AlternativeCost, CostPart, SpellMode, Trigger};

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
    let Some((head, _)) = line.split_once(':') else {
        // A keyword ability with a cost is a printed sentence too, and it
        // is the *right* sentence for the ability it compiles to — cycling
        // and equip are `AbilityDef::Activated` here, and "Cycling {B}" is
        // what a player reads when one of them goes on the stack. It has no
        // colon because the card put the cost and the effect in a keyword
        // instead of side by side.
        return if line.len() <= 40 && line.contains('{') && !line.ends_with('.') {
            LineShape::Activated
        } else {
            LineShape::Other
        };
    };
    // A line that makes mana is its own shape, and the reasons are on
    // [`LineShape::Mana`] — including why it has to be said here as well as
    // in `ability_shape`.
    let body = line.split_once(':').map_or("", |(_, body)| body.trim());
    let lower_body = body.to_lowercase();
    if lower_body.starts_with("add ") && (body.contains('{') || lower_body.contains("mana")) {
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
            // Neither names a colour the card could print, so neither has
            // anything to be held against — `{T}: Add one mana of any color
            // in your commander's color identity` carries no symbol either.
            ManaSource::CommanderIdentity | ManaSource::LandColor { .. } => return true,
        }
    }
    if made.is_empty() {
        return true;
    }
    made.sort();
    made.dedup();
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
        T::StepBegin { .. } => &["beginning"],
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
        Trigger::Draws(rel)
        | Trigger::DrawsExceptFirst(rel)
        | Trigger::FirstNoncreatureSpellCast(rel) => match rel {
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
/// lists its modes as bullets, one per mode, in order — three of the pool's
/// four modal cards. The fourth shape is **overload**, which prints no
/// bullet at all: the card's body, and a keyword line naming the other
/// cost. There, a mode with a `cost_override` finds the keyword line
/// printing that cost and a mode without one is the body, and both halves
/// have to come out to exactly the right count or the card is refused.
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
        // A bullet count that disagrees with the mode count is the printing
        // and the code describing different cards, which is exactly the
        // case where walking them in step is confidently wrong.
        if bullets.len() != modes.len() {
            return refuse();
        }
        return bullets.iter().map(|at| u8::try_from(*at).ok()).collect();
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
        _ => false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_cards_dsl::effect::{Amount, Effect};
    use baylee_cards_dsl::filter::Filter;
    use baylee_cards_dsl::loyalty;

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
            triggered!(Trigger::EntersBattlefield(&Filter::This), draw(1)),
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
            triggered!(Trigger::EntersBattlefield(&Filter::This), draw(2)),
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
            triggered!(Trigger::EntersBattlefield(&Filter::This), draw(1)),
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
    fn mode(cost_override: Option<baylee_core::mana::ManaCost>) -> SpellMode {
        SpellMode {
            effects: &[],
            targets: None,
            cost_override,
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
