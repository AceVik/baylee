//! Life totals and damage: gain/lose life, damage to players, objects,
//! and planeswalkers (loyalty removal).

#[allow(clippy::wildcard_imports)] // family modules share the resolve vocabulary
use super::*;
use baylee_cards_dsl::Filter;
use baylee_core::types::TypeSet;

/// Executes one life/damage effect.
#[allow(clippy::too_many_lines)] // the family is one flat table
pub(super) fn exec(state: &mut GameState, res: &mut Resolution, op: Effect) -> Option<Pending> {
    let you = res.controller;
    match op {
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
            for player in super::players_of(target, state, you, res) {
                // Everybody Lives: the controller can't lose life this turn.
                let cant = state.effects.iter().any(|fx| {
                    matches!(fx.modifier, baylee_cards_dsl::Modifier::CantLoseLife)
                        && fx.controller == you
                });
                if cant {
                    continue;
                }
                state.change_life(player, -n, Cause::Effect);
            }
            None
        }
        Effect::DealDamage { amount, target } => {
            let n = amount2(&amount, state, you, res) as i16;
            match target {
                TargetSpec::Player(rel) => {
                    for player in super::players_of(rel, state, you, res) {
                        deal_to_player(state, res.source, player, n);
                    }
                }
                // "Any target" (CR 115.4) chose from one set spanning both,
                // so both halves are dealt to — a spell with two any-targets
                // can have picked a creature and a face.
                TargetSpec::AnyTarget => {
                    for &target_id in &res.targets.clone() {
                        deal_to_object_with_loyalty(state, target_id, n, res.source);
                    }
                    for player in res.target_players.iter() {
                        deal_to_player(state, res.source, player, n);
                    }
                }
                // A chosen player is a player. The choice landed in
                // `target_players`, so reading `targets` here would deal to
                // whatever object the spell also happened to point at — or,
                // far more often, to nothing at all. No card in the pool
                // says this yet; they all spell "target opponent" as
                // `Player(Chosen)` with the choice on the ability's
                // `TargetReq`, which is why the catch-all that used to be
                // here could hold this and stay green.
                TargetSpec::AnyPlayer | TargetSpec::AnyOpponent => {
                    for player in res.target_players.iter() {
                        deal_to_player(state, res.source, player, n);
                    }
                }
                // Everything else names an object, and the damage goes to
                // the one that was chosen. Spelled out rather than left to
                // a `_` arm: a new player-flavoured `TargetSpec` would land
                // in a catch-all silently and be dealt to as an object.
                TargetSpec::Object(_)
                | TargetSpec::Spell(_)
                | TargetSpec::StackOrBattlefield(_)
                | TargetSpec::CardInGraveyard(..)
                | TargetSpec::ThisObject
                | TargetSpec::AbilityOnStack(_)
                | TargetSpec::SpellOrAbility(_)
                | TargetSpec::EventObject => {
                    if let Some(&target_id) = res.targets.first() {
                        deal_to_object_with_loyalty(state, target_id, n, res.source);
                    }
                }
            }
            None
        }
        Effect::Fight { fighter, foe } => {
            fight(state, res, fighter, foe);
            None
        }
        Effect::DamageEqualToPower { dealer, to } => {
            damage_equal_to_power(state, res, dealer, to);
            None
        }
        Effect::DealDamageToTargetController { amount } => {
            if let Some(&target_id) = res.targets.first() {
                let controller = state.object(target_id).map_or(you, |o| o.controller);
                let n = amount2(&amount, state, you, res) as i16;
                deal_to_player(state, res.source, controller, n);
            }
            None
        }
        Effect::DealDamageEach { amount, filter } => {
            damage_each(state, res, &amount, filter);
            None
        }
        _ => unreachable!("not a life/damage effect"),
    }
}

/// `Effect::DealDamageEach` — "deals N damage to each <noun>".
fn damage_each(state: &mut GameState, res: &Resolution, amount: &Amount, filter: &Filter) {
    // The amount and the set are both read once, before anything is dealt
    // (CR 608.2h), and every recipient is dealt its share before the
    // state-based actions look (CR 704.3) — which is what makes the loop
    // simultaneous in effect (CR 608.2f).
    //
    // `battlefield_view` and not the raw zone list: a phased-out permanent is
    // treated as though it does not exist (CR 702.26b). `DestroyAll` walks the
    // raw list, which is #209.
    let you = res.controller;
    let n = amount2(amount, state, you, res) as i16;
    let can_be_dealt = TypeSet::CREATURE.union(TypeSet::PLANESWALKER);
    let hit: Vec<ObjectId> = state
        .battlefield_view()
        .into_iter()
        .filter(|id| {
            state.object(*id).is_some_and(|o| {
                o.characteristics().types.intersects(can_be_dealt)
                    && eval::matches(filter, state, o, you, res.source)
            })
        })
        .collect();
    for id in hit {
        deal_to_object_with_loyalty(state, id, n, res.source);
    }
}

