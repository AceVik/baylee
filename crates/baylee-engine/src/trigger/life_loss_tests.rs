use super::*;
use crate::engine::synthetic::{SyntheticLookup, land, preset};
use crate::event::Cause;
use baylee_cards_dsl::{AbilityDef, Amount, Effect, PlayerRel, Trigger, triggered};

#[test]
fn life_loss_triggers_include_costs_and_capture_only_actual_loss() {
    const INDEX: u32 = 19034;
    static ABILITIES: &[AbilityDef] = &[triggered!(
        Trigger::LosesLife(PlayerRel::You),
        &[Effect::GainLife {
            amount: Amount::EventAmount
        }]
    )];
    let lookup = SyntheticLookup::new(vec![land(INDEX, "Loss observer", ABILITIES)]);
    let mut state = GameState::from_preset(&preset(53, &[INDEX]), &lookup).unwrap();
    let you = PlayerId::new(0);
    for (by, cause, expected) in [
        (-3, Cause::Cost, Some(3)),
        (2, Cause::Effect, None),
        (-2, Cause::Effect, Some(2)),
        (0, Cause::Effect, None),
    ] {
        let from = state.journal.last_seq();
        state.change_life(you, by, cause);
        let triggers = collect(&state, &lookup, from);
        assert_eq!(triggers.len(), usize::from(expected.is_some()));
        if let Some(expected) = expected {
            assert_eq!(triggers[0].event_amount, Some(expected));
        }
    }
}

#[test]
fn prohibited_life_loss_emits_neither_a_loss_trigger_nor_an_amount() {
    use baylee_cards_dsl::{Filter, Modifier, static_ability};
    const INDEX: u32 = 19035;
    static ABILITIES: &[AbilityDef] = &[
        triggered!(
            Trigger::LosesLife(PlayerRel::You),
            &[Effect::GainLife {
                amount: Amount::EventAmount
            }]
        ),
        static_ability!(
            Filter::This,
            Modifier::CantLoseLife {
                who: PlayerRel::You
            }
        ),
    ];
    let def = land(INDEX, "Protected loss observer", ABILITIES);
    let lookup = SyntheticLookup::new(vec![def]);
    let mut engine =
        crate::engine::Engine::new(&preset(54, &[INDEX]), SyntheticLookup::new(vec![def])).unwrap();
    crate::engine::synthetic::keep_mulligans(&mut engine);
    let mut state = engine.state().clone();
    let from = state.journal.last_seq();
    state.change_life(PlayerId::new(0), -7, Cause::Effect);
    assert_eq!(state.players[0].life, 20);
    assert!(collect(&state, &lookup, from).is_empty());
}
