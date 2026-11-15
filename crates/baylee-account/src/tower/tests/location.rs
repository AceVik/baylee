use super::{Error, Request, Response, api, reply_to, validate_location};
use uuid::Uuid;

#[test]
fn tower_location_is_bound_to_the_requested_run_and_transport_environment() {
    let run = Uuid::now_v7();
    let valid = api::Location {
        run,
        revision: 4,
        phase: api::Phase::Playing,
        host: Some(api::Host {
            id: Uuid::now_v7(),
            name: "Approved gateway".into(),
            origin: "http://127.0.0.1:19822".into(),
        }),
        game: Some(Uuid::now_v7()),
    };
    let response = |location: api::Location| {
        reply_to(
            &Request::Locate { run },
            200,
            "Content-Type: application/json\r\nBaylee-Tower-Version: 2\r\n",
            &serde_json::to_vec(&Response::Location { location }).unwrap(),
        )
    };
    assert!(
        matches!(response(valid.clone()), Ok(Response::Location { location }) if location == valid)
    );
    let mut foreign = valid.clone();
    foreign.run = Uuid::now_v7();
    assert!(matches!(response(foreign), Err(Error::Rejected)));
    let mut redirect = valid.clone();
    redirect.host.as_mut().unwrap().origin = "https://different-environment.invalid".into();
    assert!(matches!(response(redirect), Err(Error::Rejected)));
    for origin in [
        "http://localhost:19822",
        "http://127.0.0.1:19822/path",
        "http://secret@127.0.0.1:19822",
        "http://127.0.0.1:19822?secret=1",
    ] {
        let mut changed = valid.clone();
        changed.host.as_mut().unwrap().origin = origin.into();
        assert!(validate_location(run, &changed, true).is_err());
    }
    let mut production = valid.clone();
    production.host.as_mut().unwrap().origin = "https://approved.invalid".into();
    assert!(validate_location(run, &production, false).is_ok());
    assert!(validate_location(run, &valid, false).is_err());
    for phase in [
        api::Phase::Shop,
        api::Phase::Reserved,
        api::Phase::Verifying,
        api::Phase::Quarantined,
    ] {
        let mut changed = valid.clone();
        changed.phase = phase;
        assert!(validate_location(run, &changed, true).is_err());
    }
    let mut hidden = valid;
    hidden.host = None;
    assert!(validate_location(run, &hidden, true).is_err());
    hidden.game = None;
    assert!(validate_location(run, &hidden, true).is_ok());
}
