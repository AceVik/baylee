//! What the friend-invite sheet says and decides (`docs/client.md`
//! §"Inviting friends"), without a renderer: the validity a player may
//! choose, the link a code makes, which invites a filter shows and the words
//! for an invite's state. Central's own answers are `central_api::social`.

use crate::i18n::{Lang, Phrase};
use crate::social::ChatPhrase;
use central_api::social::{FriendInviteStatus, FriendInviteSummary, Person};

/// The validities the sheet offers, in days.
pub const DAYS: [u8; 4] = [1, 3, 7, 14];

/// The validity chosen until the player picks another: Central's default.
pub const DEFAULT_DAYS: u8 = central_api::social::DEFAULT_FRIEND_INVITE_DAYS;

/// The link a code makes: the website's invite page in the player's
/// language, which anyone can open, with or without an account and with or
/// without the client.
#[must_use]
pub fn share_url(website: &str, lang: Lang, code: &str) -> String {
    format!(
        "{}/{}/invite/{code}",
        website.trim_end_matches('/'),
        lang.code()
    )
}

/// Which invites the list shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Filter {
    /// Those whose link still works.
    #[default]
    Open,
    /// Those someone took up.
    Taken,
    /// Every one Central lists.
    All,
}

impl Filter {
    /// The three, in the order the sheet shows them.
    pub const ALL: [Self; 3] = [Self::Open, Self::Taken, Self::All];

    /// Whether an invite in `status` is shown.
    #[must_use]
    pub fn shows(self, status: FriendInviteStatus) -> bool {
        match self {
            Self::Open => status == FriendInviteStatus::Active,
            Self::Taken => status == FriendInviteStatus::Redeemed,
            Self::All => true,
        }
    }

    /// The filter's name.
    #[must_use]
    pub fn label(self, lang: Lang) -> &'static str {
        match self {
            Self::Open => ChatPhrase::InviteFilterOpen.text(lang),
            Self::Taken => ChatPhrase::InviteFilterTaken.text(lang),
            Self::All => ChatPhrase::InviteFilterAll.text(lang),
        }
    }
}

/// How many invites are open: what counts against the account's limit.
#[must_use]
pub fn open_count(invites: &[FriendInviteSummary]) -> usize {
    invites
        .iter()
        .filter(|invite| invite.status == FriendInviteStatus::Active)
        .count()
}

/// A Baylee Central account's tag as every handle spells it: four hex
/// digits or more (`00af`).
#[must_use]
pub fn tag(person: &Person) -> String {
    format!("{:04x}", person.tag)
}

/// A Baylee Central account as the client names it everywhere:
/// `Name#00af`.
#[must_use]
pub fn handle(person: &Person) -> String {
    format!("{}#{}", person.display_name, tag(person))
}

/// `seconds` as a local date and time ("3 Dec, 14:37"), at `offset`
/// seconds east of UTC.
#[must_use]
pub fn when(seconds: i64, offset: i32, lang: Lang) -> String {
    let Ok(at) = time::OffsetDateTime::from_unix_timestamp(seconds.saturating_add(offset.into()))
    else {
        return "—".into();
    };
    let month = [
        Phrase::CalendarJanuary,
        Phrase::CalendarFebruary,
        Phrase::CalendarMarch,
        Phrase::CalendarApril,
        Phrase::CalendarMay,
        Phrase::CalendarJune,
        Phrase::CalendarJuly,
        Phrase::CalendarAugust,
        Phrase::CalendarSeptember,
        Phrase::CalendarOctober,
        Phrase::CalendarNovember,
        Phrase::CalendarDecember,
    ][usize::from(u8::from(at.month()) - 1)];
    ChatPhrase::InviteDate.fill(
        lang,
        &[
            &at.day().to_string(),
            month.text(lang),
            &format!("{:02}:{:02}", at.hour(), at.minute()),
        ],
    )
}

/// How long an open invite still holds, in the largest unit it fills
/// (days, then hours, then minutes, at least one), rounded to the nearest:
/// a 14-day link made a minute ago still holds 14 days, not 13.
#[must_use]
pub fn left(expires_at: i64, now: i64, lang: Lang) -> String {
    let secs = usize::try_from(expires_at.saturating_sub(now).max(0)).unwrap_or(usize::MAX);
    let near = |unit: usize| secs.saturating_add(unit / 2) / unit;
    match secs {
        s if s >= 86_400 => ChatPhrase::InviteLeftDays.text_count(lang, near(86_400)),
        s if s >= 3_600 => ChatPhrase::InviteLeftHours.text_count(lang, near(3_600)),
        _ => ChatPhrase::InviteLeftMinutes.text_count(lang, near(60).max(1)),
    }
}

