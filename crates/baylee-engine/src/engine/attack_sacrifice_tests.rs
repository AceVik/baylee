//! Attack-declaration sacrifice choices: atomic answers, continuation hashes,
//! mana windows, forks, and a player leaving before any cost is paid.

use super::testkit::*;
use super::*;
use crate::engine::{DeckKnowledge, ForkRefusal};
use baylee_core::generated::index;
use baylee_core::ids::{CardIndex, Defender};
use baylee_core::mana::ManaColor;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn game(seats: usize, with_brainwash: bool) -> Engine<RegistryLookup> {
    let mut duel = Duel::table(508_001, index::FOREST, seats)
        .battlefield(
            0,
            &[
                index::GRIZZLY_BEARS,
                index::LLANOWAR_ELVES,
                index::FOREST,
                index::FOREST,
                index::FOREST,
            ],
        )
        .battlefield(1, &[index::FLOODED_WOODLANDS]);
    if with_brainwash {
        duel = duel.hand(0, &[index::BRAINWASH]);
    }
    let mut engine = duel.start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    engine
}

fn all_of(engine: &Engine<RegistryLookup>, player: PlayerId, card: CardIndex) -> Vec<ObjectId> {
    engine
        .state()
        .battlefield_seen()
        .filter(|id| {
            engine.state().object(*id).is_some_and(|object| {
                object.controller == player && object.card.is_some_and(|c| c.index == card)
            })
        })
        .collect()
}

fn creature(engine: &Engine<RegistryLookup>, card: CardIndex) -> ObjectId {
    on_battlefield(engine, P0, card).expect("the creature is on the battlefield")
}

fn declare(engine: &mut Engine<RegistryLookup>, attackers: &[ObjectId]) {
    pass_until(engine, |engine| {
        matches!(
            engine.pending(),
            Pending::ChooseAttackers { player: P0, .. }
        )
    });
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: attackers
                    .iter()
                    .map(|id| (*id, Defender::Player(P1)))
                    .collect(),
            },
        )
        .expect("the declaration can pay its sacrifice choices");
}

fn sacrifice_options(engine: &Engine<RegistryLookup>) -> Vec<ObjectId> {
    let Pending::ChooseCards {
        player: P0,
        options,
        min: 1,
        max: 1,
        prompt: crate::choice::ChoicePrompt::CostSacrifice,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "a single-land sacrifice choice was expected: {:?}",
            engine.pending()
        );
    };
    options
}

fn choose(engine: &mut Engine<RegistryLookup>, objects: Vec<ObjectId>) {
    engine
        .apply(P0, PlayerAction::ChooseObjects { objects })
        .expect("the choice pays the current sacrifice cost");
}

fn zone(engine: &Engine<RegistryLookup>, object: ObjectId) -> Zone {
    engine.state().object(object).expect("object exists").zone
}

fn published(engine: &Engine<RegistryLookup>) -> (u64, usize, String) {
    (
        engine.snapshot_hash(),
        engine.state().journal.len(),
        format!("{:?}", engine.pending()),
    )
}

#[test]
fn invalid_answers_are_atomic_and_the_unpaid_choice_cannot_be_forked() {
    let mut engine = game(2, false);
    let bears = creature(&engine, index::GRIZZLY_BEARS);
    let elves = creature(&engine, index::LLANOWAR_ELVES);
    let forests = all_of(&engine, P0, index::FOREST);
    declare(&mut engine, &[bears, elves]);
    let before = published(&engine);

    for objects in [Vec::new(), vec![forests[0], forests[1]]] {
        assert!(
            engine
                .apply(P0, PlayerAction::ChooseObjects { objects })
                .is_err(),
            "only exactly one offered land answers each cost"
        );
        assert_eq!(published(&engine), before);
    }
    assert_eq!(zone(&engine, forests[0]), Zone::Battlefield);
    assert!(!engine.state().combat.is_attacking(bears));
    assert!(!engine.state().combat.is_attacking(elves));
    assert_eq!(
        engine.fork_for(P0, DeckKnowledge::Known, 1).err(),
        Some(ForkRefusal::Continuation("attack_sacrifice")),
        "a simulation cannot skip an unfinished declaration"
    );
}

