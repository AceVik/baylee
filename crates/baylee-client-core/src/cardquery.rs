//! The search box's language: a Scryfall-shaped query, parsed, evaluated and
//! written back out again.
//!
//! # Why a language and not more chips
//!
//! The deck builder had a substring box and four chips — colours, one card
//! type, one mana value, "playable only" — and the zone browser had a
//! substring box and nothing. Both answer the easy half of "which card",
//! and neither can express the half a player actually asks at a deck list:
//! *creatures or artifacts that cost three or less and are not white*. The
//! owner asked for "der Skryfall vergleichbare Textfilter", which is the
//! right shape for exactly that reason — it is the language every player of
//! this game already knows, from the site they look their cards up on.
//!
//! # What a parse never does
//!
//! **It never fails and it never drops a word.** A key this module does not
//! know is kept as [`Key::Unknown`] and written back out exactly as it came
//! in; it simply never matches. That is not leniency, it is the property the
//! filter-string builder hangs on: a dialog that decomposes a string into
//! controls and recomposes it has to be able to carry the part it cannot
//! draw, or opening the dialog would silently delete half of what a player
//! typed.
//!
//! The same property is why [`render`] is held against [`parse`] rather than
//! against a string: `parse(render(q)) == q` for every query, including one
//! carrying terms nothing here understands.
//!
//! # What a term cannot answer
//!
//! Two surfaces use this and they know different things. A [`PoolCard`] has
//! rules text, a mana cost and a coverage; a row in the zone browser has a
//! name, a mana value and a type set, and no text at all. So evaluation is
//! **three**-valued: a term the surface cannot answer is
//! [`Match::Unknown`], and `Unknown` survives negation. `-o:draw` on a
//! surface with no oracle text matches nothing, which is the honest answer —
//! the alternative is a search that says "every card here lacks the word
//! draw" because it never had the text to look in.
//!
//! [`PoolCard`]: crate::deckbuilder::PoolCard

use crate::prose::sort_key;

mod eval;
#[cfg(test)]
pub(crate) mod tests;

pub use eval::{Facts, Match, Surface, matches};

/// What a term asks about.
///
/// The spellings are Scryfall's own, and both of each pair are accepted
/// where Scryfall accepts both — `c:` and `color:`, `mv` and `cmc` and
/// `manavalue`. [`Key::render`] picks one of them, which is what makes the
/// written form canonical without making the read form fussy.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Key {
    /// A word with no key in front of it: anywhere a player would look.
    ///
    /// **Wider than Scryfall's**, which reads a loose word as a name and
    /// nothing else, and the width is deliberate rather than inherited. This
    /// box has always searched the type line and the rules text beside the
    /// name — `search_looks_where_a_player_would` in
    /// `deckbuilder::tests` is the assertion, and what it is really about
    /// is that this is the *only* box there is. Scryfall has a page of
    /// controls under its box; a player here who types `instant` and is told
    /// there is no card by that name has nowhere else to go.
    ///
    /// [`Key::Name`] is the narrow reading, and it has a prefix because it
    /// is the one a player asks for on purpose.
    Loose,
    /// `name:`: the card's name, and no other part of it.
    ///
    /// Every name the card is known by, folded through [`sort_key`] on both
    /// sides — `browser.rs` already paid for the other way round, where a
    /// row drawn as "Wald" was searched as "Forest" and typing what was on
    /// the screen found nothing.
    Name,
    /// `!word`: the whole name and nothing else, case and accents folded.
    ExactName,
    /// `o:` / `oracle:`: a phrase in the rules text. `~` stands for the
    /// card's own name, as on Scryfall.
    Oracle,
    /// `t:` / `type:`: a partial word anywhere in the type line, or in the
    /// English card types beside it, so `t:kreatur` and `t:creature` both
    /// answer on a German client.
    Type,
    /// `c:` / `color:`: the card's own colours. A bare colon means **at
    /// least** these colours, which is Scryfall's `c:rg` — "cards that are
    /// red and green".
    Color,
    /// `id:` / `identity:` / `ci:`: colour identity (CR 903.4). A bare colon
    /// means **at most**, which is what makes `id:c t:land` the lands with a
    /// colourless identity rather than every land there is.
    Identity,
    /// `m:` / `mana:`: symbols in the printed mana cost, as a multiset. A
    /// bare colon means the cost *contains* them.
    Mana,
    /// `mv` / `cmc` / `manavalue`: mana value, as a number or `even`/`odd`.
    ManaValue,
    /// `pow` / `power`.
    Power,
    /// `tou` / `toughness`.
    Toughness,
    /// `loy` / `loyalty`.
    Loyalty,
    /// `is:` / `not:`: a yes-or-no property of the card.
    Is,
    /// A key nothing here knows, kept whole so it can be written back.
    ///
    /// It evaluates to [`Match::Unknown`], so it narrows a search to nothing
    /// rather than being quietly ignored — a typo that silently widened the
    /// result set would be the worse failure of the two.
    Unknown(String),
}

