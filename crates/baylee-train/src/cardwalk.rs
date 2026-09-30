//! One deterministic walk over a card's definition, for two readers.
//!
//! The trained AI must play cards it never saw in a game, so a card's meaning
//! has to come from its structure and not from a learned id alone. This walk
//! reads a `CardDef` (or a registry `TokenDef`) as data — the ability DSL tree,
//! costs, types, stats, keywords — and turns it into:
//!
//! - [`Walk::keys`]: every building block the card uses, keyed
//!   `field>Variant` (`effects>DealDamage`, `amount>Fixed`), because a variant
//!   name alone collides across enums (`Fixed` is an `Amount` and a
//!   `ManaSource`). With its parent (`DealDamage.amount>Fixed`) and numbers
//!   (`amount=#3`) as further keys. The verification ladder reads the plain
//!   `field>Variant` keys as the mechanics a card needs tested ([`mechanics`]).
//! - [`features`]: a fixed-width vector per card — explicit fields (cost,
//!   colours, types, stats, keywords, commander and partner rules) and the
//!   keys hashed into [`HASHED`] buckets — which the net reads beside the id
//!   embedding, so an unseen card is not a random vector.
//!
//! The DSL is read through its `Debug` form, which every definition type
//! derives: no second description of the DSL to keep in step, and a new
//! variant is read the day it exists. That form is part of what this module
//! promises: renaming a variant or a field changes the keys, which is a new
//! [`WALK_VERSION`].

use std::collections::BTreeMap;

use baylee_cards::dsl::{CardDef, CommanderRule, PartnerKind, TokenDef};
use baylee_core::mana::{ManaCost, ManaSymbol};

/// The walk's version; a table and a model name the one they were made with.
pub const WALK_VERSION: u32 = 1;

/// Buckets the keys are hashed into.
pub const HASHED: usize = 512;

/// The explicit features, in order, before the hashed buckets.
pub const EXPLICIT: [&str; 42] = [
    "mana_value",
    "generic",
    "pip_w",
    "pip_u",
    "pip_b",
    "pip_r",
    "pip_g",
    "pip_c",
    "x_cost",
    "color_w",
    "color_u",
    "color_b",
    "color_r",
    "color_g",
    "identity_w",
    "identity_u",
    "identity_b",
    "identity_r",
    "identity_g",
    "power",
    "toughness",
    "has_pt",
    "loyalty",
    "has_loyalty",
    "faces",
    "abilities",
    "is_token",
    "commander_legendary",
    "commander_explicit",
    "partner",
    "partner_with",
    "choose_background",
    "friends_forever",
    "doctors_companion",
    "subtypes",
    "alternative_costs",
    "additional_costs",
    "enter_modifiers",
    "prototype",
    "disguise",
    "back_face_power",
    "back_face_toughness",
];

/// Bits read as explicit features after [`EXPLICIT`]: 16 of the type set,
/// 8 of the supertypes, 128 of the keywords.
pub const TYPE_BITS: usize = 16;
/// See [`TYPE_BITS`].
pub const SUPERTYPE_BITS: usize = 8;
/// See [`TYPE_BITS`].
pub const KEYWORD_BITS: usize = 128;

/// Width of [`features`].
pub const WIDTH: usize = EXPLICIT.len() + TYPE_BITS + SUPERTYPE_BITS + KEYWORD_BITS + HASHED;

/// What one walk found.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Walk {
    /// Every key and how often it occurs.
    pub keys: BTreeMap<String, u32>,
}

impl Walk {
    fn add(&mut self, key: String) {
        *self.keys.entry(key).or_default() += 1;
    }

    /// Reads the `Debug` form of one part of a definition under `root`.
    fn read(&mut self, root: &str, debug: &str) {
        let tokens = tokenize(debug);
        let mut at = 0;
        while at < tokens.len() {
            let before = at;
            self.value(&tokens, &mut at, root, root);
            if at == before {
                at += 1;
            }
        }
    }

