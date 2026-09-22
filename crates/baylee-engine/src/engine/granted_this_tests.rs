//! `Filter::This` inside an ability one object **granted** another.
//!
//! `Modifier::GrantActivated` carries an effect list, and an effect in that
//! list may name its own source. The question this module exists for is what
//! "its own source" means when the ability was not printed on the permanent
//! activating it: the *grantor*, who owns the continuous effect that handed
//! the ability out, or the *grantee*, who activated it.
//!
//! It has to be the grantee, and nothing in the pool could say so. Wandering
//! Fumarole is the only card here whose granted ability names `Filter::This`
//! — "{0}: Switch this creature's power and toughness until end of turn",
//! granted by the animation to the land the animation is on — and it grants
//! *itself*, so grantor and grantee are one object and every reading agrees.
//! A card test over it passes whichever answer the engine gives.
//!
//! So the bench hands the ability from one object to two others. A land
//! grants every creature its controller has `{0}: this gets +2/+0 until end
//! of turn`; one creature activates and the other does not. Three answers are
//! separable that way and only one of them is right:
//!
//! - resolved against the **grantee** → the activating creature is 3/1 and
//!   its neighbour is untouched. The rule.
//! - resolved against the **grantor** → neither creature changes, and the
//!   land silently carries a P/T it has no use for.
//! - resolved against the **grant's filter** → both creatures are 3/1, which
//!   is an anthem rather than an ability.
//!
//! The second and third are what a plausible implementation does, which is
//! why the neighbour is on the table at all: a test with one creature on it
//! cannot tell the first answer from the third.

use super::synthetic::{SyntheticLookup, creature, keep_mulligans, land, preset, walk_past};
use super::*;
use baylee_cards_dsl::{AbilityDef, Cost, Duration, Effect, Filter, Modifier, static_ability};

const BENCH: u32 = 1_501;
const ALPHA: u32 = 1_502;
const BETA: u32 = 1_503;

/// "Creatures you control have `{0}: This creature gets +2/+0 until end of
/// turn.`" — Chromatic Lantern's shape with a pump in place of the mana,
/// because a pump is readable off the board and mana is not.
static GRANTOR: &[AbilityDef] = &[static_ability!(
    Filter::And(&[Filter::CREATURE, Filter::ControlledByYou]),
    Modifier::GrantActivated {
        cost: Cost::FREE,
        effects: &[Effect::continuous(
            &Filter::This,
            Modifier::ModifyPT(2, 0),
            Duration::UntilEndOfTurn,
        )],
        mana_ability: false,
    }
)];

fn lookup() -> SyntheticLookup {
    SyntheticLookup::new(vec![
        land(BENCH, "Grantor", GRANTOR),
        creature(ALPHA, "Alpha", 1, 1, &[]),
        creature(BETA, "Beta", 1, 1, &[]),
    ])
}

/// The bench at seat 0's main phase: the land and the two creatures, in the
/// order `preset` seated them.
fn seated(seed: u64) -> (Engine<SyntheticLookup>, ObjectId, ObjectId) {
    let me = PlayerId::new(0);
    let mut engine =
        Engine::new(&preset(seed, &[BENCH, ALPHA, BETA]), lookup()).expect("the game starts");
    keep_mulligans(&mut engine);
    for _ in 0..400 {
        if matches!(engine.state().turn.phase, Phase::FirstMain)
            && engine.state().turn.active == me
            && matches!(engine.pending(), Pending::Priority { player, .. } if *player == me)
        {
            let alpha = *super::synthetic::permanents(&engine, ALPHA)
                .first()
                .expect("Alpha is on the bench");
            let beta = *super::synthetic::permanents(&engine, BETA)
                .first()
                .expect("Beta is on the bench");
            return (engine, alpha, beta);
        }
        let pending = engine.pending().clone();
        assert!(walk_past(&mut engine, &pending), "walked past {pending:?}");
    }
    panic!("never reached seat 0's main phase");
}

/// What the module is for: the ability resolves against whoever activated it.
#[test]
fn a_granted_ability_naming_this_object_resolves_against_the_grantee() {
    let me = PlayerId::new(0);
    let (mut engine, alpha, beta) = seated(9_100);

    let pt = |engine: &Engine<SyntheticLookup>, id: ObjectId| {
        let c = engine
            .state()
            .object(id)
            .expect("on the battlefield")
            .characteristics();
        (c.power, c.toughness)
    };
    assert_eq!(pt(&engine, alpha), (Some(1), Some(1)), "a printed 1/1");
    assert_eq!(
        pt(&engine, beta),
        (Some(1), Some(1)),
        "and so is its neighbour"
    );

    // The grant is offered on the creature, not on the land that hands it
    // out: `choice::granted_ability(0)` is a slot on *this* permanent.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal
            .abilities
            .contains(&(alpha, crate::choice::granted_ability(0))),
        "the creature is offered the granted ability: {:?}",
        legal.abilities
    );

    engine
        .apply(
            me,
            PlayerAction::ActivateAbility {
                source: alpha,
                ability_index: crate::choice::granted_ability(0),
            },
        )
        .expect("a {0} ability with no target asks nothing");
    for _ in 0..40 {
        if matches!(engine.pending(), Pending::Priority { .. })
            && engine.state().zones.list(ZoneLocation::Stack).is_empty()
        {
            break;
        }
        let pending = engine.pending().clone();
        assert!(walk_past(&mut engine, &pending), "walked past {pending:?}");
    }

    assert_eq!(
        pt(&engine, alpha),
        (Some(3), Some(1)),
        "\"this creature gets +2/+0\" is about the creature that activated it"
    );
    assert_eq!(
        pt(&engine, beta),
        (Some(1), Some(1)),
        "and about no other creature the same static reached — a grant hands \
         out an ability, not its effect"
    );
}
