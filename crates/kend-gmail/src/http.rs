use std::sync::OnceLock;
use std::time::Duration;

use serde::de::DeserializeOwned;
use ureq::Agent;
use ureq::http::Response;

use crate::{Error, Result};

/// One shared HTTP client. Error statuses come back as responses so Google's error body can
/// be read instead of just its status code.
pub(crate) fn agent() -> &'static Agent {
    static AGENT: OnceLock<Agent> = OnceLock::new();
    AGENT.get_or_init(|| {
        Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(30)))
            .http_status_as_error(false)
            .build()
            .into()
    })
}

/// A successful response's JSON body, or Google's error as an [`Error`].
pub(crate) fn json<T: DeserializeOwned>(mut response: Response<ureq::Body>) -> Result<T> {
    if response.status().is_success() {
        return Ok(response.body_mut().read_json()?);
    }
    let status = response.status().as_u16();
    let body = response.body_mut().read_to_string().unwrap_or_default();
    Err(google_error(status, &body))
}

/// OAuth errors look like `{"error": "invalid_grant", ...}`; API errors like
/// `{"error": {"message": ..., "errors": [{"reason": ...}]}}`.
fn google_error(status: u16, body: &str) -> Error {
    let value: serde_json::Value = serde_json::from_str(body).unwrap_or_default();
    let error = &value["error"];
    let reason = error["errors"][0]["reason"].as_str().unwrap_or_default();
    // OAuth endpoint errors are plain strings and keep their own meaning, whatever the status.
    let oauth = error.is_string();
    match status {
        _ if error.as_str() == Some("invalid_grant") => return Error::SignedOut,
        _ if oauth => {}
        401 => return Error::Unauthorized,
        404 => return Error::NotFound,
        429 => return Error::RateLimited,
        403 if reason == "rateLimitExceeded" || reason == "userRateLimitExceeded" => {
            return Error::RateLimited;
        }
        500..=599 => return Error::Server(status),
        _ => {}
    }
    let message = error["message"]
        .as_str()
        .or_else(|| value["error_description"].as_str())
        .or_else(|| error.as_str())
        .unwrap_or("no details");
    Error::Google(format!("{status}: {message}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_invalid_grant_means_signed_out() {
        let err = google_error(
            400,
            r#"{"error":"invalid_grant","error_description":"Token has been expired or revoked."}"#,
        );
        assert!(matches!(err, Error::SignedOut));
    }

    #[test]
    fn statuses_map_to_errors_the_client_can_act_on() {
        assert!(matches!(google_error(401, "{}"), Error::Unauthorized));
        assert!(matches!(google_error(404, "{}"), Error::NotFound));
        assert!(matches!(google_error(429, "{}"), Error::RateLimited));
        assert!(matches!(google_error(503, ""), Error::Server(503)));
        let quota = r#"{"error":{"code":403,"errors":[{"reason":"userRateLimitExceeded"}],"message":"slow down"}}"#;
        assert!(matches!(google_error(403, quota), Error::RateLimited));
        let forbidden = r#"{"error":{"code":403,"errors":[{"reason":"insufficientPermissions"}],"message":"no"}}"#;
        assert!(matches!(google_error(403, forbidden), Error::Google(_)));
    }

    #[test]
    fn oauth_and_api_errors_keep_their_message() {
        let oauth = google_error(
            401,
            r#"{"error":"invalid_client","error_description":"The OAuth client was not found."}"#,
        );
        assert_eq!(
            oauth.to_string(),
            "Google returned an error: 401: The OAuth client was not found."
        );
        let api = google_error(
            403,
            r#"{"error":{"code":403,"message":"Gmail API has not been used"}}"#,
        );
        assert_eq!(
            api.to_string(),
            "Google returned an error: 403: Gmail API has not been used"
        );
    }
}