    /// One value at `tokens[*at]`, read under `field` of `parent`.
    fn value(&mut self, tokens: &[Tok], at: &mut usize, parent: &str, field: &str) {
        let Some(tok) = tokens.get(*at) else {
            return;
        };
        match tok {
            Tok::Ident(name) => {
                *at += 1;
                let next = tokens.get(*at);
                // `Some(x)` is x; `None`, `true` and `false` say nothing a
                // variant would not.
                if name == "Some" && next == Some(&Tok::Open('(')) {
                    *at += 1;
                    self.list(tokens, at, ')', parent, field);
                    return;
                }
                if !matches!(name.as_str(), "None" | "true" | "false") {
                    self.add(format!("{field}>{name}"));
                    self.add(format!("{parent}.{field}>{name}"));
                }
                match next {
                    Some(Tok::Open('{')) => {
                        *at += 1;
                        self.fields(tokens, at, name);
                    }
                    Some(Tok::Open('(')) => {
                        *at += 1;
                        self.list(tokens, at, ')', name, name);
                    }
                    _ => {}
                }
            }
            Tok::Open('[') => {
                *at += 1;
                self.list(tokens, at, ']', parent, field);
            }
            Tok::Open(_) | Tok::Close(_) | Tok::Comma | Tok::Colon | Tok::Str => *at += 1,
            Tok::Num(n) => {
                *at += 1;
                self.add(format!("{field}=#{}", n.clamp(&-9, &20)));
            }
            Tok::Hex(h) => {
                *at += 1;
                self.add(format!("{field}={h}"));
            }
        }
    }

    /// Values separated by commas up to `close`.
    fn list(&mut self, tokens: &[Tok], at: &mut usize, close: char, parent: &str, field: &str) {
        while let Some(tok) = tokens.get(*at) {
            match tok {
                Tok::Close(c) if *c == close => {
                    *at += 1;
                    return;
                }
                Tok::Comma => *at += 1,
                _ => {
                    let before = *at;
                    self.value(tokens, at, parent, field);
                    if *at == before {
                        *at += 1;
                    }
                }
            }
        }
    }

    /// `field: value` pairs up to `}` of a struct variant named `parent`.
    fn fields(&mut self, tokens: &[Tok], at: &mut usize, parent: &str) {
        while let Some(tok) = tokens.get(*at) {
            match tok {
                Tok::Close('}') => {
                    *at += 1;
                    return;
                }
                Tok::Ident(field) if tokens.get(*at + 1) == Some(&Tok::Colon) => {
                    let field = field.clone();
                    *at += 2;
                    self.value(tokens, at, parent, &field);
                }
                _ => *at += 1,
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Tok {
    Ident(String),
    Num(i64),
    Hex(String),
    Str,
    Open(char),
    Close(char),
    Comma,
    Colon,
}

fn tokenize(s: &str) -> Vec<Tok> {
    let chars: Vec<char> = s.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        match c {
            '{' | '(' | '[' => {
                out.push(Tok::Open(c));
                i += 1;
            }
            '}' | ')' | ']' => {
                out.push(Tok::Close(c));
                i += 1;
            }
            ',' => {
                out.push(Tok::Comma);
                i += 1;
            }
            ':' => {
                out.push(Tok::Colon);
                i += 1;
            }
            '"' => {
                i += 1;
                while i < chars.len() && chars[i] != '"' {
                    i += if chars[i] == '\\' { 2 } else { 1 };
                }
                i += 1;
                out.push(Tok::Str);
            }
            c if c.is_ascii_digit()
                || (c == '-' && chars.get(i + 1).is_some_and(char::is_ascii_digit)) =>
            {
                let start = i;
                i += 1;
                if c == '0' && chars.get(i) == Some(&'x') {
                    i += 1;
                    while i < chars.len() && chars[i].is_ascii_hexdigit() {
                        i += 1;
                    }
                    out.push(Tok::Hex(chars[start..i].iter().collect()));
                    continue;
                }
                while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                    i += 1;
                }
                let text: String = chars[start..i].iter().collect();
                out.push(Tok::Num(
                    text.split('.')
                        .next()
                        .and_then(|t| t.parse().ok())
                        .unwrap_or(0),
                ));
            }
            c if c.is_alphabetic() || c == '_' => {
                let start = i;
                while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                    i += 1;
                }
                out.push(Tok::Ident(chars[start..i].iter().collect()));
            }
            _ => i += 1,
        }
    }
    out
}

/// Every key of a card: its abilities, every face's abilities, costs and
/// modifiers, its subtypes, its keywords.
#[must_use]
pub fn walk_card(def: &CardDef) -> Walk {
    let mut w = Walk::default();
    w.read("abilities", &format!("{:?}", def.abilities));
    for face in def.faces {
        w.read("face_abilities", &format!("{:?}", face.abilities));
        w.read(
            "alternative_costs",
            &format!("{:?}", face.alternative_costs),
        );
        w.read("additional_costs", &format!("{:?}", face.additional_costs));
        w.read(
            "mandatory_additional_costs",
            &format!("{:?}", face.mandatory_additional_costs),
        );
        w.read("enter_modifiers", &format!("{:?}", face.enter_modifiers));
        w.read("prototype", &format!("{:?}", face.prototype));
        w.read("kicked_targets", &format!("{:?}", face.kicked_targets));
        for s in face.subtypes {
            w.add(format!("subtype>{}", s.get()));
        }
    }
    keyword_keys(&mut w, def.keywords.bits());
    w
}

/// Every key of a registry token.
#[must_use]
pub fn walk_token(token: &TokenDef) -> Walk {
    let mut w = Walk::default();
    w.read("abilities", &format!("{:?}", token.abilities));
    for s in token.subtypes {
        w.add(format!("subtype>{}", s.get()));
    }
    keyword_keys(&mut w, token.keywords.bits());
    w
}

fn keyword_keys(w: &mut Walk, bits: u128) {
    for b in 0..KEYWORD_BITS {
        if bits >> b & 1 == 1 {
            w.add(format!("keyword>{b}"));
        }
    }
}

/// The mechanics a card uses, as the ladder counts them: the `field>Variant`
/// keys, keywords by bit.
#[must_use]
pub fn mechanics(def: &CardDef) -> Vec<String> {
    walk_card(def)
        .keys
        .into_keys()
        .filter(|k| k.contains('>') && !k.contains('.') && !k.starts_with("subtype>"))
        .collect()
}

/// FNV-1a, 64 bits: a hash that is the same on every machine and in every
/// Rust version, which `std`'s is not promised to be.
fn fnv1a(s: &str) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in s.bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    h
}

