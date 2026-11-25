//! baylee-protocol — binary websocket protocol (protobuf), wasm-safe.
//!
//! v0: transport handshake + preset transfer (see `docs/protocol.md`).
//! Choice/action/view messages land with the engine API (M1–M3).
//!
//! Also the rules both ends of the gateway's HTTP API check alike
//! ([`names`]), so that the client can refuse what the gateway would.
#![deny(unsafe_code)]
#![warn(missing_docs)]

pub mod execution;
pub mod mind;
pub mod names;
pub mod refusal;
pub mod seathost;
pub mod social;
pub mod tape;

/// Wire protocol version; incompatible versions refuse the session.
/// Version 24 carries Camouflage's piles (`ChoicePrompt::CamouflagePile`).
/// Version 26 adds spectators (`SpectatorsChanged`, `SpectatorFrame`,
/// `Spectators`, `/games/{id}/watch`); 25 is another branch's.
/// Version 27 carries a damage decision's `DamageEffectKind::PreventAll`.
/// Version 28 carries independent delayed-life-loss cancellation offers.
/// Version 29 preserves full-width explicit life-payment amounts.
/// Version 30 adds `PlayerAction::PayWindow`, a payment window paid in one
/// all-or-nothing action.
/// Version 31 asks about draws: `ChoicePrompt::Dredge` and
/// `ChoicePrompt::DrawReplacement` (dredge, Chains of Mephistopheles,
/// Aladdin's Lamp).
/// Version 32 carries damage amount replacements, reflection, shield counters
/// and shield/additional-counter replacement choices.
/// Version 33 adds the damage-clearing destruction replacement prompt.
/// Version 34 adds battle defenders and defense counters to serialized choices.
/// Version 35 adds resolution choices for mutate positions and dungeon paths.
/// Version 36 requires authoritative per-seat participation endings from engines.
/// Version 37 asks a vote as one (`ChoicePrompt::Vote`, CR 701.38a).
/// Counter-order prompts also carry the entry replacement's counter kind.
/// Version 38 is the sign-in by gateway token, with a session token that
/// rotates, and says why a socket was refused (`HelloAck::refusal`).
/// Version 39 sends a view as a `ViewFrame` in `StateDelta::view_frame`, and
/// `ResumeGame { last_seq: 0 }` is how a seat asks for a whole one.
/// Version 40 writes that frame and `PlayerActionMsg::action` in binary
/// (`baylee_core::wire`), and a `PayWindow` holds `PaymentStep`s.
/// Version 41 writes every other frame of a game the same way.
/// Version 42 tells a printing shown in `GameStaticMsg::more`, not the whole payload.
/// Version 43 states what restricted mana may pay for (CR 106.6):
/// `LegalActions::restricted_taps` (in classes of taps) and `restricted_floating`, by `Spend`.
/// Version 44 states a suspend whose mana is still in the lands
/// (`LegalActions::unpaid_suspend`, CR 116.2f), the division of a spell's
/// damage on the stack (`PlayerView::divided`, CR 601.2d) and a log line's
/// step (`LogEntry::step`, CR 510.4).
/// Version 45 names the permanent a life payment is for
/// (`YesNoPrompt::PayLife::permanent`): Cleansing's land, CR 608.2d.
/// Version 46 carries `ChoicePrompt::SearchLibraryTappedAndHand`,
/// `YesNoPrompt::RevealHandForGuess`, and `LogEvent::DidNotResolve::ability`
/// (whether an unresolved stack item was an ability rather than a spell).
/// Version 47 carries held APNAP choices (`PlayerView::held_choices`,
/// `SpectatorView::held_choices`, `ViewField::HeldChoices`, and
/// `LogEvent::CardsChosen`), plus `PublicObject`'s `second_chosen_subtype`,
/// `chosen_card_type`, `chosen_number`, and `linked_exile`.
/// Version 49 adds the private, bounded Tower execution/recovery/whole-batch ACK plane.
/// Version 50 carries `YesNoPrompt::KickerOption` for multi-kicker spells (CR 702.33b/f).
/// Version 51 carries `ChoicePrompt::Amplify` for Amplify hand reveals (CR 702.38a/b).
/// Version 52 carries `CastModeKind::Bestow` and `CastModeKind::JumpStart` (CR 702.103, CR 702.133a).
/// Version 53 carries `CastModeKind::Blitz` (CR 702.152a).
/// Version 54 carries `CastModeKind::Emerge` (CR 702.119), `YesNoPrompt::Bargain`
/// (CR 702.166), and `LegalActions::spell_spending` (CR 609.4b).
/// Version 55 carries `CastModeKind::Fused` (CR 702.102), `NumberPrompt::Squad`
/// (CR 702.157), `YesNoPrompt::Offspring` (CR 702.175), and
/// `ChoicePrompt::ManifestDread` (CR 701.62).
/// Version 56 carries `CastModeKind::Plot` and `PublicObject::plotted`
/// (CR 702.170) alongside `YesNoPrompt::Casualty` (CR 702.153).
/// Version 57 carries `DamageEffectKind::MultiplyDamage` and
/// `DamageEffectKind::AddDamage` for CR 616.1 damage replacement ordering.
/// Version 58 carries `CastModeKind::Impending` (CR 702.176) and
/// `YesNoPrompt::Gift` (CR 702.174) on positional binary frames.
/// Version 59 carries `CastModeKind::Warp`, `PublicObject::warped`
/// (CR 702.185), `ChoicePrompt::Devour` (CR 702.82), and `ManaPoolView::snow`
/// (CR 107.4h).
/// Version 60 carries `CastModeKind::Retrace` (CR 702.81a),
/// `YesNoPrompt::{Riot, Unleash}` (CR 702.136, CR 702.98), and
/// `PublicObject::suspected` (CR 701.60) on positional binary frames.
/// Version 61–64 unify Engine Waves 9a–10b (`YesNoPrompt::AssignDamageAsThoughUnblocked`,
/// `YesNoPrompt::ClashBottom`, `ChoicePrompt::ChooseRingBearer`,
/// `YesNoPrompt::{ChooseFirstCostBranch, OpeningHandBattlefield, OpeningHandReveal}`,
/// `CastModeKind::Sneak`, `YesNoPrompt::Endure`, `CastModeKind::Awaken`,
/// `ChoicePrompt::{Splice, CipherEncode, Champion}`, and
/// `YesNoPrompt::{Conspire, Tribute}`) on positional binary frames.
/// Version 65 carries `PublicObject::class_level` and `LogEvent::ClassLevel`
/// for Class designations (CR 717) on positional binary frames.
/// Version 66 carries `CastModeKind::VoteOption`, `YesNoPrompt::AdditionalVote`,
/// `LogEvent::VotedOption` (CR 701.38), and `ChoicePrompt::CostRemoveCounter`
/// on positional binary frames.
/// Version 67 carries `PlayerView::initiative`, `SpectatorView::initiative`,
/// `LogEvent::TookInitiative` (CR 725), and `Dungeon::Undercity` (CR 309.4c) on
/// positional binary frames.
/// Version 68 carries `CastModeKind::RepeatedModes` (CR 700.2d) and
/// `NumberPrompt::DivideCounters` (CR 601.2d) on positional binary frames.
/// Version 69 carries `CastModeKind::OptionalCost` (CR 118.8),
/// `ChoicePrompt::CollectEvidence` (CR 701.59), `ChoicePrompt::BeholdOrPay`
/// (CR 701.60), `Pending::ChooseCounterKind`,
/// `PlayerAction::ChooseCounterKind`, and the remaining CR 122.1b keyword
/// counters on positional binary frames. `YesNoPrompt::SearchLibrary` asks
/// whether to inspect the library before a conditional shuffle.
/// `ChoicePrompt::SearchLibraryOrGraveyard` offers both zones after acceptance.
/// Bounded generic mana payments use `ManaPayment::Variable` during mana
/// generation. `NumberPrompt` has bounded payment amounts for mana, energy
/// and life.
pub const PROTOCOL_VERSION: u32 = 69;

