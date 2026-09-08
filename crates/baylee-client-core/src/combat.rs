//! Combat as the table draws it: lines, a focus, and what is coming at whom.
//!
//! Two different things describe a fight and the player has to read them as
//! one picture. `PlayerView::combat` is the declaration the **engine** has
//! accepted — it is the only source for an attack made *against* this seat,
//! and it is what still stands once this seat's own declaration has been
//! sent. [`Interaction::assignments`] is the declaration this seat is
//! **building** and has not sent yet, which no view knows about because it
//! exists only in this client.
//!
//! Before this module the client read neither: `view.combat` was touched in
//! one place, to stop attackers from merging into a group, and
//! `Interaction::assignment` had no caller at all. So an attack aimed at this
//! seat was invisible, and this seat's own pairing vanished the moment the
//! engine confirmed it.
//!
//! Both sources therefore produce the same [`Line`], and whether the engine
//! has accepted it is a separate `standing` flag rather than a third variant
//! — a proposed attack and a standing one are the same claim about the same
//! two objects, drawn with the same geometry and a different weight. Making
//! it a variant would have multiplied the kinds by two and left every reader
//! to remember which pairs meant "attack".
//!
//! [`Tally`] is the one piece of arithmetic this client is allowed to do:
//! `docs/design.md` §6 refuses client-side rules inference *beyond arithmetic
//! on projected public numbers*, and a sum of printed power is exactly that.
//! Nothing here consults a rule — first strike, trample and damage
//! prevention all belong to the engine, and a tally that pretended to know
//! them would be lying at the one moment a player is deciding whether to
//! block.

use crate::interaction::{CombatFocus, Interaction};
use baylee_core::ids::{Defender, ObjectId, PlayerId};
use baylee_view::PlayerView;

/// What a combat line points at.
///
/// An attack ends at a seat or at one of its planeswalkers (CR 508.1a); a
/// block ends at the attacker being blocked, which is always an object. One
/// enum for both, because the renderer resolves an end to a position on the
/// table and a seat has one as much as a card does.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LineEnd {
    /// The defending player themself — drawn at their side of the table.
    Seat(PlayerId),
    /// A planeswalker, or the attacker a blocker was put in front of.
    Object(ObjectId),
}

impl LineEnd {
    /// The end a [`Defender`] names.
    #[must_use]
    pub const fn of(defender: Defender) -> Self {
        match defender {
            Defender::Player(p) => Self::Seat(p),
            Defender::Planeswalker(o) => Self::Object(o),
        }
    }

    /// The object this end names, if it is one.
    #[must_use]
    pub const fn object(self) -> Option<ObjectId> {
        match self {
            Self::Object(o) => Some(o),
            Self::Seat(_) => None,
        }
    }
}

/// Which declaration a line is.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LineKind {
    /// A creature attacking a seat or a planeswalker.
    Attack,
    /// A creature blocking an attacker.
    Block,
}

/// One line the table draws between two things.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Line {
    /// The creature that was declared.
    pub from: ObjectId,
    /// What it was declared against.
    pub to: LineEnd,
    /// Attack or block.
    pub kind: LineKind,
    /// Whether the engine has accepted this declaration.
    ///
    /// `false` while this seat is still building it — the line is a
    /// *proposal*, and the renderer draws it lighter so a player can tell
    /// what they have committed from what they are still choosing.
    pub standing: bool,
}

/// What is coming at one defender.
///
/// Sums of projected power, nothing more. `unblocked` is the part with no
/// blocker in front of it — the number a player is actually deciding about,
/// and the reason a tally is worth drawing at all.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Tally {
    /// The seat or planeswalker being attacked.
    pub at: LineEnd,
    /// How many creatures are attacking it.
    pub attackers: usize,
    /// Their total projected power.
    pub power: i32,
    /// The power among them that nothing is blocking.
    pub unblocked: i32,
}

