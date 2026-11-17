use super::*;
use baylee_core::ids::CardIndex;
use baylee_protocol::execution::{Identity, Variant};
use domain::cards::{Card, Catalog, Collection, Stack};

mod formation;
mod ranking;
mod rewards;

#[test]
fn socket_snapshots_never_ack_an_uncertain_purchase_or_rewind_a_run() {
    let (mut state, _, run) = opened();
    let work = state.choose(Uuid::now_v7(), Command::Refresh {}).unwrap();
    assert!(state.receive(&work, None, Some(Error::Unavailable)));
    assert!(state.uncertain());
    let mut fresh = run.clone();
    fresh.revision += 1;
    let response = Response::Run {
        run: Box::new(fresh.clone()),
    };
    assert!(state.push_changes(&response));
    assert!(state.push(response.clone()));
    assert!(
        !state.push_changes(&response),
        "identical push does not rebuild the UI"
    );
    assert!(
        state.uncertain(),
        "a state update is not a mutation receipt"
    );
    assert!(!state.push(Response::Run {
        run: Box::new(run.clone())
    }));
    let mut foreign = fresh.clone();
    foreign.id = Uuid::now_v7();
    foreign.revision += 1;
    assert!(!state.push(Response::Run {
        run: Box::new(foreign)
    }));
    let retry = state.retry(Uuid::now_v7()).unwrap();
    assert_eq!(
        retry.request(),
        work.request(),
        "durable mutation identity survives reconnect"
    );
    state.receive(
        &retry,
        Some(Response::Run {
            run: Box::new(fresh.clone()),
        }),
        None,
    );
    assert!(!state.uncertain());
    assert_eq!(state.run(), Some(&fresh));
}

fn cohort() -> Cohort {
    let mut cards = vec![Card {
        card: CardIndex::new(0),
        price: 0,
        basic_land: true,
        run: None,
    }];
    cards.extend((1..=8).map(|id| Card {
        card: CardIndex::new(id),
        price: 2,
        basic_land: false,
        run: None,
    }));
    let catalog = Catalog { cards };
    let starter = Collection {
        formation: None,
        owned: Vec::new(),
        deck: vec![Stack {
            card: CardIndex::new(0),
            copies: 40,
        }],
    };
    let mut identity = Identity::native(Variant::Tower, "client-test", "synthetic", 21);
    identity.rules = "tower-4".into();
    Cohort {
        id: Uuid::now_v7(),
        contract: domain::v2::package::Cohort::for_generation(identity, &catalog, 3).unwrap(),
        catalog,
        starters: vec![starter.clone(), starter.clone(), starter],
        offered: vec![0, 1, 2],
    }
}

fn opened() -> (State, Cohort, Run) {
    let cohort = cohort();
    let raw =
        domain::v2::run::Run::begin_for(&cohort.catalog, cohort.starters[0].clone(), &[7; 32], 3)
            .unwrap();
    let run = Run::project(Uuid::now_v7(), cohort.id, raw);
    let mut state = State::default();
    let read = state.read(Uuid::now_v7(), Read::Cohorts(None)).unwrap();
    state.receive(
        &read,
        Some(Response::Cohorts {
            consent: 1,
            cohorts: vec![cohort.clone()],
        }),
        None,
    );
    let read = state.read(Uuid::now_v7(), Read::Run(run.id)).unwrap();
    state.receive(
        &read,
        Some(Response::Run {
            run: Box::new(run.clone()),
        }),
        None,
    );
    (state, cohort, run)
}

#[test]
fn a_reserved_or_held_duel_offers_only_retirement_and_a_live_one_nothing() {
    use super::draft::allowed;
    let (_, cohort, shop) = opened();
    for (phase, retire) in [
        (Phase::Reserved, true),
        (Phase::Quarantined, true),
        (Phase::Playing, false),
        (Phase::Verifying, false),
    ] {
        let mut run = shop.clone();
        run.phase = phase;
        assert_eq!(
            allowed(&run, &cohort, &Command::Abandon {}),
            retire,
            "{phase:?}"
        );
        assert!(!allowed(&run, &cohort, &Command::Refresh {}), "{phase:?}");
    }
}

