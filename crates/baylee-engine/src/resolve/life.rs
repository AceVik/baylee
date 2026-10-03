//! Life totals and damage: gain/lose life, damage to players, objects,
//! and planeswalkers (loyalty removal).

#[allow(clippy::wildcard_imports)] // family modules share the resolve vocabulary
use super::*;
use crate::damage::{Assignment, DamageWork};
use crate::prevention::{Shield, ShieldKind, ShieldOrigin, Shielded};
use baylee_cards_dsl::Filter;
use baylee_core::types::TypeSet;

/// Executes one life/damage effect.
#[allow(clippy::too_many_lines)] // the family is one flat table
pub(super) fn exec(state: &mut GameState, res: &mut Resolution, op: Effect) -> Option<Pending> {
    let you = res.controller;
    match op {
        Effect::DealDamageEvenly { .. }
        | Effect::DealDamageWithCappedLifeGain { .. }
        | Effect::DealDamage { .. }
        | Effect::DealDamageToAttached { .. }
        | Effect::DealDamageDivided { .. }
        | Effect::Fight { .. }
        | Effect::DamageEqualToPower { .. }
        | Effect::EventObjectDealsDamageEqualToPower { .. }
        | Effect::DealDamageToTargetController { .. }
        | Effect::DealDamageEach { .. } => start(state, res, op),
        Effect::GainLife { amount } => {
            let n = amount2(&amount, state, you, res) as i32;
            gain_life(state, you, n);
            None
        }
        Effect::GainLifeFor { amount, who } => {
            let n = amount2(&amount, state, you, res) as i32;
            let players = super::players_of(who, state, you, res);
            for player in players {
                gain_life(state, player, n);
            }
            None
        }
        Effect::GainLifeDoubleX => {
            let n = res.x.unwrap_or(0).saturating_mul(2) as i32;
            gain_life(state, you, n);
            None
        }
        Effect::LoseLife { amount, target } => {
            let n = amount2(&amount, state, you, res) as i32;
            // "Can't lose life" is the door's to answer, for this loss as
            // for damage.
            for player in super::players_of(target, state, you, res) {
                state.change_life(player, -n, Cause::Effect);
            }
            None
        }
        Effect::PreventNextDamage { target, amount } => {
            let n = amount2(&amount, state, you, res);
            if n == 0 {
                return None;
            }
            for recipient in recipients(state, res, you, target) {
                let protects = match recipient {
                    DamageTarget::Player(player) => Shielded::Player(player),
                    DamageTarget::Object(id) => match state.object(id) {
                        Some(obj) => Shielded::Object(id, obj.version),
                        None => continue,
                    },
                };
                let origin = origin(state, res);
                state.shields.push_from(
                    Shield {
                        protects,
                        kind: ShieldKind::Next(n),
                        controller: you,
                    },
                    Some(origin),
                );
            }
            None
        }
        Effect::PreventAllCombatDamageThisTurn => {
            let origin = origin(state, res);
            state.shields.push_from(
                Shield {
                    protects: Shielded::Everything,
                    kind: ShieldKind::AllCombat,
                    controller: you,
                },
                Some(origin),
            );
            None
        }
        // The source is chosen as this resolves (CR 609.7a), and the choice
        // is an instruction, so `min: 1`; with no source to choose there is
        // no shield and no question (CR 609.3).
        Effect::PreventNextFromChosenSource {
            sources,
            combat_only,
            all_but,
            gain_life,
        } => {
            let options = crate::prevention::source_options(state, sources, you, res.source);
            if options.is_empty() {
                return None;
            }
            res.awaiting = Some(AwaitingOp::ShieldFromChosenSource {
                sources,
                combat_only,
                all_but,
                gain_life,
            });
            Some(Pending::ChooseDamageSource {
                player: you,
                choice: state.next_source_choice(),
                options,
            })
        }
        // Jade Monolith: the creature is the target (still legal, or this
        // would not be resolving), and the source is chosen now, as the
        // prevention sibling's is (CR 609.7a) — any source at all.
        Effect::RedirectNextFromChosenSource { target } => {
            let DamageTarget::Object(id) = *recipients(state, res, you, target).first()? else {
                return None;
            };
            let version = state.object(id)?.version;
            let options = crate::prevention::source_options(state, &Filter::Any, you, res.source);
            if options.is_empty() {
                return None;
            }
            res.awaiting = Some(AwaitingOp::RedirectFromChosenSource {
                protects: Shielded::Object(id, version),
            });
            Some(Pending::ChooseDamageSource {
                player: you,
                choice: state.next_source_choice(),
                options,
            })
        }
        _ => unreachable!("not a life/damage effect"),
    }
}

/// State retained while a damage instruction asks replacement questions.
#[derive(Clone, Debug, Hash)]
pub struct DamageResolution {
    work: DamageWork,
    after: AfterDamage,
}

#[derive(Clone, Copy, Debug, Hash)]
enum AfterDamage {
    Done,
    CappedLifeGain {
        recipient: DamageTarget,
        before_cap: i32,
    },
}

fn origin(state: &GameState, res: &Resolution) -> ShieldOrigin {
    ShieldOrigin {
        source: res.source,
        ability: super::resolving_ability(state, res),
    }
}

fn start(state: &mut GameState, res: &mut Resolution, op: Effect) -> Option<Pending> {
    let assignments = assignments(state, res, op);
    let after = if matches!(op, Effect::DealDamageWithCappedLifeGain { .. }) {
        let recipient = assignments.first()?.recipient;
        let before_cap = match recipient {
            DamageTarget::Player(p) => state.players[usize::from(p.get())].life.max(0),
            DamageTarget::Object(id) => state.object(id).map_or(0, |o| {
                if o.characteristics().types.contains(TypeSet::PLANESWALKER) {
                    i32::from(o.counters.get(baylee_cards_dsl::CounterKind::Loyalty))
                } else {
                    i32::MAX
                }
            }),
        };
        AfterDamage::CappedLifeGain {
            recipient,
            before_cap,
        }
    } else {
        AfterDamage::Done
    };
    let work = DamageWork::new(state, assignments);
    finish_or_suspend(state, res, DamageResolution { work, after })
}

