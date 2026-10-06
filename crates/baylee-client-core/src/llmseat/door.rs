//! How a host's language-model chair gets to its table
//! (`docs/protocol.md` §"A host's chair for a seat bridge").
//!
//! A host signed in to an account asks its gateway for a **chair ticket**
//! for the chair, and the bridge it then starts sits down on it with no
//! account of its own: so a gateway that takes no guests, whose guest cap
//! is reached, or that is a closed beta seats it all the same. A host who
//! is a guest is handed no ticket; its bridge signs in as a guest, where
//! the gateway takes guests, as before. The ticket goes to the bridge as
//! the first line of its stdin ([`Admission::stdin`]), never in its
//! arguments, its environment or a log; the arguments only say one comes
//! (`--chair-ticket`).
//!
//! This decides everything about that and touches nothing: whether to ask
//! ([`admission_for`]), what the answer comes to ([`Doors::answered`]),
//! and which asks are still out, so that a chair waiting for its ticket is
//! not asked for twice ([`Doors::running`]). The shell sends the request
//! ([`ticket_request`]) and starts the bridge.

use super::seating::Running;
use std::collections::BTreeMap;

/// How a chair's bridge gets in.
#[derive(Clone, PartialEq, Eq)]
pub enum Admission {
    /// On the host's chair ticket, handed over on stdin.
    Ticket(String),
    /// As a guest of its own, where the gateway takes guests.
    Guest,
}

impl std::fmt::Debug for Admission {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ticket(_) => f.write_str("Ticket(..)"),
            Self::Guest => f.write_str("Guest"),
        }
    }
}

impl Admission {
    /// Whether the bridge is told a chair ticket comes on its stdin.
    #[must_use]
    pub const fn on_ticket(&self) -> bool {
        matches!(self, Self::Ticket(_))
    }

    /// The first line the bridge's stdin is written: the ticket, for one
    /// that sits on a ticket.
    #[must_use]
    pub fn stdin(&self) -> Option<String> {
        match self {
            Self::Ticket(ticket) => Some(format!("{ticket}\n")),
            Self::Guest => None,
        }
    }
}

/// What starting a chair's bridge comes to before any request: ask the
/// gateway for a ticket (`Ok(None)`), go in as a guest, or say why the
/// chair cannot be seated here.
///
/// # Errors
/// A host who is a guest at a gateway that takes no guests: there is no
/// door for its bridge.
pub fn admission_for(host_is_guest: bool, guests: bool) -> Result<Option<Admission>, String> {
    match (host_is_guest, guests) {
        (false, _) => Ok(None),
        (true, true) => Ok(Some(Admission::Guest)),
        (true, false) => Err(NO_DOOR.to_string()),
    }
}

/// Why a guest host's chair cannot be seated at a gateway with no guests.
pub const NO_DOOR: &str = "this gateway takes no guests, and a guest cannot hand a chair \
                           to a language model: sign in with an account";

/// The request for a chair's ticket: `POST`, the host's session as
/// `Authorization`, at this address relative to the gateway.
#[must_use]
pub fn ticket_request(room: &str, chair: u32) -> String {
    baylee_protocol::chair_ticket_path(room, chair)
}

/// The asks for chair tickets still out, by chair, at the plan version
/// each was asked for.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Doors {
    asked: BTreeMap<u32, u64>,
}

impl Doors {
    /// A ticket was asked for `chair` at plan `version`.
    pub fn ask(&mut self, chair: u32, version: u64) {
        self.asked.insert(chair, version);
    }

