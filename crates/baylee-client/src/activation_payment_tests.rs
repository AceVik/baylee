//! Human arming and automatic payment against real engine offers.
use super::*;
use baylee_core::preset::DeckEntry;
use baylee_engine::{engine::Engine, turn::Step};
use baylee_gamehost::{RegistryLookup, SeatContext, player_view};

fn table(board: &[&str]) -> Engine<RegistryLookup> {
    table_against(board, &[])
}

fn table_against(board: &[&str], theirs: &[&str]) -> Engine<RegistryLookup> {
    let entry = |name| DeckEntry {
        card: baylee_cards::decks::by_name(name).unwrap(),
        print: baylee_core::ids::PrintRef::new(0),
    };
    let mut preset = baylee_cards::decks::probe_preset(41, entry("Forest").card).unwrap();
    for seat in &mut preset.seats {
        seat.starting_hand = Some(vec![]);
        seat.starting_battlefield.clear();
    }
    preset.seats[0].starting_battlefield = board.iter().map(|name| entry(name)).collect();
    preset.seats[1].starting_battlefield = theirs.iter().map(|name| entry(name)).collect();
    let mut engine = Engine::new(&preset, RegistryLookup).unwrap();
    for _ in 0..200 {
        let pending = engine.pending().clone();
        let player = pending.asked().unwrap();
        if matches!(pending, Pending::Priority { .. })
            && player == PlayerId::new(0)
            && engine.state().turn.active == player
            && engine.state().turn.step == Step::Main
        {
            return engine;
        }
        let action = match pending {
            Pending::Mulligan { .. } => PlayerAction::MulliganKeep,
            Pending::Priority { .. } => PlayerAction::PassPriority,
            Pending::ChooseAttackers { .. } => PlayerAction::DeclareAttackers { attackers: vec![] },
            other => panic!("unexpected fixture question: {other:?}"),
        };
        engine.apply(player, action).unwrap();
    }
    panic!("no main phase");
}

fn sync(duel: &mut Duel, engine: &Engine<RegistryLookup>) {
    let pending = engine.pending().clone();
    let seq = duel.view.as_ref().map_or(0, |v| v.seq + 1);
    let view = player_view(
        engine.state(),
        PlayerId::new(0),
        seq,
        Some(&pending),
        &SeatContext::default(),
        &[],
    );
    duel.receive_view(view);
    duel.receive_choice(pending);
    rebuild_board(duel);
}

fn object(duel: &Duel, name: &str) -> ObjectId {
    duel.view
        .as_ref()
        .unwrap()
        .battlefield
        .iter()
        .find(|o| o.name == name)
        .unwrap()
        .id
}

fn run(duel: &mut Duel, engine: &mut Engine<RegistryLookup>) {
    for _ in 0..20 {
        if duel.mana_run.is_none() {
            return;
        }
        advance_mana_run(duel);
        for action in std::mem::take(&mut duel.outbox) {
            engine.apply(PlayerId::new(0), action).unwrap();
        }
        sync(duel, engine);
    }
    panic!("payment did not finish");
}