/// Duration of the shared portal flight; clocks start after arrival.
pub const TABLE_ENTRANCE_MS: u64 = 1_650;

/// Lead time for delivering a shared entrance to every prepared seat.
pub const TABLE_ENTRANCE_LEAD_MS: u64 = 1_500;

/// Why a peer that says it speaks protocol `theirs` is refused, or `None`
/// when it speaks this build's (#271).
///
/// One sentence for every boundary, so a refused agent, engine or client
/// is told the same thing in the same words: both numbers, because a
/// mismatch is only ever fixed by knowing which side is behind. `who` is the
/// peer as the sentence addresses it ("This agent"). A `theirs` of 0 is a
/// peer from before the version was sent at all, which proto3 cannot tell
/// from one that sent 0, and which says so rather than claim a version.
#[must_use]
pub fn version_refusal(who: &str, theirs: u32) -> Option<String> {
    (theirs != PROTOCOL_VERSION).then(|| {
        if theirs == 0 {
            format!(
                "{who} does not say which protocol it speaks; this gateway speaks {PROTOCOL_VERSION}."
            )
        } else {
            format!("{who} speaks protocol {theirs}; this gateway speaks {PROTOCOL_VERSION}.")
        }
    })
}

/// The path and query a seat socket is opened on, relative to the gateway:
/// `/games/{game_id}/ws?ticket=…&protocol=…` (#271, #294).
///
/// Every dialer builds it here, so none can leave out the protocol it
/// speaks: the gateway refuses a seat socket that does not say, before a
/// frame of the game is sent (`docs/protocol.md` §"Which side checks the
/// protocol (#271)").
///
/// It takes a **ticket** from [`WS_TICKET_PATH`], never the seat token: a
/// URL is what proxies log, and a ticket is spent by the upgrade it opens
/// (`docs/protocol.md` §"Opening a socket: tickets (#294)").
#[must_use]
pub fn seat_socket_path(game_id: &str, ticket: &str) -> String {
    format!("/games/{game_id}/ws?ticket={ticket}&protocol={PROTOCOL_VERSION}")
}