#[test]
fn the_snapshot_distinguishes_choices_while_the_board_is_still_identical() {
    let mut first = game(2, false);
    let mut second = game(2, false);
    for engine in [&mut first, &mut second] {
        let bears = creature(engine, index::GRIZZLY_BEARS);
        let elves = creature(engine, index::LLANOWAR_ELVES);
        declare(engine, &[bears, elves]);
    }
    let options = sacrifice_options(&first);
    choose(&mut first, vec![options[0]]);
    choose(&mut second, vec![options[1]]);

    assert_eq!(
        first.state().snapshot_hash(),
        second.state().snapshot_hash(),
        "choices alone have not moved a permanent"
    );
    assert_ne!(
        first.snapshot_hash(),
        second.snapshot_hash(),
        "the suspended declaration records which payment was chosen"
    );
    assert!(!sacrifice_options(&first).contains(&options[0]));
    assert!(!sacrifice_options(&second).contains(&options[1]));
}

#[test]
fn chosen_sacrifices_survive_a_mana_window_and_a_short_payment_changes_nothing() {
    let mut engine = game(2, true);
    let bears = creature(&engine, index::GRIZZLY_BEARS);
    let forests = all_of(&engine, P0, index::FOREST);
    engine.state.players[0].mana_pool.add(ManaColor::White, 1);
    engine.refresh_offer();
    cast_with_floating(&mut engine, P0, index::BRAINWASH);
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![bears],
                players: Vec::new(),
            },
        )
        .expect("Brainwash enchants the Bears");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);

    declare(&mut engine, &[bears]);
    let chosen = forests[1];
    choose(&mut engine, vec![chosen]);
    assert!(
        engine.payment_window().is_some(),
        "Brainwash's {{3}} is owed"
    );
    assert_eq!(zone(&engine, chosen), Zone::Battlefield);
    assert!(!engine.state().combat.is_attacking(bears));
    let before = published(&engine);

    assert!(
        engine
            .apply(P0, PlayerAction::PayWindow { steps: Vec::new() })
            .is_err(),
        "an empty pool cannot pay Brainwash"
    );
    assert_eq!(published(&engine), before);
    assert_eq!(zone(&engine, chosen), Zone::Battlefield);

    engine
        .apply(
            P0,
            PlayerAction::PayWindow {
                steps: forests
                    .iter()
                    .map(|source| PlayerAction::ActivateManaAbility { source: *source })
                    .collect(),
            },
        )
        .expect("the three Forests pay Brainwash after the land was chosen");
    assert_eq!(zone(&engine, chosen), Zone::Graveyard);
    assert_eq!(zone(&engine, forests[0]), Zone::Battlefield);
    assert_eq!(zone(&engine, forests[2]), Zone::Battlefield);
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    assert!(engine.state().combat.is_attacking(bears));
}

#[test]
fn the_choosers_concession_discards_the_unpaid_declaration() {
    let mut engine = game(3, false);
    let bears = creature(&engine, index::GRIZZLY_BEARS);
    let elves = creature(&engine, index::LLANOWAR_ELVES);
    declare(&mut engine, &[bears, elves]);
    let forests = sacrifice_options(&engine);
    choose(&mut engine, vec![forests[0]]);
    let mark = engine.state().journal.len();
    assert_eq!(zone(&engine, forests[0]), Zone::Battlefield);

    engine.apply(P0, PlayerAction::Concede).unwrap();

    assert!(engine.attack_sacrifice.is_none());
    assert!(!matches!(engine.pending(), Pending::GameOver(_)));
    assert!(engine.state().players[0].has_lost());
    assert!(
        !engine.state().journal.entries()[mark..]
            .iter()
            .any(|entry| matches!(
                entry.event,
                GameEvent::ZoneChanged {
                    from: Zone::Battlefield,
                    to: Zone::Graveyard,
                    cause: Cause::TurnBased,
                    ..
                } | GameEvent::BecameAttacker { .. }
            )),
        "neither a stored sacrifice nor an attacker is committed on departure"
    );
}