#[test]
fn draft_deck_edits_never_spend_or_enter_with_an_unsaved_or_illegal_deck() {
    use super::draft::{Draft, allowed};
    let (_, cohort, mut run) = opened();
    let mut draft = Draft::default();
    draft.bind(&run);
    assert!(draft.deck_unchanged(Some(&run)));
    draft.card(&run, &cohort, CardIndex::new(1), true);
    assert!(
        draft.deck_unchanged(Some(&run)),
        "unowned nonbasic is not added"
    );
    draft.card(&run, &cohort, CardIndex::new(0), false);
    assert!(!draft.deck_unchanged(Some(&run)));
    assert!(!allowed(
        &run,
        &cohort,
        &Command::Edit {
            deck: draft.deck.clone()
        }
    ));
    draft.bind(&run);
    assert_eq!(
        draft.deck[0].copies, 39,
        "same-revision refresh retains the draft"
    );
    draft.card(&run, &cohort, CardIndex::new(0), true);
    assert!(allowed(
        &run,
        &cohort,
        &Command::Edit {
            deck: draft.deck.clone()
        }
    ));
    for _ in 0..30 {
        draft.card(&run, &cohort, CardIndex::new(0), true);
    }
    assert_eq!(draft.deck[0].copies, 60);
    assert_eq!(
        run.cards.deck[0].copies, 40,
        "draft never replaces confirmed state"
    );
    run.revision += 1;
    draft.bind(&run);
    assert!(draft.deck_unchanged(Some(&run)));
    let mut other = cohort;
    other.id = Uuid::now_v7();
    assert!(!allowed(&run, &other, &Command::Edit { deck: draft.deck }));
}

#[test]
fn draft_full_bag_purchase_fuses_exact_inputs_without_requiring_an_empty_slot() {
    use super::draft::{Draft, allowed};
    use domain::v2::{
        catalog::{Definition, Family, Rarity},
        inventory::Slot,
        shop::Goods,
    };
    let (_, cohort, mut run) = opened();
    run.economy.gold = 100;
    let definition = Definition {
        family: Family::Heartwood,
        rarity: Rarity::Common,
    };
    for slot in (0..3).map(Slot::Active).chain((0..6).map(Slot::Reserve)) {
        run.economy = run.economy.bought(definition, slot).unwrap();
    }
    run.shop.offers[0].goods = Goods::Relic(definition);
    let mut draft = Draft::default();
    draft.bind(&run);
    draft.offer = Some(run.shop.offers[0].id);
    draft.slot = Slot::Active(0);
    assert!(!allowed(&run, &cohort, &draft.purchase(&run).unwrap()));
    draft.ingredient(&run, 1);
    draft.ingredient(&run, 2);
    let command = draft.purchase(&run).unwrap();
    assert!(allowed(&run, &cohort, &command));
    assert_eq!(run.economy.inventory.relics.len(), 9);
    draft.ingredients = vec![1, 1];
    assert!(!allowed(&run, &cohort, &draft.purchase(&run).unwrap()));
    draft.ingredients = vec![1, 2];
    run.shop.offers[0].sold = true;
    assert!(draft.purchase(&run).is_none());
    assert!(!allowed(&run, &cohort, &command));
}

