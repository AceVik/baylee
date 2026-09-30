//! Prevention shields (CR 615): damage that would be dealt and is not.
//!
//! A spell or ability that says "prevent" leaves a shield behind as it
//! resolves (CR 615.1, 615.3), and the shield waits for damage. Every writer
//! of damage asks [`apply`] how much of what it is about to deal still gets
//! through, at the one point where prevention belongs: after the damage is
//! known and before anything is marked, lost or journalled (CR 615.4 — a
//! shield cannot reach back to damage already dealt). Four writers do that
//! today, two in `combat` and two in `resolve::life`; a damage path that
//! skips this function deals damage no shield can see.
//!
//! What is here is the *resolved* kind of prevention — "the next 3 damage",
//! "all combat damage this turn". The standing kind a permanent's static
//! ability states (Maze of Ith's `Modifier::PreventDamageToIt`, protection)
//! is asked by each writer before this, is never used up, and is not a
//! shield.
//!
//! Every shield here lasts for the turn: all of them say "this turn", and
//! the cleanup step ends them with every other such effect (CR 514.2).
//!
//! Redirection (CR 614.9) is here too, because the same four writers ask
//! it at the same point, right after [`apply`]: [`redirect`] reads the
//! redirection shields and the statics that move a player's damage onto a
//! permanent, and says where the damage is dealt instead.

use crate::event::DamageTarget;
use crate::object::ObjectKind;
use crate::state::GameState;
use crate::zone::{Zone, ZoneLocation};
use baylee_cards_dsl::Filter;
use baylee_core::ids::{ObjectId, PlayerId};

/// What a shield stands in front of.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Shielded {
    /// A player.
    Player(PlayerId),
    /// A permanent, as the object it was when the shield was made: the id
    /// and the object's version at that moment (CR 400.7).
    Object(ObjectId, u32),
    /// Anything damage can be dealt to (Fog).
    Everything,
}

/// How a shield prevents, and how much of it is left.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum ShieldKind {
    /// "Prevent the next N damage" (CR 615.7): each 1 damage prevented
    /// takes 1 off, and the shield is gone at 0.
    Next(u32),
    /// "Prevent all combat damage that would be dealt this turn" (Fog):
    /// every combat damage event, never used up.
    AllCombat,
    /// "The next time a red source of your choice would deal damage to you
    /// this turn, prevent that damage" (CR 615.8): the next instance of
    /// damage from the chosen source, however much, and then it is gone.
    NextFrom {
        /// The source it waits for.
        source: ChosenSource,
        /// How much of that instance is still dealt (Forcefield's "all but
        /// 1"); 0 prevents all of it.
        all_but: u32,
        /// Its controller gains the life it prevented (CR 615.5).
        gain_life: bool,
        /// Only combat damage from that source.
        combat_only: bool,
    },
    /// "The next time a source of your choice would deal damage to target
    /// creature this turn, that source deals that damage to you instead"
    /// (Jade Monolith): not a prevention shield but a redirection one
    /// (CR 614.9), made by a resolved ability like the rest and used up the
    /// same way. [`apply`] passes it by; [`redirect`] reads it.
    RedirectNextFrom {
        /// The source it waits for.
        source: ChosenSource,
        /// The player the damage is dealt to instead.
        to: PlayerId,
    },
}

/// The source a "source of your choice" shield waits for (CR 609.7a), and
/// what it must still be when it would deal the damage (CR 609.7b, 615.9).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct ChosenSource {
    /// The object chosen.
    pub id: ObjectId,
    /// Its version when it was chosen (CR 400.7).
    pub version: u32,
    /// It was a spell on the stack: the permanent it becomes as it resolves
    /// is the same source (CR 609.7a).
    pub was_spell: bool,
    /// What it had to be to be chosen, and must still be.
    pub filter: &'static Filter,
    /// The shield's controller, "you" to `filter`.
    pub you: PlayerId,
    /// The object whose ability made the shield, "this" to `filter`.
    pub this: ObjectId,
}

impl ChosenSource {
    /// `chosen` as it is now, picked by `you` for the ability of `this`.
    #[must_use]
    pub fn new(
        state: &GameState,
        chosen: ObjectId,
        filter: &'static Filter,
        you: PlayerId,
        this: ObjectId,
    ) -> Option<Self> {
        let obj = state.object(chosen)?;
        Some(Self {
            id: chosen,
            version: obj.version,
            was_spell: obj.zone == Zone::Stack && obj.kind == ObjectKind::Spell,
            filter,
            you,
            this,
        })
    }

