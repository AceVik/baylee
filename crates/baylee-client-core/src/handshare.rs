//! Teammate hand-sharing state and actions for team games.
//!
//! In team formats (2v2, 3v3, Archenemy, etc.), teammates may share their hand
//! with one another in two ways:
//!
//! 1. **Continuous sharing** ([`SeatSetting::ShareHand`]): the teammate sees
//!    the current hand and every card drawn later until sharing is revoked or
//!    the game ends.
//! 2. **One-time snapshot** ([`SeatSetting::ShareHandOnce`]): the teammate sees
//!    only the exact [`baylee_core::ids::ObjectId`]s currently in hand at the
//!    moment of the offer; cards drawn later stay hidden, and cards that leave
//!    the hand drop out of the snapshot automatically.
//!
//! A teammate may also ask to see another teammate's hand
//! ([`SeatSetting::RequestHand`]), which the recipient can answer by sharing
//! continuously, showing once, or declining ([`SeatSetting::DeclineHand`]).
//!
//! Nothing in this module touches a renderer: it reads [`PlayerView`] and team
//! assignments (`&[Option<u8>]`, such as [`baylee_view::GameStatic::teams`])
//! and builds the [`SeatSetting`] envelopes the client sends over `SeatSettingMsg`
//! (or applies directly to an offline `Session::seat_setting`).

use baylee_core::ids::{PlayerId, SeatSet};
use baylee_view::{HandObject, PlayerView, SeatSetting};

use crate::i18n::{Lang, Phrase};

/// Hand-sharing relationship between the local viewer (`view.seat`) and one
/// teammate seat.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TeammateHandStatus<'a> {
    /// The teammate's seat.
    pub player: PlayerId,
    /// Whether the local seat is currently sharing its hand continuously with
    /// `player` (`view.hand_shared_with.contains(player)`).
    pub sharing_to_them: bool,
    /// Whether `player` has a pending request to see the local seat's hand
    /// (`view.hand_requests.contains(player)`).
    pub requested_by_them: bool,
    /// Cards of `player`'s hand currently visible to the local seat in
    /// `view.shared_hands` (either via continuous sharing or a one-time
    /// snapshot), or `None` if `player` has not shared any hand cards with us.
    pub visible_from_them: Option<&'a [HandObject]>,
}

/// Returns the [`SeatSet`] of living teammates of `view.seat` according to
/// `teams` (excluding `view.seat` itself and any eliminated seats in `view`).
#[must_use]
pub fn teammates_of(view: &PlayerView, teams: &[Option<u8>]) -> SeatSet {
    teammates_of_seat(view, view.seat, teams)
}

/// Returns the [`SeatSet`] of living teammates of `actor` according to `teams`
/// (excluding `actor` itself and any eliminated seats in `view`).
#[must_use]
pub fn teammates_of_seat(view: &PlayerView, actor: PlayerId, teams: &[Option<u8>]) -> SeatSet {
    let Some(Some(my_team)) = teams.get(usize::from(actor.get())).copied() else {
        return SeatSet::new();
    };
    let mut set = SeatSet::new();
    for seat in &view.seats {
        if seat.player != actor
            && seat.loss.is_none()
            && teams.get(usize::from(seat.player.get())).copied().flatten() == Some(my_team)
        {
            set.insert(seat.player);
        }
    }
    set
}

/// Returns whether `a` and `b` are distinct living teammates on the same team.
#[must_use]
pub fn is_teammate(view: &PlayerView, teams: &[Option<u8>], a: PlayerId, b: PlayerId) -> bool {
    teammates_of_seat(view, a, teams).contains(b)
}

/// Returns the [`SeatSet`] of living teammates currently asking to see the
/// local viewer's hand (`view.hand_requests`).
#[must_use]
pub fn incoming_requests(view: &PlayerView, teams: &[Option<u8>]) -> SeatSet {
    let allies = teammates_of(view, teams);
    let mut out = SeatSet::new();
    for p in view.hand_requests.iter() {
        if allies.contains(p) {
            out.insert(p);
        }
    }
    out
}

/// Returns the visible shared hand slice of `player` in `view.shared_hands`, if
/// any.
#[must_use]
pub fn shared_hand_of(view: &PlayerView, player: PlayerId) -> Option<&[HandObject]> {
    view.shared_hands
        .iter()
        .find(|h| h.player == player)
        .map(|h| h.cards.as_slice())
}