fn finish_or_suspend(
    state: &mut GameState,
    res: &mut Resolution,
    mut damage: DamageResolution,
) -> Option<Pending> {
    if let Some(pending) = damage.work.advance(state) {
        res.awaiting = Some(AwaitingOp::Damage(Box::new(damage)));
        return Some(pending);
    }
    after_damage(state, res, &damage);
    None
}

fn after_damage(state: &mut GameState, res: &Resolution, damage: &DamageResolution) {
    if let AfterDamage::CappedLifeGain {
        recipient,
        before_cap,
    } = damage.after
    {
        state.refresh_characteristics();
        let cap = match recipient {
            DamageTarget::Object(_) => target_chars(res, state)
                .filter(|c| c.types.contains(TypeSet::CREATURE))
                .map_or(before_cap, |c| {
                    before_cap.min(i32::from(c.toughness.unwrap_or(0)).max(0))
                }),
            DamageTarget::Player(_) => before_cap,
        };
        gain_life(
            state,
            res.controller,
            i32::try_from(damage.work.dealt())
                .unwrap_or(i32::MAX)
                .min(cap),
        );
    }
}

pub(crate) fn resume_damage(
    state: &mut GameState,
    res: &mut Resolution,
    answer: &crate::choice::PlayerAction,
) -> Flow {
    let Some(AwaitingOp::Damage(mut damage)) = res.awaiting.take() else {
        unreachable!("damage suspended")
    };
    if let Some(pending) = damage.work.answer(state, answer) {
        res.awaiting = Some(AwaitingOp::Damage(damage));
        return Flow::Wait(pending);
    }
    after_damage(state, res, &damage);
    res.pc += 1;
    run(state, res)
}

impl DamageResolution {
    pub(crate) fn fingerprint(&self) -> u64 {
        crate::state::structural_fingerprint(self)
    }
}

pub(crate) fn refresh_damage(state: &mut GameState, res: &mut Resolution) -> Flow {
    let Some(AwaitingOp::Damage(mut damage)) = res.awaiting.take() else {
        unreachable!("damage suspended")
    };
    if let Some(pending) = damage.work.refresh(state) {
        res.awaiting = Some(AwaitingOp::Damage(damage));
        return Flow::Wait(pending);
    }
    after_damage(state, res, &damage);
    res.pc += 1;
    run(state, res)
}

/// Prevention bought during this instruction belongs only to its damage.
pub(super) fn damage_with_payment(
    state: &mut GameState,
    res: &mut Resolution,
    player: PlayerId,
    damage: u32,
    paid: u32,
) -> Option<Pending> {
    let origin = origin(state, res);
    let assignment = assignment(state, res, res.source, DamageTarget::Player(player), damage);
    let work = DamageWork::new(state, vec![assignment]).with_paid_prevention(paid, origin, player);
    finish_or_suspend(
        state,
        res,
        DamageResolution {
            work,
            after: AfterDamage::Done,
        },
    )
}

fn assignment(
    state: &GameState,
    res: &Resolution,
    source: ObjectId,
    recipient: DamageTarget,
    amount: u32,
) -> Assignment {
    let version = if source == res.source {
        source_version(state, res)
    } else {
        None
    };
    Assignment {
        source,
        source_version: version,
        recipient,
        amount,
        is_combat: false,
    }
}