    /// Whether damage `source` deals is damage from this chosen source that
    /// still has the properties it was chosen for.
    ///
    /// A damage source is an id, not an object: which incarnation of it
    /// dealt the damage is read from where that id is now. On the
    /// battlefield or the stack it is the chosen object only as the same
    /// version, or as the permanent a chosen spell became (one move later).
    /// Anywhere else the damage comes from something that still refers to
    /// the source as it last was — an ability of a pinger killed in response
    /// — and it is checked as it last existed on the battlefield (CR 609.7a
    /// "even if that object is no longer in the zone it used to be in").
    /// Two corners that reading gets wrong, and that no card in the pool
    /// reaches today: an ability the chosen card's *new* object activates
    /// from its graveyard counts as the chosen source's, and an ability of
    /// the chosen permanent still on the stack after the card came back to
    /// the battlefield does not.
    fn deals(&self, state: &GameState, source: ObjectId) -> bool {
        if source != self.id {
            return false;
        }
        let Some(obj) = state.object_or_departed(source) else {
            return false;
        };
        if matches!(obj.zone, Zone::Battlefield | Zone::Stack) {
            let same = obj.version == self.version
                || (self.was_spell
                    && obj.zone == Zone::Battlefield
                    && obj.version == self.version + 1);
            same && crate::eval::matches(self.filter, state, obj, self.you, self.this)
        } else {
            obj.version >= self.version
                && state.last_known_characteristics(source).is_some_and(|was| {
                    crate::eval::matches_projected(
                        self.filter,
                        state,
                        obj,
                        was,
                        self.you,
                        self.this,
                    )
                })
        }
    }
}

/// What a player may choose as "a source of your choice" matching
/// `filter` (CR 609.7a): a permanent, a spell on the stack, and the source
/// of an ability on the stack even where that source has since gone (its
/// last known characteristics answer `filter` then).
///
/// Not offered: an object only a waiting replacement or prevention effect
/// or a delayed trigger refers to, a face-up object in the command zone,
/// and a token that has ceased to exist — none of which a card in the pool
/// deals damage from today.
#[must_use]
pub fn source_options(
    state: &GameState,
    filter: &'static Filter,
    you: PlayerId,
    this: ObjectId,
) -> Vec<ObjectId> {
    let mut out: Vec<ObjectId> = state
        .battlefield_seen()
        .filter(|id| {
            state
                .object(*id)
                .is_some_and(|o| crate::eval::matches(filter, state, o, you, this))
        })
        .collect();
    for &id in state.zones.list(ZoneLocation::Stack) {
        let Some(obj) = state.object(id) else {
            continue;
        };
        match obj.kind {
            ObjectKind::Spell => {
                if crate::eval::matches(filter, state, obj, you, this) {
                    out.push(id);
                }
            }
            ObjectKind::AbilityOnStack => {
                let Some(from) = obj.ability.as_ref().map(|loc| loc.source) else {
                    continue;
                };
                let Some(source) = state.object(from) else {
                    continue;
                };
                let fits = if matches!(source.zone, Zone::Battlefield | Zone::Stack) {
                    crate::eval::matches(filter, state, source, you, this)
                } else {
                    state.last_known_characteristics(from).is_some_and(|was| {
                        crate::eval::matches_projected(filter, state, source, was, you, this)
                    })
                };
                if fits {
                    out.push(from);
                }
            }
            _ => {}
        }
    }
    out.sort();
    out.dedup();
    out
}

/// One prevention shield.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Shield {
    /// Whom it protects.
    pub protects: Shielded,
    /// How it prevents.
    pub kind: ShieldKind,
    /// The player who controlled the spell or ability that made it.
    pub controller: PlayerId,
}

impl Shielded {
    /// Whether damage to `recipient` is damage to what this shields.
    fn covers(self, state: &GameState, recipient: DamageTarget) -> bool {
        match (self, recipient) {
            (Self::Everything, _) => true,
            (Self::Player(p), DamageTarget::Player(q)) => p == q,
            (Self::Object(id, version), DamageTarget::Object(target)) => {
                id == target && state.object(target).is_some_and(|o| o.version == version)
            }
            _ => false,
        }
    }
}