/// A tax window open in a written record has no sacrifices, and its life tax
/// follows from the attackers and the board: it keeps the value it was recorded
/// with. Only a declaration that chose permanents adds to the continuation.
#[test]
fn a_tax_window_without_sacrifices_keeps_the_fingerprint_a_written_record_holds() {
    let engine = game(2, false);
    let attackers = vec![(
        creature(&engine, index::GRIZZLY_BEARS),
        Defender::Player(P1),
    )];
    let cost = baylee_core::mana::ManaCost::ZERO.with_more_generic(2);
    let opened = Box::new(engine.window_start(P0));
    let recorded = crate::state::structural_fingerprint(&attackers)
        .wrapping_mul(31)
        .wrapping_add(crate::state::mana_cost_fingerprint(&cost))
        .wrapping_mul(31)
        .wrapping_add(opened.fingerprint());
    let window = PaymentContinuation::AttackTax {
        attackers,
        sacrifices: Vec::new(),
        life: 2,
        cost,
        opened,
    };
    assert_eq!(window.fingerprint(), recorded);
}

fn board(mine: &[CardIndex], theirs: &[CardIndex]) -> Engine<RegistryLookup> {
    let mut engine = Duel::table(508_002, index::FOREST, 2)
        .battlefield(0, mine)
        .battlefield(1, theirs)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    engine
}

fn at_the_declaration(engine: &mut Engine<RegistryLookup>) {
    pass_until(engine, |engine| {
        matches!(
            engine.pending(),
            Pending::ChooseAttackers { player: P0, .. }
        )
    });
}

fn attacking(engine: &Engine<RegistryLookup>, object: ObjectId) -> bool {
    engine.state().combat.is_attacking(object)
}

/// A continuous effect of no card: Propaganda and Windborn Muse are not built
/// yet, and Leviathan's own upkeep would stand in the way of the test.
fn impose(
    engine: &mut Engine<RegistryLookup>,
    controller: PlayerId,
    filter: crate::effects::EffectFilter,
    modifier: baylee_cards_dsl::Modifier,
) {
    let timestamp = engine.state.next_timestamp();
    engine
        .state
        .effects
        .register(crate::effects::ContinuousEffect {
            id: baylee_core::ids::EffectId::new(0),
            source: None,
            controller,
            origin: crate::effects::EffectOrigin::Resolution,
            layer: modifier.layer(),
            timestamp,
            duration: baylee_cards_dsl::Duration::Indefinitely,
            filter,
            modifier,
        });
}

/// "Creatures can't attack you unless their controller pays" {`mana`} each.
fn tax_attacks_on_p1(engine: &mut Engine<RegistryLookup>, modifier: baylee_cards_dsl::Modifier) {
    let everything = crate::effects::EffectFilter::Dsl(&baylee_cards_dsl::Filter::Any);
    impose(engine, P1, everything, modifier);
}

const PROPAGANDA: baylee_cards_dsl::Modifier =
    baylee_cards_dsl::Modifier::AttackTaxMana { mana: 2 };

/// CR 508.1h: the whole cost is known before anything is chosen for it. A tax
/// no source can reach ends the declaration here, because once a land is being
/// chosen nothing can take the attack back and every answer would be refused.
#[test]
fn a_tax_out_of_reach_is_refused_before_any_land_is_asked_for() {
    let mut engine = board(
        &[
            index::GRIZZLY_BEARS,
            index::LLANOWAR_ELVES,
            index::FOREST,
            index::FOREST,
        ],
        &[index::FLOODED_WOODLANDS],
    );
    tax_attacks_on_p1(&mut engine, PROPAGANDA);
    let bears = creature(&engine, index::GRIZZLY_BEARS);
    let elves = creature(&engine, index::LLANOWAR_ELVES);
    at_the_declaration(&mut engine);

    let both = vec![(bears, Defender::Player(P1)), (elves, Defender::Player(P1))];
    assert!(
        engine
            .apply(P0, PlayerAction::DeclareAttackers { attackers: both })
            .is_err(),
        "{{4}} is out of reach of three sources"
    );
    assert!(matches!(
        engine.pending(),
        Pending::ChooseAttackers { player: P0, .. }
    ));

    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(bears, Defender::Player(P1))],
            },
        )
        .expect("one attacker's {2} and one land are affordable");
    assert_eq!(sacrifice_options(&engine).len(), 2);
}