/// Summarizes the hand-sharing state for every living teammate of `view.seat`,
/// in seat order.
#[must_use]
pub fn teammate_statuses<'a>(
    view: &'a PlayerView,
    teams: &[Option<u8>],
) -> Vec<TeammateHandStatus<'a>> {
    let allies = teammates_of(view, teams);
    let mut out = Vec::with_capacity(allies.len());
    for player in allies.iter() {
        out.push(TeammateHandStatus {
            player,
            sharing_to_them: view.hand_shared_with.contains(player),
            requested_by_them: view.hand_requests.contains(player),
            visible_from_them: shared_hand_of(view, player),
        });
    }
    out
}

/// Builds a [`SeatSetting::ShareHand`] that continuously shares the local
/// viewer's hand with `target` (adding `target` to `view.hand_shared_with`).
///
/// Returns `None` if `target` is not a living teammate of `view.seat`.
#[must_use]
pub fn share_continuously(
    view: &PlayerView,
    teams: &[Option<u8>],
    target: PlayerId,
) -> Option<SeatSetting> {
    let allies = teammates_of(view, teams);
    if !allies.contains(target) {
        return None;
    }
    let mut next = view.hand_shared_with;
    next.insert(target);
    Some(SeatSetting::ShareHand(next))
}

/// Builds a [`SeatSetting::ShareHand`] that stops continuous hand sharing with
/// `target` (removing `target` from `view.hand_shared_with`).
///
/// Returns `None` if we are not currently sharing continuously with `target`.
#[must_use]
pub fn stop_sharing(view: &PlayerView, target: PlayerId) -> Option<SeatSetting> {
    if !view.hand_shared_with.contains(target) {
        return None;
    }
    let next: SeatSet = view
        .hand_shared_with
        .iter()
        .filter(|&p| p != target)
        .collect();
    Some(SeatSetting::ShareHand(next))
}

/// Toggles continuous hand sharing with `target`.
///
/// Returns `None` if `target` is neither a living teammate nor currently in
/// `view.hand_shared_with`.
#[must_use]
pub fn toggle_continuous_share(
    view: &PlayerView,
    teams: &[Option<u8>],
    target: PlayerId,
) -> Option<SeatSetting> {
    if view.hand_shared_with.contains(target) {
        stop_sharing(view, target)
    } else {
        share_continuously(view, teams, target)
    }
}

/// Builds a [`SeatSetting::ShareHand`] that continuously shares the local
/// viewer's hand with all living teammates.
///
/// Returns `None` if `view.seat` has no living teammates.
#[must_use]
pub fn share_all_teammates_continuously(
    view: &PlayerView,
    teams: &[Option<u8>],
) -> Option<SeatSetting> {
    let allies = teammates_of(view, teams);
    if allies.is_empty() {
        return None;
    }
    Some(SeatSetting::ShareHand(allies))
}

/// Builds a [`SeatSetting::ShareHandOnce`] that grants `target` a one-time
/// snapshot of the cards currently in the local viewer's hand.
///
/// Returns `None` if `target` is not a living teammate of `view.seat`.
#[must_use]
pub fn share_once(
    view: &PlayerView,
    teams: &[Option<u8>],
    target: PlayerId,
) -> Option<SeatSetting> {
    let allies = teammates_of(view, teams);
    if !allies.contains(target) {
        return None;
    }
    let mut set = SeatSet::new();
    set.insert(target);
    Some(SeatSetting::ShareHandOnce(set))
}

/// Builds a [`SeatSetting::ShareHandOnce`] that grants all living teammates a
/// one-time snapshot of the cards currently in the local viewer's hand.
///
/// Returns `None` if `view.seat` has no living teammates.
#[must_use]
pub fn share_all_teammates_once(view: &PlayerView, teams: &[Option<u8>]) -> Option<SeatSetting> {
    let allies = teammates_of(view, teams);
    if allies.is_empty() {
        return None;
    }
    Some(SeatSetting::ShareHandOnce(allies))
}

/// Builds a [`SeatSetting::RequestHand`] asking teammate `target` to share or
/// show their hand.
///
/// Returns `None` if `target` is not a living teammate of `view.seat`.
#[must_use]
pub fn request_hand(
    view: &PlayerView,
    teams: &[Option<u8>],
    target: PlayerId,
) -> Option<SeatSetting> {
    let allies = teammates_of(view, teams);
    if !allies.contains(target) {
        return None;
    }
    Some(SeatSetting::RequestHand(target))
}

/// Builds a [`SeatSetting::DeclineHand`] declining a pending hand-view request
/// from `requester`.
///
/// Returns `None` if `requester` does not have a pending request in
/// `view.hand_requests`.
#[must_use]
pub fn decline_request(view: &PlayerView, requester: PlayerId) -> Option<SeatSetting> {
    if !view.hand_requests.contains(requester) {
        return None;
    }
    Some(SeatSetting::DeclineHand(requester))
}

