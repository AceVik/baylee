//! Life totals and damage: gain/lose life, damage to players, objects,
//! and planeswalkers (loyalty removal).

#[allow(clippy::wildcard_imports)] // family modules share the resolve vocabulary
use super::*;

/// Executes one life/damage effect.
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
                let p = &mut state.players[player.get() as usize];
                let old = p.life;
                p.life -= n;
                let new = p.life;
                state.journal.record(GameEvent::LifeChanged {
                    player,
                    old,
                    new,
                    cause: Cause::Effect,
                });
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
        Effect::DealDamageToTargetController { amount } => {
            if let Some(&target_id) = res.targets.first() {
                let controller = state.object(target_id).map_or(you, |o| o.controller);
                let n = amount2(&amount, state, you, res) as i16;
                deal_to_player(state, res.source, controller, n);
            }
            None
        }
        _ => unreachable!("not a life/damage effect"),
    }
}

pub(super) fn gain_life(state: &mut GameState, player: PlayerId, n: i32) {
    if n <= 0 {
        return;
    }
    let p = &mut state.players[player.get() as usize];
    let old = p.life;
    p.life += n;
    let new = p.life;
    state.journal.record(GameEvent::LifeChanged {
        player,
        old,
        new,
        cause: Cause::Effect,
    });
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
    let p = &mut state.players[player.get() as usize];
    let old = p.life;
    p.life -= i32::from(n);
    let new = p.life;
    state.journal.record(GameEvent::LifeChanged {
        player,
        old,
        new,
        cause: Cause::Effect,
    });
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
}