/// An invite's state in a few words: how long it holds, who took it up and
/// when, or that it ended. `now` is the wall clock in Unix seconds; an
/// invite Central still lists as open past its end reads as expired.
#[must_use]
pub fn state_words(invite: &FriendInviteSummary, now: i64, offset: i32, lang: Lang) -> String {
    match invite.status {
        FriendInviteStatus::Active if invite.expires_at > now => left(invite.expires_at, now, lang),
        FriendInviteStatus::Active | FriendInviteStatus::Expired => {
            ChatPhrase::InviteExpired.text(lang).to_owned()
        }
        FriendInviteStatus::Revoked => ChatPhrase::InviteRevoked.text(lang).to_owned(),
        FriendInviteStatus::Redeemed => match (&invite.redeemed_by, invite.redeemed_at) {
            (Some(person), Some(at)) => {
                ChatPhrase::InviteTakenBy.fill(lang, &[&handle(person), &when(at, offset, lang)])
            }
            (Some(person), None) => ChatPhrase::InviteTakenByNoDate.fill(lang, &[&handle(person)]),
            _ => ChatPhrase::InviteTaken.text(lang).to_owned(),
        },
    }
}

/// What an invite is called in the list: the player's own label, or the
/// day it was made.
#[must_use]
pub fn title(invite: &FriendInviteSummary, offset: i32, lang: Lang) -> String {
    invite.label.clone().unwrap_or_else(|| {
        ChatPhrase::InviteUntitled.fill(lang, &[&when(invite.created_at, offset, lang)])
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn invite(status: FriendInviteStatus) -> FriendInviteSummary {
        FriendInviteSummary {
            id: [1; 16],
            label: None,
            note: None,
            invited_email: None,
            status,
            created_at: 1_795_536_000,
            expires_at: 1_795_536_000 + 7 * 86_400,
            redeemed_at: None,
            redeemed_by: None,
        }
    }

    #[test]
    fn a_link_is_the_website_s_invite_page_in_the_player_s_language() {
        assert_eq!(
            share_url("https://baylee.app", Lang::De, "Zm9vYmFyLWJheWxlZS0xMA"),
            "https://baylee.app/de/invite/Zm9vYmFyLWJheWxlZS0xMA"
        );
        assert_eq!(
            share_url("http://127.0.0.1:28794/", Lang::En, "abc"),
            "http://127.0.0.1:28794/en/invite/abc"
        );
    }

    #[test]
    fn the_default_validity_is_one_the_sheet_offers() {
        assert!(DAYS.contains(&DEFAULT_DAYS));
        assert!(
            DAYS.iter()
                .all(|d| (1..=central_api::social::MAX_FRIEND_INVITE_DAYS).contains(d))
        );
    }

    #[test]
    fn each_filter_shows_its_own() {
        use FriendInviteStatus::*;
        let shown = |filter: Filter| {
            [Active, Redeemed, Revoked, Expired]
                .into_iter()
                .filter(|s| filter.shows(*s))
                .collect::<Vec<_>>()
        };
        assert_eq!(shown(Filter::Open), vec![Active]);
        assert_eq!(shown(Filter::Taken), vec![Redeemed]);
        assert_eq!(shown(Filter::All).len(), 4);
        let list = [invite(Active), invite(Redeemed), invite(Active)];
        assert_eq!(open_count(&list), 2);
    }

    #[test]
    fn an_open_invite_says_how_long_it_holds_in_its_largest_unit() {
        let now = 1_000;
        assert_eq!(left(now + 6 * 86_400 + 5, now, Lang::En), "6 days left");
        assert_eq!(
            left(now + 14 * 86_400 - 60, now, Lang::En),
            "14 days left",
            "a link made a minute ago holds its whole two weeks"
        );
        assert_eq!(left(now + 86_400, now, Lang::De), "noch 1 Tag");
        assert_eq!(left(now + 5 * 3_600, now, Lang::En), "5 hours left");
        assert_eq!(left(now + 30, now, Lang::En), "1 minute left");
        assert_eq!(left(now - 30, now, Lang::En), "1 minute left");
    }

    #[test]
    fn an_invite_past_its_end_reads_as_expired_whatever_the_list_said() {
        let mut open = invite(FriendInviteStatus::Active);
        let after = open.expires_at + 1;
        assert_eq!(state_words(&open, after, 0, Lang::En), "Expired");
        open.status = FriendInviteStatus::Revoked;
        assert_eq!(state_words(&open, 0, 0, Lang::De), "Zurückgezogen");
    }

    #[test]
    fn a_taken_invite_names_who_took_it_and_when() {
        let mut taken = invite(FriendInviteStatus::Redeemed);
        taken.redeemed_by = Some(Person {
            tag: 42,
            display_name: "Mara".into(),
        });
        taken.redeemed_at = Some(1_796_126_220); // 1 December 2026, 11:57 UTC
        assert_eq!(
            state_words(&taken, 0, 3_600, Lang::En),
            "Accepted by Mara#002a on 1 Dec, 12:57"
        );
        taken.redeemed_by = None;
        assert_eq!(state_words(&taken, 0, 0, Lang::En), "Accepted");
    }

    #[test]
    fn an_invite_without_a_label_is_named_by_its_day() {
        let mut named = invite(FriendInviteStatus::Active);
        assert!(title(&named, 0, Lang::En).starts_with("Invite of "));
        named.label = Some("Mara".into());
        assert_eq!(title(&named, 0, Lang::En), "Mara");
    }
}