#[allow(clippy::cast_precision_loss)] // a mana cost's generic part is small
fn pips(cost: &ManaCost) -> [f32; 7] {
    // generic, W, U, B, R, G, C
    let mut p = [0.0_f32; 7];
    for s in cost.symbols() {
        match s {
            ManaSymbol::Generic(n) => p[0] += n as f32,
            ManaSymbol::White => p[1] += 1.0,
            ManaSymbol::Blue => p[2] += 1.0,
            ManaSymbol::Black => p[3] += 1.0,
            ManaSymbol::Red => p[4] += 1.0,
            ManaSymbol::Green => p[5] += 1.0,
            ManaSymbol::Colorless => p[6] += 1.0,
            _ => {}
        }
    }
    p
}

fn bits_into(out: &mut Vec<f32>, bits: u128, n: usize) {
    for b in 0..n {
        out.push(f32::from(u8::from(bits >> b & 1 == 1)));
    }
}

#[allow(clippy::cast_precision_loss)] // key counts are small
fn hashed_into(out: &mut Vec<f32>, walk: &Walk) {
    let mut buckets = [0.0_f32; HASHED];
    for (key, n) in &walk.keys {
        buckets[(fnv1a(key) % HASHED as u64) as usize] += *n as f32;
    }
    out.extend(buckets.iter().map(|c| c.ln_1p()));
}

/// A card's structural features: [`EXPLICIT`], then the type, supertype and
/// keyword bits, then the hashed keys. [`WIDTH`] long.
#[must_use]
#[allow(clippy::cast_precision_loss)] // small counts and stats
pub fn features(def: &CardDef) -> Vec<f32> {
    let front = &def.faces[0];
    let p = pips(&front.mana_cost);
    let colors = front.mana_cost.colors().bits();
    let identity = def.color_identity.bits();
    let bit = |bits: u8, i: u8| f32::from(u8::from(bits >> i & 1 == 1));
    let stat = |v: Option<i16>| f32::from(v.unwrap_or(0));
    let back = def.faces.get(1);
    let mut out: Vec<f32> = vec![
        front.mana_cost.cmc() as f32,
        p[0],
        p[1],
        p[2],
        p[3],
        p[4],
        p[5],
        p[6],
        f32::from(u8::from(front.mana_cost.has_variable())),
        bit(colors, 0),
        bit(colors, 1),
        bit(colors, 2),
        bit(colors, 3),
        bit(colors, 4),
        bit(identity, 0),
        bit(identity, 1),
        bit(identity, 2),
        bit(identity, 3),
        bit(identity, 4),
        stat(front.power),
        stat(front.toughness),
        f32::from(u8::from(front.power.is_some() || front.toughness.is_some())),
        f32::from(front.loyalty.unwrap_or(0)),
        f32::from(u8::from(front.loyalty.is_some())),
        def.faces.len() as f32,
        (def.abilities.len() + def.faces.iter().map(|f| f.abilities.len()).sum::<usize>()) as f32,
        0.0,
        f32::from(u8::from(matches!(def.commander, CommanderRule::Legendary))),
        f32::from(u8::from(matches!(
            def.commander,
            CommanderRule::ExplicitlyAllowed
        ))),
        f32::from(u8::from(matches!(def.partner, PartnerKind::Partner))),
        f32::from(u8::from(matches!(def.partner, PartnerKind::PartnerWith(_)))),
        f32::from(u8::from(matches!(
            def.partner,
            PartnerKind::ChooseABackground
        ))),
        f32::from(u8::from(matches!(def.partner, PartnerKind::FriendsForever))),
        f32::from(u8::from(matches!(
            def.partner,
            PartnerKind::DoctorsCompanion
        ))),
        front.subtypes.len() as f32,
        front.alternative_costs.len() as f32,
        front.additional_costs.len() as f32,
        front.enter_modifiers.len() as f32,
        f32::from(u8::from(front.prototype.is_some())),
        f32::from(u8::from(front.disguise.is_some())),
        stat(back.and_then(|b| b.power)),
        stat(back.and_then(|b| b.toughness)),
    ];
    debug_assert_eq!(out.len(), EXPLICIT.len());
    bits_into(&mut out, u128::from(front.types.bits()), TYPE_BITS);
    bits_into(
        &mut out,
        u128::from(front.supertypes.bits()),
        SUPERTYPE_BITS,
    );
    bits_into(&mut out, def.keywords.bits(), KEYWORD_BITS);
    hashed_into(&mut out, &walk_card(def));
    out
}

