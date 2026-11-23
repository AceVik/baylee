use super::*;
use crate::stock::{Error, Report, periods, reconcile};
use central_api::challenges::{HostRequest, HostResponse};

const NO_WORK: fn(Period) -> Result<Publication, Error> =
    |_| panic!("nothing may be generated from this inventory");

/// Runs a pass whose inventory is `tasks` and which may publish nothing.
fn with_inventory(tasks: Vec<Challenge>) -> Result<Report, Error> {
    let mut tasks = Some(tasks);
    reconcile(
        MONDAY,
        1,
        |request| match request {
            HostRequest::Inventory { .. } => Ok(HostResponse::Inventory {
                challenges: tasks.take().expect("asked once"),
            }),
            _ => panic!("nothing may be published from this inventory"),
        },
        NO_WORK,
    )
}

fn certified(period: Period) -> Publication {
    publish(&package(), period, "Certified")
}

/// Runs a pass over an empty calendar, letting `answer` reply to each publication.
fn with_answer(answer: impl Fn(&Publication) -> HostResponse) -> Result<Report, Error> {
    reconcile(
        MONDAY,
        1,
        |request| match request {
            HostRequest::Inventory { .. } => Ok(HostResponse::Inventory {
                challenges: Vec::new(),
            }),
            HostRequest::Publish { publication } => Ok(answer(&publication)),
            _ => panic!("unexpected request"),
        },
        |period| Ok(certified(period)),
    )
}

#[test]
fn an_empty_calendar_is_filled_once_and_found_full_on_the_next_pass() {
    let mut store: Vec<Challenge> = Vec::new();
    let mut asked = Vec::new();
    let mut pass = |store: &mut Vec<Challenge>| {
        reconcile(
            MONDAY,
            2,
            |request| match request {
                HostRequest::Inventory { from, until } => {
                    asked.push((from, until));
                    Ok(HostResponse::Inventory {
                        challenges: store.clone(),
                    })
                }
                HostRequest::Publish { publication } => {
                    let challenge = held(&publication, u8::try_from(store.len()).unwrap());
                    store.push(challenge.clone());
                    Ok(HostResponse::Challenge { challenge })
                }
                _ => panic!("unexpected request"),
            },
            |period| Ok(certified(period)),
        )
        .unwrap()
    };
    let first = pass(&mut store);
    assert_eq!(first.present, 0);
    assert_eq!(first.published, 3, "two days and one week");
    let second = pass(&mut store);
    assert_eq!(second.present, 3);
    assert_eq!(second.published, 0);
    assert_eq!(asked, [(MONDAY, MONDAY + DAY + 1); 2]);
}

#[test]
fn a_withdrawn_task_keeps_its_slot_occupied() {
    let mut tasks: Vec<_> = [Kind::Daily, Kind::Weekly]
        .into_iter()
        .zip(1..)
        .map(|(kind, n)| held(&certified(slot(kind, MONDAY)), n))
        .collect();
    tasks[0].withdrawn = true;
    let report = with_inventory(tasks).unwrap();
    assert_eq!((report.present, report.published), (2, 0));
}

#[test]
fn the_daily_task_comes_before_the_weekly_one_of_the_same_moment() {
    let slots = periods(MONDAY, 8).unwrap();
    let kinds: Vec<_> = slots
        .iter()
        .filter(|slot| slot.starts_at == MONDAY || slot.starts_at == MONDAY + 7 * DAY)
        .map(|slot| slot.kind)
        .collect();
    assert_eq!(
        kinds,
        [Kind::Daily, Kind::Weekly, Kind::Daily, Kind::Weekly]
    );
}

