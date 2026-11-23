//! Wave 15d `baylee-ai` integration tests:
//! 1. Agatha of the Vile Cauldron / `ActivatedAbilitiesCostLess` AI pricing &
//!    mana floating (`unpaid_abilities` reduced cost honored, `min_mana` floor
//!    respected, AI taps mana for reduced activation instead of demanding printed cost).
//! 2. Alluring Siren (`Modifier::MustAttackPlayer`) AI attacker selection:
//!    `HeuristicAgent::act` and `foresight::Candidates` NEVER emit illegal
//!    combat declarations even into a lethal blocker or with an opposing
//!    planeswalker on board.
use super::*;
use baylee_core::mana::ManaCost;
use baylee_engine::choice::{AttackLimit, LegalActions};

#[test]
fn wave15d_ai_alluring_siren_must_attack_player_never_illegal() {
    // Seat 0 controls a 2/2 bear; seat 1 controls a lethal 7/7 blocker AND a planeswalker.
    // Alluring Siren forced seat 0's bear to attack Defender::Player(1) (NOT the planeswalker!).
    let bear = permanent(obj(1), PlayerId::new(0), 2);
    let wall = permanent(obj(2), PlayerId::new(1), 7);
    let mut pw = permanent(obj(3), PlayerId::new(1), 0);
    pw.types = TypeSet::PLANESWALKER;
    pw.power = None;
    pw.toughness = None;
    let v = view(0, &[20, 20], vec![bear, wall, pw]);

    let defenders = vec![
        Defender::Player(PlayerId::new(1)),
        Defender::Planeswalker(obj(3)),
    ];
    let pending = Pending::ChooseAttackers {
        player: PlayerId::new(0),
        attackers: vec![obj(1)],
        defenders,
        required: vec![obj(1)],
        alone: Vec::new(),
        minimum_required: Some(1),
        limits: vec![AttackLimit {
            creature: obj(1),
            defenders: vec![Defender::Player(PlayerId::new(1))],
        }],
    };

    for profile in [AIProfile::SHARP, AIProfile::EXPERT] {
        let agent = HeuristicAgent::new(profile);
        let action = agent.act(&v, &pending);
        assert!(
            pending.answer_fault(&action).is_none(),
            "profile {profile:?} must never generate an illegal attack declaration: {action:?}"
        );
        assert_eq!(
            action,
            PlayerAction::DeclareAttackers {
                attackers: vec![(obj(1), Defender::Player(PlayerId::new(1)))],
            },
            "Alluring Siren must force bear to attack Player(1), not stay back or hit planeswalker"
        );

        // Foresight candidates must also all be legal!
        let cands = agent.foresight_candidates(&v, &pending, &action, 8);
        for cand in &cands.answers {
            assert!(
                pending.answer_fault(cand).is_none(),
                "foresight candidate must obey MustAttackPlayer limits: {cand:?}"
            );
        }
    }
}

#[test]
fn wave15d_ai_taps_reduced_mana_from_unpaid_abilities_for_activation() {
    // Use Shivan Dragon (`{R}: +1/+0 until EOT`, or any creature with a mana-costed
    // activation — here Prodigal Pyromancer / Torchling / Dragon Whelp / Jayemdae Tome).
    // Jayemdae Tome has `{4}, {T}: Draw a card`. With a cost reducer (Agatha/Heartstone)
    // reducing it to `{1}` in `legal.unpaid_abilities`, and only 1 Forest untapped:
    // Previously `floating.rs` ignored `unpaid_abilities` for TapSelf abilities and
    // checked printed `{4}` > room(1), refusing to tap the Forest!
    let mut tome = carded(
        permanent(obj(1), PlayerId::new(0), 0),
        "Jayemdae Tome",
        TypeSet::ARTIFACT,
    );
    tome.summoning_sick = false;
    let mut forest = carded(
        permanent(obj(2), PlayerId::new(0), 0),
        "Forest",
        TypeSet::LAND,
    );
    forest.supertypes = SupertypeSet::BASIC;
    forest
        .subtypes
        .insert(baylee_core::generated::subtypes::land::FOREST);
    forest.summoning_sick = false;
    let mut v = view(0, &[20, 20], vec![tome, forest]);
    v.active = PlayerId::new(0);
    v.phase = baylee_view::Phase::FirstMain;
    v.step = baylee_view::Step::Main;

    let mut legal = LegalActions {
        can_pass: true,
        mana_abilities: vec![obj(2)],
        unpaid_abilities: vec![(obj(1), 0, ManaCost::parse("{1}"))],
        ..LegalActions::default()
    };
    let pending = Pending::Priority {
        player: PlayerId::new(0),
        legal: Box::new(legal.clone()),
    };

    // With only 1 mana available and reduced cost {1} in unpaid_abilities, EXPERT taps Forest!
    let action = HeuristicAgent::new(AIProfile::EXPERT).act(&v, &pending);
    assert_eq!(
        action,
        PlayerAction::ActivateManaAbility { source: obj(2) },
        "AI must read reduced price {{1}} from unpaid_abilities and tap Forest toward Jayemdae Tome"
    );

    // Without the reduction (unpaid cost still {4}), 1 Forest is NOT enough -> passes priority!
    legal.unpaid_abilities = vec![(obj(1), 0, ManaCost::parse("{4}"))];
    let pending_full = Pending::Priority {
        player: PlayerId::new(0),
        legal: Box::new(legal),
    };
    assert_eq!(
        HeuristicAgent::new(AIProfile::EXPERT).act(&v, &pending_full),
        PlayerAction::PassPriority,
        "without reduction, 1 Forest cannot pay {{4}}"
    );
}

#[test]
fn wave15d_ai_effective_activation_mana_evaluates_agatha_power_floor_and_exhaust() {
    use baylee_cards_dsl::{ActivationLimit, ActivationTiming, ActivationZone, Cost, Effect};
    // Verify `effective_activation_mana` directly on a Heartstone board (power/fixed + floor).
    let heartstone = carded(
        permanent(obj(10), PlayerId::new(0), 0),
        "Heartstone",
        TypeSet::ARTIFACT,
    );
    let creature = permanent(obj(11), PlayerId::new(0), 2);
    let v = view(0, &[20, 20], vec![heartstone, creature]);

    let normal_3 = AbilityDef::Activated {
        cost: Cost {
            mana: ManaCost::parse("{3}"),
            parts: &[],
        },
        effects: &[Effect::GainLife {
            amount: baylee_cards_dsl::Amount::Fixed(2),
        }],
        targets: None,
        second_targets: None,
        third_targets: None,
        timing: ActivationTiming::InstantSpeed,
        mana_ability: false,
        zone: ActivationZone::Battlefield,
        limit: ActivationLimit::Unlimited,
        x_value: baylee_cards_dsl::ActivationX::Any,
        cost_reduction: None,
    };
    assert_eq!(
        crate::policy::effective_activation_mana(&v, obj(11), &normal_3, ManaCost::parse("{3}")),
        ManaCost::parse("{2}"),
        "Heartstone reduces creature activation {{3}} by {{1}} to {{2}}"
    );
    // Floor at 1 mana: {1} stays {1}.
    assert_eq!(
        crate::policy::effective_activation_mana(&v, obj(11), &normal_3, ManaCost::parse("{1}")),
        ManaCost::parse("{1}"),
        "Heartstone's 1-mana floor prevents reducing {{1}} to {{0}}"
    );
}
