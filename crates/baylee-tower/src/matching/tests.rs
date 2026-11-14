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