/// A spectator's socket on `game_id`, opened with a ticket for
/// `{"socket":"watch","game":"<id>"}` (`docs/protocol.md` §"Spectators").
#[must_use]
pub fn watch_socket_path(game_id: &str, ticket: &str) -> String {
    format!("/games/{game_id}/watch?ticket={ticket}&protocol={PROTOCOL_VERSION}")
}

/// Where a room's host asks for a chair ticket for its seat bridge:
/// `POST`, `Authorization: Bearer <session>`, answered `{ticket,
/// expires_in}` (`docs/protocol.md` §"A host's chair for a seat bridge").
/// The ticket goes to the bridge on its stdin, never in an address.
#[must_use]
pub fn chair_ticket_path(game_id: &str, seat: u32) -> String {
    format!("/lobby/games/{game_id}/chairs/{seat}/ticket")
}

/// Where a seat bridge sits down on its host's chair ticket: `POST`,
/// `Authorization: Bearer <chair ticket>`, body `{display_name, deck}`,
/// answered `{game_id, seat, seat_token, decide_secs}`.
#[must_use]
pub fn chair_redeem_path(game_id: &str, seat: u32) -> String {
    format!("/lobby/games/{game_id}/chairs/{seat}/redeem")
}

/// Where a client trades its bearer token for a ticket to open one socket
/// with (#294): `POST`, `Authorization: Bearer <session or seat token>`,
/// body `{"socket":"lobby"}` or `{"socket":"seat","game":"<id>"}`.
pub const WS_TICKET_PATH: &str = "/ws-ticket";