/// A registry token's structural features, in [`features`]' layout.
#[must_use]
pub fn token_features(token: &TokenDef) -> Vec<f32> {
    let stat = |v: Option<i16>| f32::from(v.unwrap_or(0));
    let colors = token.colors.bits();
    let mut out = vec![0.0_f32; EXPLICIT.len()];
    for i in 0..5 {
        out[9 + i] = f32::from(u8::from(colors >> i & 1 == 1));
    }
    out[19] = stat(token.power);
    out[20] = stat(token.toughness);
    out[21] = f32::from(u8::from(token.power.is_some() || token.toughness.is_some()));
    out[24] = 1.0;
    #[allow(clippy::cast_precision_loss)]
    {
        out[25] = token.abilities.len() as f32;
        out[34] = token.subtypes.len() as f32;
    }
    out[26] = 1.0;
    bits_into(&mut out, u128::from(token.types.bits()), TYPE_BITS);
    bits_into(
        &mut out,
        u128::from(token.supertypes.bits()),
        SUPERTYPE_BITS,
    );
    bits_into(&mut out, token.keywords.bits(), KEYWORD_BITS);
    hashed_into(&mut out, &walk_token(token));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn card(name: &str) -> &'static CardDef {
        baylee_cards::by_index(baylee_cards::decks::by_name(name).expect("in the pool")).unwrap()
    }

    #[test]
    fn a_burn_spell_and_a_mana_elf_read_as_what_they_are() {
        let bolt = walk_card(card("Lightning Bolt"));
        for key in [
            "effects>DealDamage",
            "amount>Fixed",
            "DealDamage.amount>Fixed",
            "Fixed=#3",
            "target>AnyTarget",
        ] {
            assert!(
                bolt.keys.contains_key(key),
                "Lightning Bolt lacks {key}: {:?}",
                bolt.keys
            );
        }
        let elves = mechanics(card("Llanowar Elves"));
        assert!(elves.contains(&"effects>AddMana".to_owned()), "{elves:?}");
        assert!(elves.contains(&"parts>TapSelf".to_owned()), "{elves:?}");
        assert!(!elves.iter().any(|m| m.contains("DealDamage")));
    }

    /// The same variant in two enums keys apart: `Fixed` as an amount and as
    /// a mana source.
    #[test]
    fn a_variant_is_keyed_by_the_field_it_fills() {
        let elves = walk_card(card("Llanowar Elves"));
        assert!(elves.keys.contains_key("source>Fixed"));
        assert!(elves.keys.contains_key("amount>Fixed"));
    }

    #[test]
    fn features_have_their_width_for_every_pool_card_and_token() {
        let mut distinct = std::collections::BTreeSet::new();
        for def in baylee_cards::all() {
            let f = features(def);
            assert_eq!(f.len(), WIDTH, "{}", def.name());
            assert!(f.iter().all(|x| x.is_finite()));
            distinct.insert(f.iter().map(|x| x.to_bits()).collect::<Vec<_>>());
        }
        // Different cards read differently: almost every card is its own vector.
        assert!(
            distinct.len() * 10 > baylee_cards::count() * 9,
            "{} distinct",
            distinct.len()
        );
        for t in baylee_cards::tokens::ALL {
            assert_eq!(token_features(t).len(), WIDTH);
        }
    }

    #[test]
    fn the_walk_is_deterministic() {
        let def = card("Swords to Plowshares");
        assert_eq!(walk_card(def), walk_card(def));
        assert_eq!(features(def), features(def));
    }
}
