use super::{Error, Refusal, StoreError, VerificationFailure, backoff, discovery, failure, fatal};
use std::time::Duration;

#[test]
fn one_job_s_refused_settlement_is_quarantined_not_raised() {
    for (error, reason) in [
        (
            Error::Store(StoreError::Refused(Refusal::Capacity)),
            VerificationFailure::Budget,
        ),
        (
            Error::Store(StoreError::Refused(Refusal::Invalid)),
            VerificationFailure::Protocol,
        ),
        (
            Error::Store(StoreError::Conflict),
            VerificationFailure::Protocol,
        ),
        (Error::Record, VerificationFailure::InvalidRecord),
        (Error::Process, VerificationFailure::Worker),
    ] {
        assert_eq!(failure(error), Ok(Some(reason)), "{error:?}");
    }
    // A lost claim belongs to the newer generation; nothing is quarantined.
    assert_eq!(
        failure(Error::Store(StoreError::Refused(Refusal::Stale))),
        Ok(None)
    );
    // A passing outage is retried later rather than quarantining valid evidence.
    assert_eq!(
        failure(Error::Store(StoreError::Unavailable)),
        Err(Error::Store(StoreError::Unavailable))
    );
}

#[test]
fn only_configuration_and_schema_stop_the_service() {
    assert!(fatal(Error::Contract));
    assert!(fatal(Error::Store(StoreError::Schema)));
    assert!(fatal(discovery(StoreError::NotFound)));
    for error in [
        Error::Store(StoreError::Unavailable),
        Error::Store(StoreError::NotFound),
        Error::Store(StoreError::Conflict),
        Error::Store(StoreError::Refused(Refusal::Capacity)),
        Error::Process,
        Error::Budget,
        discovery(StoreError::Unavailable),
    ] {
        assert!(!fatal(error), "{error:?}");
    }
}

#[test]
fn faults_back_off_doubling_to_a_minute() {
    let pauses: Vec<_> = [1, 2, 3, 6, 7, 8, u32::MAX].map(backoff).into();
    assert_eq!(
        pauses,
        [1, 2, 4, 32, 60, 60, 60].map(Duration::from_secs).to_vec()
    );
}
