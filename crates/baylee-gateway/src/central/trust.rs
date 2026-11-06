//! What this gateway holds from Baylee Central so that it need not ask: the
//! keys Central signs with and what Central has revoked (`docs/central-login.md`
//! §"Sessions"). The rules of a token are `central_crypto::jwt`'s; this keeps the
//! state they are checked against.
//!
//! Nothing here is written down. After a start it takes no sign-in until it has
//! keys and one finished catch-up, and Central holds every revocation longer
//! than a token lives, so that catch-up leaves out nothing still valid.
#![forbid(unsafe_code)]

use central_crypto::jwt::{
    self, Expected, GatewayClaims, JwtError, KeySet, ReplayCache, RevocationClaims,
};
use parking_lot::Mutex;
use std::{collections::HashMap, sync::Arc};

/// How often the keys are fetched with nothing asking for them.
const KEYS_EVERY_SECS: i64 = 300;
/// How soon after one fetch another may go, so that a token naming an unknown
/// `kid` cannot be used to make this gateway hammer Central.
const KEYS_AT_MOST_SECS: i64 = 60;
/// How long a revoked `sid` is remembered: a token may be presented after its
/// revocation, and lives this long at most.
const REVOKED_KEEP_SECS: i64 = jwt::GATEWAY_TOKEN_SECONDS + jwt::LEEWAY_SECONDS;
/// How many pushed revocations are remembered, for refusing one sent twice.
const REPLAY_CAPACITY: usize = 4096;

/// Why a token or a message was not taken. The peer hears one fixed refusal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Refused {
    /// No keys yet, or no catch-up finished since this process started.
    NotReady,
    /// It names a key this gateway does not hold: worth a fetch.
    UnknownKey,
    /// Refused by the token rules.
    Token,
    /// Its session was revoked.
    Revoked,
    /// A pushed message that was already taken.
    Replayed,
    /// Too many pushed messages are live at once: Central should try later.
    Full,
}

/// What a revocation message came to.
pub(crate) struct Applied {
    /// The sessions it ended.
    pub(crate) sids: Vec<String>,
    /// It starts past what was applied, so something is missing.
    pub(crate) gap: bool,
}

struct State {
    keys: Option<Arc<KeySet>>,
    fetched_at: Option<i64>,
    asked_at: Option<i64>,
    synced: bool,
    applied: u64,
    /// Each revoked `sid` with when it was learned. Only Central's signed
    /// messages add to it, and [`Trust::sweep`] empties it by age.
    revoked: HashMap<String, i64>,
    replay: ReplayCache,
}

/// See the module.
pub(crate) struct Trust {
    issuer: String,
    /// This gateway's origins: one, or the official gateway's with its alias.
    audiences: Vec<String>,
    state: Mutex<State>,
}

impl Trust {
    pub(crate) fn new(issuer: String, audiences: Vec<String>) -> Self {
        Self {
            issuer,
            audiences,
            state: Mutex::new(State {
                keys: None,
                fetched_at: None,
                asked_at: None,
                synced: false,
                applied: 0,
                revoked: HashMap::new(),
                replay: ReplayCache::new(REPLAY_CAPACITY),
            }),
        }
    }

    /// Takes the keys Central published. A failed fetch calls nothing, so the
    /// keys held stay in use.
    pub(crate) fn set_keys(&self, keys: KeySet, now: i64) {
        let mut state = self.state.lock();
        state.keys = Some(Arc::new(keys));
        state.fetched_at = Some(now);
    }

    /// Whether a fetch may go now, and if so counts it as gone: at most once
    /// a minute, whatever asks. `unknown_key` asks ahead of the five minutes.
    pub(crate) fn may_fetch_keys(&self, now: i64, unknown_key: bool) -> bool {
        let mut state = self.state.lock();
        if state
            .asked_at
            .is_some_and(|asked| now < asked + KEYS_AT_MOST_SECS)
        {
            return false;
        }
        let due = state
            .fetched_at
            .is_none_or(|fetched| now >= fetched + KEYS_EVERY_SECS);
        if due || unknown_key {
            state.asked_at = Some(now);
        }
        due || unknown_key
    }

    /// The first catch-up since this process started has finished.
    pub(crate) fn caught_up(&self) {
        self.state.lock().synced = true;
    }

    /// The number up to which every revocation is applied.
    pub(crate) fn applied(&self) -> u64 {
        self.state.lock().applied
    }

    /// Whether any keys are held, without which no message can be read.
    pub(crate) fn has_keys(&self) -> bool {
        self.state.lock().keys.is_some()
    }

    /// Whether this gateway may take a sign-in: keys held and caught up once.
    pub(crate) fn is_ready(&self) -> bool {
        let state = self.state.lock();
        state.keys.is_some() && state.synced
    }

