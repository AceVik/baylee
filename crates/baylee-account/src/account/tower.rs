use super::{Account, Error};
use crate::TowerHttp;
use baylee_tower_api::{Host, Location, Phase, Request, Response};
use uuid::Uuid;

/// Confirmed hosted Tower handover. Credentials stay separate and are never printable.
pub struct TowerPlay {
    /// The requested owned run.
    pub run: Uuid,
    /// Fresh approved destination, confirmed again after gateway admission/start.
    pub host: Host,
    /// Gateway-local login, never a Central or Tower bearer.
    pub admission: crate::Admission,
    /// Current human execution seat.
    pub seat: crate::TowerSeat,
}

impl Account {
    /// Open the distinct Tower origin delegated by Central. No gateway chooses it
    /// or receives this credential. Refresh this handle after its five-minute expiry.
    /// Like all account methods, run on the client's worker, never the render thread.
    /// # Errors
    /// Missing restoration/terms, unavailable service or refused identity delegation.
    pub fn tower(&self) -> Result<TowerHttp, Error> {
        let grant = self.session.tower_token(&self.http)?;
        TowerHttp::new(self.http.endpoint(), self.session.identity().account, grant)
    }

    /// Start/resume only the current global assignment, including another gateway.
    /// The caller supplies no destination. Central issues a separate gateway-audience
    /// admission, then a fresh authority read binds the returned seat to this run.
    /// Reuse this account's open Tower handle so starting a game does not rotate its
    /// shop credential. Run on the account worker; retry an ambiguous failure with
    /// the same run, refreshing the handle only when its access needs renewal.
    /// # Errors
    /// Missing/foreign assignment, changed execution, terms or transport refusal.
    pub fn play_tower(&self, tower: &TowerHttp, run: Uuid) -> Result<TowerPlay, Error> {
        if !tower.belongs_to(self.http.endpoint(), self.session.identity().account) {
            return Err(Error::Rejected);
        }
        let before = locate(tower, run)?;
        let selected = before.host.ok_or(Error::Conflict)?;
        let (gateway, token) = self.gateway_token(&selected.origin)?;
        let admission = gateway
            .admit(&token)
            .map_err(|turned| gateway_refusal(turned.error))?;
        let seat = gateway
            .tower_play(&admission.token, run)
            .map_err(gateway_refusal)?;
        let after = locate(tower, run)?;
        let host = confirmed(&selected, &after, seat.game_id)?;
        Ok(TowerPlay {
            run,
            host,
            admission,
            seat,
        })
    }
}

fn gateway_refusal(error: Error) -> Error {
    // A remote gateway cannot demand a Central terms/sign-out flow. Only Central
    // (or its introspected Tower scope) can identify that account requirement.
    match error {
        Error::TermsRequired => Error::Rejected,
        other => other,
    }
}

fn locate(tower: &TowerHttp, run: Uuid) -> Result<Location, Error> {
    let Response::Location { location } = tower.exchange(&Request::Locate { run })? else {
        return Err(Error::Rejected);
    };
    Ok(location)
}

fn confirmed(selected: &Host, location: &Location, game: Uuid) -> Result<Host, Error> {
    let host = location.host.as_ref().ok_or(Error::Conflict)?;
    if location.phase != Phase::Playing
        || location.game != Some(game)
        || host.id != selected.id
        || host.origin != selected.origin
    {
        return Err(Error::Conflict);
    }
    Ok(host.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_gateway_seat_cannot_outlive_or_redirect_the_fresh_authority_assignment() {
        let host = Host {
            id: Uuid::now_v7(),
            name: "Approved".into(),
            origin: "https://approved.invalid".into(),
        };
        let game = Uuid::now_v7();
        let valid = Location {
            run: Uuid::now_v7(),
            revision: 4,
            phase: Phase::Playing,
            host: Some(host.clone()),
            game: Some(game),
        };
        assert_eq!(confirmed(&host, &valid, game).unwrap(), host);
        assert!(confirmed(&host, &valid, Uuid::now_v7()).is_err());
        let mut changed = valid.clone();
        changed.host.as_mut().unwrap().id = Uuid::now_v7();
        assert!(confirmed(&host, &changed, game).is_err());
        changed = valid.clone();
        changed.host.as_mut().unwrap().origin = "https://different.invalid".into();
        assert!(confirmed(&host, &changed, game).is_err());
        changed = valid.clone();
        changed.phase = Phase::Verifying;
        assert!(confirmed(&host, &changed, game).is_err());
        changed = valid;
        changed.host = None;
        assert!(confirmed(&host, &changed, game).is_err());
    }
}
