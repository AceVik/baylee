//! Native definitions and stored emblems follow the same rules pipeline.
use super::*;
use baylee_cards_dsl::{
    AbilityDef, Effect, Modifier, PlayerRel, StepKind, TokenDef, Trigger, f, static_ability,
    triggered,
};
use baylee_core::{
    ids::CardIndex,
    preset::{LoopPolicy, SeatSupplies, TowerSupply},
};

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

static NATIVE: TokenDef = TokenDef {
    name: "Synthetic upkeep emblem",
    abilities: &[
        static_ability!(f!(your CREATURE), Modifier::ModifyPT(1, 1)),
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You
            },
            &[Effect::gain_life(2)]
        ),
    ],
    ..TokenDef::DEFAULT
};
static EMPTY: TokenDef = TokenDef {
    name: "Synthetic empty emblem",
    ..TokenDef::DEFAULT
};
static STORED: &[AbilityDef] = &[
    static_ability!(f!(your CREATURE), Modifier::ModifyPT(2, 2)),
    triggered!(
        Trigger::StepBegin {
            step: StepKind::Upkeep,
            whose: PlayerRel::You
        },
        &[Effect::gain_life(3)]
    ),
];

struct Lookup {
    stored: bool,
}
impl CardLookup for Lookup {
    fn card(&self, index: CardIndex) -> Option<&'static baylee_cards_dsl::CardDef> {
        baylee_cards::by_index(index)
    }
    fn tower_supply(&self, supply: TowerSupply) -> Option<&'static TokenDef> {
        Some(match supply {
            TowerSupply::HealingPotion => {
                if self.stored {
                    &EMPTY
                } else {
                    &NATIVE
                }
            }
            _ => baylee_cards::tokens::tower::definition(supply),
        })
    }
}

fn game<D: LoopDetection>(policy: LoopPolicy, stored: bool) -> Engine<Lookup, D> {
    let mut preset = super::opening_rules_tests::preset(Vec::new());
    preset.house_rules.loop_policy = policy;
    preset.house_rules.supplies = (0..2)
        .map(|seat| SeatSupplies {
            seat,
            healing: 1,
            mana: 0,
            seedling: true,
            concealed: false,
        })
        .collect();
    // Empty hands avoid unrelated cleanup choices on the way to the next upkeep.
    for seat in &mut preset.seats {
        seat.starting_hand = Some(Vec::new());
    }
    let mut engine = Engine::with_detection(&preset, Lookup { stored }).unwrap();
    if stored {
        for player in [P0, P1] {
            let id = engine.state.zones.list(ZoneLocation::Command(player))[0];
            let object = engine.state.object_mut(id).unwrap();
            object.token = Some(&NATIVE);
            object.own_abilities =
                Some(crate::object::AbilityList::from_static(STORED, None, None).into_bundle());
        }
    }
    engine.apply(P0, PlayerAction::MulliganKeep).unwrap();
    engine.apply(P1, PlayerAction::MulliganKeep).unwrap();
    engine
}

fn to_main<D: LoopDetection>(engine: &mut Engine<Lookup, D>, seat: PlayerId) {
    for _ in 0..80 {
        if engine.state.turn.active == seat && engine.state.turn.phase == Phase::FirstMain {
            return;
        }
        let (player, action) = match engine.pending().clone() {
            Pending::Priority { player, .. } => (player, PlayerAction::PassPriority),
            Pending::ChooseAttackers { player, .. } => (
                player,
                PlayerAction::DeclareAttackers {
                    attackers: Vec::new(),
                },
            ),
            Pending::ChooseBlockers { player, .. } => (
                player,
                PlayerAction::DeclareBlockers {
                    blockers: Vec::new(),
                },
            ),
            pending => panic!("unexpected fixture choice: {pending:?}"),
        };
        engine.apply(player, action).unwrap();
    }
    panic!("fixture did not reach the requested main phase");
}

fn check<D: LoopDetection>(policy: LoopPolicy, stored: bool) {
    let mut engine = game::<D>(policy, stored);
    let delta = if stored { 3 } else { 2 };
    for &id in engine.state.zones.list(ZoneLocation::Battlefield) {
        let object = engine.state.object(id).unwrap();
        assert_eq!(object.characteristics().power, Some(delta as i16));
        assert_eq!(object.characteristics().toughness, Some(delta as i16));
    }
    assert_eq!(
        engine.state.players[0].life, 20,
        "the trigger waits on the stack"
    );
    assert_eq!(engine.state.zones.list(ZoneLocation::Stack).len(), 1);
    to_main(&mut engine, P0);
    assert_eq!(engine.state.players[0].life, 20 + delta);
    assert_eq!(
        engine.state.players[1].life, 20,
        "only its controller's upkeep fires"
    );
    to_main(&mut engine, P1);
    assert_eq!(engine.state.players[0].life, 20 + delta);
    assert_eq!(engine.state.players[1].life, 20 + delta);
    for &id in engine.state.zones.list(ZoneLocation::Battlefield) {
        assert_eq!(
            engine.state.object(id).unwrap().characteristics().power,
            Some(delta as i16),
            "repeated synchronization cannot duplicate a static"
        );
    }
}

#[test]
fn native_emblem_triggers_and_statics_follow_every_loop_policy() {
    check::<NoLoopDetection>(LoopPolicy::Off, false);
    check::<DetectLoops>(LoopPolicy::RunOnceThenBreak, false);
    check::<DetectLoops>(LoopPolicy::CompRulesDraw, false);
}

#[test]
fn stored_emblem_abilities_keep_precedence_over_a_native_definition() {
    check::<NoLoopDetection>(LoopPolicy::Off, true);
}
