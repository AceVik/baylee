//! The counters Magic prints that carry no rule of their own.
//!
//! [`CounterKind`] names counters with intrinsic rules: signed P/T,
//! loyalty, Saga lore, suspend/vanishing time, player counters, keyword
//! counters (CR 122.1b), Equipment hone counters (CR 122.1j), and Class levels.
//! An engine that did not know those rules would play the card wrong.
//!
//! A great many other counters are **a name and a number**. A depletion
//! counter does nothing at all; the card that prints it says what happens
//! when it runs out, and the rules have never heard of it. Counted over
//! every `//! Oracle:` header on 2026-09-16, this pool's 1536 cards print
//! **38** distinct counter words and ten of them have a variant here, so
//! twenty-eight do not — in one pool, before any set ships. Giving each one
//! a variant would grow the enum, and with it the wire-stable enum in
//! `baylee-view` that [`crate::CounterKind`] is projected onto, to teach
//! the engine nothing.
//!
//! **A word that earns a rule earns a variant**, and that line is not
//! decorative: shield, stun and finality counters are in those twenty-eight
//! and each has a rule of its own (CR 122.1c, CR 122.1d, CR 122.1h), so
//! each belongs in the enum when it is written and not in the table below.
//! `every_assigned_counter_is_a_custom_id` is where that is enforced.
//!
//! So they are [`CounterKind::Custom`] ids, and this module is the place the
//! ids are **assigned**. Before it there was one: Luminarch Ascension wrote
//! a bare `CounterKind::Custom(1)` for its quest counters, which is a number
//! the next card had no way to know was taken. A constant with a doc comment
//! is the same zero-cost value and says which word it stands for.
//!
//! What a client draws for one of these is still a bare dot — the view
//! carries the id and has no table to turn it into a name. That is a real
//! hole and it is named in `docs/history/engine-gaps.md`; the answer is a counter
//! name table in `GameStatic`, not a variant per word.

use crate::effect::CounterKind;

/// Quest counters (Luminarch Ascension).
pub const QUEST: CounterKind = CounterKind::Custom(1);

/// Depletion counters.
///
/// **Two cycles print the word**, which is the reason this constant exists
/// rather than a number per card. The five Mercadian Masques lands —
/// Hickory Woodlot, Peat Bog, Remote Farm, Sandstone Needle, Saprazzan
/// Skerry — arrive with two and sacrifice themselves at nought. The five
/// Mirage ones — Land Cap, Lava Tubes, River Delta, Timberline Ridge,
/// Veldt — *put* a depletion counter on themselves, do not untap while one
/// is there and take it off at upkeep, which is `engine-gaps.md`'s G5 and
/// is not written yet. When it is, it is this id: two ids for one printed
/// word would be two counters a Doubling Season and a proliferate could
/// tell apart and a player could not.
pub const DEPLETION: CounterKind = CounterKind::Custom(2);

/// Mining counters (Gemstone Mine).
pub const MINING: CounterKind = CounterKind::Custom(3);

/// Storage counters — the seventeen lands that bank mana a turn at a time
/// (Fountain of Cho, Mage-Ring Network, Calciform Pools, …).
pub const STORAGE: CounterKind = CounterKind::Custom(4);

/// Wish counters (Wishclaw Talisman).
pub const WISH: CounterKind = CounterKind::Custom(5);

/// Hatchling counters (Eumidian Hatchery).
pub const HATCHLING: CounterKind = CounterKind::Custom(6);

/// Dread counters (Grasping Shadows // Shadows' Lair).
pub const DREAD: CounterKind = CounterKind::Custom(7);

/// Landmark counters (Treasure Map).
pub const LANDMARK: CounterKind = CounterKind::Custom(8);

/// Void counters (Dauthi Voidwalker).
pub const VOID: CounterKind = CounterKind::Custom(9);

/// Age counters — cumulative upkeep's (CR 702.24a; Mystic Remora). The rules
/// name the counter and give it no rule of its own: the ability that puts it
/// there counts it.
pub const AGE: CounterKind = CounterKind::Custom(10);

/// Corpse counters (Scavenging Ghoul).
pub const CORPSE: CounterKind = CounterKind::Custom(11);

/// Vitality counters (Living Artifact).
pub const VITALITY: CounterKind = CounterKind::Custom(12);

/// Mire counters (Cyclopean Tomb).
pub const MIRE: CounterKind = CounterKind::Custom(13);

/// Matrix counters (Life Matrix).
pub const MATRIX: CounterKind = CounterKind::Custom(14);

/// Carrion counters (Osai Vultures).
pub const CARRION: CounterKind = CounterKind::Custom(15);

/// Oil counters (The Monumental Facade).
pub const OIL: CounterKind = CounterKind::Custom(16);

/// Pressure counters (Hellion Crucible).
pub const PRESSURE: CounterKind = CounterKind::Custom(17);

