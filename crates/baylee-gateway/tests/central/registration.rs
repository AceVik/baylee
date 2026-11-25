//! The website's registration proof, using the real Argon2id solver.
use baylee_account::wire::{Request, Response};
use central_crypto::proof::{self, Work};

pub(super) fn request(
    call: impl FnOnce(&Request) -> Response,
    username: &str,
    display_name: Option<&str>,
) -> Request {
    let Response::RegisterWork {
        nonce,
        parts,
        difficulty,
        memory,
        passes,
        ..
    } = call(&Request::RegisterChallengeFor { signals: 0 })
    else {
        panic!("registration offers its memory-bound proof");
    };
    let counters = proof::solve(
        &nonce,
        Work {
            memory,
            passes,
            parts,
            difficulty,
        },
    )
    .unwrap();
    Request::Register {
        username: username.into(),
        display_name: display_name.map(str::to_owned),
        nonce,
        counters,
        invite_code: None,
    }
}