impl Key {
    /// The spelling this key is written back out in, without its operator.
    ///
    /// `None` for [`Key::Loose`] and [`Key::ExactName`], which are written
    /// as a bare word and a bare `!word`: those two are the shapes a player
    /// types without thinking about the language at all, and writing
    /// `name:lightning` back at them would teach the wrong lesson about what
    /// the box wants.
    #[must_use]
    pub fn render(&self) -> Option<&str> {
        Some(match self {
            Self::Loose | Self::ExactName => return None,
            Self::Name => "name",
            Self::Oracle => "o",
            Self::Type => "t",
            Self::Color => "c",
            Self::Identity => "id",
            Self::Mana => "m",
            Self::ManaValue => "mv",
            Self::Power => "pow",
            Self::Toughness => "tou",
            Self::Loyalty => "loy",
            Self::Is => "is",
            Self::Unknown(word) => word,
        })
    }

    /// The key a written prefix asks for, or [`Key::Unknown`].
    ///
    /// Case-folded, because a player who types `T:Creature` means the same
    /// thing and Scryfall reads it the same way.
    fn of(word: &str) -> Self {
        match word.to_ascii_lowercase().as_str() {
            "name" => Self::Name,
            "o" | "oracle" => Self::Oracle,
            "t" | "type" => Self::Type,
            "c" | "color" | "colour" => Self::Color,
            "id" | "ci" | "identity" => Self::Identity,
            "m" | "mana" => Self::Mana,
            "mv" | "cmc" | "manavalue" => Self::ManaValue,
            "pow" | "power" => Self::Power,
            "tou" | "toughness" => Self::Toughness,
            "loy" | "loyalty" => Self::Loyalty,
            "is" => Self::Is,
            _ => Self::Unknown(word.to_string()),
        }
    }

    /// Whether a value for this key is read as a number rather than a word.
    const fn numeric(&self) -> bool {
        matches!(
            self,
            Self::ManaValue | Self::Power | Self::Toughness | Self::Loyalty
        )
    }

    /// Whether a value for this key is read as a set of colours.
    const fn coloured(&self) -> bool {
        matches!(self, Self::Color | Self::Identity)
    }
}

/// How a term compares.
///
/// The six Scryfall writes plus the bare colon, which is **not** a synonym
/// for any of them: what it means is the key's own business — at least, for
/// [`Key::Color`]; at most, for [`Key::Identity`]; contains, for a word or a
/// mana cost; equals, for a number. Keeping it as its own operator is what
/// lets a query be written back in the spelling it was typed in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Op {
    /// `:`
    Colon,
    /// `=`
    Eq,
    /// `!=`
    Ne,
    /// `<`
    Lt,
    /// `<=`
    Le,
    /// `>`
    Gt,
    /// `>=`
    Ge,
}

impl Op {
    /// The characters this operator is written with.
    #[must_use]
    pub const fn render(self) -> &'static str {
        match self {
            Self::Colon => ":",
            Self::Eq => "=",
            Self::Ne => "!=",
            Self::Lt => "<",
            Self::Le => "<=",
            Self::Gt => ">",
            Self::Ge => ">=",
        }
    }
}