#[test]
fn draft_mythic_requires_exactly_one_rare_and_one_owned_potion() {
    use super::draft::{Draft, allowed};
    use domain::v2::{
        catalog::{Definition, Family, Rarity},
        inventory::Slot,
    };
    let (_, cohort, mut run) = opened();
    run.economy.gold = 100;
    run.economy = run
        .economy
        .bought(
            Definition {
                family: Family::Heartwood,
                rarity: Rarity::Rare,
            },
            Slot::Active(0),
        )
        .unwrap();
    run.economy = run.economy.bought_material().unwrap();
    let mut draft = Draft::default();
    draft.bind(&run);
    draft.slot = Slot::Active(0);
    draft.ingredient(&run, 1);
    assert!(!allowed(
        &run,
        &cohort,
        &Command::Fuse {
            recipe: draft.recipe()
        }
    ));
    draft.material = Some(2);
    let command = Command::Fuse {
        recipe: draft.recipe(),
    };
    assert!(allowed(&run, &cohort, &command));
    let result = run.economy.fused(&draft.recipe()).unwrap();
    assert_eq!(
        result.inventory.relics[0].definition.rarity,
        Rarity::MythicRare
    );
    assert!(result.inventory.materials.is_empty());
    run.phase = Phase::Playing;
    assert!(!allowed(&run, &cohort, &command));
}

#[test]
fn malformed_catalogs_and_inventory_never_reach_the_renderer() {
    let (mut state, mut cohort, mut run) = opened();
    cohort.starters[0].deck[0].copies = u8::MAX;
    let work = state.read(Uuid::now_v7(), Read::Cohort(cohort.id)).unwrap();
    state.receive(
        &work,
        Some(Response::Cohort {
            cohort: Box::new(cohort),
        }),
        None,
    );
    assert_eq!(state.failure(), Some(Error::Unavailable));
    run.economy
        .inventory
        .relics
        .push(domain::v2::inventory::Relic {
            id: 1,
            definition: domain::v2::catalog::Definition {
                family: domain::v2::catalog::Family::Heartwood,
                rarity: domain::v2::catalog::Rarity::Common,
            },
            slot: domain::v2::inventory::Slot::Active(u8::MAX),
            basis: 0,
        });
    let work = state.read(Uuid::now_v7(), Read::Run(run.id)).unwrap();
    state.receive(&work, Some(Response::Run { run: Box::new(run) }), None);
    assert_eq!(state.failure(), Some(Error::Unavailable));
    assert!(state.run().unwrap().economy.inventory.relics.is_empty());
}

#[test]
fn echo_withdrawal_is_explicit_repeatable_and_cannot_resolve_an_ambiguous_purchase() {
    let (mut state, _, run) = opened();
    let first = state.withdraw(Uuid::now_v7()).unwrap();
    assert_eq!(first.request(), Some(&Request::WithdrawEchoes {}));
    assert!(first.play().is_none());
    state.receive(&first, Some(Response::Withdrawn {}), None);
    assert_eq!(state.run(), Some(&run));
    let repeated = state.withdraw(Uuid::now_v7()).unwrap();
    state.receive(&repeated, Some(Response::Withdrawn {}), None);
    let pending = state.choose(Uuid::now_v7(), Command::Refresh {}).unwrap();
    state.receive(&pending, None, Some(Error::Unavailable));
    assert!(state.withdraw(Uuid::now_v7()).is_none());
    assert!(state.uncertain());
}