/// What a gateway answers an upgrade whose ticket opened nothing (#294):
/// the `error` of its `401`, the same whether the ticket was unknown, used,
/// expired or for another socket. A client that reads it fetches a fresh
/// ticket and dials again.
pub const TICKET_REFUSED: &str = "ticket expired or used";

/// The socket file a gateway address names, when it names one.
///
/// `unix:<path>` is a gateway reached on its unix socket
/// (`BAYLEE_UNIX_SOCKET`) rather than over TCP: what an agent on the same
/// machine is given as `BAYLEE_GATEWAY`, and what it hands its engines as
/// `--attach`. It stands for the gateway as a whole, like an `http://` base;
/// the dialer adds the path it wants (`/agent/ws`, `/engine/ws`).
///
/// Anything else is a URL and is `None` here. An empty path is not a socket.
#[must_use]
pub fn unix_socket(address: &str) -> Option<&str> {
    address
        .strip_prefix("unix:")
        .filter(|path| !path.is_empty())
}

/// Generated protobuf types (`baylee.v1`).
#[allow(missing_docs, clippy::all, clippy::pedantic)]
pub mod v1 {
    include!(concat!(env!("OUT_DIR"), "/baylee.v1.rs"));
}

#[cfg(test)]
mod tests {
    use super::{PROTOCOL_VERSION, v1};
    use prost::Message;

    /// The nesting contract, which is what lets the gateway run no rules.
    ///
    /// A `SeatFrame` carries an **encoded** player-facing `Envelope` as
    /// bytes rather than as a field on `Envelope` itself, so the gateway
    /// forwards exactly what it was handed: it never decodes a player
    /// message, never re-encodes one, and the player-facing protocol keeps
    /// the shape it had before the engine moved into its own process. The
    /// bytes have to come back *identical*, not merely equivalent — a
    /// gateway that round-tripped them through its own prost would be a
    /// second encoder on a wire that is supposed to have one.
    #[test]
    fn a_seat_frame_carries_bytes_the_gateway_never_decodes() {
        let inner = v1::Envelope {
            msg: Some(v1::envelope::Msg::Error(v1::Error {
                code: 7,
                message: "no such seat".into(),
            })),
        };
        let bytes = inner.encode_to_vec();

        let outer = v1::Envelope {
            msg: Some(v1::envelope::Msg::SeatFrame(v1::SeatFrame {
                seat: 3,
                envelope: bytes.clone().into(),
            })),
        };
        let wire = outer.encode_to_vec();

        let back = v1::Envelope::decode(&wire[..]).expect("the frame decodes");
        let Some(v1::envelope::Msg::SeatFrame(frame)) = back.msg else {
            panic!("a seat frame came back as something else");
        };
        assert_eq!(frame.seat, 3);
        assert_eq!(
            frame.envelope, bytes,
            "the payload is the bytes it was handed, byte for byte"
        );
        assert_eq!(
            v1::Envelope::decode(&frame.envelope[..]).expect("the payload decodes"),
            inner,
            "and the seat reads the message the engine wrote"
        );
    }

    /// A message kind this build has never heard of decodes as *no message*
    /// rather than as a failure. That is what lets one end of the wire learn
    /// a new envelope without the other end refusing every frame that
    /// follows it: prost skips a field number it does not know, so the
    /// `oneof` is simply empty and the reader answers the unknown the way it
    /// answers anything it cannot act on.
    #[test]
    fn an_envelope_from_a_newer_peer_is_read_as_nothing_rather_than_refused() {
        // Field 999, wire type 2 (length-delimited), four bytes of payload —
        // the shape any future member of the `oneof` would have.
        let mut wire = Vec::new();
        prost::encoding::encode_key(999, prost::encoding::WireType::LengthDelimited, &mut wire);
        prost::encoding::encode_varint(4, &mut wire);
        wire.extend_from_slice(b"soon");

        let envelope = v1::Envelope::decode(&wire[..]).expect("an unknown field is skipped");
        assert!(
            envelope.msg.is_none(),
            "an unknown message must arrive as no message, not as a wrong one"
        );
    }