/// A land spent on a cost any land pays can leave a stricter cost unpayable, so
/// the question offers only what still lets every cost behind it pay, and no
/// payable declaration is refused for the order its costs happen to come in.
#[test]
fn no_answer_strands_a_later_sacrifice_and_no_payable_attack_is_refused() {
    static ISLAND: baylee_cards_dsl::Filter =
        baylee_cards_dsl::Filter::HasSubtype(baylee_core::generated::subtypes::land::ISLAND);
    for lands in [
        [index::FOREST, index::ISLAND, index::ISLAND],
        [index::ISLAND, index::ISLAND, index::FOREST],
    ] {
        let mut mine = vec![index::SAVANNAH_LIONS, index::GRIZZLY_BEARS];
        mine.extend(lands);
        let mut engine = board(&mine, &[index::FLOODED_WOODLANDS]);
        let lions = creature(&engine, index::SAVANNAH_LIONS);
        let bears = creature(&engine, index::GRIZZLY_BEARS);
        let forest = creature(&engine, index::FOREST);
        let islands = all_of(&engine, P0, index::ISLAND);
        // Leviathan's restriction.
        let filter = crate::effects::EffectFilter::object(&engine.state, lions);
        impose(
            &mut engine,
            P0,
            filter,
            baylee_cards_dsl::Modifier::CantAttackUnlessYouSacrifice {
                count: 2,
                filter: &ISLAND,
            },
        );
        declare(&mut engine, &[bears, lions]);

        assert_eq!(
            sacrifice_options(&engine),
            vec![forest],
            "an Island here would leave the second cost's two Islands short"
        );
        choose(&mut engine, vec![forest]);
        choose(&mut engine, islands.clone());

        assert!(attacking(&engine, bears) && attacking(&engine, lions));
        for land in islands.into_iter().chain([forest]) {
            assert_eq!(zone(&engine, land), Zone::Graveyard);
        }
    }
}

/// What the window's mana abilities did to the declaration is read again as it
/// closes. Llanowar Elves tapped for its own tax is a tapped creature, which
/// cannot be declared (CR 508.1a); the tax itself stays the one determined.
#[test]
fn a_creature_that_tapped_for_the_tax_is_not_declared() {
    let mut engine = board(&[index::LLANOWAR_ELVES, index::FOREST], &[]);
    tax_attacks_on_p1(&mut engine, PROPAGANDA);
    let elves = creature(&engine, index::LLANOWAR_ELVES);
    let forest = creature(&engine, index::FOREST);
    declare(&mut engine, &[elves]);
    assert!(engine.payment_window().is_some());

    engine
        .apply(
            P0,
            PlayerAction::ActivateAbility {
                source: elves,
                ability_index: 0,
            },
        )
        .expect("the Elves make mana in the window");
    engine
        .apply(P0, PlayerAction::ActivateManaAbility { source: forest })
        .expect("the Forest makes mana in the window");
    engine.apply(P0, PlayerAction::PassPriority).unwrap();

    assert!(!attacking(&engine, elves));
}

/// An attacker a mana ability sacrificed is no attacker, whatever paid for it.
#[test]
fn a_creature_sacrificed_for_the_tax_is_not_declared() {
    let mut engine = board(&[index::BLOOD_PET, index::FOREST], &[]);
    tax_attacks_on_p1(&mut engine, PROPAGANDA);
    let pet = creature(&engine, index::BLOOD_PET);
    let forest = creature(&engine, index::FOREST);
    declare(&mut engine, &[pet]);
    assert!(engine.payment_window().is_some());

    engine
        .apply(
            P0,
            PlayerAction::ActivateAbility {
                source: pet,
                ability_index: 0,
            },
        )
        .expect("Blood Pet makes mana in the window");
    engine
        .apply(P0, PlayerAction::ActivateManaAbility { source: forest })
        .expect("the Forest makes mana in the window");
    engine.apply(P0, PlayerAction::PassPriority).unwrap();

    assert_eq!(zone(&engine, pet), Zone::Graveyard);
    assert!(!attacking(&engine, pet));
}