/// A yes-or-no property, as `is:` asks for it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Flag {
    /// The engine plays the card in full.
    Playable,
    /// The engine plays part of it, and says which part it does not.
    Partial,
    /// A generated stub: the engine does not play it at all.
    Stub,
    /// It may lead a commander deck.
    Commander,
    /// A basic land — the one card a deck may hold any number of.
    Basic,
    /// It is printed on both sides.
    Dfc,
    /// It is a token rather than a card (CR 111.1).
    Token,
}

impl Flag {
    /// The word this flag is written with.
    #[must_use]
    pub const fn render(self) -> &'static str {
        match self {
            Self::Playable => "playable",
            Self::Partial => "partial",
            Self::Stub => "stub",
            Self::Commander => "commander",
            Self::Basic => "basic",
            Self::Dfc => "dfc",
            Self::Token => "token",
        }
    }

    /// Every flag there is, which is what a filter dialog offers from.
    pub const ALL: [Self; 7] = [
        Self::Playable,
        Self::Partial,
        Self::Stub,
        Self::Commander,
        Self::Basic,
        Self::Dfc,
        Self::Token,
    ];

    /// The flag a written word asks for.
    fn of(word: &str) -> Option<Self> {
        let folded = word.to_ascii_lowercase();
        Self::ALL
            .into_iter()
            .find(|flag| flag.render() == folded || Self::alias(*flag, &folded))
    }

    /// The second spelling a flag answers to, where it has one.
    fn alias(flag: Self, folded: &str) -> bool {
        match flag {
            Self::Dfc => matches!(folded, "doublefaced" | "double-faced" | "transform"),
            Self::Playable => folded == "implemented",
            Self::Stub => folded == "unimplemented",
            _ => false,
        }
    }
}

/// The five colours, as a bitmask in `WUBRG` order.
///
/// A mask and not `baylee_core::color::ColorSet` because both surfaces hand
/// this module *letters* — `PoolCard::colors` is the string `"WU"` — so a
/// mask is what the conversion lands in either way, and one that this
/// module owns can carry the two things a query needs that a rules colour
/// set does not: "colourless" as a value a player can type, and a count.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Colors(pub u8);

impl Colors {
    /// White.
    pub const W: Self = Self(1);
    /// Blue.
    pub const U: Self = Self(2);
    /// Black.
    pub const B: Self = Self(4);
    /// Red.
    pub const R: Self = Self(8);
    /// Green.
    pub const G: Self = Self(16);

    /// The letters in `WUBRG` order, which is the order a cost is printed in
    /// and therefore the one a player reads a colour pair back in.
    const LETTERS: [(char, Self); 5] = [
        ('w', Self::W),
        ('u', Self::U),
        ('b', Self::B),
        ('r', Self::R),
        ('g', Self::G),
    ];

    /// The mask a string of letters comes to, ignoring anything else in it.
    #[must_use]
    pub fn of_letters(text: &str) -> Self {
        let mut mask = 0;
        for c in text.chars().flat_map(char::to_lowercase) {
            for (letter, bit) in Self::LETTERS {
                if c == letter {
                    mask |= bit.0;
                }
            }
        }
        Self(mask)
    }

    /// How many colours are in it.
    #[must_use]
    pub const fn count(self) -> u32 {
        self.0.count_ones()
    }

    /// Whether every colour of `other` is in this one.
    #[must_use]
    pub const fn holds(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// The letters, in `WUBRG` order. Empty for colourless.
    #[must_use]
    pub fn letters(self) -> String {
        Self::LETTERS
            .iter()
            .filter(|(_, bit)| self.0 & bit.0 != 0)
            .map(|(letter, _)| *letter)
            .collect()
    }
}

/// What a term is compared against.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Value {
    /// A word or phrase, for a name, a type line or rules text.
    Word(String),
    /// A number, for a mana value or a printed statistic.
    Number(i32),
    /// `mv:even` / `mv:odd`. `true` is even.
    Parity(bool),
    /// A set of colours, or `c` for colourless.
    Colors(Colors),
    /// `c:m`: more than one colour, whatever they are.
    ///
    /// Its own value rather than [`Value::ColorCount`] at two with the
    /// operator rewritten, because rewriting the operator would write
    /// `c>=2` back into a box a player typed `c:m` into — and this module's
    /// whole reason for keeping [`Op::Colon`] is that it does not do that.
    Multicolor,
    /// A number of colours, as `c=2` asks for it.
    ColorCount(i32),
    /// The symbols of a mana cost, as a multiset in printed order.
    Cost(Vec<String>),
    /// A yes-or-no property.
    Flag(Flag),
    /// A word for a key that does not know what to do with it.
    Opaque(String),
}

