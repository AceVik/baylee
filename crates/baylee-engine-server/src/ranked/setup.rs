use super::{Replica, hash_fields, uuid_v7};
use baylee_core::ids::PlayerId;
use baylee_core::preset::{FormatId, GamePreset, HouseRules, SeatCapabilities, SeatController};
use baylee_protocol::v1;
use prost::Message as _;

impl Replica {
    /// Starts only from a registered two-seat ranked preset and this exact
    /// compiled artifact. Returns the private opening vote at journal position 0.
    ///
    /// # Errors
    /// The job, execution identity, roster or ranked configuration is invalid.
    pub fn open(order: v1::RankedEngineSetup) -> Result<(Self, Vec<v1::Envelope>), &'static str> {
        Self::open_with_identity(order, &crate::variant::identity())
    }

    pub(super) fn open_with_identity(
        order: v1::RankedEngineSetup,
        identity: &baylee_protocol::execution::Identity,
    ) -> Result<(Self, Vec<v1::Envelope>), &'static str> {
        let setup = order.setup.ok_or("missing protected setup")?;
        if order.generation == 0
            || !uuid_v7(&order.match_id)
            || !uuid::Uuid::parse_str(&setup.game_id).is_ok_and(|id| uuid_v7(id.as_bytes()))
            || setup.seat_names.len() != 2
            || setup.seat_names.iter().any(|n| n.len() > 256)
            || !setup.seat_guests.is_empty() && setup.seat_guests != [false, false]
            || !setup.seat_accounts.is_empty() && setup.seat_accounts != [true, true]
            || setup.preset_json.len() > 1024 * 1024
            || identity.ranked_compatibility().is_none()
            || setup.execution != identity.to_metadata()
        {
            return Err("invalid protected setup");
        }
        let preset: GamePreset =
            serde_json::from_slice(&setup.preset_json).map_err(|_| "invalid protected preset")?;
        if !ranked_duel(&preset) || preset.validate().is_err() {
            return Err("unsupported protected rules");
        }
        let opening = hash_fields(&[
            b"baylee/ranked/setup/v1",
            &order.match_id,
            &setup.encode_to_vec(),
        ]);
        let mut session = crate::variant::HostedSession::recorded_for_execution(&preset, identity)
            .ok_or("incompatible protected game")?;
        session.describe(setup.game_id.clone(), setup.seat_names);
        session.mark_people(&[true, true]);
        // Prime the private per-seat reveal history, without releasing frames.
        session.pump_slice(0);
        let mut replica = Self {
            session,
            game_id: setup.game_id,
            generation: order.generation,
            primary: order.primary,
            committed: None,
            staged: None,
        };
        let mut frames = replica.whole(PlayerId::new(0));
        frames.extend(replica.whole(PlayerId::new(1)));
        let vote = replica.stage(0, vec![], opening, &[], true, frames);
        Ok((replica, vote))
    }
}

fn ranked_duel(preset: &GamePreset) -> bool {
    let life = match preset.format {
        FormatId::Commander => 40,
        FormatId::Freeform | FormatId::Highlander => 20,
        FormatId::Custom => return false,
    };
    preset.house_rules == HouseRules::ranked()
        && preset.modifiers.is_empty()
        && preset.seats.len() == 2
        && preset.seats.iter().all(|seat| {
            seat.controller == SeatController::Open
                && seat.capabilities == SeatCapabilities::default()
                && seat.starting_life == Some(life)
                && seat.starting_hand.is_none()
                && seat.starting_battlefield.is_empty()
                && seat.team.is_none()
                && seat.emblems.is_empty()
                && seat.deck.len() + seat.sideboard.len() + seat.commanders.len() <= 250
        })
}