/// `Effect::Fight` (CR 701.14a).
fn fight(state: &mut GameState, res: &Resolution, fighter: TargetSlot, foe: TargetSlot) {
    // CR 701.14b, both halves at once: a side that is gone — left the
    // battlefield, stopped being a creature, or was dropped by CR 608.2b's
    // re-check as an illegal target, which is what an empty slot means here —
    // and *neither* creature deals damage.
    let (Some(a), Some(b)) = (
        fighting(state, slot_object(res, fighter)),
        fighting(state, slot_object(res, foe)),
    ) else {
        return;
    };
    // Both amounts are read before either is dealt: the damage is dealt at
    // once (CR 701.14a, "each of those creatures deals damage"), and a
    // creature's power does not depend on the damage marked on it, so this is
    // the order that cannot matter — which is what makes it the right one to
    // write.
    let (power_a, power_b) = (power_of(state, a), power_of(state, b));
    // Each creature is the source of its own damage, which is what makes
    // deathtouch and protection read the right object (CR 702.2b,
    // CR 702.16e). A creature fighting itself runs both lines at itself —
    // twice its power, as CR 701.14c says.
    deal_to_object_with_loyalty(state, b, power_a, a);
    deal_to_object_with_loyalty(state, a, power_b, b);
}

/// `Effect::DamageEqualToPower` — "deals damage equal to its power to".
fn damage_equal_to_power(
    state: &mut GameState,
    res: &Resolution,
    dealer: TargetSlot,
    to: TargetSlot,
) {
    // Not a fight, so CR 701.14b does not govern it; CR 608.2b does, and it
    // lands in the same place. An illegal dealer is one whose power the
    // effect "fails to determine", so no damage happens, and an illegal
    // recipient is not affected by the part of the effect it is illegal for.
    // The recipient may be a planeswalker (Stump Stomp), which
    // `deal_to_object_with_loyalty` turns into loyalty (CR 306.8), so only the
    // dealer has to be a creature.
    let Some(from) = fighting(state, slot_object(res, dealer)) else {
        return;
    };
    let Some(target) = slot_object(res, to).filter(|id| {
        state
            .object(*id)
            .is_some_and(|o| o.zone == crate::zone::Zone::Battlefield)
    }) else {
        return;
    };
    let n = power_of(state, from);
    deal_to_object_with_loyalty(state, target, n, from);
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

pub(super) fn deal_to_object_with_loyalty(
    state: &mut GameState,
    target: ObjectId,
    n: i16,
    source: ObjectId,
) {
    if n <= 0 {
        return;
    }
    // Protection (CR 702.16e): matching sources deal no damage.
    if eval::protected_from(state, target, source) {
        return;
    }
    let is_walker = state.object(target).is_some_and(|o| {
        o.characteristics()
            .types
            .contains(baylee_core::types::TypeSet::PLANESWALKER)
    });
    if is_walker {
        // Damage to a planeswalker removes loyalty counters (CR 306.8).
        let old = state.object(target).map_or(0, |o| {
            o.counters.get(baylee_cards_dsl::CounterKind::Loyalty)
        });
        let new = old.saturating_sub(n as u16);
        if let Some(obj) = state.object_mut(target) {
            obj.counters
                .set(baylee_cards_dsl::CounterKind::Loyalty, new);
        }
        state.journal.record(GameEvent::CounterChanged {
            object: target,
            kind: baylee_cards_dsl::CounterKind::Loyalty,
            old,
            new,
        });
    } else {
        // CR 702.2b: deathtouch is a property of the *source*, and it
        // applies to any damage it deals, not just combat damage.
        let deathtouch = state.object(source).is_some_and(|o| {
            o.characteristics()
                .keywords
                .contains(baylee_cards_dsl::KeywordSet::DEATHTOUCH)
        });
        if let Some(obj) = state.object_mut(target) {
            obj.damage = obj.damage.saturating_add(n as u16);
            obj.deathtouched |= deathtouch;
        }
    }
    state.journal.record(GameEvent::DamageDealt {
        source: Some(source),
        target: DamageTarget::Object(target),
        amount: n as u16,
        is_combat: false,
    });
}

pub(super) fn deal_to_player(state: &mut GameState, source: ObjectId, player: PlayerId, n: i16) {
    if n <= 0 {
        return;
    }
    state.change_life(player, -i32::from(n), Cause::Effect);
    state.journal.record(GameEvent::DamageDealt {
        source: Some(source),
        target: DamageTarget::Player(player),
        amount: n as u16,
        is_combat: false,
    });
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
            target_players: baylee_core::ids::SeatSet::new(),
            event_object: None,
            awaiting: None,
            targeted: false,
            mana_ability: false,
            countered_source: None,
        }
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