/// One condition.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Term {
    /// What it asks about.
    pub key: Key,
    /// How it compares.
    pub op: Op,
    /// What it compares against.
    pub value: Value,
}

/// A whole query.
///
/// `All` is what juxtaposition builds and `Any` is what `or` builds, which
/// is Scryfall's precedence: `a b or c d` is `(a and b) or (c and d)`.
/// [`parse`] flattens and collapses both, so a query has exactly one shape
/// per meaning and two strings that mean the same thing compare equal.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Query {
    /// Matches everything: the empty box.
    #[default]
    Anything,
    /// One condition.
    Term(Term),
    /// Every one of them.
    All(Vec<Query>),
    /// Any one of them.
    Any(Vec<Query>),
    /// The opposite of one.
    Not(Box<Query>),
}

impl Query {
    /// Whether this query lets everything through.
    #[must_use]
    pub const fn is_anything(&self) -> bool {
        matches!(self, Self::Anything)
    }

    /// Every term in it, in written order, however deeply nested.
    ///
    /// For *reading* a query — what keys it mentions — and never for taking
    /// one apart. The filter dialog does that with
    /// [`crate::filterdialog::FilterForm`], which walks the top-level parts
    /// and keeps each one in its place; a caller that collected these and
    /// rebuilt from them would have flattened a bracket away.
    #[must_use]
    pub fn terms(&self) -> Vec<&Term> {
        let mut out = Vec::new();
        self.walk(&mut out);
        out
    }

    fn walk<'a>(&'a self, out: &mut Vec<&'a Term>) {
        match self {
            Self::Anything => {}
            Self::Term(term) => out.push(term),
            Self::All(parts) | Self::Any(parts) => {
                for part in parts {
                    part.walk(out);
                }
            }
            Self::Not(inner) => inner.walk(out),
        }
    }
}

/// The conjunction of these parts, in the one shape the language has for it.
///
/// The door [`crate::filterdialog`] rebuilds a query through, and it is this
/// rather than `Query::All(parts)` because `All` is a *variant* and not a
/// constructor: a list of one is that one thing, a list with an `Anything` in
/// it is the same list without it, and an `All` inside an `All` is one `All`.
/// Building the variant by hand skips all three, and the result renders as a
/// string that parses back to something else — which is precisely the round
/// trip a filter dialog lives or dies by.
#[must_use]
pub fn all_of(parts: Vec<Query>) -> Query {
    collapse(parts, true)
}

/// One branch of a rebuilt tree, with four shapes that have no meaning
/// flattened out of it.
///
/// A query has **one** shape per meaning, which is what lets two written
/// strings be compared by parsing them. So a branch drops the parts that
/// match everything, splices in any child of its own kind, and stops being a
/// branch at all when one child is left — without the splice, `(a or b) or c`
/// and `a or b or c` are two trees for one question, and the first is written
/// back out as the second and no longer compares equal to itself.
fn collapse(parts: Vec<Query>, all: bool) -> Query {
    let mut flat: Vec<Query> = Vec::with_capacity(parts.len());
    for part in parts {
        match part {
            Query::Anything => {}
            Query::All(inner) if all => flat.extend(inner),
            Query::Any(inner) if !all => flat.extend(inner),
            other => flat.push(other),
        }
    }
    match flat.len() {
        0 => Query::Anything,
        1 => flat.remove(0),
        _ if all => Query::All(flat),
        _ => Query::Any(flat),
    }
}

// ------------------------------------------------------------ tokenising