#[test]
fn nexus_arms_without_spending_then_taps_three_lands_and_creates_its_token() {
    let mut engine = table(&["Maskwood Nexus", "Forest", "Forest", "Forest"]);
    let mut duel = Duel::default();
    sync(&mut duel, &engine);
    let nexus = object(&duel, "Maskwood Nexus");
    assert_eq!(duel.reach_of(nexus), Some(Reach::Taps));
    let legal = duel.interaction.as_ref().unwrap().legal_actions().unwrap();
    let (_, index, _) = legal
        .unpaid_abilities
        .iter()
        .find(|(id, _, _)| *id == nexus)
        .unwrap();
    let index = *index;
    assert!(!legal.abilities.contains(&(nexus, index)));
    let before = engine.snapshot_hash();
    assert!(
        engine
            .apply(
                PlayerId::new(0),
                PlayerAction::ActivateAbility {
                    source: nexus,
                    ability_index: index
                }
            )
            .is_err()
    );
    assert_eq!(
        before,
        engine.snapshot_hash(),
        "a hint is not an accepted action"
    );
    input::activate_card(&mut duel, nexus);
    assert!(duel.outbox.is_empty());
    assert_eq!(duel.ability_menu, Some(nexus));
    let Some(Armed {
        deed: Deed::Run {
            plan,
            then: RunEnd::Ability(_),
        },
        ..
    }) = &duel.armed
    else {
        panic!("armed automatic payment")
    };
    assert_eq!(plan.steps.len(), 3);
    assert!(plan.steps.iter().all(|step| step.source != nexus));
    // Canceling the staged deed sends nothing. Selecting again makes a new plan.
    duel.armed = None;
    input::activate_card(&mut duel, nexus);
    assert!(duel.outbox.is_empty());
    input::fire_armed(&mut duel);
    run(&mut duel, &mut engine);
    assert!(duel.last_error.is_none());
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    assert!(!engine.state().zones.stack_is_empty());
    for _ in 0..10 {
        if engine.state().zones.stack_is_empty() {
            break;
        }
        engine
            .apply(
                engine.pending().asked().unwrap(),
                PlayerAction::PassPriority,
            )
            .unwrap();
    }
    sync(&mut duel, &engine);
    assert!(
        duel.view
            .as_ref()
            .unwrap()
            .battlefield
            .iter()
            .any(|o| o.name == "Shapeshifter"
                && o.power == Some(2)
                && o.toughness == Some(2)
                && o.colors == baylee_core::color::ColorSet::of(baylee_core::color::Color::Blue))
    );
    assert!(
        !duel.ability_reach.contains(&nexus),
        "the tapped Nexus cannot activate again"
    );
}

#[test]
fn equip_makes_only_its_mana_then_hands_target_selection_to_the_player() {
    let mut engine = table(&["Bonesplitter", "Grizzly Bears", "Forest", "Forest"]);
    let mut duel = Duel::default();
    sync(&mut duel, &engine);
    let equipment = object(&duel, "Bonesplitter");
    let bear = object(&duel, "Grizzly Bears");
    input::activate_card(&mut duel, equipment);
    input::fire_armed(&mut duel);
    run(&mut duel, &mut engine);
    let Pending::ChooseTargets { options, .. } = engine.pending() else {
        panic!("equip target choice")
    };
    assert!(options.contains(&bear));
    assert!(duel.mana_run.is_none());
    engine
        .apply(
            PlayerId::new(0),
            PlayerAction::ChooseObjects {
                objects: vec![bear],
            },
        )
        .unwrap();
    for _ in 0..10 {
        if engine.state().zones.stack_is_empty() {
            break;
        }
        engine
            .apply(
                engine.pending().asked().unwrap(),
                PlayerAction::PassPriority,
            )
            .unwrap();
    }
    sync(&mut duel, &engine);
    assert_eq!(
        duel.view.as_ref().unwrap().object(bear).unwrap().power,
        Some(4)
    );
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
}

#[test]
fn an_artifact_activation_lock_removes_the_payment_hint_and_the_client_plan() {
    let engine = table_against(
        &["Maskwood Nexus", "Forest", "Forest", "Forest"],
        &["Karn, the Great Creator"],
    );
    let mut duel = Duel::default();
    sync(&mut duel, &engine);
    let nexus = object(&duel, "Maskwood Nexus");
    let legal = duel.interaction.as_ref().unwrap().legal_actions().unwrap();
    assert!(
        !legal
            .unpaid_abilities
            .iter()
            .any(|(source, _, _)| *source == nexus)
    );
    assert!(!duel.ability_reach.contains(&nexus));
    assert!(
        abilities::options(
            baylee_client_core::i18n::Lang::En,
            duel.view.as_ref().unwrap(),
            duel.interaction.as_ref().unwrap(),
            nexus
        )
        .is_empty()
    );
}

#[test]
fn confirming_after_a_manual_tap_only_makes_the_remaining_mana() {
    let mut engine = table(&["Maskwood Nexus", "Forest", "Forest", "Forest"]);
    let mut duel = Duel::default();
    sync(&mut duel, &engine);
    let nexus = object(&duel, "Maskwood Nexus");
    input::activate_card(&mut duel, nexus);
    let forest = object(&duel, "Forest");
    engine
        .apply(
            PlayerId::new(0),
            PlayerAction::ActivateManaAbility { source: forest },
        )
        .unwrap();
    sync(&mut duel, &engine);
    input::fire_armed(&mut duel);
    assert_eq!(duel.mana_run.as_ref().unwrap().steps.len(), 2);
    run(&mut duel, &mut engine);
    assert!(duel.last_error.is_none());
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    assert!(!engine.state().zones.stack_is_empty());
}