    /// Moves past numbers Central no longer holds, never backwards.
    pub(crate) fn skip_to(&self, number: u64) {
        let mut state = self.state.lock();
        state.applied = state.applied.max(number);
    }

    fn verified<C: jwt::Claims>(
        &self,
        keys: &KeySet,
        token: &str,
        now: i64,
    ) -> Result<C, JwtError> {
        let mut refused = JwtError::Audience;
        for audience in &self.audiences {
            let expected = Expected {
                issuer: &self.issuer,
                audience,
                now,
            };
            match jwt::verify::<C>(token, keys, expected) {
                Err(JwtError::Audience) => {}
                Err(other) => refused = other,
                done @ Ok(_) => return done,
            }
        }
        Err(refused)
    }

    /// A gateway token this gateway may act on, at sign-in and refresh.
    ///
    /// # Errors
    /// [`Refused`], and nothing to act on.
    pub(crate) fn gateway_token(&self, token: &str, now: i64) -> Result<GatewayClaims, Refused> {
        let keys = {
            let state = self.state.lock();
            state.keys.clone().filter(|_| state.synced)
        };
        let keys = keys.ok_or(Refused::NotReady)?;
        // Outside the lock, as below: a flood of tokens to verify must not
        // stand between a revocation and the sessions it ends.
        let claims: GatewayClaims = self.verified(&keys, token, now).map_err(refused)?;
        if self.revoked(&claims.sid) {
            return Err(Refused::Revoked);
        }
        Ok(claims)
    }

    fn held_keys(&self) -> Result<Arc<KeySet>, Refused> {
        self.state.lock().keys.clone().ok_or(Refused::NotReady)
    }

    /// Whether this `sid` was revoked.
    pub(crate) fn revoked(&self, sid: &str) -> bool {
        self.state.lock().revoked.contains_key(sid)
    }

    /// A revocation message from a catch-up, which this gateway asked for.
    ///
    /// # Errors
    /// [`Refused`], and nothing is applied.
    pub(crate) fn fetched(&self, token: &str, now: i64) -> Result<Applied, Refused> {
        let keys = self.held_keys()?;
        let claims: RevocationClaims = self.verified(&keys, token, now).map_err(refused)?;
        Ok(apply(&mut self.state.lock(), claims, now))
    }

    /// A revocation message pushed to this gateway, which anyone can send:
    /// what is not Central's, or was taken before, applies nothing.
    ///
    /// # Errors
    /// [`Refused`], and nothing is applied.
    pub(crate) fn pushed(&self, token: &str, now: i64) -> Result<Applied, Refused> {
        let keys = self.held_keys()?;
        let claims: RevocationClaims = self.verified(&keys, token, now).map_err(refused)?;
        let mut state = self.state.lock();
        state.replay.take(&claims, now).map_err(refused)?;
        Ok(apply(&mut state, claims, now))
    }

    /// Forgets the revoked `sid`s no token can still name.
    pub(crate) fn sweep(&self, now: i64) {
        self.state
            .lock()
            .revoked
            .retain(|_, learned| now < *learned + REVOKED_KEEP_SECS);
    }
}

const fn refused(error: JwtError) -> Refused {
    match error {
        JwtError::UnknownKey => Refused::UnknownKey,
        JwtError::Replayed => Refused::Replayed,
        JwtError::Full => Refused::Full,
        _ => Refused::Token,
    }
}