/// One piece of a written query.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Tok {
    /// `(`
    Open,
    /// `)`
    Close,
    /// `or`, in any case.
    Or,
    /// `-`, which binds to whatever comes next.
    Minus,
    /// Everything else, with its quotes still on.
    Word(String),
}

/// Splits a written query into pieces.
///
/// Quotes are kept rather than resolved, because where a quote *is* decides
/// what it means: `o:"draw a card"` quotes a value and `"Fire // Ice"` quotes
/// a whole loose word, and the value cannot be cut off the key until the key
/// is known.
fn tokens(text: &str) -> Vec<Tok> {
    let mut out = Vec::new();
    let mut chars = text.chars().peekable();
    while let Some(&c) = chars.peek() {
        match c {
            c if c.is_whitespace() => {
                chars.next();
            }
            '(' => {
                chars.next();
                out.push(Tok::Open);
            }
            ')' => {
                chars.next();
                out.push(Tok::Close);
            }
            '-' => {
                chars.next();
                // A bare hyphen is not a negation of anything, and a hyphen
                // in the middle of a word is part of it — this arm is only
                // ever reached at the start of one.
                match chars.peek() {
                    Some(next) if !next.is_whitespace() => out.push(Tok::Minus),
                    _ => {}
                }
            }
            _ => {
                let word = word_at(&mut chars);
                if word.eq_ignore_ascii_case("or") {
                    out.push(Tok::Or);
                } else if !word.is_empty() {
                    out.push(Tok::Word(word));
                }
            }
        }
    }
    out
}

/// One word, from here to the next thing that ends a word.
///
/// A quote suspends all of that: everything up to the closing quote is part
/// of the word, spaces and brackets included. An *unclosed* quote runs to the
/// end of the text, which is the only reading that lets a player see results
/// while they are still typing the phrase.
fn word_at(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) -> String {
    let mut word = String::new();
    let mut quoted = false;
    while let Some(&c) = chars.peek() {
        match c {
            '"' => {
                quoted = !quoted;
                word.push(c);
                chars.next();
            }
            '(' | ')' if !quoted => break,
            c if c.is_whitespace() && !quoted => break,
            c => {
                word.push(c);
                chars.next();
            }
        }
    }
    word
}

// --------------------------------------------------------------- parsing

/// Reads a written query. Never fails; see the module header.
#[must_use]
pub fn parse(text: &str) -> Query {
    let toks = tokens(text);
    let mut at = 0;
    let query = parse_any(&toks, &mut at);
    // A stray `)` leaves the cursor short of the end. Whatever is left is
    // read as more of the same conjunction rather than discarded, because a
    // player halfway through typing a group is not asking for the rest of
    // their query to stop working.
    if at < toks.len() {
        let mut rest = vec![query];
        while at < toks.len() {
            if toks[at] == Tok::Close {
                at += 1;
                continue;
            }
            rest.push(parse_any(&toks, &mut at));
        }
        rest.retain(|q| !q.is_anything());
        return collapse(rest, true);
    }
    query
}

/// `and_expr ( "or" and_expr )*`
fn parse_any(toks: &[Tok], at: &mut usize) -> Query {
    let mut parts = vec![parse_all(toks, at)];
    while toks.get(*at) == Some(&Tok::Or) {
        *at += 1;
        parts.push(parse_all(toks, at));
    }
    parts.retain(|q| !q.is_anything());
    collapse(parts, false)
}

/// `atom+`
fn parse_all(toks: &[Tok], at: &mut usize) -> Query {
    let mut parts = Vec::new();
    while let Some(tok) = toks.get(*at) {
        if matches!(tok, Tok::Or | Tok::Close) {
            break;
        }
        parts.push(parse_atom(toks, at));
    }
    parts.retain(|q| !q.is_anything());
    collapse(parts, true)
}