/// Localized menu/button label for sharing continuously with `teammate_name`.
#[must_use]
pub fn label_share_continuous(lang: Lang, teammate_name: &str) -> String {
    Phrase::HandShareWithTeammate.fill(lang, &[teammate_name])
}

/// Localized menu/button label for showing the current hand once to `teammate_name`.
#[must_use]
pub fn label_share_once(lang: Lang, teammate_name: &str) -> String {
    Phrase::HandShareOnceWithTeammate.fill(lang, &[teammate_name])
}

/// Localized menu/button label for revoking continuous hand sharing with `teammate_name`.
#[must_use]
pub fn label_stop_sharing(lang: Lang, teammate_name: &str) -> String {
    Phrase::HandStopSharingWithTeammate.fill(lang, &[teammate_name])
}

/// Localized menu/button label for asking to see `teammate_name`'s hand.
#[must_use]
pub fn label_request_hand(lang: Lang, teammate_name: &str) -> String {
    Phrase::HandRequestFromTeammate.fill(lang, &[teammate_name])
}

/// Localized menu/button label for declining `teammate_name`'s request.
#[must_use]
pub fn label_decline_request(lang: Lang, teammate_name: &str) -> String {
    Phrase::HandDeclineRequestFromTeammate.fill(lang, &[teammate_name])
}

/// Localized banner/toast line when `teammate_name` asks to see our hand.
#[must_use]
pub fn label_requested_by(lang: Lang, teammate_name: &str) -> String {
    Phrase::HandRequestedByTeammate.fill(lang, &[teammate_name])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::ViewBuilder;
    use baylee_view::SharedHand;

    #[test]
    fn teammates_and_actions_respect_team_boundaries_and_requests() {
        let mut view = ViewBuilder::new(4).build();
        let p0 = PlayerId::new(0);
        let p1 = PlayerId::new(1);
        let p2 = PlayerId::new(2);
        let p3 = PlayerId::new(3);
        view.seat = p0;
        let teams = [Some(0), Some(0), Some(1), Some(1)];

        let allies = teammates_of(&view, &teams);
        assert!(allies.contains(p1));
        assert!(!allies.contains(p0));
        assert!(!allies.contains(p2));
        assert!(!allies.contains(p3));

        // Cannot share with or request from an opponent.
        assert!(share_continuously(&view, &teams, p2).is_none());
        assert!(share_once(&view, &teams, p2).is_none());
        assert!(request_hand(&view, &teams, p3).is_none());

        // Can share continuously or once with teammate p1.
        let mut expected_set = SeatSet::new();
        expected_set.insert(p1);
        assert_eq!(
            share_continuously(&view, &teams, p1),
            Some(SeatSetting::ShareHand(expected_set))
        );
        assert_eq!(
            share_once(&view, &teams, p1),
            Some(SeatSetting::ShareHandOnce(expected_set))
        );
        assert_eq!(
            request_hand(&view, &teams, p1),
            Some(SeatSetting::RequestHand(p1))
        );

        // Incoming request and decline.
        assert!(decline_request(&view, p1).is_none());
        view.hand_requests.insert(p1);
        assert!(incoming_requests(&view, &teams).contains(p1));
        assert_eq!(
            decline_request(&view, p1),
            Some(SeatSetting::DeclineHand(p1))
        );

        // Statuses reflect continuous sharing and shared hand cards.
        view.hand_shared_with.insert(p1);
        view.shared_hands.push(SharedHand {
            player: p1,
            public: false,
            cards: Vec::new(),
        });
        let statuses = teammate_statuses(&view, &teams);
        assert_eq!(statuses.len(), 1);
        assert_eq!(statuses[0].player, p1);
        assert!(statuses[0].sharing_to_them);
        assert!(statuses[0].requested_by_them);
        assert!(statuses[0].visible_from_them.is_some());

        // Toggle off continuous sharing.
        assert_eq!(
            toggle_continuous_share(&view, &teams, p1),
            Some(SeatSetting::ShareHand(SeatSet::new()))
        );
    }

    #[test]
    fn localized_labels_render_in_english_and_german() {
        assert_eq!(
            label_share_continuous(Lang::En, "Alice"),
            "Share hand with Alice (whole game)"
        );
        assert_eq!(
            label_share_continuous(Lang::De, "Alice"),
            "Hand dauerhaft mit Alice teilen"
        );
        assert_eq!(
            label_share_once(Lang::En, "Alice"),
            "Show current hand to Alice (once)"
        );
        assert_eq!(
            label_share_once(Lang::De, "Alice"),
            "Aktuelle Hand Alice einmalig zeigen"
        );
        assert_eq!(
            label_request_hand(Lang::De, "Bob"),
            "Bob um Handsicht bitten"
        );
    }
}
