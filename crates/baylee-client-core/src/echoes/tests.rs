use super::*;
use baylee_core::{ids::CardIndex, preset::AIProfile};
use baylee_protocol::execution::{Identity, Variant};
use domain::cards::{Card, Catalog, Collection, Stack};

fn cohort() -> Cohort {
    let mut cards = vec![Card {
        card: CardIndex::new(0),
        price: 0,
        basic_land: true,
    }];
    cards.extend((1..=8).map(|id| Card {
        card: CardIndex::new(id),
        price: 2,
        basic_land: false,
    }));
    let catalog = Catalog { cards };
    let starter = Collection {
        owned: Vec::new(),
        deck: vec![Stack {
            card: CardIndex::new(0),
            copies: 40,
        }],
    };
    Cohort {
        id: Uuid::now_v7(),
        contract: domain::v2::package::Cohort::new(
            Identity::native(
                Variant::Tower,
                "client-test",
                "synthetic",
                u32::from(AIProfile::GENERATION),
            ),
            &catalog,
        )
        .unwrap(),
        catalog,
        starters: [starter.clone(), starter.clone(), starter],
    }
}

fn opened() -> (State, Cohort, Run) {
    let cohort = cohort();
    let raw =
        domain::v2::run::Run::begin(&cohort.catalog, cohort.starters[0].clone(), &[7; 32]).unwrap();
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
        domain::v2::run::Run::begin(&cohort.catalog, cohort.starters[0].clone(), &[7; 32]).unwrap();
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