/// The life half is paid when the window closes, so it is checked there too:
/// life a mana ability cost in the window is life that no longer pays it.
#[test]
fn life_lost_in_the_window_no_longer_pays_the_declaration() {
    let mut engine = board(&[index::GRIZZLY_BEARS, index::FOREST, index::FOREST], &[]);
    tax_attacks_on_p1(&mut engine, PROPAGANDA);
    tax_attacks_on_p1(
        &mut engine,
        baylee_cards_dsl::Modifier::AttackTaxLife { life: 5 },
    );
    let bears = creature(&engine, index::GRIZZLY_BEARS);
    declare(&mut engine, &[bears]);
    assert!(engine.payment_window().is_some());

    engine.state.players[0].life = 4;
    for forest in all_of(&engine, P0, index::FOREST) {
        engine
            .apply(P0, PlayerAction::ActivateManaAbility { source: forest })
            .expect("the Forests make mana in the window");
    }
    engine.apply(P0, PlayerAction::PassPriority).unwrap();

    assert!(!attacking(&engine, bears));
    assert_eq!(engine.state().players[0].life, 4);
}

/// A land borrowed from a player who then leaves goes with them (CR 800.4a), and
/// the costs may no longer be payable. Nothing has been paid, so the declaration
/// is illegal and is made again rather than left asking what nobody can answer.
#[test]
fn a_leaver_taking_the_land_a_cost_needed_reopens_the_declaration() {
    let p2 = PlayerId::new(2);
    let mut engine = Duel::table(508_003, index::FOREST, 3)
        .battlefield(
            0,
            &[index::GRIZZLY_BEARS, index::LLANOWAR_ELVES, index::FOREST],
        )
        .battlefield(1, &[index::FLOODED_WOODLANDS])
        .battlefield(2, &[index::FOREST])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    let borrowed = on_battlefield(&engine, p2, index::FOREST).expect("the third seat's land");
    let land = engine.state.object_mut(borrowed).expect("the land");
    land.controller = P0;
    land.base_controller = P0;
    let bears = creature(&engine, index::GRIZZLY_BEARS);
    let elves = creature(&engine, index::LLANOWAR_ELVES);
    declare(&mut engine, &[bears, elves]);
    assert_eq!(sacrifice_options(&engine).len(), 2);

    engine.apply(p2, PlayerAction::Concede).unwrap();

    assert!(engine.attack_sacrifice.is_none());
    assert!(
        matches!(
            engine.pending(),
            Pending::ChooseAttackers { player: P0, .. }
        ),
        "one land cannot pay two costs: {:?}",
        engine.pending()
    );
    assert!(!attacking(&engine, bears) && !attacking(&engine, elves));
}

/// CR 508.1k: only a chosen creature still under its controller's control
/// becomes an attacker. A green land creature may pay Flooded Woodlands with
/// itself, and then there is nothing left to declare.
#[test]
fn a_creature_sacrificed_to_its_own_attack_never_becomes_an_attacker() {
    let mut engine = board(
        &[index::DRYAD_ARBOR, index::FOREST],
        &[index::FLOODED_WOODLANDS],
    );
    let arbor = creature(&engine, index::DRYAD_ARBOR);
    declare(&mut engine, &[arbor]);
    let mark = engine.state().journal.len();
    assert!(sacrifice_options(&engine).contains(&arbor));

    choose(&mut engine, vec![arbor]);

    assert_eq!(zone(&engine, arbor), Zone::Graveyard);
    assert!(!attacking(&engine, arbor));
    assert!(
        !engine.state().journal.entries()[mark..]
            .iter()
            .any(|entry| matches!(entry.event, GameEvent::BecameAttacker { .. })),
        "no attack was made"
    );
}