/// The order shields are offered damage in.
///
/// CR 616.1 (and 615.7's last sentence, for one shield and several sources
/// at once) gives that choice to the affected player or the controller of
/// the affected permanent. The engine does not ask; it applies the shields
/// in this fixed order, oldest first within a rank:
///
/// 0. A chosen-source shield that pays life back (Reverse Damage).
/// 1. A shield that prevents all combat damage and is never used up (Fog).
/// 2. A chosen-source shield that prevents all of an instance (Circle of
///    Protection).
/// 3. A chosen-source shield that prevents all but some (Forcefield).
/// 4. "The next N damage" shields.
///
/// Most pairs are dominant in this order — the one the player would always
/// pick: a shield that prevents nothing is not used up (CR 609.7b), so the
/// fuller shield first leaves the other standing (Fog before any
/// chosen-source shield, Circle of Protection before Forcefield, Reverse
/// Damage before Circle of Protection on one source), and two "next N"
/// shields spend the same total either way. Two pairs are trades, and this
/// is where the engine decides what the player would be asked:
///
/// - Reverse Damage before Fog gains the life now and spends Reverse
///   Damage; Fog first keeps it for that source's next damage this turn.
/// - A chosen-source shield before "the next N" keeps the N for any source;
///   the other order keeps the chosen-source shield for its one source.
///
/// Redirection comes after all of them (rank 5, which [`apply`] never uses):
/// every writer of damage asks [`redirect`] only once the shields in front of
/// the first recipient have prevented what they prevent, and the damage it
/// moves then meets the new recipient's own shields. CR 616.1 lets the
/// affected player choose that order too, and it is a trade: preventing
/// first spares the creature a Veteran Bodyguard would put in the way and
/// spends the shield; redirecting first keeps the shield for later and
/// costs the creature.
fn rank(shield: &Shield) -> u8 {
    match shield.kind {
        ShieldKind::NextFrom {
            gain_life: true, ..
        } => 0,
        ShieldKind::AllCombat => 1,
        ShieldKind::NextFrom { all_but: 0, .. } => 2,
        ShieldKind::NextFrom { .. } => 3,
        ShieldKind::Next(_) => 4,
        ShieldKind::RedirectNextFrom { .. } => 5,
    }
}

/// How much of `amount` damage from `source` to `recipient` is still dealt
/// once the shields in front of it have prevented what they prevent.
///
/// Uses the shields up as it goes: an emptied "next N" is removed, and so is
/// a chosen-source shield that prevented anything; one that prevented
/// nothing stays (CR 609.7b). Damage that can't be prevented (CR 615.12,
/// `unpreventable`) passes every shield untouched and reduces none of them.
/// Life a shield pays back is gained as the damage is prevented (CR 615.5).
pub fn apply(
    state: &mut GameState,
    source: ObjectId,
    recipient: DamageTarget,
    amount: u32,
    is_combat: bool,
) -> u32 {
    if amount == 0 || state.shields.is_empty() {
        return amount;
    }
    if crate::combat::unpreventable(state, source, is_combat) {
        return amount;
    }
    let mut order: Vec<usize> = (0..state.shields.len())
        .filter(|&i| state.shields[i].protects.covers(state, recipient))
        .collect();
    order.sort_by_key(|&i| (rank(&state.shields[i]), i));
    let mut left = amount;
    let mut spent: Vec<usize> = Vec::new();
    let mut paid_back: Vec<(PlayerId, u32)> = Vec::new();
    for i in order {
        if left == 0 {
            break;
        }
        let shield = state.shields[i];
        match shield.kind {
            ShieldKind::AllCombat => {
                if is_combat {
                    left = 0;
                }
            }
            ShieldKind::Next(n) => {
                let prevented = left.min(n);
                left -= prevented;
                state.shields[i].kind = ShieldKind::Next(n - prevented);
                if n == prevented {
                    spent.push(i);
                }
            }
            ShieldKind::NextFrom {
                source: chosen,
                all_but,
                gain_life,
                combat_only,
            } => {
                if (combat_only && !is_combat) || !chosen.deals(state, source) {
                    continue;
                }
                let prevented = left.saturating_sub(all_but);
                if prevented == 0 {
                    continue;
                }
                left -= prevented;
                spent.push(i);
                if gain_life {
                    paid_back.push((shield.controller, prevented));
                }
            }
            // Not prevention: `redirect` reads it once this is done.
            ShieldKind::RedirectNextFrom { .. } => {}
        }
    }
    spent.sort_unstable();
    for i in spent.into_iter().rev() {
        state.shields.remove(i);
    }
    for (player, life) in paid_back {
        let life = i32::try_from(life).unwrap_or(i32::MAX);
        state.change_life(player, life, crate::event::Cause::Effect);
    }
    left
}