#[test]
fn missing_targets_and_insufficient_mana_never_become_clickable_plans() {
    let engine = table(&["Bonesplitter", "Maskwood Nexus", "Forest"]);
    let mut duel = Duel::default();
    sync(&mut duel, &engine);
    let legal = duel.interaction.as_ref().unwrap().legal_actions().unwrap();
    let equipment = object(&duel, "Bonesplitter");
    let nexus = object(&duel, "Maskwood Nexus");
    assert!(
        !legal
            .unpaid_abilities
            .iter()
            .any(|(id, _, _)| *id == equipment)
    );
    assert!(legal.unpaid_abilities.iter().any(|(id, _, _)| *id == nexus));
    assert!(duel.ability_reach.is_empty());
    input::activate_card(&mut duel, nexus);
    assert!(duel.armed.is_none() && duel.outbox.is_empty());
}

#[test]
fn sacrificed_food_keeps_its_stack_picture_and_sentence_after_automatic_payment() {
    use baylee_client_core::images::{ArtSize, ImageKey};
    let mut engine = table(&["Oko, Thief of Crowns", "Forest", "Forest"]);
    let mut duel = Duel::default();
    sync(&mut duel, &engine);
    let oko = object(&duel, "Oko, Thief of Crowns");
    engine
        .apply(
            PlayerId::new(0),
            PlayerAction::ActivateAbility {
                source: oko,
                ability_index: 0,
            },
        )
        .unwrap();
    settle_stack(&mut engine);
    sync(&mut duel, &engine);
    let food = object(&duel, "Food");
    let token = baylee_cards::tokens::token_id(&baylee_cards::tokens::FOOD);
    input::activate_card(&mut duel, food);
    assert!(duel.outbox.is_empty());
    let option = abilities::options(
        baylee_client_core::i18n::Lang::En,
        duel.view.as_ref().unwrap(),
        duel.interaction.as_ref().unwrap(),
        food,
    )
    .remove(0);
    let words = abilities::printed_words(None, duel.view.as_ref().unwrap(), food, &option).unwrap();
    assert_eq!(
        words.head.as_deref(),
        Some("{2}, {T}, Sacrifice this token")
    );
    input::fire_armed(&mut duel);
    run(&mut duel, &mut engine);
    let view = duel.view.as_ref().unwrap();
    assert!(
        view.object(food).is_none(),
        "the sacrificed token has ceased to exist"
    );
    assert_eq!(view.seats[0].mana_pool.total(), 0);
    let stack = view.stack.last().unwrap();
    assert!(stack.token.is_none(), "the ability itself is not a token");
    assert!(
        matches!(stack.stack_item, Some(baylee_view::StackItem::Ability {
        token: Some(baylee_view::TokenAbility { token: id, index: 0 }), ..
    }) if id == token)
    );
    let model = duel.board.as_ref().unwrap();
    assert_eq!(
        model.stack[0].art,
        Some(ImageKey::token(token, ArtSize::Small))
    );
    assert!(
        model
            .required_images()
            .contains(&ImageKey::token(token, ArtSize::Small))
    );
    assert_eq!(
        crate::cardtext::token_sentence(token, 0).unwrap(),
        baylee_client_core::card_face::split_blocks(
            "{2}, {T}, Sacrifice this token: You gain 3 life."
        )
    );
    let life = view.seats[0].life;
    // A fresh view (as after reconnect) carries everything; no previous client cache.
    let mut fresh = Duel::default();
    sync(&mut fresh, &engine);
    assert_eq!(
        fresh.board.as_ref().unwrap().stack[0].art,
        model.stack[0].art
    );
    settle_stack(&mut engine);
    sync(&mut duel, &engine);
    assert_eq!(duel.view.as_ref().unwrap().seats[0].life, life + 3);
    assert!(duel.last_error.is_none());
}

fn settle_stack(engine: &mut Engine<RegistryLookup>) {
    for _ in 0..20 {
        if engine.state().zones.stack_is_empty() {
            return;
        }
        engine
            .apply(
                engine.pending().asked().unwrap(),
                PlayerAction::PassPriority,
            )
            .unwrap();
    }
    panic!("stack did not settle");
}