/// Pupa counters (Cocoon).
pub const PUPA: CounterKind = CounterKind::Custom(18);
/// Sleep counters (Venarian Gold).
pub const SLEEP: CounterKind = CounterKind::Custom(19);
/// Pin counters (Voodoo Doll).
pub const PIN: CounterKind = CounterKind::Custom(20);
/// Dream counters (Rasputin Dreamweaver).
pub const DREAM: CounterKind = CounterKind::Custom(21);

/// Doom counters accumulated by Armageddon Clock.
pub const DOOM: CounterKind = CounterKind::Custom(22);
/// Eon counters mark stored turns on Magosi.
pub const EON: CounterKind = CounterKind::Custom(23);

/// Burden counters accumulated by The One Ring.
pub const BURDEN: CounterKind = CounterKind::Custom(24);

/// Intervention counters (Divine Intervention: the game is a draw when the
/// last one is removed).
pub const INTERVENTION: CounterKind = CounterKind::Custom(25);

/// Wind counters (Cyclone).
pub const WIND: CounterKind = CounterKind::Custom(26);

/// Glyph counters (Glyph of Delusion).
pub const GLYPH: CounterKind = CounterKind::Custom(27);

/// Scream counters (All Hallow's Eve).
pub const SCREAM: CounterKind = CounterKind::Custom(28);

/// Soul counters (Hostile Hostel).
pub const SOUL: CounterKind = CounterKind::Custom(29);

/// Luck counters (Gemstone Caverns).
pub const LUCK: CounterKind = CounterKind::Custom(30);

/// Ice counters (Dark Depths).
pub const ICE: CounterKind = CounterKind::Custom(31);

/// Hunger counters (Fasting).
pub const HUNGER: CounterKind = CounterKind::Custom(32);

/// Spore counters (Thallid, Elvish Farmer, Spore Flower, Fungal Bloom, …).
pub const SPORE: CounterKind = CounterKind::Custom(33);

/// Javelin counters (Icatian Javelineers).
pub const JAVELIN: CounterKind = CounterKind::Custom(34);

/// Credit counters (Icatian Moneychanger).
pub const CREDIT: CounterKind = CounterKind::Custom(35);

/// Tide counters (Homarid, Tidal Influence).
pub const TIDE: CounterKind = CounterKind::Custom(36);

/// Cube counters (Delif's Cube).
pub const CUBE: CounterKind = CounterKind::Custom(37);

/// Net counters (Merseine).
pub const NET: CounterKind = CounterKind::Custom(38);

/// Healing counters (Fylgja).
pub const HEALING: CounterKind = CounterKind::Custom(39);

/// Sleight counters (Chromatic Armor).
pub const SLEIGHT: CounterKind = CounterKind::Custom(40);

/// Paralyzation counters (Dread Wight).
pub const PARALYZATION: CounterKind = CounterKind::Custom(41);

/// Music counters (Musician).
pub const MUSIC: CounterKind = CounterKind::Custom(42);

/// Arrowhead counters (Serrated Arrows).
pub const ARROWHEAD: CounterKind = CounterKind::Custom(43);

/// Currency counters (Trade Caravan).
pub const CURRENCY: CounterKind = CounterKind::Custom(44);

/// Infection counters (Diseased Vermin).
pub const INFECTION: CounterKind = CounterKind::Custom(45);

/// Wage counters (Rogue Skycaptain).
pub const WAGE: CounterKind = CounterKind::Custom(46);

/// Velocity counters (Tornado).
pub const VELOCITY: CounterKind = CounterKind::Custom(47);

/// Vortex counters (Energy Vortex).
pub const VORTEX: CounterKind = CounterKind::Custom(48);

/// Growth counters (Malignant Growth).
pub const GROWTH: CounterKind = CounterKind::Custom(49);

/// Fungus counters (Mindbender Spores).
pub const FUNGUS: CounterKind = CounterKind::Custom(50);

/// Echo counters (Soul Echo).
pub const ECHO: CounterKind = CounterKind::Custom(51);

/// Death counters (Bogardan Phoenix).
pub const DEATH: CounterKind = CounterKind::Custom(52);

/// Rust counters (Corrosion).
pub const RUST: CounterKind = CounterKind::Custom(53);

/// Fuse counters (Goblin Bomb).
pub const FUSE: CounterKind = CounterKind::Custom(54);

/// Shell counters (Roc Hatchling).
pub const SHELL: CounterKind = CounterKind::Custom(55);

/// Bounty counters (Bounty Hunter).
pub const BOUNTY: CounterKind = CounterKind::Custom(56);

/// Elixir counters (Essence Bottle).
pub const ELIXIR: CounterKind = CounterKind::Custom(57);

/// Treasure counters (Legacy's Allure).
pub const TREASURE: CounterKind = CounterKind::Custom(58);

/// Magnet counters (Magnetic Web).
pub const MAGNET: CounterKind = CounterKind::Custom(59);

/// Pain counters (Torture Chamber).
pub const PAIN: CounterKind = CounterKind::Custom(60);

/// Study counters (Pursuit of Knowledge).
pub const STUDY: CounterKind = CounterKind::Custom(61);

/// Page counters (Barrin's Codex).
pub const PAGE: CounterKind = CounterKind::Custom(62);

