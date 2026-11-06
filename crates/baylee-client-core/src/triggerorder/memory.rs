//! What the player taught: "this ability before that one", for the pairs
//! they turned around against a suggestion, and nothing else.
//!
//! It travels with the account in [`crate::prefs::Preferences`], which needs
//! nothing new from a server: a gateway keeps the whole object under
//! `MAX_SETTINGS_BYTES`, and Central keeps it with the account. That bound
//! is why a pair is one short string and why the list has an end
//! ([`MAX_PAIRS`]): the oldest teaching goes first.

use super::TriggerKey;
use baylee_core::ids::{AbilityRef, CardIndex};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// How many pairs are kept, the oldest given up first: at about twenty
/// bytes each, a small share of what a gateway keeps for a player.
pub const MAX_PAIRS: usize = 128;

/// One taught pair.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct Pair {
    first: TriggerKey,
    then: TriggerKey,
}

/// The player's teaching, and what they allow it to do.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct TriggerMemory {
    /// Oldest first, each pair of abilities at most once.
    #[serde(serialize_with = "write_pairs", deserialize_with = "read_pairs")]
    pairs: Vec<Pair>,
    /// Goes on without asking when every part of a question has been
    /// answered before. Off unless the player turns it on.
    pub accept_known: bool,
}

impl TriggerMemory {
    /// What is taught, the newest first: the newest has the most say.
    pub fn pairs_newest_first(&self) -> impl Iterator<Item = (TriggerKey, TriggerKey)> + '_ {
        self.pairs.iter().rev().map(|pair| (pair.first, pair.then))
    }

    /// How many pairs are kept.
    #[must_use]
    pub fn len(&self) -> usize {
        self.pairs.len()
    }

    /// Whether nothing is taught.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.pairs.is_empty()
    }

    /// Whether `a` goes before `b`, if the player ever said.
    #[must_use]
    pub fn knows(&self, a: TriggerKey, b: TriggerKey) -> Option<bool> {
        self.pairs.iter().find_map(|pair| {
            if pair.first == a && pair.then == b {
                Some(true)
            } else if pair.first == b && pair.then == a {
                Some(false)
            } else {
                None
            }
        })
    }

    /// Takes what the player did with a suggestion, both lists in resolution
    /// order: every two abilities they turned around become a pair, the
    /// later word over the earlier. An ability they split, one copy on each
    /// side of the other, says nothing. Answers whether anything was learned,
    /// so a plain confirmation writes no settings.
    pub fn learn(
        &mut self,
        suggested: &[Option<TriggerKey>],
        chosen: &[Option<TriggerKey>],
    ) -> bool {
        let mut keys: Vec<TriggerKey> = chosen.iter().flatten().copied().collect();
        keys.sort_unstable();
        keys.dedup();
        let mut learned = false;
        for (index, &a) in keys.iter().enumerate() {
            for &b in &keys[index + 1..] {
                let was = whole_before(suggested, a, b);
                let now = whole_before(chosen, a, b);
                match now {
                    Some(first) if was != now => {
                        let (first, then) = if first { (a, b) } else { (b, a) };
                        self.teach(first, then);
                        learned = true;
                    }
                    _ => {}
                }
            }
        }
        learned
    }

    fn teach(&mut self, first: TriggerKey, then: TriggerKey) {
        self.pairs.retain(|pair| {
            !(pair.first == first && pair.then == then || pair.first == then && pair.then == first)
        });
        self.pairs.push(Pair { first, then });
        if self.pairs.len() > MAX_PAIRS {
            self.pairs.remove(0);
        }
    }

    /// Forgets everything taught, and keeps what the player allowed.
    pub fn forget(&mut self) {
        self.pairs.clear();
    }
}

/// `Some(true)` when every `a` comes before every `b`, `Some(false)` the
/// other way round, and `None` when they are mixed or one is missing.
fn whole_before(order: &[Option<TriggerKey>], a: TriggerKey, b: TriggerKey) -> Option<bool> {
    let span = |key: TriggerKey| {
        let first = order.iter().position(|k| *k == Some(key))?;
        let last = order.iter().rposition(|k| *k == Some(key))?;
        Some((first, last))
    };
    let (a_first, a_last) = span(a)?;
    let (b_first, b_last) = span(b)?;
    if a_last < b_first {
        Some(true)
    } else if b_last < a_first {
        Some(false)
    } else {
        None
    }
}

impl std::fmt::Display for TriggerKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Card(ability) => write!(f, "c{}.{}", ability.card.get(), ability.index),
            Self::Token { token, index } => write!(f, "t{token}.{index}"),
        }
    }
}

impl std::str::FromStr for TriggerKey {
    type Err = ();

    fn from_str(text: &str) -> Result<Self, ()> {
        let (number, index) = text.get(1..).ok_or(())?.split_once('.').ok_or(())?;
        let index = index.parse().map_err(|_| ())?;
        match text.as_bytes().first() {
            Some(b'c') => {
                let card = CardIndex::new(number.parse().map_err(|_| ())?);
                Ok(Self::Card(AbilityRef::new(card, index)))
            }
            Some(b't') => Ok(Self::Token {
                token: number.parse().map_err(|_| ())?,
                index,
            }),
            _ => Err(()),
        }
    }
}

fn write_pairs<S: Serializer>(pairs: &[Pair], serializer: S) -> Result<S::Ok, S::Error> {
    serializer.collect_seq(
        pairs
            .iter()
            .map(|pair| format!("{}>{}", pair.first, pair.then)),
    )
}