/// The redirection effects that have already moved one event of damage,
/// which may not move it again (CR 614.5).
///
/// A writer of damage starts one of these for each event and hands it on
/// with the damage when it is redirected, so a Veteran Bodyguard that took
/// the damage off its controller is not asked again when a Jade Monolith
/// sends it back. A shield needs no entry: one that redirects is used up.
#[derive(Default, Debug)]
pub(crate) struct Redirected {
    statics: Vec<baylee_core::ids::EffectId>,
}

/// Where damage `source` would deal to `recipient` is dealt instead, if a
/// redirection effect moves it (CR 614.9): all of it, as the same damage
/// from the same source.
///
/// Asked by every writer of damage once [`apply`] has had the damage, and
/// again, with the same `done`, by the writer the damage is moved to. First
/// the resolved shields on `recipient`, oldest first: one waiting for this
/// source (rechecked, CR 609.7b) whose player is still in the game moves it
/// and is used up; one that moves nothing stays. Then, for damage to a
/// player, the static abilities that player controls saying "damage that
/// would be dealt to you by … is dealt to this instead", oldest first, each
/// once (CR 614.5). Such a static reads its source filter on the source as
/// it is now, wherever it is (CR 609.7c), and does nothing while what it
/// applies to is not a creature on the battlefield (CR 614.9).
///
/// CR 616.1 gives the choice among several to the affected player; the
/// engine takes them in this fixed order and does not ask.
pub(crate) fn redirect(
    state: &mut GameState,
    source: ObjectId,
    recipient: DamageTarget,
    done: &mut Redirected,
) -> Option<DamageTarget> {
    let shield = state.shields.iter().position(|shield| {
        let ShieldKind::RedirectNextFrom { source: chosen, to } = shield.kind else {
            return false;
        };
        shield.protects.covers(state, recipient)
            && still_a_creature(state, recipient)
            && !state.has_left(to)
            && chosen.deals(state, source)
    });
    if let Some(i) = shield {
        let ShieldKind::RedirectNextFrom { to, .. } = state.shields.remove(i).kind else {
            unreachable!("found as a redirection shield");
        };
        return Some(DamageTarget::Player(to));
    }
    let DamageTarget::Player(player) = recipient else {
        return None;
    };
    if state.has_left(player) {
        return None;
    }
    let from = state.object(source)?;
    let moved = state.effects.iter().find_map(|fx| {
        let baylee_cards_dsl::Modifier::RedirectDamageToYou(filter) = fx.modifier else {
            return None;
        };
        if fx.controller != player
            || done.statics.contains(&fx.id)
            || !crate::eval::matches(filter, state, from, player, fx.source.unwrap_or(source))
        {
            return None;
        }
        let to = state.battlefield_view().into_iter().find(|&id| {
            still_a_creature(state, DamageTarget::Object(id))
                && state
                    .object(id)
                    .is_some_and(|obj| crate::effects::applies_to(state, fx, obj))
        })?;
        Some((fx.id, to))
    });
    let (id, to) = moved?;
    done.statics.push(id);
    Some(DamageTarget::Object(to))
}