#[allow(clippy::too_many_lines)] // the damage op family is one collection table
fn assignments(state: &GameState, res: &Resolution, op: Effect) -> Vec<Assignment> {
    let you = res.controller;
    let for_spec = |source, n, spec| {
        recipients(state, res, you, spec)
            .into_iter()
            .map(|to| assignment(state, res, source, to, n))
            .collect::<Vec<_>>()
    };
    match op {
        Effect::DealDamage { amount, target } => {
            for_spec(res.source, amount2(&amount, state, you, res), target)
        }
        Effect::DealDamageWithCappedLifeGain { amount } => for_spec(
            res.source,
            amount2(&amount, state, you, res),
            TargetSpec::AnyTarget,
        ),
        Effect::DealDamageEvenly { amount, target } => {
            let targets = recipients(state, res, you, target);
            let count = u32::try_from(targets.len()).unwrap_or(u32::MAX);
            let share = amount2(&amount, state, you, res)
                .checked_div(count)
                .unwrap_or(0);
            targets
                .into_iter()
                .map(|to| assignment(state, res, res.source, to, share))
                .collect()
        }
        Effect::DealDamageToAttached { amount } => {
            let host = crate::eval::attached_for_ability(
                state,
                res.source,
                source_version(state, res),
                source_attachment_lki(state, res.on_stack),
            );
            host.into_iter()
                .map(|host| {
                    assignment(
                        state,
                        res,
                        res.source,
                        DamageTarget::Object(host.id),
                        amount2(&amount, state, you, res),
                    )
                })
                .collect()
        }
        Effect::DealDamageDivided { .. } => {
            let shares = state
                .divided
                .iter()
                .find(|(id, _)| *id == res.on_stack)
                .map_or(&[][..], |(_, shares)| shares.as_slice());
            res.targets
                .iter()
                .filter_map(|id| {
                    shares
                        .iter()
                        .find(|(target, _)| {
                            target.object == *id && state.source_identity(*id) == Some(*target)
                        })
                        .map(|(_, n)| {
                            assignment(state, res, res.source, DamageTarget::Object(*id), *n)
                        })
                })
                .collect()
        }
        Effect::Fight { fighter, foe } => {
            let (Some(a), Some(b)) = (
                fighting(state, slot_object(res, fighter)),
                fighting(state, slot_object(res, foe)),
            ) else {
                return Vec::new();
            };
            vec![
                assignment(
                    state,
                    res,
                    a,
                    DamageTarget::Object(b),
                    u32::try_from(power_of(state, a)).unwrap_or(0),
                ),
                assignment(
                    state,
                    res,
                    b,
                    DamageTarget::Object(a),
                    u32::try_from(power_of(state, b)).unwrap_or(0),
                ),
            ]
        }
        Effect::DamageEqualToPower { dealer, to } => {
            let Some(source) = fighting(state, slot_object(res, dealer)) else {
                return Vec::new();
            };
            slot_object(res, to)
                .filter(|id| {
                    state
                        .object(*id)
                        .is_some_and(|o| o.zone == crate::zone::Zone::Battlefield)
                })
                .into_iter()
                .map(|id| {
                    assignment(
                        state,
                        res,
                        source,
                        DamageTarget::Object(id),
                        u32::try_from(power_of(state, source)).unwrap_or(0),
                    )
                })
                .collect()
        }
        Effect::EventObjectDealsDamageEqualToPower { target } => {
            let Some(source) = res.event_object else {
                return Vec::new();
            };
            let identity = event_object_identity(state, res);
            let power = state
                .object(source)
                .filter(|o| {
                    o.zone == crate::zone::Zone::Battlefield
                        && identity.is_none_or(|(v, _)| o.version == v)
                })
                .map(|o| o.characteristics().power.unwrap_or(0))
                .or_else(|| identity.map(|(_, p)| p))
                .or_else(|| {
                    state
                        .ltb_powers
                        .iter()
                        .find(|(id, _)| *id == source)
                        .map(|(_, p)| *p)
                })
                .unwrap_or(0);
            // The event object may share the source's arena handle but be
            // its new incarnation after death (CR 400.7e). This instruction
            // explicitly names that object; ordinary DealDamage names the
            // ability's source instead (CR 113.7a).
            let mut assignments = for_spec(source, u32::try_from(power).unwrap_or(0), target);
            for assignment in &mut assignments {
                assignment.source_version = identity.map(|(version, _)| version);
            }
            assignments
        }
        Effect::DealDamageToTargetController { amount } => res
            .targets
            .first()
            .and_then(|id| state.object(*id))
            .map(|obj| {
                vec![assignment(
                    state,
                    res,
                    res.source,
                    DamageTarget::Player(obj.controller),
                    amount2(&amount, state, you, res),
                )]
            })
            .unwrap_or_default(),
        Effect::DealDamageEach { amount, filter } => {
            let n = amount2(&amount, state, you, res);
            state
                .battlefield_seen()
                .filter(|id| {
                    state.object(*id).is_some_and(|o| {
                        o.characteristics()
                            .types
                            .intersects(TypeSet::CREATURE.union(TypeSet::PLANESWALKER))
                            && eval::matches(filter, state, o, you, res.source)
                    })
                })
                .map(|id| assignment(state, res, res.source, DamageTarget::Object(id), n))
                .collect()
        }
        _ => unreachable!("damage collection"),
    }
}

/// Whom `target` names as this resolves: the objects and players an
/// effect that deals damage deals it to, and an effect that prevents
/// damage shields, in the order they are dealt to.
fn recipients(
    state: &GameState,
    res: &Resolution,
    you: PlayerId,
    target: TargetSpec,
) -> Vec<DamageTarget> {
    let players = |list: baylee_core::ids::SeatSet| list.iter().map(DamageTarget::Player);
    match target {
        TargetSpec::Player(rel) => super::players_of(rel, state, you, res)
            .into_iter()
            .map(DamageTarget::Player)
            .collect(),
        // "Any target" (CR 115.4) chose from one set spanning both,
        // so both halves are dealt to — a spell with two any-targets
        // can have picked a creature and a face.
        TargetSpec::AnyTarget => res
            .targets
            .iter()
            .copied()
            .map(DamageTarget::Object)
            .chain(players(res.target_players))
            .collect(),
        // "Target opponent or planeswalker": one choice over both lists, so
        // whichever half it landed in is dealt to.
        TargetSpec::OpponentOrObject(_) => res
            .targets
            .first()
            .copied()
            .map(DamageTarget::Object)
            .into_iter()
            .chain(players(res.target_players))
            .collect(),
        // Only ever a second instance of "target": the damage goes to what
        // that instance chose, if it chose anything ("up to one").
        TargetSpec::ObjectOfFirstTargetsPlayer(_) => res
            .second_targets
            .first()
            .copied()
            .map(DamageTarget::Object)
            .into_iter()
            .collect(),
        // A chosen player is a player. The choice landed in
        // `target_players`, so reading `targets` here would deal to
        // whatever object the spell also happened to point at — or,
        // far more often, to nothing at all. No card in the pool
        // says this yet; they all spell "target opponent" as
        // `Player(Chosen)` with the choice on the ability's
        // `TargetReq`, which is why the catch-all that used to be
        // here could hold this and stay green.
        TargetSpec::AnyPlayer | TargetSpec::AnyOpponent => players(res.target_players).collect(),
        // "This creature": the source, which nothing chose (#147, the rule
        // `zones::spec_object` keeps for the moving effects). Rock Hydra's
        // "prevent the next 1 damage that would be dealt to this creature"
        // read `targets`, found nothing, and shielded nobody.
        TargetSpec::ThisObject => vec![DamageTarget::Object(res.source)],
        // Everything else names an object, and the damage goes to
        // the one that was chosen. Spelled out rather than left to
        // a `_` arm: a new player-flavoured `TargetSpec` would land
        // in a catch-all silently and be dealt to as an object.
        TargetSpec::Object(_)
        | TargetSpec::ObjectOfEachOpponent(_)
        | TargetSpec::ObjectControlledBy(..)
        | TargetSpec::ObjectOfEventPlayer(_)
        | TargetSpec::Spell(_)
        | TargetSpec::StackOrBattlefield(_)
        | TargetSpec::CardInGraveyard(..)
        | TargetSpec::CardInGraveyardBelowEvent(..)
        | TargetSpec::CardInGraveyardBelowValue(..)
        | TargetSpec::AbilityOnStack(_)
        | TargetSpec::SpellOrAbility(_)
        | TargetSpec::EventObject => res
            .targets
            .first()
            .copied()
            .map(DamageTarget::Object)
            .into_iter()
            .collect(),
    }
}