/// A revocation can only end sessions, so its `sid`s apply whatever its numbers
/// say; the applied number moves only when nothing lies between.
fn apply(state: &mut State, claims: RevocationClaims, now: i64) -> Applied {
    for sid in &claims.sids {
        state.revoked.insert(sid.clone(), now);
    }
    let gap = claims.from > state.applied.saturating_add(1);
    if !gap {
        state.applied = state.applied.max(claims.seq);
    }
    Applied {
        sids: claims.sids,
        gap,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use central_crypto::jwt::{Purpose, SigningKey};
    use zeroize::Zeroizing;

    const ISSUER: &str = "https://central.example";
    const HERE: &str = "https://gateway.example";
    const ALIAS: &str = "https://old.example";
    const NOW: i64 = 1_800_000_000;
    const SID: &str = "0123456789abcdef0123456789abcdef";

    fn key(seed: u8) -> SigningKey {
        SigningKey::from_seed(&Zeroizing::new([seed; 32]))
    }

    fn keys() -> KeySet {
        KeySet::new(vec![key(1).public().clone()]).unwrap()
    }

    fn trust() -> Trust {
        let trust = Trust::new(ISSUER.into(), vec![HERE.into(), ALIAS.into()]);
        trust.set_keys(keys(), NOW);
        trust.caught_up();
        trust
    }

    fn gateway_claims() -> GatewayClaims {
        GatewayClaims {
            iss: ISSUER.into(),
            aud: HERE.into(),
            sub: "018f0000-0000-7000-8000-000000000001".into(),
            sid: SID.into(),
            jti: "0123456789abcdef0123456789abcde0".into(),
            iat: NOW,
            nbf: NOW,
            exp: NOW + 3600,
            purpose: Purpose::Admit,
            name: None,
            tag: None,
            legacy_id: None,
            terms: None,
        }
    }

    fn revocation(jti: char, from: u64, seq: u64) -> RevocationClaims {
        RevocationClaims {
            iss: ISSUER.into(),
            aud: HERE.into(),
            jti: jti.to_string().repeat(32),
            iat: NOW,
            nbf: NOW,
            exp: NOW + 60,
            from,
            seq,
            // One `sid` for each number, the first of them the session the
            // tests watch.
            sids: std::iter::once(SID.to_owned())
                .chain((from..seq).map(|n| format!("{n:032x}")))
                .collect(),
        }
    }

    fn signed<C: jwt::Claims>(claims: &C) -> String {
        key(1).sign(claims).unwrap().to_string()
    }

    fn refusal(trust: &Trust, claims: &GatewayClaims) -> Option<Refused> {
        trust.gateway_token(&signed(claims), NOW).err()
    }

    #[test]
    fn a_good_token_is_taken_at_either_origin_of_this_gateway() {
        let trust = trust();
        assert_eq!(refusal(&trust, &gateway_claims()), None);
        let mut alias = gateway_claims();
        alias.aud = ALIAS.into();
        assert_eq!(refusal(&trust, &alias), None);
    }

    #[test]
    fn a_token_for_another_gateway_is_refused() {
        let mut claims = gateway_claims();
        claims.aud = "https://other.example".into();
        assert_eq!(refusal(&trust(), &claims), Some(Refused::Token));
    }

    #[test]
    fn a_token_from_another_issuer_is_refused() {
        let mut claims = gateway_claims();
        claims.iss = "https://central.evil.example".into();
        assert_eq!(refusal(&trust(), &claims), Some(Refused::Token));
    }

    #[test]
    fn an_expired_token_is_refused_past_the_tolerance_and_not_before() {
        let mut claims = gateway_claims();
        claims.iat = NOW - 3600;
        claims.nbf = NOW - 3600;
        claims.exp = NOW - jwt::LEEWAY_SECONDS - 1;
        assert_eq!(refusal(&trust(), &claims), Some(Refused::Token));
        claims.exp = NOW - jwt::LEEWAY_SECONDS + 1;
        assert_eq!(refusal(&trust(), &claims), None);
    }

    #[test]
    fn a_token_not_yet_valid_is_refused() {
        let mut claims = gateway_claims();
        claims.iat = NOW + 600;
        claims.nbf = NOW + 600;
        claims.exp = NOW + 1200;
        assert_eq!(refusal(&trust(), &claims), Some(Refused::Token));
    }

    #[test]
    fn a_token_under_a_key_not_held_asks_for_keys() {
        let foreign = key(2).sign(&gateway_claims()).unwrap().to_string();
        assert_eq!(
            trust().gateway_token(&foreign, NOW).err(),
            Some(Refused::UnknownKey)
        );
    }

    #[test]
    fn a_token_naming_a_held_key_with_another_signature_is_refused() {
        let token = signed(&gateway_claims());
        let (body, signature) = token.rsplit_once('.').unwrap();
        let mut bytes = signature.as_bytes().to_vec();
        bytes[0] = if bytes[0] == b'A' { b'B' } else { b'A' };
        let forged = format!("{body}.{}", String::from_utf8(bytes).unwrap());
        assert_eq!(trust().gateway_token(&forged, NOW).err(), Some(Refused::Token));
    }

    #[test]
    fn another_algorithm_is_refused_whatever_the_token_says() {
        let token = signed(&gateway_claims());
        let (_, rest) = token.split_once('.').unwrap();
        let none = "eyJhbGciOiJub25lIiwidHlwIjoiYmF5bGVlLWdhdGV3YXkrand0In0";
        let forged = format!("{none}.{rest}");
        assert_eq!(trust().gateway_token(&forged, NOW).err(), Some(Refused::Token));
    }

    #[test]
    fn a_revocation_message_is_not_a_gateway_token() {
        let token = signed(&revocation('a', 1, 1));
        assert_eq!(trust().gateway_token(&token, NOW).err(), Some(Refused::Token));
    }

    #[test]
    fn a_gateway_token_is_not_a_revocation_message() {
        let token = signed(&gateway_claims());
        assert_eq!(trust().pushed(&token, NOW).err(), Some(Refused::Token));
    }

    #[test]
    fn nothing_is_taken_before_the_first_catch_up_has_finished() {
        let starting = Trust::new(ISSUER.into(), vec![HERE.into()]);
        let token = signed(&gateway_claims());
        assert_eq!(starting.gateway_token(&token, NOW).err(), Some(Refused::NotReady));
        starting.set_keys(keys(), NOW);
        assert_eq!(starting.gateway_token(&token, NOW).err(), Some(Refused::NotReady));
        starting.caught_up();
        assert!(starting.gateway_token(&token, NOW).is_ok());
    }

    #[test]
    fn a_revoked_session_stays_refused_when_its_token_comes_back() {
        let trust = trust();
        trust.pushed(&signed(&revocation('a', 1, 1)), NOW).ok().unwrap();
        assert_eq!(refusal(&trust, &gateway_claims()), Some(Refused::Revoked));
    }

    #[test]
    fn a_revoked_sid_is_kept_as_long_as_a_token_can_name_it() {
        let trust = trust();
        trust.pushed(&signed(&revocation('a', 1, 1)), NOW).ok().unwrap();
        trust.sweep(NOW + REVOKED_KEEP_SECS - 1);
        assert_eq!(trust.state.lock().revoked.len(), 1);
        trust.sweep(NOW + REVOKED_KEEP_SECS);
        assert!(trust.state.lock().revoked.is_empty());
    }

    #[test]
    fn a_pushed_message_sent_twice_is_refused_the_second_time() {
        let trust = trust();
        let token = signed(&revocation('a', 1, 1));
        assert!(trust.pushed(&token, NOW).is_ok());
        assert_eq!(trust.pushed(&token, NOW).err(), Some(Refused::Replayed));
    }

    #[test]
    fn a_message_from_a_stranger_applies_nothing() {
        let trust = trust();
        let forged = key(2).sign(&revocation('a', 1, 1)).unwrap().to_string();
        assert_eq!(trust.pushed(&forged, NOW).err(), Some(Refused::UnknownKey));
        assert!(trust.state.lock().revoked.is_empty());
        assert_eq!(trust.applied(), 0);
    }

    #[test]
    fn a_message_for_another_gateway_applies_nothing() {
        let trust = trust();
        let mut claims = revocation('a', 1, 1);
        claims.aud = "https://other.example".into();
        assert_eq!(trust.pushed(&signed(&claims), NOW).err(), Some(Refused::Token));
        assert!(trust.state.lock().revoked.is_empty());
    }

    #[test]
    fn a_gap_still_applies_its_sessions_and_holds_the_number_back() {
        let trust = trust();
        let applied = trust.pushed(&signed(&revocation('a', 5, 5)), NOW).ok().unwrap();
        assert!(applied.gap);
        assert_eq!(applied.sids, vec![SID.to_owned()]);
        assert_eq!(trust.applied(), 0, "1 to 4 are still missing");
        assert_eq!(refusal(&trust, &gateway_claims()), Some(Refused::Revoked));
    }

    #[test]
    fn numbers_move_with_a_page_and_never_back() {
        let trust = trust();
        assert!(!trust.fetched(&signed(&revocation('a', 1, 3)), NOW).ok().unwrap().gap);
        assert_eq!(trust.applied(), 3);
        assert!(!trust.fetched(&signed(&revocation('b', 2, 2)), NOW).ok().unwrap().gap);
        assert_eq!(trust.applied(), 3);
        assert!(!trust.fetched(&signed(&revocation('c', 4, 9)), NOW).ok().unwrap().gap);
        assert_eq!(trust.applied(), 9);
    }

    #[test]
    fn numbers_central_no_longer_holds_are_passed_and_never_gone_back_to() {
        let trust = trust();
        trust.skip_to(7);
        assert_eq!(trust.applied(), 7);
        trust.skip_to(3);
        assert_eq!(trust.applied(), 7);
        assert!(!trust.fetched(&signed(&revocation('a', 8, 8)), NOW).ok().unwrap().gap);
        assert_eq!(trust.applied(), 8);
    }

    #[test]
    fn a_gateway_is_ready_only_with_keys_and_a_finished_catch_up() {
        let starting = Trust::new(ISSUER.into(), vec![HERE.into()]);
        assert!(!starting.has_keys() && !starting.is_ready());
        starting.set_keys(keys(), NOW);
        assert!(starting.has_keys() && !starting.is_ready());
        starting.caught_up();
        assert!(starting.is_ready());
    }

    #[test]
    fn keys_are_fetched_on_time_and_an_unknown_kid_cannot_hammer_central() {
        let trust = trust();
        assert!(!trust.may_fetch_keys(NOW + 10, false), "not due yet");
        assert!(trust.may_fetch_keys(NOW + 10, true));
        assert!(!trust.may_fetch_keys(NOW + 11, true), "once a minute");
        assert!(!trust.may_fetch_keys(NOW + 69, true));
        assert!(trust.may_fetch_keys(NOW + 70, true));
        assert!(trust.may_fetch_keys(NOW + KEYS_EVERY_SECS, false));
    }
}
