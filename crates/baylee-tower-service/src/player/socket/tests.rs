use super::*;

#[test]
fn starter_invalidations_only_refresh_the_owning_accounts_catalog() {
    let account = Uuid::now_v7();
    let mut watches = vec![Watch {
        request: Request::Cohorts { before: None },
        digest: [0; 32],
        dirty: false,
    }];
    dirty(&mut watches, Change::Draft(Uuid::now_v7()), account);
    assert!(!watches[0].dirty);
    dirty(&mut watches, Change::Draft(account), account);
    assert!(watches[0].dirty);
}