/// `"-"? ( "(" any ")" | term )`
fn parse_atom(toks: &[Tok], at: &mut usize) -> Query {
    let mut negated = false;
    while toks.get(*at) == Some(&Tok::Minus) {
        negated = !negated;
        *at += 1;
    }
    let inner = match toks.get(*at) {
        Some(Tok::Open) => {
            *at += 1;
            let inner = parse_any(toks, at);
            if toks.get(*at) == Some(&Tok::Close) {
                *at += 1;
            }
            inner
        }
        Some(Tok::Word(word)) => {
            let word = word.clone();
            *at += 1;
            term(&word)
        }
        // A `-` or an `or` with nothing after it: the player is still typing.
        _ => {
            *at += 1;
            Query::Anything
        }
    };
    if negated && !inner.is_anything() {
        Query::Not(Box::new(inner))
    } else {
        inner
    }
}

/// One written word, read as a condition.
fn term(word: &str) -> Query {
    if let Some(rest) = word.strip_prefix('!') {
        let value = unquote(rest);
        return if value.is_empty() && !wrote_nothing(rest) {
            Query::Anything
        } else {
            Query::Term(Term {
                key: Key::ExactName,
                op: Op::Eq,
                value: Value::Word(value),
            })
        };
    }
    let Some((head, op, tail)) = split_op(word) else {
        let value = unquote(word);
        return if value.is_empty() && !wrote_nothing(word) {
            Query::Anything
        } else {
            Query::Term(Term {
                key: Key::Loose,
                op: Op::Colon,
                value: Value::Word(value),
            })
        };
    };
    // `not:x` is `-is:x`, which the page states outright. Reading it here
    // rather than carrying a second key is what keeps the evaluator from
    // having to know the rule twice.
    if head.eq_ignore_ascii_case("not") {
        let inner = Query::Term(Term {
            key: Key::Is,
            op,
            value: value_for(&Key::Is, &unquote(tail)),
        });
        return Query::Not(Box::new(inner));
    }
    let key = Key::of(head);
    Query::Term(Term {
        value: value_for(&key, &unquote(tail)),
        key,
        op,
    })
}

/// Where the operator is in a written term, if there is one.
///
/// The **first** one outside quotes, so `o:"a:b"` splits at the colon that
/// belongs to the key and not at the one inside the phrase. A term that
/// starts with its operator (`:foo`) has no key and is a loose word.
fn split_op(word: &str) -> Option<(&str, Op, &str)> {
    let bytes = word.as_bytes();
    let mut quoted = false;
    for (at, &b) in bytes.iter().enumerate() {
        match b {
            b'"' => quoted = !quoted,
            _ if quoted => {}
            b':' | b'=' | b'<' | b'>' | b'!' => {
                if at == 0 {
                    return None;
                }
                let (op, width) = match (b, bytes.get(at + 1)) {
                    (b':', _) => (Op::Colon, 1),
                    (b'=', _) => (Op::Eq, 1),
                    (b'!', Some(b'=')) => (Op::Ne, 2),
                    (b'!', _) => return None,
                    (b'<', Some(b'=')) => (Op::Le, 2),
                    (b'<', _) => (Op::Lt, 1),
                    (b'>', Some(b'=')) => (Op::Ge, 2),
                    (b'>', _) => (Op::Gt, 1),
                    _ => unreachable!("the arm matched one of those five bytes"),
                };
                return Some((&word[..at], op, &word[at + width..]));
            }
            _ => {}
        }
    }
    None
}

/// Whether the player wrote a value and it was empty, rather than writing
/// none at all.
///
/// The two keyless terms are the only place the difference is invisible after
/// [`unquote`], and it is the whole difference between a row surviving a
/// round trip and vanishing from the box that was supposed to be holding it:
/// `\"\"` is a term asking for an empty word, and a bare nothing is a player
/// who has typed a `-` or an `!` and not yet what follows it. A lone `\"` is
/// the third case and is read as the second — a quote has been opened and
/// what goes inside it is still being typed, which is the same reading
/// `o:\"draw a` already gets.
fn wrote_nothing(raw: &str) -> bool {
    raw.len() >= 2 && raw.starts_with('"') && raw.ends_with('"')
}

/// The written text without the quotes that were holding it together.
fn unquote(text: &str) -> String {
    text.chars().filter(|c| *c != '"').collect()
}