/// Every pair it can read. One it cannot is dropped and not an error: this
/// sits inside the account's preferences, and a sign-in refuses preferences
/// it cannot read, so one bad entry would lock the player out.
fn read_pairs<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<Pair>, D::Error> {
    let written = Vec::<serde_json::Value>::deserialize(deserializer)?;
    let mut pairs: Vec<Pair> = Vec::new();
    for entry in &written {
        let Some((first, then)) = entry.as_str().and_then(|text| text.split_once('>')) else {
            continue;
        };
        let (Ok(first), Ok(then)) = (first.parse(), then.parse()) else {
            continue;
        };
        if first != then {
            pairs.retain(|pair| {
                !(pair.first == first && pair.then == then
                    || pair.first == then && pair.then == first)
            });
            pairs.push(Pair { first, then });
        }
    }
    if pairs.len() > MAX_PAIRS {
        pairs.drain(..pairs.len() - MAX_PAIRS);
    }
    Ok(pairs)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(card: u32) -> TriggerKey {
        TriggerKey::Card(AbilityRef::new(CardIndex::new(card), 0))
    }

    fn keys(cards: &[u32]) -> Vec<Option<TriggerKey>> {
        cards.iter().map(|card| Some(key(*card))).collect()
    }

    #[test]
    fn only_what_the_player_turned_around_is_learned() {
        let mut memory = TriggerMemory::default();
        assert!(memory.learn(&keys(&[1, 2, 3]), &keys(&[1, 3, 2])));
        assert_eq!(memory.len(), 1);
        assert_eq!(memory.knows(key(3), key(2)), Some(true));
        assert_eq!(memory.knows(key(2), key(3)), Some(false));
        assert_eq!(memory.knows(key(1), key(2)), None);
    }

    #[test]
    fn a_plain_confirmation_learns_nothing() {
        let mut memory = TriggerMemory::default();
        assert!(!memory.learn(&keys(&[1, 2, 3]), &keys(&[1, 2, 3])));
        assert!(memory.is_empty());
    }

    #[test]
    fn a_later_change_overwrites_the_earlier_one() {
        let mut memory = TriggerMemory::default();
        memory.learn(&keys(&[1, 2]), &keys(&[2, 1]));
        memory.learn(&keys(&[2, 1]), &keys(&[1, 2]));
        assert_eq!(memory.len(), 1);
        assert_eq!(memory.knows(key(1), key(2)), Some(true));
    }

    #[test]
    fn an_ability_split_around_another_says_nothing() {
        let mut memory = TriggerMemory::default();
        assert!(!memory.learn(&keys(&[1, 1, 2]), &keys(&[1, 2, 1])));
        assert!(memory.is_empty());
    }

    #[test]
    fn two_copies_moved_together_are_one_pair() {
        let mut memory = TriggerMemory::default();
        assert!(memory.learn(&keys(&[1, 1, 2]), &keys(&[2, 1, 1])));
        assert_eq!(memory.len(), 1);
        assert_eq!(memory.knows(key(2), key(1)), Some(true));
    }

    #[test]
    fn an_ability_nothing_names_is_left_out() {
        let mut memory = TriggerMemory::default();
        let suggested = vec![None, Some(key(1)), Some(key(2))];
        let chosen = vec![Some(key(2)), None, Some(key(1))];
        assert!(memory.learn(&suggested, &chosen));
        assert_eq!(memory.len(), 1);
    }

    #[test]
    fn the_oldest_teaching_goes_first() {
        let mut memory = TriggerMemory::default();
        for card in 0..u32::try_from(MAX_PAIRS).unwrap() + 1 {
            memory.teach(key(card * 2 + 1), key(card * 2 + 2));
        }
        assert_eq!(memory.len(), MAX_PAIRS);
        assert_eq!(memory.knows(key(1), key(2)), None);
        assert_eq!(memory.knows(key(3), key(4)), Some(true));
    }

    #[test]
    fn forgetting_keeps_what_the_player_allowed() {
        let mut memory = TriggerMemory {
            accept_known: true,
            ..TriggerMemory::default()
        };
        memory.teach(key(1), key(2));
        memory.forget();
        assert!(memory.is_empty());
        assert!(memory.accept_known);
    }

    #[test]
    fn it_survives_the_trip_through_the_settings() {
        let mut memory = TriggerMemory::default();
        memory.teach(key(7), TriggerKey::Token { token: 3, index: 1 });
        memory.teach(key(9), key(7));
        let text = serde_json::to_string(&memory).unwrap();
        assert_eq!(
            text,
            r#"{"pairs":["c7.0>t3.1","c9.0>c7.0"],"accept_known":false}"#
        );
        assert_eq!(
            serde_json::from_str::<TriggerMemory>(&text).unwrap(),
            memory
        );
    }

    #[test]
    fn a_bad_entry_costs_that_entry_and_never_the_sign_in() {
        let read: TriggerMemory =
            serde_json::from_str(r#"{"pairs":["c1.0>c2.0",7,"nonsense","c3>c4.0","c5.0>c5.0"]}"#)
                .unwrap();
        assert_eq!(read.len(), 1);
        assert_eq!(read.knows(key(1), key(2)), Some(true));
    }

    #[test]
    fn a_full_memory_fits_beside_the_rest_of_the_settings() {
        let mut memory = TriggerMemory::default();
        for card in 0..u32::try_from(MAX_PAIRS).unwrap() {
            memory.teach(key(40_000 + card), key(50_000 + card));
        }
        let bytes = serde_json::to_string(&memory).unwrap().len();
        assert!(bytes < 4 * 1024, "{bytes} bytes of a gateway's 16 KiB");
    }
}