#[test]
fn the_horizon_ends_before_the_slot_that_starts_on_it() {
    let count = |now, days, kind| {
        let slots = periods(now, days).unwrap();
        slots.iter().filter(|slot| slot.kind == kind).count()
    };
    assert_eq!(count(MONDAY, 1, Kind::Daily), 1);
    assert_eq!(count(MONDAY + 1, 1, Kind::Daily), 2);
    assert_eq!(count(MONDAY, 7, Kind::Weekly), 1);
    assert_eq!(count(MONDAY, 8, Kind::Weekly), 2);
    assert_eq!(count(MONDAY, 24, Kind::Daily), 24);
    assert!(periods(MONDAY, 25).is_err());
    assert!(periods(-MONDAY, 1).is_err());
}

#[test]
fn an_inventory_that_contradicts_itself_publishes_nothing() {
    let daily = held(&certified(slot(Kind::Daily, MONDAY)), 1);
    let weekly = held(&certified(slot(Kind::Weekly, MONDAY)), 2);
    assert!(with_inventory(vec![daily.clone(), weekly.clone()]).is_ok());

    let mut twice = daily.clone();
    twice.id = id(3);
    let same_slot = vec![daily.clone(), weekly.clone(), twice];
    assert!(matches!(with_inventory(same_slot), Err(Error::Invalid)));

    let mut reused = weekly;
    reused.id = daily.id;
    let same_id = vec![daily.clone(), reused];
    assert!(matches!(with_inventory(same_id), Err(Error::Invalid)));

    let mut blank = daily;
    blank.description.title = words(" ");
    assert!(matches!(with_inventory(vec![blank]), Err(Error::Invalid)));
}

#[test]
fn an_inventory_outside_the_window_asked_for_is_refused() {
    for at in [MONDAY - DAY, MONDAY + DAY] {
        let stray = held(&certified(slot(Kind::Daily, at)), 1);
        assert!(
            matches!(with_inventory(vec![stray]), Err(Error::Invalid)),
            "{at}"
        );
    }
}

#[test]
fn an_inventory_answered_with_something_else_is_refused() {
    let challenge = held(&certified(slot(Kind::Daily, MONDAY)), 1);
    let result = reconcile(
        MONDAY,
        1,
        |_| {
            Ok(HostResponse::Challenge {
                challenge: challenge.clone(),
            })
        },
        NO_WORK,
    );
    assert!(matches!(result, Err(Error::Invalid)));
}

#[test]
fn a_candidate_for_another_slot_or_without_proof_is_never_sent() {
    let pass = |prepare: fn(Period) -> Publication| {
        reconcile(
            MONDAY,
            1,
            |request| match request {
                HostRequest::Inventory { .. } => Ok(HostResponse::Inventory {
                    challenges: Vec::new(),
                }),
                _ => panic!("this candidate may not be published"),
            },
            |period| Ok(prepare(period)),
        )
    };
    let elsewhere = pass(|_| certified(slot(Kind::Daily, MONDAY + DAY)));
    assert!(matches!(elsewhere, Err(Error::Invalid)));
    let unproven = pass(|period| {
        let mut publication = certified(period);
        publication.certificate.witnesses.clear();
        sign(&mut publication);
        publication
    });
    assert!(matches!(unproven, Err(Error::Uncertified)));
}

#[test]
fn only_an_exact_confirmation_counts_as_published() {
    let confirm = |change: fn(&mut Challenge)| {
        with_answer(move |publication| {
            let mut challenge = held(publication, 1);
            change(&mut challenge);
            HostResponse::Challenge { challenge }
        })
    };
    assert_eq!(
        confirm(|_| ()).unwrap(),
        Report {
            present: 0,
            published: 2
        }
    );
    let changes: [fn(&mut Challenge); 3] = [
        |c| c.withdrawn = true,
        |c| c.digest[0] ^= 1,
        |c| c.description.target_turn += 1,
    ];
    for (at, change) in changes.into_iter().enumerate() {
        assert!(matches!(confirm(change), Err(Error::Invalid)), "{at}");
    }
    let shrug = with_answer(|_| HostResponse::Inventory {
        challenges: Vec::new(),
    });
    assert!(matches!(shrug, Err(Error::Invalid)));
}