/// The object a [`TargetSlot`] names as the effect resolves, if there still
/// is one.
///
/// Empty when the slot's target was dropped by CR 608.2b's re-check, and when
/// a "choose up to one" was answered with none — the two are the same fact to
/// every effect that reads it: there is nothing on that side.
fn slot_object(res: &Resolution, slot: TargetSlot) -> Option<ObjectId> {
    match slot {
        TargetSlot::This => Some(res.source),
        TargetSlot::First => res.targets.first().copied(),
        TargetSlot::Second => res.second_targets.first().copied(),
    }
}

/// `id`, if it is still a creature on the battlefield — CR 701.14b's two
/// conditions for a creature to fight at all.
fn fighting(state: &GameState, id: Option<ObjectId>) -> Option<ObjectId> {
    id.filter(|id| {
        state.object(*id).is_some_and(|o| {
            o.zone == crate::zone::Zone::Battlefield
                && o.characteristics()
                    .types
                    .contains(baylee_core::types::TypeSet::CREATURE)
        })
    })
}

/// "Damage equal to its power": the projected power, and nought for a
/// negative one (CR 107.1b — a negative number of damage is no damage).
fn power_of(state: &GameState, id: ObjectId) -> i16 {
    state
        .object(id)
        .and_then(|o| o.characteristics().power)
        .map_or(0, |p| p.max(0))
}

pub(super) fn gain_life(state: &mut GameState, player: PlayerId, n: i32) {
    if n <= 0 {
        return;
    }
    state.change_life(player, n, Cause::Effect);
}

#[cfg(test)]
pub(super) fn deal_to_object_with_loyalty(
    state: &mut GameState,
    target: ObjectId,
    n: i16,
    source: ObjectId,
) {
    let assignment = Assignment {
        source,
        source_version: None,
        recipient: DamageTarget::Object(target),
        amount: u32::try_from(n).unwrap_or(0),
        is_combat: false,
    };
    assert!(
        DamageWork::new(state, vec![assignment])
            .advance(state)
            .is_none(),
        "fixture needs a damage choice"
    );
}

