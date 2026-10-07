//! A parked seat's permanents as chips (DESIGN-v5 §4.2's rows, DESIGN-v6
//! §2.2's peek line; DESIGN-v8 WA9): what a Focus ring peek shows of a
//! board the table does not draw, so a target or a defender on it is one
//! press away without bringing the seat across.
//!
//! Read off the board model (`board::SeatPod`), never the view: a pile is
//! already one group (thirty identical tokens are one chip, its count on
//! it), a face-down card is already a group whose status says so (its chip
//! names nothing), and hidden information is as unrepresentable here as on
//! the table.
//!
//! The order: while a question is open every legal target on the seat
//! first (any kind, the rows' rule S-D: never folded away), then its
//! planeswalkers (each a defender: `Defender::Planeswalker`), then its
//! creatures; support and lands only as counts, unless they are targets.

use crate::board::{CardGroup, SeatPod};
use crate::layout::LaneKind;
use baylee_core::ids::ObjectId;

/// What a chip shows of its permanent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChipKind {
    /// A creature: its power and toughness, and the damage marked on it.
    Creature {
        /// Its power.
        power: i16,
        /// Its toughness.
        toughness: i16,
        /// The damage marked on it.
        damage: u16,
    },
    /// A planeswalker: its loyalty. A defender in an attack.
    Planeswalker {
        /// Its loyalty.
        loyalty: u16,
    },
    /// Anything else that is a legal target now (an enchantment, a land).
    Other,
}

/// One chip.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Chip {
    /// The permanent a press on the chip answers with: the pile's
    /// representative.
    pub object: ObjectId,
    /// How many permanents it stands for: a pile's size.
    pub count: usize,
    /// What it shows.
    pub kind: ChipKind,
    /// Its name; `None` face down (a face-down chip names nothing).
    pub name: Option<String>,
    /// Tapped: drawn dimmed.
    pub tapped: bool,
    /// Summoning-sick: the moon.
    pub sick: bool,
    /// A legal target of the open question: outlined, and sorted first.
    pub target: bool,
}

/// The chips of one seat, and what they leave out.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SeatChips {
    /// The chips shown, at most the limit asked for.
    pub chips: Vec<Chip>,
    /// How many chips there were past the limit: the `+n` chip.
    pub folded: usize,
    /// How many lands the seat has: `n ⛰`.
    pub lands: usize,
    /// How many other support permanents (artifacts, enchantments) that are
    /// not chips.
    pub support: usize,
}

fn chip_of(group: &CardGroup, target: bool) -> Chip {
    let kind = match (group.power, group.toughness, group.loyalty) {
        (_, _, Some(loyalty)) => ChipKind::Planeswalker { loyalty },
        (Some(power), Some(toughness), None) => ChipKind::Creature {
            power,
            toughness,
            damage: group.damage,
        },
        _ => ChipKind::Other,
    };
    Chip {
        object: group.representative,
        count: group.members.len().max(1),
        kind,
        name: (!group.status.is_face_down()).then(|| group.name.clone()),
        tapped: group.status.is_tapped(),
        sick: group.summoning_sick,
        target,
    }
}