/// Combat, resolved from both sources and ready to draw.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct Combat {
    /// Every attack and block, standing or proposed.
    ///
    /// Ordered: the engine's declarations in the order it listed them, then
    /// this seat's proposals in the order they were made. Stable frame to
    /// frame, which is what lets the renderer diff it.
    pub lines: Vec<Line>,
    /// One entry per defender under attack, in first-seen order.
    pub tallies: Vec<Tally>,
    /// What a declaration made right now would be aimed at.
    ///
    /// `None` outside a combat declaration. This is the thing the focus pulse
    /// draws, and it is deliberately *not* folded into `lines`: the focus is
    /// where the next line would go, not a line.
    pub focus: Option<LineEnd>,
}

impl Combat {
    /// Reads combat out of the view and the declaration being built.
    ///
    /// Cheap enough to call every frame at these board sizes, and pure, so
    /// the renderer never has to cache it.
    #[must_use]
    pub fn read(view: &PlayerView, interaction: Option<&Interaction>) -> Self {
        let mut lines = Vec::new();

        for attacker in &view.combat.attackers {
            lines.push(Line {
                from: attacker.creature,
                to: LineEnd::of(attacker.defending),
                kind: LineKind::Attack,
                standing: true,
            });
        }
        for blocker in &view.combat.blockers {
            lines.push(Line {
                from: blocker.blocker,
                to: LineEnd::Object(blocker.attacker),
                kind: LineKind::Block,
                standing: true,
            });
        }

        // A proposal the engine has already accepted is not drawn twice. It
        // cannot normally happen — a seat's own declaration is sent whole and
        // the mode ends — but a re-offered question can hand back a pending
        // whose pairs are still filled, and two lines in the same place read
        // as one heavier line rather than as a bug.
        if let Some(interaction) = interaction {
            for (from, against) in interaction.assignments() {
                let (to, kind) = match against {
                    CombatFocus::Defender(d) => (LineEnd::of(d), LineKind::Attack),
                    CombatFocus::Attacker(a) => (LineEnd::Object(a), LineKind::Block),
                    CombatFocus::None => continue,
                };
                let already = lines
                    .iter()
                    .any(|l| l.from == from && l.to == to && l.kind == kind);
                if !already {
                    lines.push(Line {
                        from,
                        to,
                        kind,
                        standing: false,
                    });
                }
            }
        }

        let focus = interaction.and_then(|i| match i.combat_focus() {
            CombatFocus::Defender(d) => Some(LineEnd::of(d)),
            CombatFocus::Attacker(a) => Some(LineEnd::Object(a)),
            CombatFocus::None => None,
        });

        let tallies = tally(view, &lines);
        Self {
            lines,
            tallies,
            focus,
        }
    }

    /// Whether there is anything to draw.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.lines.is_empty() && self.focus.is_none()
    }

    /// What a creature has been declared against, if anything.
    #[must_use]
    pub fn line_from(&self, creature: ObjectId) -> Option<&Line> {
        self.lines.iter().find(|l| l.from == creature)
    }

    /// The tally at one defender.
    #[must_use]
    pub fn tally_at(&self, end: LineEnd) -> Option<&Tally> {
        self.tallies.iter().find(|t| t.at == end)
    }
}