    /// The asks still out, as bridges already started: a chair waiting for
    /// its ticket is not started a second time ([`super::seating::Seating::steps`]).
    pub fn running(&self) -> impl Iterator<Item = Running> + '_ {
        self.asked.iter().map(|(chair, version)| Running {
            chair: *chair,
            version: *version,
            exited: false,
        })
    }

    /// No bridge for `chair` any more: an answer still out for it is
    /// dropped when it comes.
    pub fn forget(&mut self, chair: u32) {
        self.asked.remove(&chair);
    }

    /// The gateway's answer to the ask for `chair` at `version`: `status`
    /// and `body`, or `None` when it did not answer. `None` back for an
    /// answer nobody waits for any more (the plan moved on, or the chair
    /// was let go).
    ///
    /// A ticket is taken. Anything else falls back to a guest where the
    /// gateway takes guests (an older gateway that sells no tickets
    /// answers 404); where it takes none, the gateway's sentence is why the
    /// chair stays empty.
    pub fn answered(
        &mut self,
        chair: u32,
        version: u64,
        answer: Option<(u16, &str)>,
        guests: bool,
    ) -> Option<Result<Admission, String>> {
        if self.asked.get(&chair) != Some(&version) {
            return None;
        }
        self.asked.remove(&chair);
        let read = |body: &str, field: &str| {
            serde_json::from_str::<serde_json::Value>(body)
                .ok()
                .and_then(|v| v.get(field)?.as_str().map(str::to_string))
        };
        if let Some((200..=299, body)) = answer
            && let Some(ticket) = read(body, "ticket").filter(|t| !t.is_empty())
        {
            return Some(Ok(Admission::Ticket(ticket)));
        }
        if guests {
            return Some(Ok(Admission::Guest));
        }
        let why = match answer {
            Some((status, body)) => {
                read(body, "error").unwrap_or_else(|| format!("the gateway answered {status}"))
            }
            None => "the gateway did not answer".to_string(),
        };
        Some(Err(format!("the chair could not be handed over: {why}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llmseat::seating::{ChairModel, Launch, Phase, Seating, Step, bridge_args};
    use crate::llmseat::{Profile, Provider};

    const TICKET: &str = "5eed5eed5eed5eed5eed5eed5eed5eed5eed5eed5eed5eed5eed5eed5eed5eed";

    fn launch(chair_ticket: bool) -> Launch<'static> {
        Launch {
            room: "TEST-room",
            gateway: "http://127.0.0.1:28766",
            chair: 1,
            config: "/tmp/TEST/llm-seat.json",
            deck: "Allytifact",
            level: "steady",
            chair_ticket,
        }
    }

    /// The whole way of a host's chair: planned, a ticket asked for once
    /// while the room lists the chair open, the answer taken, and the bridge
    /// told a ticket comes on its stdin, which carries it; the arguments
    /// never do, and neither does `Debug`.
    #[test]
    fn an_account_hosts_chair_is_seated_on_a_ticket_on_stdin() {
        let profile = Profile::new(Provider::Anthropic, "claude-sonnet-5-5");
        let mut seating = Seating::default();
        seating.enter("TEST-room");
        seating.plan(1, ChairModel::of("sonnet", &profile), None);
        let version = seating.chair(1).unwrap().version;
        assert_eq!(admission_for(false, false), Ok(None), "an account asks");
        assert_eq!(
            seating.steps(Phase::Waiting, &[1], &[]),
            vec![Step::Launch(1)]
        );
        let mut doors = Doors::default();
        doors.ask(1, version);
        assert_eq!(
            ticket_request("TEST-room", 1),
            "/lobby/games/TEST-room/chairs/1/ticket"
        );
        // While the ask is out, the chair is not started again.
        let out: Vec<Running> = doors.running().collect();
        assert_eq!(seating.steps(Phase::Waiting, &[1], &out), vec![]);
        let body = format!(r#"{{"ticket":"{TICKET}","expires_in":120}}"#);
        let admitted = doors
            .answered(1, version, Some((200, &body)), false)
            .expect("waited for")
            .expect("a ticket");
        assert!(admitted.on_ticket());
        assert_eq!(admitted.stdin().as_deref(), Some(&*format!("{TICKET}\n")));
        assert!(!format!("{admitted:?}").contains(TICKET));
        let args = bridge_args(
            &launch(admitted.on_ticket()),
            seating.chair(1).map(|p| &p.model).unwrap(),
            &profile,
        );
        assert!(args.contains(&"--chair-ticket".to_string()), "{args:?}");
        assert!(args.contains(&"--tethered".to_string()), "{args:?}");
        assert!(!args.iter().any(|a| a.contains(TICKET)), "{args:?}");
        assert_eq!(doors.running().count(), 0, "the ask is answered");
        // An answer nobody waits for (asked again, or let go) is dropped.
        doors.ask(1, version + 1);
        assert_eq!(doors.answered(1, version, Some((200, &body)), true), None);
        doors.forget(1);
        assert_eq!(
            doors.answered(1, version + 1, Some((200, &body)), true),
            None
        );
    }

    /// Without a ticket a bridge goes in as a guest where the gateway takes
    /// guests (a guest host; an older gateway that sells none), and where it
    /// takes none the chair stays empty with the gateway's reason.
    #[test]
    fn without_a_ticket_a_guest_where_guests_are_taken_else_the_reason() {
        assert_eq!(admission_for(true, true), Ok(Some(Admission::Guest)));
        assert_eq!(admission_for(true, false), Err(NO_DOOR.to_string()));
        let mut doors = Doors::default();
        doors.ask(2, 0);
        assert_eq!(
            doors.answered(2, 0, Some((404, "")), true),
            Some(Ok(Admission::Guest)),
            "an older gateway"
        );
        doors.ask(2, 1);
        assert_eq!(
            doors.answered(
                2,
                1,
                Some((409, r#"{"error":"that seat is not open"}"#)),
                false
            ),
            Some(Err(
                "the chair could not be handed over: that seat is not open".to_string()
            ))
        );
        doors.ask(2, 2);
        assert_eq!(
            doors.answered(2, 2, None, false),
            Some(Err(
                "the chair could not be handed over: the gateway did not answer".to_string()
            ))
        );
        doors.ask(2, 3);
        assert_eq!(
            doors.answered(2, 3, Some((200, r#"{"ticket":""}"#)), false),
            Some(Err(
                "the chair could not be handed over: the gateway answered 200".to_string()
            )),
            "an empty ticket is none"
        );
        assert_eq!(Admission::Guest.stdin(), None);
        let args = bridge_args(
            &launch(false),
            &ChairModel::of("p", &Profile::new(Provider::Anthropic, "claude-sonnet-5-5")),
            &Profile::new(Provider::Anthropic, "claude-sonnet-5-5"),
        );
        assert!(!args.contains(&"--chair-ticket".to_string()), "{args:?}");
    }

    /// The room lists a host's model as the host's: the chair's player is
    /// the name the bridge sat under, and `delegated_by` says whose it is;
    /// an older gateway's row, without it, reads as before.
    #[test]
    fn the_room_lists_a_hosts_model_as_the_hosts() {
        let seat: crate::lobby::GameSeat = serde_json::from_str(
            r#"{"seat":1,"kind":"human","taken":true,"player":"LLM-sonnet-5-5",
                "delegated_by":"Alice#af03","ready":false}"#,
        )
        .unwrap();
        assert_eq!(seat.delegated_by.as_deref(), Some("Alice#af03"));
        assert!(!seat.open(), "taken");
        let older: crate::lobby::GameSeat =
            serde_json::from_str(r#"{"seat":1,"taken":false}"#).unwrap();
        assert_eq!(older.delegated_by, None);
    }
}