/// What a key makes of the text written after its operator.
///
/// Public because the filter dialog types into a row and has to arrive at
/// exactly the value the player would have got by typing the same thing into
/// the box — one reader for both, rather than a second one that agrees until
/// it does not.
#[must_use]
pub fn value_of(key: &Key, text: &str) -> Value {
    value_for(key, text)
}

/// What a key makes of the text after its operator.
fn value_for(key: &Key, text: &str) -> Value {
    if key.numeric() {
        return match text.to_ascii_lowercase().as_str() {
            "even" => Value::Parity(true),
            "odd" => Value::Parity(false),
            _ => text
                .parse::<i32>()
                .map_or_else(|_| Value::Opaque(text.to_string()), Value::Number),
        };
    }
    if key.coloured() {
        if let Ok(count) = text.parse::<i32>() {
            return Value::ColorCount(count);
        }
        if is_multicolor(text) {
            return Value::Multicolor;
        }
        return colors_of(text).map_or_else(|| Value::Opaque(text.to_string()), Value::Colors);
    }
    match key {
        Key::Mana => Value::Cost(cost_symbols(text)),
        Key::Is => Flag::of(text).map_or_else(|| Value::Opaque(text.to_string()), Value::Flag),
        _ => Value::Word(text.to_string()),
    }
}

/// The colours a written value names.
///
/// Letters, full colour names, and `c`/`colorless` for the empty set —
/// `m`/`multicolor` is taken before this is reached, because it is a count
/// and not a set.
///
/// Guild, shard and wedge nicknames are deliberately absent: there are
/// seventy of them, they are a dictionary rather than a rule, and this pool
/// is not the place that dictionary belongs. What matters is **how** they
/// are absent. Read letter by letter, `esper` is `{R}` — the `r` in it —
/// and `boros` is `{B}{R}`, which is a wrong answer wearing a right one's
/// clothes. So a word that is not a colour name and is not made of nothing
/// but `wubrg` letters is refused by the caller instead, and matches
/// nothing until somebody teaches this the nicknames.
fn colors_of(text: &str) -> Option<Colors> {
    let folded = text.to_ascii_lowercase();
    match folded.as_str() {
        "white" => return Some(Colors::W),
        "blue" => return Some(Colors::U),
        "black" => return Some(Colors::B),
        "red" => return Some(Colors::R),
        "green" => return Some(Colors::G),
        "c" | "colorless" | "colourless" => return Some(Colors::default()),
        _ => {}
    }
    folded
        .chars()
        .all(|c| matches!(c, 'w' | 'u' | 'b' | 'r' | 'g'))
        .then(|| Colors::of_letters(&folded))
}

/// Whether a written colour value was the word for "more than one colour".
fn is_multicolor(text: &str) -> bool {
    matches!(
        text.to_ascii_lowercase().as_str(),
        "m" | "multicolor" | "multicolour"
    )
}

/// The symbols of a written mana cost, in printed order.
///
/// Both spellings, because Scryfall accepts both and a player pastes
/// whichever their source wrote: `{2}{W}{W}` and `2WW` come to the same three
/// symbols. A braced group is taken whole, which is the only way to read
/// `{2/G}` or `{R/P}`; outside braces a run of digits is one generic symbol,
/// so `m:12` is twelve and not a one and a two.
fn cost_symbols(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '{' => {
                let mut sym = String::new();
                for c in chars.by_ref() {
                    if c == '}' {
                        break;
                    }
                    sym.push(c);
                }
                if !sym.is_empty() {
                    out.push(sym.to_ascii_uppercase());
                }
            }
            c if c.is_ascii_digit() => {
                let mut number = c.to_string();
                while chars.peek().is_some_and(char::is_ascii_digit) {
                    number.push(chars.next().expect("peeked"));
                }
                out.push(number);
            }
            c if c.is_whitespace() => {}
            c => out.push(c.to_ascii_uppercase().to_string()),
        }
    }
    out
}

// -------------------------------------------------------------- rendering

