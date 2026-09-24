//! Printed card text, and the one rule that says which translated sentence
//! stands for which English one.
//!
//! A client draws an ability as the sentence the card prints for it. The
//! index of that sentence is computed once, by codegen, against the English
//! Oracle text (`baylee_cards::lines`); what a player reads is a printing in
//! their own language, and Scryfall's translations are not the Oracle cut
//! into another language line for line. A modal double-faced card's German
//! face carries the other face's type and ability as two extra lines, a
//! Class prints `//Level_2//` markers, an Adventure prints its second half
//! after `//ADV//`, a borderless frame drops the reminder line a regular one
//! prints, and old printings spell `{2}{B}{B}` as `{2BB}` or print two
//! abilities in the other order. Counting lines and refusing on a mismatch —
//! the rule this crate replaces — drew 146 of 2021 German sheet faces blank.
//!
//! So the question is answered in four steps, each a function here:
//!
//! 1. [`pick`] — which printing of the card speaks for the language: one
//!    that is really translated ([`untranslated`]), preferably one whose
//!    every face lines up, then the newest, then a tie-break that makes the
//!    answer the same on every run;
//! 2. [`align`] — which of its lines stands for which Oracle line: raw, then
//!    with markers and the double-faced hint removed, then with reminder-only
//!    lines set aside on both sides, in that order and never the other way
//!    round, because a later stage can undo an alignment an earlier one had;
//! 3. [`verify`] — whether the line in that position is the same ability,
//!    by the symbols its cost prints, and which line it is when the
//!    printing swapped two;
//! 4. [`split_cost`] and [`licensed`] — where the printed cost ends, so the
//!    cost column is the card's own words and never the client's.
//!
//! Whatever falls out of the chain falls to the **English Oracle sentence**,
//! which is the owner's rule for every row a client draws from card text:
//! localized first, English second, and never a blank row or a wording the
//! client made up. Nothing here returns a sentence it could not pair.
//!
//! # Why a crate of its own
//!
//! Both ends need the same answer and neither can see the other. The catalog
//! picks the printing it serves and cannot link a renderer's crate; the
//! client aligns what it was served (and what Scryfall answered when there
//! was no gateway) and cannot link an ORM. A rule written twice drifts, and a
//! drifted alignment does not fail — it points at the sentence beside the
//! right one. This crate depends on serde and nothing else, so the catalog,
//! `baylee-core` and a wasm client can all link it.
//!
//! The wire shape between the catalog and a client lives here for the same
//! reason: it was written twice, with a JSON pin on each side to keep the two
//! copies equal. So does [`card_entry`], which builds that shape from a
//! card's printings: the gateway serves it from the catalog's rows, and a
//! client without a translated gateway builds it from Scryfall's answer.

mod align;
mod cost;
mod entry;
mod pick;
mod text;
mod verify;
mod wire;

pub use align::{Aligned, Stage, align, untranslated};
pub use cost::{Split, licensed, split_cost};
pub use entry::{TextFace, TextPrinting, card_entry};
pub use pick::{Layer, Printing, pick};
pub use text::{repair_braces, sentence_count, sentences, symbols};
pub use verify::{Verdict, localized, verify};
pub use wire::{CardTextEntry, FaceText};