/// The chips `pod` shows, at most `limit` of them; `target` says whether a
/// permanent is a legal target of the question open now.
#[must_use]
pub fn seat_chips(pod: &SeatPod, limit: usize, target: impl Fn(ObjectId) -> bool) -> SeatChips {
    let groups = |kind: LaneKind| {
        pod.lanes
            .iter()
            .filter(move |lane| lane.kind == kind)
            .flat_map(|lane| lane.groups.iter())
    };
    let aimed = |group: &CardGroup| group.members.iter().any(|m| target(*m));
    let mut all: Vec<Chip> = Vec::new();
    // Targets first, from every lane.
    for kind in LaneKind::ALL {
        all.extend(groups(kind).filter(|g| aimed(g)).map(|g| chip_of(g, true)));
    }
    // Then planeswalkers, then creatures, that are not targets already.
    all.extend(
        groups(LaneKind::Support)
            .filter(|g| !aimed(g) && g.loyalty.is_some())
            .map(|g| chip_of(g, false)),
    );
    all.extend(
        groups(LaneKind::Creatures)
            .filter(|g| !aimed(g))
            .map(|g| chip_of(g, false)),
    );
    let count = |kind: LaneKind, skip_walkers: bool| {
        groups(kind)
            .filter(|g| !aimed(g) && (!skip_walkers || g.loyalty.is_none()))
            .map(|g| g.members.len().max(1))
            .sum()
    };
    let folded = all.len().saturating_sub(limit);
    all.truncate(limit);
    SeatChips {
        chips: all,
        folded,
        lands: count(LaneKind::Lands, false),
        support: count(LaneKind::Support, true),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::{BoardModel, Openings};
    use crate::test_support::{ViewBuilder, token};
    use baylee_core::ids::PlayerId;
    use baylee_core::types::TypeSet;
    use baylee_view::ObjectStatus;

    fn pod_of(view: &baylee_view::PlayerView, seat: u8) -> SeatPod {
        let model =
            BoardModel::from_view(view, Openings::none(), &[], crate::board::Registry::none());
        model.pod(PlayerId::new(seat)).expect("a pod").clone()
    }

    /// The rows' guarantees (DESIGN-v5 §12, *Rows*): thirty identical
    /// tokens are one chip with its count; a face-down chip names nothing;
    /// a planeswalker is a chip (a defender); an enchantment is a chip only
    /// while it is a legal target, and then first; lands and the other
    /// support are counts; past the limit the rest fold into `+n`.
    #[test]
    fn a_parked_board_reads_as_chips() {
        let mut objects: Vec<_> = (0..30)
            .map(|i| token(100 + i, 1, "Soldier", 1, 1))
            .collect();
        let mut hidden = token(200, 1, "Secret Thing", 2, 2);
        hidden.status = ObjectStatus::FACE_DOWN;
        objects.push(hidden);
        let mut walker = token(201, 1, "Karn", 0, 0);
        walker.types = TypeSet::PLANESWALKER;
        walker.power = None;
        walker.toughness = None;
        walker.loyalty = Some(5);
        objects.push(walker);
        let mut aura = token(202, 1, "Pacifism", 0, 0);
        aura.types = TypeSet::ENCHANTMENT;
        aura.power = None;
        aura.toughness = None;
        objects.push(aura);
        for i in 0..4 {
            let mut land = token(300 + i, 1, "Forest", 0, 0);
            land.types = TypeSet::LAND;
            land.power = None;
            land.toughness = None;
            objects.push(land);
        }
        let view = ViewBuilder::new(3).with_battlefield(1, objects).build();
        let pod = pod_of(&view, 1);

        let quiet = seat_chips(&pod, 8, |_| false);
        assert_eq!(quiet.lands, 4);
        assert_eq!(quiet.support, 1, "the enchantment, as a count");
        let soldiers: Vec<&Chip> = quiet
            .chips
            .iter()
            .filter(|c| c.name.as_deref() == Some("Soldier"))
            .collect();
        assert_eq!(soldiers.len(), 1, "thirty tokens, one chip");
        assert_eq!(soldiers[0].count, 30);
        assert!(
            quiet.chips.iter().any(|c| c.name.is_none()),
            "the face-down creature is a chip that names nothing"
        );
        assert!(
            quiet
                .chips
                .iter()
                .all(|c| c.name.as_deref() != Some("Secret Thing")),
            "and never its name"
        );
        assert!(
            matches!(
                quiet.chips.first().map(|c| c.kind),
                Some(ChipKind::Planeswalker { loyalty: 5 })
            ),
            "a planeswalker first: a defender"
        );
        assert!(
            quiet
                .chips
                .iter()
                .all(|c| c.name.as_deref() != Some("Pacifism"))
        );

        let aimed = seat_chips(&pod, 8, |o| o == baylee_core::ids::ObjectId::new(202, 0));
        assert_eq!(
            aimed.chips[0].name.as_deref(),
            Some("Pacifism"),
            "a target first"
        );
        assert!(aimed.chips[0].target);
        assert_eq!(aimed.support, 0, "and no longer a count");

        let tight = seat_chips(&pod, 2, |_| false);
        assert_eq!(tight.chips.len(), 2);
        assert_eq!(tight.folded, quiet.chips.len() - 2, "the rest fold into +n");
    }
}
