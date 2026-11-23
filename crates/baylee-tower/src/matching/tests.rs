use super::*;

fn id(n: u8) -> [u8; 16] {
    let mut id = [n; 16];
    id[6] = 0x70;
    id[8] = 0x80;
    id
}
fn cohort() -> Cohort {
    Cohort {
        rules: 1,
        executor: "test-executor".into(),
        catalog: [9; 32],
    }
}
fn candidate(n: u8) -> Candidate {
    Candidate {
        account: id(n),
        snapshot: id(n + 100),
        stage: 3,
        cohort: cohort(),
        eligible: true,
    }
}

#[test]
fn matches_progress_not_calendar_or_gateway_and_never_its_own_account() {
    let cohort = cohort();
    let criteria = Criteria {
        account: id(1),
        stage: 3,
        cohort: &cohort,
        previous: &[],
    };
    let mut pool = vec![
        candidate(1),
        candidate(2),
        candidate(3),
        candidate(4),
        candidate(5),
    ];
    pool[1].stage = 4;
    pool[2].cohort.rules = 2;
    pool[3].cohort.executor = "different-executor".into();
    pool[4].cohort.catalog = [8; 32];
    assert!(select(&criteria, &pool, &[1; 32]).unwrap().is_none());
    pool.push(candidate(6));
    assert_eq!(
        select(&criteria, &pool, &[1; 32]).unwrap().unwrap().account,
        id(6)
    );
    pool[5].eligible = false;
    assert!(select(&criteria, &pool, &[1; 32]).unwrap().is_none());
}

#[test]
fn reconnect_and_pool_order_preserve_selection_and_recent_opponents_are_avoided() {
    let cohort = cohort();
    let previous = [id(2)];
    let criteria = Criteria {
        account: id(1),
        stage: 3,
        cohort: &cohort,
        previous: &previous,
    };
    let mut pool = vec![candidate(2), candidate(3), candidate(4)];
    for byte in 0..32 {
        let seed = [byte; 32];
        let chosen = select(&criteria, &pool, &seed).unwrap().unwrap().account;
        assert_ne!(chosen, id(2));
        pool.reverse();
        assert_eq!(
            select(&criteria, &pool, &seed).unwrap().unwrap().account,
            chosen
        );
    }
    pool.retain(|c| c.account == id(2));
    assert_eq!(
        select(&criteria, &pool, &[0; 32]).unwrap().unwrap().account,
        id(2)
    );
}

#[test]
fn prolific_contributors_cannot_inflate_weight_and_query_work_is_bounded() {
    let cohort = cohort();
    let criteria = Criteria {
        account: id(1),
        stage: 3,
        cohort: &cohort,
        previous: &[],
    };
    let repeated = vec![candidate(2), candidate(2)];
    assert!(matches!(
        select(&criteria, &repeated, &[1; 32]),
        Err(Refusal::Invalid)
    ));
    let excessive = vec![candidate(2); MAX_CANDIDATES + 1];
    assert!(matches!(
        select(&criteria, &excessive, &[1; 32]),
        Err(Refusal::Invalid)
    ));
    let invalid = Criteria {
        stage: 0,
        ..criteria
    };
    assert!(matches!(
        select(&invalid, &[], &[1; 32]),
        Err(Refusal::Invalid)
    ));
}

/// `count` candidates of as many accounts, all of them compatible.
fn crowd(count: u16) -> Vec<Candidate> {
    (0..count)
        .map(|n| {
            let own = |tag| {
                let mut own = id(tag);
                own[0..2].copy_from_slice(&n.to_be_bytes());
                own
            };
            Candidate {
                account: own(50),
                snapshot: own(60),
                ..candidate(2)
            }
        })
        .collect()
}

fn choose(
    account: [u8; 16],
    stage: u8,
    previous: &[[u8; 16]],
    pool: &[Candidate],
) -> Result<Option<[u8; 16]>, Refusal> {
    let cohort = cohort();
    let criteria = Criteria {
        account,
        stage,
        cohort: &cohort,
        previous,
    };
    select(&criteria, pool, &[1; 32]).map(|found| found.map(|c| c.account))
}

#[test]
fn a_cohort_names_rules_a_catalog_and_an_executor_in_bounded_printable_ascii() {
    let with = |change: fn(&mut Cohort)| {
        let mut changed = cohort();
        change(&mut changed);
        changed.validate()
    };
    assert_eq!(with(|_| {}), Ok(()));
    assert_eq!(with(|c| c.executor = "e".repeat(160)), Ok(()));
    for refused in [
        with(|c| c.rules = 0),
        with(|c| c.executor.clear()),
        with(|c| c.executor = "e".repeat(161)),
        with(|c| c.executor = "two words".into()),
        with(|c| c.executor = "line\n".into()),
        with(|c| c.executor = "\u{e9}xecutor".into()),
        with(|c| c.catalog = [0; 32]),
    ] {
        assert_eq!(refused, Err(Refusal::Invalid));
    }
}

#[test]
fn a_pool_is_read_up_to_its_bound_and_refused_past_it() {
    // All of different accounts: only the bound can refuse them.
    let bound = u16::try_from(MAX_CANDIDATES).unwrap();
    assert!(choose(id(1), 3, &[], &crowd(bound)).unwrap().is_some());
    assert_eq!(
        choose(id(1), 3, &[], &crowd(bound + 1)),
        Err(Refusal::Invalid)
    );
}

#[test]
fn a_search_is_refused_for_an_impossible_stage_or_history() {
    let pool = [candidate(2)];
    for (stage, legal) in [
        (0, false),
        (1, true),
        (MAX_STAGES, true),
        (MAX_STAGES + 1, false),
    ] {
        assert_eq!(choose(id(1), stage, &[], &pool).is_ok(), legal, "{stage}");
    }
    let history = |count| (0..count).map(|n| id(n + 20)).collect::<Vec<_>>();
    let (full, over) = (history(MAX_STAGES), history(MAX_STAGES + 1));
    assert_eq!(choose(id(1), 3, &full, &pool), Ok(Some(id(2))));
    assert_eq!(choose(id(1), 3, &over, &pool), Err(Refusal::Invalid));
}

#[test]
fn every_identity_in_a_search_is_a_version_seven_one() {
    let version_four = {
        let mut other = id(9);
        other[6] = 0x40;
        other
    };
    let no_variant = {
        let mut other = id(9);
        other[8] = 0x00;
        other
    };
    for bad in [version_four, no_variant, [0; 16]] {
        assert_eq!(choose(bad, 3, &[], &[]), Err(Refusal::Invalid));
        let previous = [id(3), bad];
        assert_eq!(choose(id(1), 3, &previous, &[]), Err(Refusal::Invalid));
        let mut account = candidate(2);
        account.account = bad;
        let mut snapshot = candidate(2);
        snapshot.snapshot = bad;
        for mut candidate in [account, snapshot] {
            assert_eq!(
                choose(id(1), 3, &[], std::slice::from_ref(&candidate)),
                Err(Refusal::Invalid)
            );
            // Nobody who could not be chosen anyway spoils the search.
            candidate.eligible = false;
            assert_eq!(choose(id(1), 3, &[], &[candidate]), Ok(None));
        }
    }
}

#[test]
fn a_search_with_an_invalid_cohort_is_refused_before_the_pool_is_read() {
    let mut invalid = cohort();
    invalid.rules = 0;
    let criteria = Criteria {
        account: id(1),
        stage: 3,
        cohort: &invalid,
        previous: &[],
    };
    assert!(matches!(
        select(&criteria, &[], &[1; 32]),
        Err(Refusal::Invalid)
    ));
}
