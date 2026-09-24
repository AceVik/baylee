//! How a printed oracle text is cut into sentences.
//!
//! Codegen writes a per-ability *sentence index* from the English oracle
//! text; a client resolves that index against the printed text of whatever
//! printing and language the player chose. The split both ends count with
//! lives in `baylee-cardtext`, beside the rule that pairs a translated
//! sentence with its Oracle one, and is re-exported here so that everything
//! that already reached it through `baylee-core` still does.

pub use baylee_cardtext::{sentence_count, sentences};