/// Whether `recipient` is something a redirection may move damage from or
/// to: a player, or a creature on the battlefield (CR 614.9).
fn still_a_creature(state: &GameState, recipient: DamageTarget) -> bool {
    match recipient {
        DamageTarget::Player(_) => true,
        DamageTarget::Object(id) => state.object(id).is_some_and(|obj| {
            obj.zone == Zone::Battlefield
                && obj
                    .characteristics()
                    .types
                    .contains(baylee_core::types::TypeSet::CREATURE)
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::object::ObjectKind;
    use crate::state::CardLookup;
    use crate::zone::ZoneLocation;
    use baylee_core::color::{Color, ColorSet};
    use baylee_core::ids::CardIndex;
    use baylee_core::preset::{FormatId, GamePreset, HouseRules, SeatController, SeatSpec};
    use baylee_core::types::TypeSet;

    struct NoCards;
    impl CardLookup for NoCards {
        fn card(&self, _: CardIndex) -> Option<&'static baylee_cards_dsl::CardDef> {
            None
        }
    }

    const P0: PlayerId = PlayerId::new(0);
    const P1: PlayerId = PlayerId::new(1);

    fn board() -> (GameState, ObjectId) {
        let seat = || SeatSpec {
            controller: SeatController::Open,
            capabilities: baylee_core::preset::SeatCapabilities::default(),
            deck: vec![],
            sideboard: vec![],
            commanders: vec![],
            starting_life: Some(20),
            starting_hand: None,
            starting_battlefield: vec![],
            emblems: vec![],
            team: None,
        };
        let mut state = GameState::from_preset(
            &GamePreset {
                format: FormatId::Freeform,
                seed: 1,
                house_rules: HouseRules::default(),
                modifiers: vec![],
                prints: vec![],
                seats: vec![seat(), seat()],
            },
            &NoCards,
        )
        .expect("an empty board");
        let name = state.names.intern("Test Creature");
        let id = state.create_bare(P0, ObjectKind::Permanent, name, ZoneLocation::Battlefield);
        let b = state.object_mut(id).expect("just created").base_mut();
        b.types = TypeSet::CREATURE;
        b.power = Some(2);
        b.toughness = Some(2);
        (state, id)
    }

    fn shield(protects: Shielded, kind: ShieldKind) -> Shield {
        Shield {
            protects,
            kind,
            controller: P0,
        }
    }

    /// CR 615.7: "the next 3" prevents 3 in all, across events, and only
    /// to what it shields.
    #[test]
    fn a_shield_prevents_its_amount_and_is_used_up() {
        let (mut state, creature) = board();
        state
            .shields
            .push(shield(Shielded::Player(P1), ShieldKind::Next(3)));
        assert_eq!(
            apply(&mut state, creature, DamageTarget::Player(P0), 2, false),
            2
        );
        assert_eq!(
            apply(&mut state, creature, DamageTarget::Player(P1), 2, false),
            0
        );
        assert_eq!(state.shields[0].kind, ShieldKind::Next(1));
        assert_eq!(
            apply(&mut state, creature, DamageTarget::Player(P1), 2, true),
            1
        );
        assert!(state.shields.is_empty(), "used up");
    }

    /// A shield on a permanent is on that object: once it has left and
    /// come back, the new object is not shielded (CR 400.7).
    #[test]
    fn a_shield_on_a_permanent_is_on_that_object() {
        let (mut state, creature) = board();
        let version = state.object(creature).unwrap().version;
        state.shields.push(shield(
            Shielded::Object(creature, version),
            ShieldKind::Next(1),
        ));
        state.object_mut(creature).unwrap().version += 1;
        assert_eq!(
            apply(
                &mut state,
                creature,
                DamageTarget::Object(creature),
                1,
                false
            ),
            1
        );
        assert_eq!(state.shields.len(), 1, "not used on a different object");
    }

    /// Fog prevents combat damage only, to anything, and first: a "next 2"
    /// beside it is still whole afterwards.
    #[test]
    fn fog_prevents_combat_damage_and_spares_the_other_shields() {
        let (mut state, creature) = board();
        state
            .shields
            .push(shield(Shielded::Player(P1), ShieldKind::Next(2)));
        state
            .shields
            .push(shield(Shielded::Everything, ShieldKind::AllCombat));
        assert_eq!(
            apply(&mut state, creature, DamageTarget::Player(P1), 5, true),
            0
        );
        assert_eq!(state.shields[0].kind, ShieldKind::Next(2));
        assert_eq!(
            apply(&mut state, creature, DamageTarget::Player(P1), 5, false),
            3
        );
        assert_eq!(state.shields.len(), 1, "the amount is spent, Fog stays");
    }

    static RED: Filter = Filter::HasColor(ColorSet::from_slice(&[Color::Red]));

    fn painted(state: &mut GameState, id: ObjectId, colors: ColorSet) {
        state
            .object_mut(id)
            .expect("on the board")
            .base_mut()
            .colors = colors;
        state.refresh_characteristics();
    }

    /// A red creature beside `board()`'s, for a source that was not chosen.
    fn another(state: &mut GameState) -> ObjectId {
        let name = state.names.intern("Another Creature");
        let id = state.create_bare(P0, ObjectKind::Permanent, name, ZoneLocation::Battlefield);
        state.object_mut(id).expect("just created").base_mut().types = TypeSet::CREATURE;
        painted(state, id, ColorSet::from_slice(&[Color::Red]));
        id
    }

    /// P1's shield against `source`, as resolving the ability would leave it.
    fn shield_against(
        state: &mut GameState,
        source: ObjectId,
        filter: &'static Filter,
        all_but: u32,
        gain_life: bool,
        combat_only: bool,
    ) {
        let chosen = ChosenSource::new(state, source, filter, P1, source).expect("on the board");
        state.shields.push(Shield {
            protects: Shielded::Player(P1),
            kind: ShieldKind::NextFrom {
                source: chosen,
                all_but,
                gain_life,
                combat_only,
            },
            controller: P1,
        });
    }

    fn moved(state: &mut GameState, id: ObjectId, to: ZoneLocation) {
        state
            .move_object(
                id,
                to,
                crate::zone::ZonePosition::Top,
                crate::event::Cause::StateBased,
            )
            .expect("it moves");
    }

    /// CR 615.8 and 609.7b: the next instance from the chosen source, all
    /// of it — and only while the source is still what it was chosen as.
    /// Damage it does not prevent does not use it up.
    #[test]
    fn a_chosen_source_shield_waits_for_its_source_and_rechecks_it() {
        let (mut state, chosen) = board();
        painted(&mut state, chosen, ColorSet::from_slice(&[Color::Red]));
        let other = another(&mut state);
        shield_against(&mut state, chosen, &RED, 0, false, false);

        let to_p1 = DamageTarget::Player(P1);
        assert_eq!(
            apply(&mut state, other, to_p1, 3, false),
            3,
            "another source"
        );
        assert_eq!(
            apply(&mut state, chosen, DamageTarget::Player(P0), 3, false),
            3,
            "another player"
        );
        painted(&mut state, chosen, ColorSet::from_slice(&[Color::White]));
        assert_eq!(
            apply(&mut state, chosen, to_p1, 3, false),
            3,
            "no longer red, so the shield does not apply (CR 609.7b)"
        );
        assert_eq!(state.shields.len(), 1, "and is not used up");

        painted(&mut state, chosen, ColorSet::from_slice(&[Color::Red]));
        assert_eq!(apply(&mut state, chosen, to_p1, 7, true), 0, "all of it");
        assert!(state.shields.is_empty(), "one instance, then gone");
        assert_eq!(apply(&mut state, chosen, to_p1, 2, false), 2);
    }

    /// The pinger killed in response: its ability still deals the damage,
    /// from the source it was, and the shield on that source still applies
    /// (CR 609.7a) — rechecked against what it last was. The card that came
    /// back is a new object (CR 400.7), and not the source chosen.
    #[test]
    fn a_chosen_source_that_has_left_is_still_the_source_it_was() {
        let to_p1 = DamageTarget::Player(P1);

        let (mut state, pinger) = board();
        painted(&mut state, pinger, ColorSet::from_slice(&[Color::Red]));
        shield_against(&mut state, pinger, &RED, 0, false, false);
        moved(&mut state, pinger, ZoneLocation::Graveyard(P0));
        assert_eq!(apply(&mut state, pinger, to_p1, 2, false), 0);
        assert!(state.shields.is_empty());

        let (mut state, pinger) = board();
        painted(&mut state, pinger, ColorSet::from_slice(&[Color::Red]));
        shield_against(&mut state, pinger, &RED, 0, false, false);
        painted(&mut state, pinger, ColorSet::from_slice(&[Color::White]));
        moved(&mut state, pinger, ZoneLocation::Graveyard(P0));
        assert_eq!(
            apply(&mut state, pinger, to_p1, 2, false),
            2,
            "it left white, and that is what it last was"
        );

        let (mut state, pinger) = board();
        painted(&mut state, pinger, ColorSet::from_slice(&[Color::Red]));
        shield_against(&mut state, pinger, &RED, 0, false, false);
        moved(&mut state, pinger, ZoneLocation::Graveyard(P0));
        moved(&mut state, pinger, ZoneLocation::Battlefield);
        painted(&mut state, pinger, ColorSet::from_slice(&[Color::Red]));
        assert_eq!(
            apply(&mut state, pinger, to_p1, 2, false),
            2,
            "the creature that came back is another object"
        );
        assert_eq!(state.shields.len(), 1);
    }

    /// Forcefield: combat damage only, all but 1 of it, and an instance of
    /// 1 is one it prevents nothing of — so it is still there after it.
    #[test]
    fn all_but_one_of_combat_damage_and_nothing_else() {
        let (mut state, attacker) = board();
        shield_against(&mut state, attacker, &Filter::Any, 1, false, true);
        let to_p1 = DamageTarget::Player(P1);
        assert_eq!(
            apply(&mut state, attacker, to_p1, 3, false),
            3,
            "not combat"
        );
        assert_eq!(apply(&mut state, attacker, to_p1, 1, true), 1);
        assert_eq!(state.shields.len(), 1, "it prevented nothing (CR 609.7b)");
        assert_eq!(apply(&mut state, attacker, to_p1, 4, true), 1);
        assert!(state.shields.is_empty());
    }

    /// Reverse Damage gains what it prevents (CR 615.5), and goes before a
    /// Fog: the one ordering here that is a trade, decided for the life now.
    #[test]
    fn a_shield_that_pays_life_back_is_offered_the_damage_first() {
        let (mut state, source) = board();
        let life = state.players[1].life;
        state
            .shields
            .push(shield(Shielded::Everything, ShieldKind::AllCombat));
        shield_against(&mut state, source, &Filter::Any, 0, true, false);
        assert_eq!(
            apply(&mut state, source, DamageTarget::Player(P1), 3, true),
            0
        );
        assert_eq!(state.players[1].life, life + 3, "paid back");
        assert_eq!(state.shields.len(), 1);
        assert_eq!(state.shields[0].kind, ShieldKind::AllCombat, "Fog stays");
    }

    /// The dominant orders: Fog before a Circle, which prevents nothing and
    /// stays; a Circle before Forcefield, which likewise stays.
    #[test]
    fn the_fuller_shield_goes_first_and_the_other_stays() {
        let (mut state, source) = board();
        let to_p1 = DamageTarget::Player(P1);
        shield_against(&mut state, source, &Filter::Any, 0, false, false);
        state
            .shields
            .push(shield(Shielded::Everything, ShieldKind::AllCombat));
        assert_eq!(apply(&mut state, source, to_p1, 3, true), 0);
        assert_eq!(
            state.shields.len(),
            2,
            "Fog prevented it, the Circle stands"
        );

        let (mut state, source) = board();
        shield_against(&mut state, source, &Filter::Any, 1, false, true);
        shield_against(&mut state, source, &Filter::Any, 0, false, false);
        assert_eq!(apply(&mut state, source, to_p1, 3, true), 0);
        assert_eq!(state.shields.len(), 1);
        assert!(
            matches!(
                state.shields[0].kind,
                ShieldKind::NextFrom { all_but: 1, .. }
            ),
            "Forcefield stands"
        );
    }

    /// Jade Monolith's shield on P0's creature, sending `source`'s next
    /// damage to P1.
    fn redirect_shield(state: &mut GameState, creature: ObjectId, source: ObjectId) {
        let version = state.object(creature).expect("on the board").version;
        let chosen = ChosenSource::new(state, source, &Filter::Any, P1, source).expect("there");
        state.shields.push(Shield {
            protects: Shielded::Object(creature, version),
            kind: ShieldKind::RedirectNextFrom {
                source: chosen,
                to: P1,
            },
            controller: P1,
        });
    }

    /// CR 614.9 with 609.7b: the chosen source's next damage to the
    /// creature goes to the player instead, all of it, and the shield is
    /// used up; damage from another source, or to anything else, leaves it
    /// waiting. It prevents nothing, so `apply` passes it by.
    #[test]
    fn a_redirection_shield_moves_its_sources_next_damage_once() {
        let (mut state, creature) = board();
        let source = another(&mut state);
        redirect_shield(&mut state, creature, source);
        let to_it = DamageTarget::Object(creature);

        assert_eq!(
            apply(&mut state, source, to_it, 3, false),
            3,
            "not prevention"
        );
        let mut done = Redirected::default();
        assert_eq!(
            redirect(&mut state, creature, to_it, &mut done),
            None,
            "another source"
        );
        assert_eq!(
            redirect(&mut state, source, DamageTarget::Object(source), &mut done),
            None,
            "another creature"
        );
        assert_eq!(
            redirect(&mut state, source, DamageTarget::Player(P0), &mut done),
            None,
            "a player"
        );
        assert_eq!(state.shields.len(), 1, "still waiting (CR 609.7b)");

        assert_eq!(
            redirect(&mut state, source, to_it, &mut done),
            Some(DamageTarget::Player(P1))
        );
        assert!(state.shields.is_empty(), "used up");
        assert_eq!(redirect(&mut state, source, to_it, &mut done), None);
    }

    /// CR 614.9: to a player who has left the game, a redirection does
    /// nothing, and so replaces nothing and is not used up — nor from a
    /// permanent that is no longer a creature.
    #[test]
    fn a_redirection_to_a_player_who_has_left_does_nothing() {
        let (mut state, creature) = board();
        let source = another(&mut state);
        redirect_shield(&mut state, creature, source);
        state.players[1].loss = Some(crate::event::LossReason::Conceded);
        assert_eq!(
            redirect(
                &mut state,
                source,
                DamageTarget::Object(creature),
                &mut Redirected::default()
            ),
            None
        );
        assert_eq!(state.shields.len(), 1);

        let (mut state, creature) = board();
        let source = another(&mut state);
        redirect_shield(&mut state, creature, source);
        state
            .object_mut(creature)
            .expect("on the board")
            .base_mut()
            .types = TypeSet::ARTIFACT;
        state.refresh_characteristics();
        assert_eq!(
            redirect(
                &mut state,
                source,
                DamageTarget::Object(creature),
                &mut Redirected::default()
            ),
            None,
            "no longer a creature"
        );
        assert_eq!(state.shields.len(), 1);
    }

    /// Veteran Bodyguard's static, on `bodyguard`, for P0: red sources'
    /// damage to P0 is dealt to it instead.
    fn bodyguard(state: &mut GameState, bodyguard: ObjectId) {
        let modifier = baylee_cards_dsl::Modifier::RedirectDamageToYou(&RED);
        state.effects.register(crate::effects::ContinuousEffect {
            // `register` assigns the real one.
            id: baylee_core::ids::EffectId::new(0),
            source: Some(bodyguard),
            controller: P0,
            origin: crate::effects::EffectOrigin::Static,
            layer: modifier.layer(),
            timestamp: 1,
            duration: baylee_cards_dsl::Duration::WhileSourceOnBattlefield,
            filter: crate::effects::EffectFilter::Dsl(&Filter::This),
            modifier,
        });
    }

    /// A redirecting static: damage to its controller from a matching
    /// source goes to the permanent, and never twice in one event
    /// (CR 614.5) — a new event is another matter. Not another player's
    /// damage, not a source that does not match, and nothing once the
    /// permanent is no longer a creature (CR 614.9).
    #[test]
    fn a_redirecting_static_moves_its_controllers_damage_once_an_event() {
        let (mut state, guard) = board();
        let red = another(&mut state);
        bodyguard(&mut state, guard);
        let to_p0 = DamageTarget::Player(P0);
        let mut done = Redirected::default();

        assert_eq!(
            redirect(&mut state, red, DamageTarget::Player(P1), &mut done),
            None,
            "its controller's damage only"
        );
        assert_eq!(
            redirect(&mut state, guard, to_p0, &mut done),
            None,
            "not red"
        );
        assert_eq!(
            redirect(&mut state, red, to_p0, &mut done),
            Some(DamageTarget::Object(guard))
        );
        assert_eq!(
            redirect(&mut state, red, to_p0, &mut done),
            None,
            "once to an event (CR 614.5)"
        );
        assert_eq!(
            redirect(&mut state, red, to_p0, &mut Redirected::default()),
            Some(DamageTarget::Object(guard)),
            "and again to the next one"
        );

        state
            .object_mut(guard)
            .expect("on the board")
            .base_mut()
            .types = TypeSet::ARTIFACT;
        state.refresh_characteristics();
        assert_eq!(
            redirect(&mut state, red, to_p0, &mut Redirected::default()),
            None,
            "no longer a creature (CR 614.9)"
        );

        let (mut state, guard) = board();
        let red = another(&mut state);
        bodyguard(&mut state, guard);
        state.players[0].loss = Some(crate::event::LossReason::Conceded);
        assert_eq!(
            redirect(&mut state, red, to_p0, &mut Redirected::default()),
            None,
            "from a player who has left the game (CR 614.9)"
        );
    }
}