/// Writes a query back out.
///
/// Canonical: two queries that are equal are written identically, and
/// `parse(render(q)) == q` for every query — which is the whole contract,
/// and is what the filter dialog's round trip rests on.
#[must_use]
pub fn render(query: &Query) -> String {
    let mut out = String::new();
    write_query(query, &mut out, Level::Top);
    out
}

/// Where in a written query a part is standing, which is what decides
/// whether it needs brackets round it.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Level {
    /// The whole thing.
    Top,
    /// A branch of an `or`: anything but a bare `or` fits without brackets.
    Alternative,
    /// One of a run of conditions, or the thing a `-` is about.
    Atom,
}

fn write_query(query: &Query, out: &mut String, level: Level) {
    match query {
        Query::Anything => {}
        Query::Term(term) => write_term(term, out),
        Query::Not(inner) => {
            out.push('-');
            write_query(inner, out, Level::Atom);
        }
        Query::Any(parts) => {
            let brackets = level == Level::Atom;
            if brackets {
                out.push('(');
            }
            for (at, part) in parts.iter().enumerate() {
                if at > 0 {
                    out.push_str(" or ");
                }
                write_query(part, out, Level::Alternative);
            }
            if brackets {
                out.push(')');
            }
        }
        Query::All(parts) => {
            let brackets = level == Level::Atom;
            if brackets {
                out.push('(');
            }
            for (at, part) in parts.iter().enumerate() {
                if at > 0 {
                    out.push(' ');
                }
                write_query(part, out, Level::Atom);
            }
            if brackets {
                out.push(')');
            }
        }
    }
}

fn write_term(term: &Term, out: &mut String) {
    if term.key == Key::ExactName {
        out.push('!');
        out.push_str(&quoted(&term.value.written()));
        return;
    }
    match term.key.render() {
        None => out.push_str(&quoted(&term.value.written())),
        Some(key) => {
            out.push_str(key);
            out.push_str(term.op.render());
            out.push_str(&quoted(&term.value.written()));
        }
    }
}

impl Value {
    /// How this value is written, without the key or the operator in front
    /// of it and without the quotes [`render`] may put round it.
    ///
    /// The filter dialog seeds a row's text box with this, so that typing
    /// into a row starts from what the row already says rather than from
    /// whatever the control happened to draw.
    #[must_use]
    pub fn written(&self) -> String {
        match self {
            Self::Word(text) | Self::Opaque(text) => text.clone(),
            // One arm for two values, because *writing* one is the same
            // question for both: `mv=3` and `c=3` are the same three on the
            // page, and what tells them apart is the key beside them, which
            // is already written.
            Self::Number(n) | Self::ColorCount(n) => n.to_string(),
            Self::Parity(even) => (if *even { "even" } else { "odd" }).to_string(),
            Self::Colors(colors) => {
                let letters = colors.letters();
                if letters.is_empty() {
                    "c".to_string()
                } else {
                    letters
                }
            }
            Self::Multicolor => "m".to_string(),
            Self::Cost(symbols) => {
                let mut out = String::with_capacity(symbols.len() * 3);
                for symbol in symbols {
                    out.push('{');
                    out.push_str(symbol);
                    out.push('}');
                }
                out
            }
            Self::Flag(flag) => flag.render().to_string(),
        }
    }
}

/// The text with quotes round it, if it needs them to survive a re-read.
///
/// Three cases and each is a way a round trip could fail. A value holding
/// an operator or a bracket would be read back as two terms. The word `or`
/// would be read back as the joint between them. And an empty value would
/// vanish from the written form entirely, which is the one way a term could
/// be lost with nothing saying so — `t:""` is a term and `t:` is not.
fn quoted(text: &str) -> String {
    let needs = text.is_empty()
        || text.eq_ignore_ascii_case("or")
        || text.chars().any(|c| {
            c.is_whitespace() || matches!(c, ':' | '=' | '<' | '>' | '!' | '"' | '(' | ')')
        })
        || text.starts_with('-');
    if needs {
        format!("\"{}\"", text.replace('"', ""))
    } else {
        text.to_string()
    }
}

/// The folded form both sides of a word comparison are put through.
fn folded(text: &str) -> String {
    sort_key(text)
}