    /// An empty envelope is legal on the wire and means nothing was said.
    /// Every reader matches on `Some(msg)`, so this is the case they all
    /// have to fall through — a decoder that refused it would turn a
    /// harmless frame into a dropped connection.
    #[test]
    fn an_envelope_with_no_message_survives_the_trip() {
        let empty = v1::Envelope { msg: None };
        let wire = empty.encode_to_vec();
        assert!(wire.is_empty(), "nothing said is nothing sent");
        assert_eq!(
            v1::Envelope::decode(&wire[..]).expect("an empty envelope decodes"),
            empty
        );
    }

    /// A peer is refused by the one sentence every boundary shares, and it
    /// names both numbers; this build's own version is not refused.
    #[test]
    fn a_peer_of_another_protocol_is_told_both_numbers() {
        let newer = PROTOCOL_VERSION + 1;
        assert!(super::version_refusal("This client", 32).is_some());
        let said = super::version_refusal("This agent", newer).expect("refused");
        assert!(said.starts_with("This agent"), "{said}");
        assert!(said.contains(&newer.to_string()), "{said}");
        assert!(said.contains(&PROTOCOL_VERSION.to_string()), "{said}");
        let silent = super::version_refusal("This engine", 0).expect("refused");
        assert!(silent.contains("does not say"), "{silent}");
        assert!(silent.contains(&PROTOCOL_VERSION.to_string()), "{silent}");
        assert_eq!(
            super::version_refusal("This client", PROTOCOL_VERSION),
            None
        );
    }

    /// A seat socket says which protocol it speaks in the one path every
    /// dialer opens it on.
    #[test]
    fn a_seat_socket_says_its_protocol() {
        assert_eq!(
            super::seat_socket_path("g", "t"),
            format!("/games/g/ws?ticket=t&protocol={PROTOCOL_VERSION}")
        );
    }

    /// A seat socket's address carries a ticket and never a token (#294):
    /// whatever it is handed goes under `ticket=`, and no `token=` appears.
    #[test]
    fn a_seat_socket_path_names_no_token() {
        let path = super::seat_socket_path("g", "secret");
        assert!(path.contains("ticket=secret"), "{path}");
        assert!(!path.contains("token="), "{path}");
    }