/// Sums the attacks at each defender, counting a block from either source.
///
/// First-seen order rather than a map, because the renderer draws these in a
/// row and a row that reshuffled itself between frames would be unreadable.
fn tally(view: &PlayerView, lines: &[Line]) -> Vec<Tally> {
    let power =
        |id: ObjectId| -> i32 { view.object(id).and_then(|o| o.power).map_or(0, i32::from) };
    // A blocker declared but not yet sent counts: the whole point of the
    // number is to answer "what still gets through if I block here".
    let blocked = |attacker: ObjectId| -> bool {
        lines
            .iter()
            .any(|l| l.kind == LineKind::Block && l.to == LineEnd::Object(attacker))
    };

    let mut out: Vec<Tally> = Vec::new();
    for line in lines.iter().filter(|l| l.kind == LineKind::Attack) {
        let p = power(line.from);
        let through = if blocked(line.from) { 0 } else { p };
        if let Some(existing) = out.iter_mut().find(|t| t.at == line.to) {
            existing.attackers += 1;
            existing.power += p;
            existing.unblocked += through;
        } else {
            out.push(Tally {
                at: line.to,
                attackers: 1,
                power: p,
                unblocked: through,
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{ViewBuilder, token};
    use baylee_engine::choice::{BlockOption, Pending};
    use baylee_view::{AttackerView, BlockerView};

    fn obj(slot: u32) -> ObjectId {
        ObjectId::new(slot, 0)
    }

    fn seat(id: u8) -> PlayerId {
        PlayerId::new(id)
    }

    /// Two 2/2s and a 3/3 belonging to seat 1, the seat across the table.
    fn attackers_of_seat_one() -> ViewBuilder {
        ViewBuilder::new(2).with_battlefield(
            1,
            vec![
                token(1, 1, "Bear", 2, 2),
                token(2, 1, "Bear", 2, 2),
                token(3, 1, "Ogre", 3, 3),
            ],
        )
    }

    #[test]
    fn an_attack_against_this_seat_is_drawn_although_this_seat_declared_nothing() {
        // The whole reason `view.combat` has to be read: the seat being
        // attacked is not the seat that declared, so nothing in this client's
        // own interaction knows the attack exists.
        let view = attackers_of_seat_one()
            .with_combat(
                vec![AttackerView {
                    creature: obj(1),
                    defending: Defender::Player(seat(0)),
                }],
                Vec::new(),
            )
            .build();

        let combat = Combat::read(&view, None);
        assert_eq!(
            combat.lines,
            vec![Line {
                from: obj(1),
                to: LineEnd::Seat(seat(0)),
                kind: LineKind::Attack,
                standing: true,
            }]
        );
    }

    #[test]
    fn a_declaration_being_built_is_drawn_before_it_is_sent() {
        let view = ViewBuilder::new(2)
            .with_battlefield(0, vec![token(1, 0, "Bear", 2, 2)])
            .build();
        let mut i = Interaction::new(
            Pending::ChooseAttackers {
                player: seat(0),
                attackers: vec![obj(1)],
                defenders: vec![Defender::Player(seat(1))],
            },
            seat(0),
        );
        assert!(i.declare_attacker(obj(1), Defender::Player(seat(1))));

        let combat = Combat::read(&view, Some(&i));
        assert_eq!(combat.lines.len(), 1);
        assert!(
            !combat.lines[0].standing,
            "nothing has been sent, so the line is a proposal"
        );
        assert_eq!(combat.lines[0].to, LineEnd::Seat(seat(1)));
    }

    #[test]
    fn a_proposal_the_engine_has_already_accepted_is_not_drawn_twice() {
        // A re-offered question hands back a pending whose pairs are still
        // filled while the same declaration already stands; two lines in the
        // same place read as one heavier line rather than as a bug.
        let view = ViewBuilder::new(2)
            .with_battlefield(0, vec![token(1, 0, "Bear", 2, 2)])
            .with_combat(
                vec![AttackerView {
                    creature: obj(1),
                    defending: Defender::Player(seat(1)),
                }],
                Vec::new(),
            )
            .build();
        let mut i = Interaction::new(
            Pending::ChooseAttackers {
                player: seat(0),
                attackers: vec![obj(1)],
                defenders: vec![Defender::Player(seat(1))],
            },
            seat(0),
        );
        assert!(i.declare_attacker(obj(1), Defender::Player(seat(1))));

        let combat = Combat::read(&view, Some(&i));
        assert_eq!(combat.lines.len(), 1, "the same claim is drawn once");
        assert!(combat.lines[0].standing, "and it is the standing one");
    }

    #[test]
    fn a_tally_sums_the_power_aimed_at_each_defender() {
        let view = attackers_of_seat_one()
            .with_combat(
                vec![
                    AttackerView {
                        creature: obj(1),
                        defending: Defender::Player(seat(0)),
                    },
                    AttackerView {
                        creature: obj(2),
                        defending: Defender::Player(seat(0)),
                    },
                    AttackerView {
                        creature: obj(3),
                        defending: Defender::Planeswalker(obj(50)),
                    },
                ],
                Vec::new(),
            )
            .build();

        let combat = Combat::read(&view, None);
        let at_seat = combat.tally_at(LineEnd::Seat(seat(0))).expect("seat tally");
        assert_eq!(at_seat.attackers, 2);
        assert_eq!(at_seat.power, 4, "two 2/2s");
        assert_eq!(at_seat.unblocked, 4, "and nothing is blocking them");

        let at_walker = combat
            .tally_at(LineEnd::Object(obj(50)))
            .expect("walker tally");
        assert_eq!(at_walker.power, 3);
    }

    #[test]
    fn a_block_this_seat_has_only_proposed_already_changes_what_gets_through() {
        // The number exists to answer "what still reaches me if I block
        // here", so a block that has not been sent has to count. Anything
        // else would show the player the board they are trying to leave.
        let view = attackers_of_seat_one()
            .with_battlefield(0, vec![token(10, 0, "Wall", 0, 4)])
            .with_combat(
                vec![
                    AttackerView {
                        creature: obj(1),
                        defending: Defender::Player(seat(0)),
                    },
                    AttackerView {
                        creature: obj(3),
                        defending: Defender::Player(seat(0)),
                    },
                ],
                Vec::new(),
            )
            .build();

        let before = Combat::read(&view, None);
        assert_eq!(
            before
                .tally_at(LineEnd::Seat(seat(0)))
                .expect("tally")
                .unblocked,
            5,
            "a 2/2 and a 3/3, nothing in the way"
        );

        let mut i = Interaction::new(
            Pending::ChooseBlockers {
                player: seat(0),
                attacker: seat(1),
                blockers: vec![BlockOption {
                    blocker: obj(10),
                    attackers: vec![obj(1), obj(3)],
                }],
            },
            seat(0),
        );
        assert!(i.declare_blocker(obj(10), obj(3)));

        let after = Combat::read(&view, Some(&i));
        assert_eq!(
            after
                .tally_at(LineEnd::Seat(seat(0)))
                .expect("tally")
                .unblocked,
            2,
            "the 3/3 is stopped, the 2/2 still gets there"
        );
        assert_eq!(
            after.tally_at(LineEnd::Seat(seat(0))).expect("tally").power,
            5,
            "and the total coming at the seat has not changed"
        );
    }

    #[test]
    fn a_standing_block_counts_the_same_as_a_proposed_one() {
        let view = attackers_of_seat_one()
            .with_battlefield(0, vec![token(10, 0, "Wall", 0, 4)])
            .with_combat(
                vec![AttackerView {
                    creature: obj(3),
                    defending: Defender::Player(seat(0)),
                }],
                vec![BlockerView {
                    blocker: obj(10),
                    attacker: obj(3),
                }],
            )
            .build();

        let combat = Combat::read(&view, None);
        let tally = combat.tally_at(LineEnd::Seat(seat(0))).expect("tally");
        assert_eq!(tally.power, 3);
        assert_eq!(tally.unblocked, 0);
        assert_eq!(combat.lines.len(), 2, "the attack and the block");
    }

    #[test]
    fn the_focus_is_where_the_next_line_would_go_and_is_not_itself_a_line() {
        let view = ViewBuilder::new(2)
            .with_battlefield(0, vec![token(1, 0, "Bear", 2, 2)])
            .build();
        let i = Interaction::new(
            Pending::ChooseAttackers {
                player: seat(0),
                attackers: vec![obj(1)],
                defenders: vec![Defender::Player(seat(1))],
            },
            seat(0),
        );

        let combat = Combat::read(&view, Some(&i));
        assert_eq!(combat.focus, Some(LineEnd::Seat(seat(1))));
        assert!(combat.lines.is_empty(), "nothing has been declared yet");
        assert!(!combat.is_empty(), "but there is still something to draw");
    }

    #[test]
    fn outside_combat_there_is_nothing_to_draw() {
        let view = ViewBuilder::new(2)
            .with_battlefield(0, vec![token(1, 0, "Bear", 2, 2)])
            .build();
        let combat = Combat::read(&view, None);
        assert!(combat.is_empty());
        assert!(combat.tallies.is_empty());
    }
}