#[test]
fn lost_mutation_retries_identically_and_an_old_receipt_cannot_rewind_the_shop() {
    let (mut state, cohort, initial) = opened();
    let work = state.choose(Uuid::now_v7(), Command::Refresh {}).unwrap();
    let bytes = serde_json::to_vec(work.request().unwrap()).unwrap();
    assert_eq!(
        state.run(),
        Some(&initial),
        "no optimistic gold, stock or progress"
    );
    assert!(
        state.read(Uuid::now_v7(), Read::Locate).is_none(),
        "one worker operation at a time"
    );
    state.receive(&work, None, Some(Error::Unavailable));
    assert!(state.uncertain());
    assert!(state.choose(Uuid::now_v7(), Command::Refresh {}).is_none());
    assert!(state.start(Uuid::now_v7(), cohort.id, 0, 1).is_none());
    let raw =
        domain::v2::run::Run::begin_for(&cohort.catalog, cohort.starters[0].clone(), &[7; 32], 3)
            .unwrap();
    let once = raw
        .command(0, &Command::Refresh {}, &cohort.catalog, &[8; 32])
        .unwrap();
    let twice = once
        .command(1, &Command::Refresh {}, &cohort.catalog, &[9; 32])
        .unwrap();
    let advanced = Run::project(initial.id, cohort.id, twice);
    let read = state.read(Uuid::now_v7(), Read::Run(initial.id)).unwrap();
    state.receive(
        &read,
        Some(Response::Run {
            run: Box::new(advanced.clone()),
        }),
        None,
    );
    assert!(
        state.uncertain(),
        "a newer read cannot identify which mutation committed"
    );
    let retry = state.retry(Uuid::now_v7()).unwrap();
    assert_ne!(retry.id, work.id);
    assert_eq!(serde_json::to_vec(retry.request().unwrap()).unwrap(), bytes);
    assert!(
        !state.receive(&work, None, Some(Error::Conflict)),
        "late old worker ignored"
    );
    assert!(state.busy());
    state.receive(
        &retry,
        Some(Response::Run {
            run: Box::new(Run::project(initial.id, cohort.id, once)),
        }),
        None,
    );
    assert!(!state.uncertain());
    assert_eq!(
        state.run(),
        Some(&advanced),
        "the original receipt is acknowledged without rollback"
    );
    let next = state.choose(Uuid::now_v7(), Command::Refresh {}).unwrap();
    assert!(matches!(
        next.request(),
        Some(Request::Command { revision: 2, .. })
    ));
}

#[test]
fn malformed_success_or_ambiguous_rejection_keeps_the_original_mutation() {
    let (mut state, _, initial) = opened();
    let work = state.choose(Uuid::now_v7(), Command::Refresh {}).unwrap();
    let mut wrong = initial.clone();
    wrong.id = Uuid::now_v7();
    state.receive(
        &work,
        Some(Response::Run {
            run: Box::new(wrong),
        }),
        None,
    );
    assert_eq!(state.failure(), Some(Error::Unavailable));
    assert!(state.uncertain());
    assert_eq!(state.run(), Some(&initial));
    let retry = state.retry(Uuid::now_v7()).unwrap();
    state.receive(&retry, None, Some(Error::Rejected));
    assert!(
        state.uncertain(),
        "Rejected also represents malformed transport responses"
    );
    let retry = state.retry(Uuid::now_v7()).unwrap();
    state.receive(&retry, None, Some(Error::Conflict));
    assert!(
        !state.uncertain(),
        "an explicit authority conflict permits a fresh read/choice"
    );
    assert_eq!(state.run(), Some(&initial));
}

#[test]
fn a_confirmed_reservation_survives_failed_admission_without_another_reservation() {
    let (mut state, _, initial) = opened();
    assert!(state.play(Uuid::now_v7()).is_none());
    assert!(state.reserve(Uuid::now_v7(), Uuid::now_v7()).is_none());
    let host = Host {
        id: Uuid::now_v7(),
        name: "Approved".into(),
        origin: "https://approved.invalid".into(),
    };
    let read = state.read(Uuid::now_v7(), Read::Hosts(None)).unwrap();
    state.receive(
        &read,
        Some(Response::Hosts {
            run: initial.id,
            hosts: vec![host.clone()],
        }),
        None,
    );
    let reserve = state.reserve(Uuid::now_v7(), host.id).unwrap();
    assert_eq!(reserve.play(), Some(initial.id));
    let mut reserved = initial;
    reserved.phase = Phase::Reserved;
    reserved.revision += 1;
    state.receive(
        &reserve,
        Some(Response::Run {
            run: Box::new(reserved.clone()),
        }),
        Some(Error::Unavailable),
    );
    assert!(
        !state.uncertain(),
        "reservation ACK and subsequent admission are distinct"
    );
    assert_eq!(state.run(), Some(&reserved));
    assert!(
        state.hosts().is_empty(),
        "changed phase invalidates old free-capacity hints"
    );
    assert!(state.choose(Uuid::now_v7(), Command::Refresh {}).is_none());
    let play = state.play(Uuid::now_v7()).unwrap();
    assert!(play.request().is_none(), "resume never rerolls an opponent");
    state.receive(&play, None, Some(Error::Unavailable));
    assert_eq!(state.run(), Some(&reserved));
    reserved.phase = Phase::Quarantined;
    reserved.revision += 1;
    let read = state.read(Uuid::now_v7(), Read::Run(reserved.id)).unwrap();
    state.receive(
        &read,
        Some(Response::Run {
            run: Box::new(reserved.clone()),
        }),
        None,
    );
    assert!(state.play(Uuid::now_v7()).is_none());
    assert!(state.choose(Uuid::now_v7(), Command::Refresh {}).is_none());
    assert_eq!(
        state.run(),
        Some(&reserved),
        "technical quarantine is not a defeat"
    );
}

