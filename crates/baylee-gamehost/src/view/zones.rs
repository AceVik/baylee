use super::SeatContext;
use super::object::public_object;
use baylee_ai::pending_player;
use baylee_core::ids::{ObjectId, PlayerId, SeatSet};
use baylee_engine::choice::Pending;
use baylee_engine::object::GameObject;
use baylee_engine::state::GameState;
use baylee_engine::zone::{Zone, ZoneLocation};
use baylee_view::PublicObject;

/// A controller may inspect what the controlled player can inspect (CR 720.4).
pub(super) fn entitled_viewer(obj: &GameObject, seat: PlayerId, controlled: SeatSet) -> PlayerId {
    let holder = if obj.zone.is_hidden_by_default() {
        obj.owner
    } else {
        obj.controller
    };
    if controlled.contains(holder) {
        holder
    } else {
        seat
    }
}

/// Collects a public zone into view objects.
pub(super) fn zone(
    state: &GameState,
    loc: ZoneLocation,
    seat: PlayerId,
    controlled: SeatSet,
) -> Vec<PublicObject> {
    state
        .zones
        .list(loc)
        .iter()
        .filter_map(|&id| {
            let viewer = entitled_viewer(state.object(id)?, seat, controlled);
            public_object(state, id, viewer)
        })
        .collect()
}

/// Collects one zone per seat, indexed by seat order.
pub(super) fn per_seat_zone(
    state: &GameState,
    loc: fn(PlayerId) -> ZoneLocation,
    seat: PlayerId,
    controlled: SeatSet,
) -> Vec<Vec<PublicObject>> {
    state
        .players
        .iter()
        .map(|p| zone(state, loc(p.id), seat, controlled))
        .collect()
}

/// The floating mana of one seat, for the view.
///
/// Restricted mana is summed *per colour* rather than into one number. The
/// engine holds one `RestrictedMana` per production, so two taps of the same
/// Cavern naming white are two entries; the view is what a player reads, and
/// "two restricted white" is the reading — how it got there is not.
pub(super) fn mana_pool(state: &GameState, player: PlayerId) -> baylee_view::ManaPoolView {
    use baylee_core::mana::ManaColor;
    let pool = &state.players[player.get() as usize].mana_pool;
    let mut restricted = [0u64; 6];
    for mana in pool.restricted() {
        let slot = &mut restricted[mana.color.index()];
        *slot += u64::from(mana.amount);
    }
    baylee_view::ManaPoolView {
        spending: baylee_engine::casting::mana_spending(state, player),
        white: pool.available(ManaColor::White),
        blue: pool.available(ManaColor::Blue),
        black: pool.available(ManaColor::Black),
        red: pool.available(ManaColor::Red),
        green: pool.available(ManaColor::Green),
        colorless: pool.available(ManaColor::Colorless),
        restricted,
    }
}

/// The objects a pending choice puts in front of the seat it is asking.
///
/// Only the choices that can name a *hidden* object are listed. Combat is
/// not: [`Pending::ChooseAttackers`] and [`Pending::ChooseBlockers`] name
/// creatures, and every creature in a declaration is on the battlefield, which
/// the view already carries in full. Neither is [`Pending::YesNo`]'s miracle
/// card — a miracle is revealed from the hand it was drawn into, and that is
/// the asking seat's own hand. A pile choice ([`Pending::ChoosePile`]) names
/// every card of every pile: all of them were revealed, and the pile taken
/// is chosen by what is in it.
fn offered(pending: &Pending) -> Vec<ObjectId> {
    match pending {
        Pending::ChooseCards { options, .. }
        | Pending::ChooseTargets { options, .. }
        | Pending::LegendChoice { options, .. } => options.clone(),
        Pending::Arrange { cards, .. } => cards.clone(),
        Pending::ChoosePile { piles, .. } => piles.concat(),
        _ => Vec::new(),
    }
}

/// Whether `seat`'s view already carries this object somewhere.
///
/// The zones a view sends in full are the public ones plus the seat's own
/// hand; a library, a sideboard and somebody else's hand are counts. An
/// object in one of those is an object the client has no other way to draw,
/// which is exactly what [`PlayerView::looking_at`] is for.
pub(super) const fn shown_elsewhere(obj: &GameObject, seat: PlayerId) -> bool {
    match obj.zone {
        Zone::Battlefield | Zone::Stack | Zone::Graveyard | Zone::Exile | Zone::Command => true,
        Zone::Hand => match obj.zone_owner {
            Some(owner) => owner.get() == seat.get(),
            None => false,
        },
        Zone::Library | Zone::OutsideGame => false,
    }
}

/// The hidden cards this seat is being shown, if any.
///
/// The entitlement rule is one sentence and it is the whole safety argument:
/// **an object the engine asks you about is an object you may see.** A search
/// is only offered to the searcher, a scry only to the scrying player, and
/// the engine has already filtered both down to what that player is allowed
/// to look at — so this function adds no judgement of its own beyond checking
/// that the question is addressed to `seat`.
///
/// It is deliberately not a memory. The list is rebuilt from the outstanding
/// choice on every view, so a card stops being visible the instant the choice
/// is answered, and there is no place for one to linger.
pub(super) fn looking_at(
    state: &GameState,
    seat: PlayerId,
    pending: Option<&Pending>,
    ctx: &SeatContext,
) -> Vec<PublicObject> {
    let Some(pending) = pending else {
        return Vec::new();
    };
    if pending_player(pending) != Some(seat) && ctx.awaiting != Some(seat) {
        return Vec::new();
    }
    offered(pending)
        .into_iter()
        .filter(|id| {
            state.object(*id).is_some_and(|obj| {
                !shown_elsewhere(obj, seat)
                    || matches!(
                        pending,
                        Pending::ChooseCards {
                            prompt: baylee_engine::choice::ChoicePrompt::CastFaceDown { .. }
                                | baylee_engine::choice::ChoicePrompt::CommandCard,
                            ..
                        }
                    )
            })
        })
        .filter_map(|id| {
            public_object(
                state,
                id,
                entitled_viewer(state.object(id)?, seat, ctx.controlled_players),
            )
        })
        .collect()
}
