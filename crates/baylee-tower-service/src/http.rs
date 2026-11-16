//! Shared native-only headers and value-free HTTP failures.
use axum::{
    body::Body,
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
};
use baylee_tower_api as api;
use central_api::Token;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Failure {
    Invalid,
    Unauthorized,
    Terms,
    Missing,
    Conflict,
    Busy,
    Unavailable,
    Version,
    Oversized,
}

impl From<baylee_tower_store::Error> for Failure {
    fn from(value: baylee_tower_store::Error) -> Self {
        use baylee_tower::Refusal;
        use baylee_tower_store::Error;
        match value {
            Error::NotFound => Self::Missing,
            Error::Conflict
            | Error::Refused(
                Refusal::Stale | Refusal::Unavailable | Refusal::Gold | Refusal::Capacity,
            ) => Self::Conflict,
            Error::Refused(Refusal::Invalid) => Self::Invalid,
            Error::Schema | Error::Unavailable => Self::Unavailable,
        }
    }
}

pub(crate) fn reply(result: Result<Vec<u8>, Failure>) -> Response {
    let mut response = match result {
        Ok(bytes) => ([(header::CONTENT_TYPE, "application/json")], bytes).into_response(),
        Err(failure) => failure.into_response(),
    };
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        header::HeaderValue::from_static("no-store"),
    );
    response.headers_mut().insert(
        "x-content-type-options",
        header::HeaderValue::from_static("nosniff"),
    );
    response.headers_mut().insert(
        api::VERSION_HEADER,
        header::HeaderValue::from_static(api::VERSION),
    );
    response
}

pub(crate) fn credential(headers: &HeaderMap, content_type: &str) -> Result<Token, Failure> {
    if headers.contains_key(header::ORIGIN)
        || headers.contains_key(header::COOKIE)
        || headers.contains_key(header::CONTENT_ENCODING)
    {
        return Err(Failure::Invalid);
    }
    if one(headers, api::VERSION_HEADER) != Some(api::VERSION) {
        return Err(Failure::Version);
    }
    if one(headers, "content-type") != Some(content_type) {
        return Err(Failure::Invalid);
    }
    Token::from_bearer(one(headers, "authorization").ok_or(Failure::Unauthorized)?)
        .map_err(|_| Failure::Unauthorized)
}

fn one<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    let mut values = headers.get_all(name).iter();
    let value = values.next()?.to_str().ok()?;
    values.next().is_none().then_some(value)
}

impl IntoResponse for Failure {
    fn into_response(self) -> Response {
        let status = match self {
            Self::Invalid => StatusCode::BAD_REQUEST,
            Self::Unauthorized => StatusCode::UNAUTHORIZED,
            Self::Terms => StatusCode::FORBIDDEN,
            Self::Missing => StatusCode::NOT_FOUND,
            Self::Conflict => StatusCode::CONFLICT,
            Self::Busy => StatusCode::TOO_MANY_REQUESTS,
            Self::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
            Self::Version => StatusCode::UPGRADE_REQUIRED,
            Self::Oversized => StatusCode::PAYLOAD_TOO_LARGE,
        };
        let mut response = Response::new(Body::empty());
        *response.status_mut() = status;
        if self == Self::Busy {
            response
                .headers_mut()
                .insert(header::RETRY_AFTER, header::HeaderValue::from_static("1"));
        }
        response
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clients_without_token_workshop_support_are_upgraded_before_authentication() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::CONTENT_TYPE,
            header::HeaderValue::from_static("application/json"),
        );
        for version in ["1", "2", "3", "4", "5"] {
            headers.insert(
                api::VERSION_HEADER,
                header::HeaderValue::from_static(version),
            );
            assert!(matches!(
                credential(&headers, "application/json"),
                Err(Failure::Version)
            ));
        }
        let response = reply(Err(Failure::Version));
        assert_eq!(response.status(), StatusCode::UPGRADE_REQUIRED);
        assert_eq!(response.headers()[api::VERSION_HEADER], api::VERSION);
        headers.insert(
            api::VERSION_HEADER,
            header::HeaderValue::from_static(api::VERSION),
        );
        assert!(matches!(
            credential(&headers, "application/json"),
            Err(Failure::Unauthorized)
        ));
    }
}
