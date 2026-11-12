//! What the card that opens over a person's name says.
//!
//! The card is a description: who this is, how this account stands to them
//! and what it has been told about them. It says only what the client holds
//! (the friends list, the requests, the recent players, the nickname); the
//! gateway tells nobody another player's games or wins, so the one line of
//! statistics is on the account's own card, from its own decks' counts on
//! this device.

use baylee_protocol::social::Presence;

use super::{ChatPhrase, Social};
use crate::i18n::Lang;

/// A day, in milliseconds.
const DAY_MS: u64 = 86_400_000;

/// How this account stands to the person a card is about.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Relation {
    /// The card is about this account.
    Me,
    /// A friend, since this time (Unix milliseconds).
    Friend {
        /// Since when.
        since: u64,
    },
    /// They asked to be friends; the answer is this account's.
    AskedYou,
    /// This account asked them.
    Asked,
    /// Blocked by this account.
    Blocked,
    /// Nobody in particular.
    Stranger,
}

/// A person, as their card describes them.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct PersonCard {
    /// `Alice#af03`: what the avatar is drawn for.
    pub handle: String,
    /// The handle without its tag.
    pub name: String,
    /// The tag, where the handle has one.
    pub tag: Option<String>,
    /// What this account calls them, where that is not their name.
    pub nickname: Option<String>,
    /// A guest, who has no account.
    pub guest: bool,
    /// How this account stands to them.
    pub relation: Relation,
    /// Their presence, where this account is told it: a friend's.
    pub presence: Option<Presence>,
    /// When this account last sat at a table with them, Unix milliseconds.
    pub played: Option<u64>,
}

impl Social {
    /// The card of the person a handle names. `guest` is what the line or
    /// the row that named them said: a guest is in no list to look it up in.
    #[must_use]
    pub fn person_card(&self, handle: &str, guest: bool) -> PersonCard {
        let (name, tag) = match handle.rsplit_once('#') {
            Some((name, tag)) if baylee_protocol::social::valid_tag(tag) => {
                (name.to_owned(), Some(tag.to_owned()))
            }
            _ => (handle.to_owned(), None),
        };
        let friends = self.friends();
        let among = |people: &[baylee_protocol::social::Person]| {
            tag.as_deref()
                .is_some_and(|tag| people.iter().any(|p| p.tag == tag))
        };
        let friend = tag.as_deref().and_then(|tag| {
            friends
                .online
                .iter()
                .chain(&friends.offline)
                .find(|friend| friend.person.tag == tag)
        });
        let me = self.me();
        let relation = if tag.as_deref() == Some(me.tag.as_str()) || handle == me.handle {
            Relation::Me
        } else if let Some(friend) = friend {
            Relation::Friend {
                since: friend.since,
            }
        } else if among(friends.blocked) {
            Relation::Blocked
        } else if among(friends.incoming) {
            Relation::AskedYou
        } else if among(friends.outgoing) {
            Relation::Asked
        } else {
            Relation::Stranger
        };
        let nickname = tag
            .as_deref()
            .and_then(|tag| self.nickname(tag))
            .filter(|nick| *nick != handle && *nick != name)
            .map(str::to_owned);
        let played = tag.as_deref().and_then(|tag| {
            self.recent_players()
                .iter()
                .filter(|player| player.tag == tag)
                .map(|player| player.at)
                .max()
        });
        PersonCard {
            handle: handle.to_owned(),
            name,
            tag,
            nickname,
            guest,
            relation,
            presence: friend.map(|friend| friend.presence.clone()),
            played,
        }
    }
}

impl PersonCard {
    /// How this account stands to them, in words, at `now` (Unix
    /// milliseconds); nothing for a stranger.
    #[must_use]
    pub fn relation_words(&self, lang: Lang, now: u64) -> Option<String> {
        match self.relation {
            Relation::Me => Some(ChatPhrase::CardYou.text(lang).to_owned()),
            Relation::Friend { since } => Some(friend_for(lang, now.saturating_sub(since))),
            Relation::AskedYou => Some(ChatPhrase::CardAskedYou.text(lang).to_owned()),
            Relation::Asked => Some(ChatPhrase::CardAsked.text(lang).to_owned()),
            Relation::Blocked => Some(ChatPhrase::CardBlocked.text(lang).to_owned()),
            Relation::Stranger => self
                .guest
                .then(|| ChatPhrase::CardGuest.text(lang).to_owned()),
        }
    }