/// Verse counters (Discordant Dirge, Serra's Hymn, Recantation, …).
pub const VERSE: CounterKind = CounterKind::Custom(63);

/// Petal counters (Lotus Blossom).
pub const PETAL: CounterKind = CounterKind::Custom(64);

/// Soot counters (Smokestack).
pub const SOOT: CounterKind = CounterKind::Custom(65);

/// Arrow counters (Archery Training).
pub const ARROW: CounterKind = CounterKind::Custom(66);

/// Fade counters (Fading N, CR 702.32a).
pub const FADE: CounterKind = CounterKind::Custom(67);

/// Feather counters (Soulcatchers' Aerie).
pub const FEATHER: CounterKind = CounterKind::Custom(68);

/// Fate counters (Oblivion Stone).
pub const FATE: CounterKind = CounterKind::Custom(69);

/// Every id this module assigns, with the word it stands for.
///
/// It exists so the ids can be checked rather than trusted, which is the
/// whole difference between a registry and a handful of numbers that happen not to
/// clash today.
pub const ASSIGNED: &[(&str, CounterKind)] = &[
    ("quest", QUEST),
    ("depletion", DEPLETION),
    ("mining", MINING),
    ("storage", STORAGE),
    ("wish", WISH),
    ("hatchling", HATCHLING),
    ("dread", DREAD),
    ("landmark", LANDMARK),
    ("void", VOID),
    ("age", AGE),
    ("corpse", CORPSE),
    ("vitality", VITALITY),
    ("mire", MIRE),
    ("matrix", MATRIX),
    ("carrion", CARRION),
    ("oil", OIL),
    ("pressure", PRESSURE),
    ("pupa", PUPA),
    ("sleep", SLEEP),
    ("pin", PIN),
    ("dream", DREAM),
    ("doom", DOOM),
    ("eon", EON),
    ("burden", BURDEN),
    ("intervention", INTERVENTION),
    ("wind", WIND),
    ("glyph", GLYPH),
    ("scream", SCREAM),
    ("soul", SOUL),
    ("luck", LUCK),
    ("ice", ICE),
    ("hunger", HUNGER),
    ("spore", SPORE),
    ("javelin", JAVELIN),
    ("credit", CREDIT),
    ("tide", TIDE),
    ("cube", CUBE),
    ("net", NET),
    ("healing", HEALING),
    ("sleight", SLEIGHT),
    ("paralyzation", PARALYZATION),
    ("music", MUSIC),
    ("arrowhead", ARROWHEAD),
    ("currency", CURRENCY),
    ("infection", INFECTION),
    ("wage", WAGE),
    ("velocity", VELOCITY),
    ("vortex", VORTEX),
    ("growth", GROWTH),
    ("fungus", FUNGUS),
    ("echo", ECHO),
    ("death", DEATH),
    ("rust", RUST),
    ("fuse", FUSE),
    ("shell", SHELL),
    ("bounty", BOUNTY),
    ("elixir", ELIXIR),
    ("treasure", TREASURE),
    ("magnet", MAGNET),
    ("pain", PAIN),
    ("study", STUDY),
    ("page", PAGE),
    ("verse", VERSE),
    ("petal", PETAL),
    ("soot", SOOT),
    ("arrow", ARROW),
    ("fade", FADE),
    ("feather", FEATHER),
    ("fate", FATE),
];

#[cfg(test)]
mod tests {
    use super::{ASSIGNED, CounterKind};

    /// Two words sharing an id would be one counter wearing two names: a
    /// permanent carrying both would count each as the other, and nothing
    /// in the engine could tell them apart, because to the engine an id is
    /// all a custom counter is.
    #[test]
    fn no_two_counter_words_share_an_id() {
        for (i, (word, kind)) in ASSIGNED.iter().enumerate() {
            for (other_word, other_kind) in &ASSIGNED[i + 1..] {
                assert_ne!(
                    kind, other_kind,
                    "{word} and {other_word} are the same counter"
                );
            }
        }
    }

    /// The word is what a *reader* matches on, and the constant that holds
    /// the id is that word in capitals — which is only well defined while
    /// every word is lowercase ASCII with nothing in it that shifts case
    /// oddly. `scriptgen::assigned_counter` turns a reference script's
    /// `CounterType$ STORAGE` into `counters::STORAGE` by exactly that
    /// rule, so a word written any other way here would emit a constant
    /// that does not exist.
    #[test]
    fn every_assigned_word_is_lowercase_ascii() {
        for (word, _) in ASSIGNED {
            assert!(
                !word.is_empty()
                    && word
                        .chars()
                        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()),
                "{word} is not a lowercase ASCII word"
            );
        }
    }

    /// And every one of them is a `Custom` id, which is what says this
    /// module is about the counters the rules have nothing to say about.
    /// A word that earns a rule earns a variant instead, and moving it
    /// would fail here.
    #[test]
    fn every_assigned_counter_is_a_custom_id() {
        for (word, kind) in ASSIGNED {
            assert!(
                matches!(kind, CounterKind::Custom(_)),
                "{word} is a named kind and does not belong in this table"
            );
        }
    }
}
