use super::{Error, Request, Response, api, reply_to};
use api::ranking::{Board, Lane, Listing, Name, Standing};
use uuid::Uuid;

#[test]
fn ranking_transport_rejects_wrong_cohort_lane_scores_ties_and_privacy_ack() {
    let cohort = Uuid::now_v7();
    let request = Request::Ranking {
        cohort,
        lane: Lane::Echoes,
    };
    let headers = format!(
        "Content-Type: application/json\r\nBaylee-Tower-Version: {}\r\n",
        api::VERSION
    );
    let entry = Standing {
        rank: Some(1),
        name: Some(Name {
            display: "Synthetic".into(),
            tag: 4096,
        }),
        wins: 8,
        defeats: 0,
        yours: false,
    };
    let board = Board {
        cohort,
        lane: Lane::Echoes,
        listing: Listing::default(),
        best: None,
        leaders: vec![entry.clone(), entry],
    };
    let exchange = |board| {
        reply_to(
            &request,
            200,
            &headers,
            &serde_json::to_vec(&Response::Ranking { board }).unwrap(),
        )
    };
    assert!(exchange(board.clone()).is_ok());
    let mut wrong = board.clone();
    wrong.cohort = Uuid::now_v7();
    assert!(matches!(exchange(wrong), Err(Error::Rejected)));
    let mut wrong = board.clone();
    wrong.lane = Lane::House;
    assert!(exchange(wrong).is_err());
    let mut wrong = board.clone();
    wrong.leaders[1].rank = Some(2);
    assert!(exchange(wrong).is_err());
    let mut wrong = board.clone();
    wrong.leaders[0].wins = 9;
    assert!(exchange(wrong).is_err());
    let mut wrong = board;
    wrong.leaders[0].yours = true;
    assert!(exchange(wrong).is_err());
    for listing in [
        Listing {
            revision: 1,
            listed: false,
        },
        Listing {
            revision: 2,
            listed: true,
        },
    ] {
        assert!(
            reply_to(
                &Request::Listing {
                    revision: 0,
                    listed: true
                },
                200,
                &headers,
                &serde_json::to_vec(&Response::Listing { listing }).unwrap()
            )
            .is_err()
        );
    }
}