#[test]
fn enrollment_is_explicit_and_a_departed_worker_cannot_populate_a_new_account() {
    let (mut state, cohort, initial) = opened();
    assert!(state.start(Uuid::now_v7(), cohort.id, 0, 0).is_none());
    assert!(state.start(Uuid::now_v7(), cohort.id, 3, 1).is_none());
    assert!(state.start(Uuid::now_v7(), Uuid::now_v7(), 0, 1).is_none());
    let enrolling = state.start(Uuid::now_v7(), cohort.id, 1, 1).unwrap();
    assert!(enrolling.play().is_none(), "enrollment only opens the shop");
    let mut other = State::default();
    let current = other.read(Uuid::now_v7(), Read::Cohorts(None)).unwrap();
    assert!(!other.receive(
        &enrolling,
        Some(Response::Run {
            run: Box::new(initial)
        }),
        None
    ));
    assert!(other.run().is_none() && other.busy());
    other.receive(
        &current,
        Some(Response::Cohorts {
            consent: 1,
            cohorts: Vec::new(),
        }),
        None,
    );
    assert!(!other.busy());
    assert!(other.run().is_none());
}

#[test]
fn a_retained_run_uses_its_frozen_catalog_after_enrollment_closes() {
    let (mut state, cohort, run) = opened();
    let read = state.read(Uuid::now_v7(), Read::Cohorts(None)).unwrap();
    state.receive(
        &read,
        Some(Response::Cohorts {
            consent: 1,
            cohorts: Vec::new(),
        }),
        None,
    );
    assert!(state.cohorts().is_empty());
    assert!(state.publication().is_none());
    assert_eq!(state.run(), Some(&run));
    let read = state.read(Uuid::now_v7(), Read::Cohort(cohort.id)).unwrap();
    state.receive(
        &read,
        Some(Response::Cohort {
            cohort: Box::new(cohort.clone()),
        }),
        None,
    );
    assert_eq!(state.publication(), Some(&cohort));
    assert!(
        state.start(Uuid::now_v7(), cohort.id, 0, 1).is_none(),
        "archive lookup cannot reopen enrollment"
    );
    assert!(
        state.choose(Uuid::now_v7(), Command::Refresh {}).is_some(),
        "the retained run can still use its shop"
    );
}

#[test]
fn enrolling_inserts_the_new_run_into_the_owned_runs_list() {
    let (mut state, cohort, initial) = opened();
    let mut fresh = initial.clone();
    fresh.id = Uuid::now_v7();
    let enrolling = state.start(Uuid::now_v7(), cohort.id, 1, 1).unwrap();
    assert!(state.receive(
        &enrolling,
        Some(Response::Run {
            run: Box::new(fresh.clone()),
        }),
        None,
    ));
    assert_eq!(state.run(), Some(&fresh));
    assert_eq!(state.runs().first().map(|r| r.id), Some(fresh.id));
    assert!(state.runs().iter().any(|r| r.id == initial.id));
}