    /// When this account last played with them, in words; nothing if it
    /// never did, and nothing on its own card.
    #[must_use]
    pub fn played_words(&self, lang: Lang, now: u64) -> Option<String> {
        if self.relation == Relation::Me {
            return None;
        }
        let days = now.saturating_sub(self.played?) / DAY_MS;
        Some(match usize::try_from(days) {
            Ok(0) => ChatPhrase::CardPlayedToday.text(lang).to_owned(),
            Ok(days) => ChatPhrase::CardPlayedDays.text_count(lang, days),
            Err(_) => ChatPhrase::CardPlayedDays.text_count(lang, usize::MAX),
        })
    }
}

/// "Friends for 3 days", by the largest whole unit: days up to two months,
/// months up to two years, then years.
fn friend_for(lang: Lang, ms: u64) -> String {
    let days = usize::try_from(ms / DAY_MS).unwrap_or(usize::MAX);
    match days {
        0 => ChatPhrase::CardFriendToday.text(lang).to_owned(),
        1..=60 => ChatPhrase::CardFriendDays.text_count(lang, days),
        61..=730 => ChatPhrase::CardFriendMonths.text_count(lang, days / 30),
        _ => ChatPhrase::CardFriendYears.text_count(lang, days / 365),
    }
}

/// The account's own card's line of statistics: what its decks have played
/// on this device, summed.
#[must_use]
pub fn own_games(lang: Lang, games: u32, wins: u32) -> String {
    if games == 0 {
        return ChatPhrase::CardNoGames.text(lang).to_owned();
    }
    ChatPhrase::CardOwnGames.fill_count(lang, games as usize, &[&wins.to_string()])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn card(relation: Relation, played: Option<u64>, guest: bool) -> PersonCard {
        PersonCard {
            handle: "Alice#af03".into(),
            name: "Alice".into(),
            tag: Some("af03".into()),
            nickname: None,
            guest,
            relation,
            presence: None,
            played,
        }
    }

    /// A friendship is said in its largest whole unit, in both languages,
    /// and never as a count of zero.
    #[test]
    fn a_friendship_is_said_in_its_largest_whole_unit() {
        let now = 1_000 * DAY_MS;
        let since = |days: u64| Relation::Friend {
            since: now - days * DAY_MS,
        };
        let says = |days: u64, lang: Lang| {
            card(since(days), None, false)
                .relation_words(lang, now)
                .expect("a friend has a line")
        };
        assert_eq!(says(0, Lang::En), "Friends since today");
        assert_eq!(says(1, Lang::En), "Friends for 1 day");
        assert_eq!(says(45, Lang::En), "Friends for 45 days");
        assert_eq!(says(61, Lang::En), "Friends for 2 months");
        assert_eq!(says(400, Lang::En), "Friends for 13 months");
        assert_eq!(says(800, Lang::En), "Friends for 2 years");
        assert_eq!(says(1, Lang::De), "Seit 1 Tag befreundet");
        assert_eq!(says(3, Lang::De), "Seit 3 Tagen befreundet");
        // A clock set back does not make a friendship negative.
        let ahead = card(
            Relation::Friend {
                since: now + DAY_MS,
            },
            None,
            false,
        );
        assert_eq!(
            ahead.relation_words(Lang::En, now).as_deref(),
            Some("Friends since today")
        );
    }

    /// A stranger has no line of their own unless they are a guest, and the
    /// account's own card says so and never when it last played itself.
    #[test]
    fn a_stranger_says_nothing_and_a_guest_says_so() {
        let now = 10 * DAY_MS;
        assert_eq!(
            card(Relation::Stranger, None, false).relation_words(Lang::En, now),
            None
        );
        assert_eq!(
            card(Relation::Stranger, None, true)
                .relation_words(Lang::En, now)
                .as_deref(),
            Some("Guest · cannot be added")
        );
        let me = card(Relation::Me, Some(now), false);
        assert_eq!(
            me.relation_words(Lang::De, now).as_deref(),
            Some("Das bist du")
        );
        assert_eq!(me.played_words(Lang::En, now), None);
        let played = card(Relation::Stranger, Some(now - 3 * DAY_MS), false);
        assert_eq!(
            played.played_words(Lang::En, now).as_deref(),
            Some("Played together 3 days ago")
        );
        assert_eq!(
            card(Relation::Stranger, Some(now - 1000), false)
                .played_words(Lang::De, now)
                .as_deref(),
            Some("Heute zusammen gespielt")
        );
    }

    /// The account's own statistics are its decks' counts, in words.
    #[test]
    fn the_own_cards_statistics_are_counted_in_words() {
        assert_eq!(own_games(Lang::En, 0, 0), "No games played yet");
        assert_eq!(own_games(Lang::En, 1, 1), "1 game · 1 won");
        assert_eq!(own_games(Lang::En, 42, 24), "42 games · 24 won");
        assert_eq!(own_games(Lang::De, 42, 24), "42 Spiele · 24 gewonnen");
    }
}