#[cfg(test)]
pub(super) fn deal_to_player(state: &mut GameState, source: ObjectId, player: PlayerId, n: i16) {
    let assignment = Assignment {
        source,
        source_version: None,
        recipient: DamageTarget::Player(player),
        amount: u32::try_from(n).unwrap_or(0),
        is_combat: false,
    };
    assert!(
        DamageWork::new(state, vec![assignment])
            .advance(state)
            .is_none(),
        "fixture needs a damage choice"
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effects::{ContinuousEffect, EffectFilter};
    use crate::object::ObjectKind;
    use crate::state::CardLookup;
    use crate::zone::ZoneLocation;
    use baylee_cards_dsl::{Duration, Filter, KeywordSet, Modifier};
    use baylee_core::ids::CardIndex;
    use baylee_core::preset::{
        AIProfile, DeckEntry, FormatId, GamePreset, HouseRules, PrintInfo, SeatCapabilities,
        SeatController, SeatSpec,
    };
    use baylee_core::types::TypeSet;

    struct RegistryLookup;
    impl CardLookup for RegistryLookup {
        fn card(&self, index: CardIndex) -> Option<&'static baylee_cards_dsl::CardDef> {
            baylee_cards::by_index(index)
        }
    }

    fn me() -> PlayerId {
        PlayerId::new(0)
    }

    fn state() -> GameState {
        let forest = baylee_cards::by_oracle_id("b34bb2dc-c1af-4d77-b0b3-a0fb342a5fc6")
            .expect("registry contains Forest")
            .index;
        let deck: Vec<DeckEntry> = (0..60)
            .map(|_| DeckEntry {
                card: forest,
                print: baylee_core::ids::PrintRef::new(0),
            })
            .collect();
        let seat = || SeatSpec {
            controller: SeatController::Ai(AIProfile::default()),
            capabilities: SeatCapabilities::default(),
            deck: deck.clone(),
            sideboard: vec![],
            commanders: vec![],
            starting_life: None,
            starting_hand: None,
            starting_battlefield: vec![],
            emblems: vec![],
            team: None,
        };
        let preset = GamePreset {
            format: FormatId::Freeform,
            seed: 4,
            house_rules: HouseRules::default(),
            modifiers: vec![],
            prints: vec![PrintInfo {
                scryfall_id: uuid::Uuid::nil(),
                lang: "EN".into(),
                finish: baylee_core::preset::Finish::Normal,
            }],
            seats: vec![seat(), seat()],
        };
        GameState::from_preset(&preset, &RegistryLookup).expect("game starts")
    }

    fn permanent(state: &mut GameState, name: &str) -> ObjectId {
        let name = state.names.intern(name);
        state.create_bare(me(), ObjectKind::Permanent, name, ZoneLocation::Battlefield)
    }

    fn life(state: &GameState, player: PlayerId) -> i32 {
        state.players[player.get() as usize].life
    }

    /// "Whenever you gain life" reads the journal, so a gain of nothing has
    /// to leave nothing behind: a lifelink source dealing 0 damage, a
    /// `GainLife` with an [`Amount`] that counted an empty board, and every
    /// negative that arithmetic can hand this door would otherwise fire a
    /// trigger the player never earned — and the negative one would *take*
    /// life through the gain door.
    #[test]
    fn gaining_no_life_is_neither_a_change_nor_an_event() {
        let mut state = state();
        let start = life(&state, me());
        let entries = state.journal.len();

        gain_life(&mut state, me(), 0);
        gain_life(&mut state, me(), -3);

        assert_eq!(life(&state, me()), start, "no life moved");
        assert_eq!(state.journal.len(), entries, "and nothing was recorded");

        gain_life(&mut state, me(), 3);
        assert_eq!(life(&state, me()), start + 3);
        assert_eq!(state.journal.len(), entries + 1);
        assert!(
            matches!(
                state.journal.entries().last().expect("an entry").event,
                GameEvent::LifeChanged { player, old, new, .. }
                    if player == me() && old == start && new == start + 3
            ),
            "a life change carries both totals, because the triggers that \
             read it care by how much"
        );
    }

    /// Damage to a player is two events and not one: CR 120.3c reduces the
    /// life total, and the damage itself is what "whenever a source deals
    /// damage to a player" reads. A door recording only the life change
    /// would leave every damage trigger blind, and one recording only the
    /// damage would leave every life trigger blind.
    /// Both writers here ask the prevention shields (CR 615.7): damage an
    /// effect deals to a shielded player or permanent is prevented before
    /// anything is lost, marked or journalled, and what the shield does not
    /// cover is dealt. On the old writers the player lost all 3 and the
    /// creature was marked with all 3.
    #[test]
    fn an_effects_damage_meets_the_prevention_shields() {
        let mut state = state();
        let source = permanent(&mut state, "Shock");
        let creature = permanent(&mut state, "Grizzly Bears");
        state.shields.push(Shield {
            protects: Shielded::Player(me()),
            kind: ShieldKind::Next(2),
            controller: me(),
        });
        let version = state.object(creature).unwrap().version;
        state.shields.push(Shield {
            protects: Shielded::Object(creature, version),
            kind: ShieldKind::Next(1),
            controller: me(),
        });
        let start = life(&state, me());
        let entries = state.journal.len();

        deal_to_player(&mut state, source, me(), 3);
        assert_eq!(life(&state, me()), start - 1, "2 of the 3 prevented");
        assert!(matches!(
            state.journal.entries().last().expect("an entry").event,
            GameEvent::DamageDealt { amount: 1, .. }
        ));

        deal_to_object_with_loyalty(&mut state, creature, 3, source);
        assert_eq!(state.object(creature).unwrap().damage, 2);
        assert!(state.shields.is_empty(), "both used up");

        // Damage a shield prevents in full is never dealt at all.
        state.shields.push(Shield {
            protects: Shielded::Player(me()),
            kind: ShieldKind::Next(5),
            controller: me(),
        });
        let entries_now = state.journal.len();
        deal_to_player(&mut state, source, me(), 3);
        assert_eq!(life(&state, me()), start - 1);
        assert_eq!(state.journal.len(), entries_now, "no event, no life change");
        assert!(entries_now > entries);
    }

    #[test]
    fn damage_to_a_player_is_a_life_change_and_a_damage_event() {
        let mut state = state();
        let source = permanent(&mut state, "Shock");
        let start = life(&state, me());
        let entries = state.journal.len();

        deal_to_player(&mut state, source, me(), 3);

        assert_eq!(life(&state, me()), start - 3);
        assert_eq!(state.journal.len(), entries + 2);
        let recorded = &state.journal.entries()[entries..];
        assert!(matches!(
            recorded[0].event,
            GameEvent::LifeChanged { player, cause: Cause::Effect, .. } if player == me()
        ));
        assert!(
            matches!(
                recorded[1].event,
                GameEvent::DamageDealt {
                    source: Some(src),
                    target: DamageTarget::Player(player),
                    amount: 3,
                    is_combat: false,
                } if src == source && player == me()
            ),
            "the damage names its source: protection, prevention and \
             lifelink are all properties of the source rather than of the \
             number"
        );

        let entries = state.journal.len();
        deal_to_player(&mut state, source, me(), 0);
        deal_to_player(&mut state, source, me(), -2);
        assert_eq!(life(&state, me()), start - 3, "no damage moves no life");
        assert_eq!(state.journal.len(), entries, "and records no event");
    }

    /// Damage to a player who can't lose life (Everybody Lives!) is still
    /// damage dealt, so "whenever a source deals damage" and lifelink both
    /// have something to read. The loss is what doesn't happen: no life
    /// moves, no `LifeChanged` is recorded, and the turn has no loss in it
    /// (#244).
    ///
    /// The effect belongs to the other seat, because "players" is the whole
    /// table and the check used to be keyed on the wrong player.
    #[test]
    fn damage_to_a_player_who_cant_lose_life_is_dealt_and_costs_nothing() {
        let mut state = state();
        let source = permanent(&mut state, "Shock");
        let modifier = Modifier::CantLoseLife {
            who: baylee_cards_dsl::PlayerRel::EachPlayer,
        };
        state.effects.register(ContinuousEffect {
            // `register` assigns the real one.
            id: baylee_core::ids::EffectId::new(0),
            source: None,
            controller: PlayerId::new(1),
            origin: crate::effects::EffectOrigin::Resolution,
            layer: modifier.layer(),
            timestamp: 1,
            duration: Duration::UntilEndOfTurn,
            filter: EffectFilter::Dsl(&Filter::Any),
            modifier,
        });
        let start = life(&state, me());
        let entries = state.journal.len();

        deal_to_player(&mut state, source, me(), 3);

        assert_eq!(life(&state, me()), start, "no life moved");
        let recorded = &state.journal.entries()[entries..];
        assert_eq!(recorded.len(), 1, "one event: {recorded:?}");
        assert!(matches!(
            recorded[0].event,
            GameEvent::DamageDealt { amount: 3, .. }
        ));
        assert!(!state.per_turn.life_lost[me().get() as usize]);
        assert!(
            !state.can_pay_life(me(), 1) && state.can_pay_life(me(), 0),
            "nor can life be paid, except none at all (CR 119.8, CR 119.4b)"
        );
    }

    /// "The damage dealt to you this turn" (Simulacrum) is damage and only
    /// damage: what reached the player past the shields (CR 615.1), whether
    /// or not it cost life, and not a payment of life or a gain after it.
    /// The turn's reset starts it again. On the old doors nothing counted it,
    /// and `Amount::DamageDealtToYouThisTurn` reads what this door counts.
    #[test]
    fn the_damage_dealt_to_a_player_this_turn_is_counted_where_it_is_dealt() {
        let mut state = state();
        let source = permanent(&mut state, "Shock");
        let seat = me().get() as usize;
        let dealt = |state: &GameState| {
            crate::eval::amount(
                &baylee_cards_dsl::Amount::DamageDealtToYouThisTurn,
                state,
                me(),
                source,
                None,
            )
        };

        deal_to_player(&mut state, source, me(), 3);
        assert_eq!(state.per_turn.damage_dealt_to[seat], 3);
        assert_eq!(dealt(&state), 3, "the amount reads the tally");
        gain_life(&mut state, me(), 1);
        state.change_life(me(), -2, Cause::Cost);
        assert_eq!(dealt(&state), 3, "a gain and a payment are no damage");

        state.shields.push(Shield {
            protects: Shielded::Player(me()),
            kind: ShieldKind::Next(1),
            controller: me(),
        });
        deal_to_player(&mut state, source, me(), 2);
        assert_eq!(dealt(&state), 4, "what the shield let through");
        state.shields.push(Shield {
            protects: Shielded::Player(me()),
            kind: ShieldKind::Next(5),
            controller: me(),
        });
        deal_to_player(&mut state, source, me(), 2);
        assert_eq!(dealt(&state), 4, "prevented in full, never dealt");
        state.shields.clear(); // three of the five are left

        let modifier = Modifier::CantLoseLife {
            who: baylee_cards_dsl::PlayerRel::EachPlayer,
        };
        state.effects.register(ContinuousEffect {
            id: baylee_core::ids::EffectId::new(0),
            source: None,
            controller: me(),
            origin: crate::effects::EffectOrigin::Resolution,
            layer: modifier.layer(),
            timestamp: 1,
            duration: Duration::UntilEndOfTurn,
            filter: EffectFilter::Dsl(&Filter::Any),
            modifier,
        });
        let before = life(&state, me());
        deal_to_player(&mut state, source, me(), 2);
        assert_eq!(life(&state, me()), before, "no life moved");
        assert_eq!(dealt(&state), 6, "and the damage was still dealt");

        assert_eq!(
            state.per_turn.damage_dealt_to[1], 0,
            "the other seat's is its own"
        );
        state.per_turn.reset();
        assert_eq!(dealt(&state), 0, "a new turn counts from nothing");
    }

    /// A point of damage and a point of life gained back leave the total
    /// where it was and the turn different: "if you didn't lose life this
    /// turn" (Luminarch Ascension) is now false, because damage is life lost
    /// (CR 119.2) and gaining it back does not unlose it.
    ///
    /// The two states have the same total on purpose. A pair whose totals
    /// differed would hash apart through `life` alone and prove nothing
    /// about the history. With the totals equal, the fact has to be held
    /// somewhere `snapshot_hash` reads (#241). It used to be only in the
    /// journal, which the hash does not read.
    #[test]
    fn a_life_lost_and_gained_back_is_still_a_life_lost_this_turn() {
        let mut untouched = state();
        let mut touched = state();
        permanent(&mut untouched, "Shock");
        let source = permanent(&mut touched, "Shock");

        deal_to_player(&mut touched, source, me(), 1);
        gain_life(&mut touched, me(), 1);

        assert_eq!(life(&touched, me()), life(&untouched, me()));
        let seat = me().get() as usize;
        assert!(
            touched.per_turn.life_lost[seat],
            "the damage is a loss of life"
        );
        assert!(!untouched.per_turn.life_lost[seat]);
        assert_ne!(
            touched.snapshot_hash(),
            untouched.snapshot_hash(),
            "the two states answer the Ascension differently, so they are \
             not the same state"
        );
    }

    /// CR 306.8: damage to a planeswalker removes that many loyalty
    /// counters. It is not marked on the permanent — a planeswalker has no
    /// toughness for it to be measured against, and a walker that took
    /// damage *and* kept its loyalty would die to neither rule.
    #[test]
    fn damage_to_a_planeswalker_removes_loyalty_instead_of_marking_damage() {
        let mut state = state();
        let source = permanent(&mut state, "Bolt");
        let walker = permanent(&mut state, "Walker");
        {
            let obj = state.object_mut(walker).expect("just made it");
            obj.base_mut().types = TypeSet::PLANESWALKER;
            obj.counters.set(baylee_cards_dsl::CounterKind::Loyalty, 4);
        }

        deal_to_object_with_loyalty(&mut state, walker, 3, source);
        let obj = state.object(walker).expect("still there");
        assert_eq!(obj.counters.get(baylee_cards_dsl::CounterKind::Loyalty), 1);
        assert_eq!(obj.damage, 0, "and no damage is marked on it");

        // More than it has takes it to nought rather than wrapping: the
        // subtraction is on a u16, and 1 - 5 there is 65532.
        deal_to_object_with_loyalty(&mut state, walker, 5, source);
        assert_eq!(
            state
                .object(walker)
                .expect("still there")
                .counters
                .get(baylee_cards_dsl::CounterKind::Loyalty),
            0
        );
    }

    /// CR 702.2b: deathtouch is a property of the **source** and applies to
    /// any damage it deals, not only combat damage. So the flag is read off
    /// the source at the moment the damage lands and remembered on the
    /// creature that took it — the state-based action that destroys it
    /// (CR 704.5h) runs later and has no way back to the source.
    #[test]
    fn deathtouch_is_read_off_the_source_and_owes_nothing_to_combat() {
        let mut state = state();
        let plain = permanent(&mut state, "Plain Source");
        let deadly = permanent(&mut state, "Deadly Source");
        state
            .object_mut(deadly)
            .expect("just made it")
            .base_mut()
            .keywords = KeywordSet::DEATHTOUCH;
        let bear = permanent(&mut state, "Bear");

        deal_to_object_with_loyalty(&mut state, bear, 1, plain);
        let obj = state.object(bear).expect("still there");
        assert_eq!(obj.damage, 1);
        assert!(
            !obj.deathtouched,
            "an ordinary source marks ordinary damage"
        );

        deal_to_object_with_loyalty(&mut state, bear, 1, deadly);
        let obj = state.object(bear).expect("still there");
        assert_eq!(obj.damage, 2, "damage accumulates until cleanup");
        assert!(obj.deathtouched);
        assert!(
            matches!(
                state.journal.entries().last().expect("an entry").event,
                GameEvent::DamageDealt {
                    target: DamageTarget::Object(target),
                    is_combat: false,
                    ..
                } if target == bear
            ),
            "this door is never combat damage — combat has its own"
        );
    }

    /// CR 702.16e: a matching source deals no damage at all. Not zero
    /// damage — *no* damage, so the event is not recorded either and
    /// "whenever this creature is dealt damage" never fires.
    #[test]
    fn protection_stops_the_damage_and_the_event_with_it() {
        let mut state = state();
        let source = permanent(&mut state, "Bolt");
        let bear = permanent(&mut state, "Bear");
        let version = state.object(bear).expect("just made it").version;
        let modifier = Modifier::ProtectionFrom(&Filter::ControlledByYou);
        state.effects.register(ContinuousEffect {
            // `register` assigns the real one.
            id: baylee_core::ids::EffectId::new(0),
            source: Some(bear),
            controller: me(),
            origin: crate::effects::EffectOrigin::Resolution,
            layer: modifier.layer(),
            timestamp: 1,
            duration: Duration::WhileSourceOnBattlefield,
            filter: EffectFilter::ObjectIs(bear, version),
            modifier,
        });
        let entries = state.journal.len();

        deal_to_object_with_loyalty(&mut state, bear, 3, source);

        let obj = state.object(bear).expect("still there");
        assert_eq!(obj.damage, 0);
        assert!(!obj.deathtouched);
        assert_eq!(
            state.journal.len(),
            entries,
            "no damage was dealt, so nothing happened that a trigger could see"
        );

        // The other branch of the same call, so a green run cannot mean the
        // damage failed to land for some reason of its own: an identical
        // creature without the effect takes it.
        let unprotected = permanent(&mut state, "Other Bear");
        deal_to_object_with_loyalty(&mut state, unprotected, 3, source);
        assert_eq!(state.object(unprotected).expect("still there").damage, 3);
        assert_eq!(state.journal.len(), entries + 1);
    }

    /// Maze of Ith's two modifiers prevent combat damage only ("Prevent
    /// all combat damage that would be dealt to and dealt by that
    /// creature"), as Kor Haven's, the other card that carries one, does.
    /// Combat's doors ask them; an effect's damage is dealt.
    #[test]
    fn maze_of_ith_s_modifiers_leave_an_effect_s_damage_alone() {
        let mut state = state();
        let source = permanent(&mut state, "Bolt");
        let bear = permanent(&mut state, "Bear");
        for (on, modifier) in [
            (bear, Modifier::PreventDamageToIt),
            (source, Modifier::PreventDamageFromIt),
        ] {
            let version = state.object(on).expect("just made it").version;
            state.effects.register(ContinuousEffect {
                // `register` assigns the real one.
                id: baylee_core::ids::EffectId::new(0),
                source: Some(on),
                controller: me(),
                origin: crate::effects::EffectOrigin::Resolution,
                layer: modifier.layer(),
                timestamp: 1,
                duration: Duration::UntilEndOfTurn,
                filter: EffectFilter::ObjectIs(on, version),
                modifier,
            });
        }

        deal_to_object_with_loyalty(&mut state, bear, 3, source);
        assert_eq!(
            state.object(bear).expect("still there").damage,
            3,
            "neither modifier stops an effect's damage to the creature"
        );
        let before = life(&state, me());
        deal_to_player(&mut state, source, me(), 2);
        assert_eq!(
            life(&state, me()),
            before - 2,
            "nor the creature's damage to a player"
        );
    }

    /// A permanent of `types` on `seat`'s side, and nothing else about it.
    fn typed(state: &mut GameState, seat: PlayerId, name: &str, types: TypeSet) -> ObjectId {
        let name = state.names.intern(name);
        let id = state.create_bare(seat, ObjectKind::Permanent, name, ZoneLocation::Battlefield);
        state.object_mut(id).expect("just made it").base_mut().types = types;
        id
    }

    /// A resolution of `source`, controlled by seat 0, targeting nothing.
    fn untargeted(source: ObjectId) -> Resolution {
        Resolution {
            source,
            on_stack: source,
            controller: me(),
            effects: vec![],
            pc: 0,
            targets: SmallVec::new(),
            second_targets: SmallVec::new(),
            x: None,
            chosen_player: None,
            target_lki: None,
            event_mana: None,
            retarget_left: None,
            target_players: baylee_core::ids::SeatSet::new(),
            event_object: None,
            awaiting: None,
            targeted: false,
            mana_ability: false,
            countered_source: None,
        }
    }

    #[test]
    fn capped_gain_reprojects_toughness_after_damage_prevention_changes_counters() {
        use baylee_cards_dsl::CounterKind;

        let mut state = state();
        let source = permanent(&mut state, "Capped damage source");
        let target = typed(&mut state, me(), "Counter body", TypeSet::CREATURE);
        let obj = state.object_mut(target).unwrap();
        obj.base_mut().power = Some(0);
        obj.base_mut().toughness = Some(0);
        obj.counters.set(CounterKind::P1P1, 3);
        let modifier = Modifier::CountersPreventDamage(CounterKind::P1P1);
        state.effects.register(ContinuousEffect {
            id: baylee_core::ids::EffectId::new(0),
            source: Some(target),
            controller: me(),
            origin: crate::effects::EffectOrigin::Static,
            layer: modifier.layer(),
            timestamp: 1,
            duration: Duration::WhileSourceOnBattlefield,
            filter: EffectFilter::Dsl(&Filter::This),
            modifier,
        });
        state.refresh_characteristics();
        assert_eq!(
            state.object(target).unwrap().characteristics().toughness,
            Some(3)
        );
        let mut res = untargeted(source);
        res.targets.push(target);
        res.targeted = true;
        let before = life(&state, me());
        assert!(
            exec(
                &mut state,
                &mut res,
                Effect::DealDamageWithCappedLifeGain {
                    amount: Amount::Fixed(4),
                },
            )
            .is_none()
        );
        let body = state.object(target).unwrap();
        assert_eq!(body.damage, 1, "the unprevented damage was dealt");
        assert_eq!(body.characteristics().toughness, Some(0));
        assert_eq!(
            body.zone,
            crate::zone::Zone::Battlefield,
            "no mid-effect SBA"
        );
        assert_eq!(
            life(&state, me()),
            before,
            "current zero toughness caps gain"
        );
    }

    /// "~ deals 2 damage to each …" over a filter that matches everything:
    /// both sides' creatures are marked, the planeswalker loses loyalty, and
    /// the land, the artifact and the phased-out creature get nothing.
    ///
    /// The filter is `Any` on purpose. A card's filter names its printed
    /// noun, so no card in the pool would ever put a land in front of this
    /// resolver — which is exactly why the type skip needs a test of its own:
    /// `deal_to_object_with_loyalty` marks damage on anything that is not a
    /// walker (CR 120.1a says it may not), and a phased-out permanent is
    /// treated as though it does not exist (CR 702.26b).
    #[test]
    fn damage_to_each_reaches_what_can_be_dealt_damage_and_nothing_else() {
        let mut state = state();
        let them = PlayerId::new(1);
        let source = permanent(&mut state, "Sweep");
        let mine = typed(&mut state, me(), "Bear", TypeSet::CREATURE);
        let theirs = typed(&mut state, them, "Ogre", TypeSet::CREATURE);
        let walker = typed(&mut state, them, "Walker", TypeSet::PLANESWALKER);
        state
            .object_mut(walker)
            .expect("just made it")
            .counters
            .set(baylee_cards_dsl::CounterKind::Loyalty, 5);
        let land = typed(&mut state, them, "Land", TypeSet::LAND);
        let relic = typed(&mut state, them, "Relic", TypeSet::ARTIFACT);
        let gone = typed(&mut state, them, "Phased Bear", TypeSet::CREATURE);
        state
            .object_mut(gone)
            .expect("just made it")
            .status
            .insert(crate::object::Status::PHASED_OUT);
        let entries = state.journal.len();

        let mut res = untargeted(source);
        exec(
            &mut state,
            &mut res,
            Effect::DealDamageEach {
                amount: Amount::Fixed(2),
                filter: &Filter::Any,
            },
        );

        let damage = |id: ObjectId| state.object(id).expect("on the battlefield").damage;
        assert_eq!(damage(mine), 2, "the controller's own creature is dealt it");
        assert_eq!(damage(theirs), 2, "and so is the opponent's");
        let walked = state.object(walker).expect("on the battlefield");
        assert_eq!(
            walked.counters.get(baylee_cards_dsl::CounterKind::Loyalty),
            3,
            "a planeswalker loses loyalty (CR 120.3c)"
        );
        assert_eq!(walked.damage, 0, "and has no damage marked");
        assert_eq!(damage(land), 0, "a land is not dealt damage");
        assert_eq!(damage(relic), 0, "nor an artifact");
        assert_eq!(damage(gone), 0, "nor a phased-out creature");

        let dealt: Vec<ObjectId> = state.journal.entries()[entries..]
            .iter()
            .filter_map(|e| match e.event {
                GameEvent::DamageDealt {
                    source: Some(from),
                    target: DamageTarget::Object(to),
                    is_combat: false,
                    ..
                } if from == source => Some(to),
                _ => None,
            })
            .collect();
        assert_eq!(
            dealt,
            vec![mine, theirs, walker],
            "one damage event per recipient, and none for what was skipped"
        );
    }

    /// Among what can be dealt damage, the filter decides: "each creature"
    /// leaves a planeswalker alone, which is Surtland Frostpyre's sentence
    /// and not Dragonback Assault's.
    #[test]
    fn damage_to_each_creature_leaves_a_planeswalker_alone() {
        let mut state = state();
        let source = permanent(&mut state, "Sweep");
        let bear = typed(&mut state, me(), "Bear", TypeSet::CREATURE);
        let walker = typed(&mut state, me(), "Walker", TypeSet::PLANESWALKER);
        state
            .object_mut(walker)
            .expect("just made it")
            .counters
            .set(baylee_cards_dsl::CounterKind::Loyalty, 5);

        let mut res = untargeted(source);
        exec(
            &mut state,
            &mut res,
            Effect::damage_each(2, &Filter::CREATURE),
        );

        assert_eq!(state.object(bear).expect("there").damage, 2);
        assert_eq!(
            state
                .object(walker)
                .expect("there")
                .counters
                .get(baylee_cards_dsl::CounterKind::Loyalty),
            5
        );
    }
}