    /// The version is a refusal, not a label: an engine and a client that
    /// disagree on it do not talk. It is written down here so that raising
    /// it is a deliberate line in a diff rather than a number that drifted.
    #[test]
    fn the_wire_version_covers_all_positional_and_private_contracts() {
        // 2: `SeatReady` and `Curtain` (#256).
        // 3: `SeatSettingMsg` (#265). An engine built before it drops the
        // frame without a word, so a client that sent one would wait for a
        // view that never comes.
        // 4: render readiness, clock probes and the scheduled shared entrance.
        // 5: explicit free-mulligan counts in room presets.
        // 6: per-permanent starting counters in room presets.
        // Not 7 for `FlushRecord`/`RecordFlushed` (#323): engine link only,
        // and a peer from before them passes over either
        // (`an_envelope_from_a_newer_peer_is_read_as_nothing_rather_than_refused`).
        // 7: PayLife choices for Ward; older clients cannot decode this enum.
        // 8: a `ManaCost` in a `Pending` is its notation (`"{2}{U}{U}"`), not
        // the sixteen-slot list a replicated cost overflowed.
        // 9: private hand-inspection prompt; older clients cannot decode it.
        // 10: explicit keep-card choices and public permanent selections.
        // 11: spell cost increases; older clients would plan insufficient mana.
        // 12: explicit land-counter removal choice.
        // 13: counter amounts and retargeting; older clients cannot decode these prompts.
        // 14: optional mana payments, opponent sacrifices and target counts beyond 255.
        // 15: damage-effect ordering and prevention allocation with stable choice identities.
        // 16: exact incarnation source choices and their historical view projection.
        // 17: exact target incarnations and historical target projections.
        // 18: finite damage-redirection choices and allocation semantics.
        // 19: temporary player special actions and payment-window offers.
        // 20: semantic text choices, constrained mana and controlled decisions.
        // Not 21 for `SeatMind` (a seat's declared mind, for the record):
        // seat to engine only, and an engine from before it drops it in
        // `seat_frame`'s `_` arm, as with `AiLog`.
        // 21: Library of Leng's arrangement, an `ArrangePrompt` variant an
        // older client cannot decode.
        // 22: False Orders' re-block, a `ChoicePrompt` variant likewise.
        // 23: Raging River's division prompt, a variant likewise.
        // 24: Camouflage's pile prompt, likewise.
        // 26: spectators (25 is another branch's): `SpectatorsChanged`,
        // `SpectatorFrame`, `Spectators`, `/games/{id}/watch`.
        // 27: a damage decision's `DamageEffectKind::PreventAll`.
        // 28: independent delayed-life-loss cancellation offers.
        // 29: explicit life-payment amounts above 65535 require u32 readers.
        // 30: `PlayerAction::PayWindow`, an action an older engine cannot
        // decode. Not for `CloseGame`/`GameClosed` (an admin ends a game,
        // beta.8): `CloseGame` is engine link only, and an engine or client
        // from before `GameClosed` passes over it, as with `AiLog`.
        // 31: draw questions, `ChoicePrompt::Dredge` and
        // `ChoicePrompt::DrawReplacement`, variants an older client cannot
        // decode.
        // 32: additional `DamageEffectKind` variants an older client cannot decode.
        // 33: damage-clearing destruction replacement prompt.
        // 34: Battle defender and Defense counter variants.
        // 35: MutatePosition, Dungeon and DungeonRoom cast-mode choices.
        // 36: engines report each seat's participation ending, which the
        // gateway now requires to release an account-bound game.
        // 37: a vote, `ChoicePrompt::Vote`, a variant an older client cannot
        // decode.
        // 38: the sign-in by gateway token and a session token that rotates, which
        // an older client cannot speak; `HelloAck::refusal` says why a socket was
        // refused (`docs/protocol.md` §"Refusals").
        // 39: a view goes as a `ViewFrame`, whole once and then what changed, which
        // a reader from before it cannot read at all.
        // 40: that frame and an action go in binary, which a reader of 39 cannot read.
        // 41: no JSON is left in a game's frames.
        // 42: a printing shown goes alone, which a reader of 41 would take for an
        // empty payload.
        // 43: an offer says what restricted mana may pay for
        // (`LegalActions::restricted_taps`, `restricted_floating`), two fields
        // a positional reader of 42 would take for bytes after the value.
        // 44: an offer states a suspend still to be paid for
        // (`LegalActions::unpaid_suspend`), a view the division a spell on the
        // stack announced (`PlayerView::divided`) and a log line its step
        // (`LogEntry::step`): fields a positional reader of 43 would take for
        // bytes after the value.
        // 45: a life payment names the permanent it is for
        // (`YesNoPrompt::PayLife::permanent`, Cleansing's land): a field a
        // positional reader of 44 would take for the next one.
        // 46: `ChoicePrompt::SearchLibraryTappedAndHand`,
        // `YesNoPrompt::RevealHandForGuess`, and
        // `LogEvent::DidNotResolve::ability` on positional binary frames.
        // 47: `PlayerView::held_choices`, `SpectatorView::held_choices`,
        // `ViewField::HeldChoices`, `LogEvent::CardsChosen`, and
        // `PublicObject`'s chosen card type, second subtype, chosen number,
        // and linked exile on positional binary frames.
        // 48: compiled execution identity; 49: private Tower durable transport.
        // 50: `YesNoPrompt::KickerOption` for multi-kicker spells on positional binary frames.
        // 51: `ChoicePrompt::Amplify` for Amplify hand reveals on positional binary frames.
        // 52: `CastModeKind::Bestow` and `CastModeKind::JumpStart` on positional binary frames.
        // 53: `CastModeKind::Blitz` on positional binary frames.
        // 54: `CastModeKind::Emerge`, `YesNoPrompt::Bargain`, and `LegalActions::spell_spending`.
        // 55: `CastModeKind::Fused`, `NumberPrompt::Squad`, `YesNoPrompt::Offspring`, and
        // `ChoicePrompt::ManifestDread` on positional binary frames.
        // 56: `CastModeKind::Plot`, `PublicObject::plotted`, and `YesNoPrompt::Casualty`
        // on positional binary frames.
        // 57: `DamageEffectKind::MultiplyDamage` and `DamageEffectKind::AddDamage` on
        // positional binary frames.
        // 58: `CastModeKind::Impending` and `YesNoPrompt::Gift` on positional binary frames.
        // 59: `CastModeKind::Warp`, `PublicObject::warped`, `ChoicePrompt::Devour`, and
        // `ManaPoolView::snow` on positional binary frames.
        // 60: `CastModeKind::Retrace`, `YesNoPrompt::{Riot, Unleash}`, and
        // `PublicObject::suspected` on positional binary frames.
        // 61–64: Engine Waves 9a–10b (`AssignDamageAsThoughUnblocked`, `ClashBottom`,
        // `ChooseRingBearer`, `ChooseFirstCostBranch`, `OpeningHandBattlefield`,
        // `OpeningHandReveal`, `Sneak`, `Endure`, `Awaken`, `Splice`, `CipherEncode`,
        // `Champion`, `Conspire`, and `Tribute`) on positional binary frames.
        // 65: `PublicObject::class_level` and `LogEvent::ClassLevel`.
        // 66: `CastModeKind::VoteOption`, `YesNoPrompt::AdditionalVote`,
        // `LogEvent::VotedOption`, and `ChoicePrompt::CostRemoveCounter`.
        // 67: `PlayerView/SpectatorView::initiative`, `LogEvent::TookInitiative`,
        // and `Dungeon::Undercity`.
        // 68: `CastModeKind::RepeatedModes` and `NumberPrompt::DivideCounters`.
        // 69: `CastModeKind::OptionalCost`, `ChoicePrompt::CollectEvidence`,
        // `Pending::ChooseCounterKind`, `PlayerAction::ChooseCounterKind`, and
        // CR 122.1b keyword counters, `YesNoPrompt::SearchLibrary`,
        // `ChoicePrompt::SearchLibraryOrGraveyard`,
        // `ManaPayment::Variable`, `NumberPrompt::VariableManaPayment`,
        // `NumberPrompt::VariableEnergyPayment`, and `NumberPrompt::VariableLifePayment`.
        assert_eq!(PROTOCOL_VERSION, 69);
    }

    #[test]
    fn a_unix_address_names_its_socket_file_and_a_url_names_none() {
        use super::unix_socket;
        assert_eq!(
            unix_socket("unix:/run/baylee/gateway.sock"),
            Some("/run/baylee/gateway.sock")
        );
        assert_eq!(unix_socket("unix:relative.sock"), Some("relative.sock"));
        assert_eq!(unix_socket("unix:"), None, "no path is no socket");
        assert_eq!(unix_socket("http://127.0.0.1:28766"), None);
        assert_eq!(unix_socket("ws://127.0.0.1:28766/engine/ws"), None);
    }
}
