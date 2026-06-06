use std::sync::Arc;

use axum::{
    extract::{Query, State},
    http::header,
    response::{Html, IntoResponse, Redirect, Response},
    routing::get,
    Router,
};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use hmac::{Hmac, Mac};
use leptos::prelude::use_context;
use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::auth::TeamSession;

type HmacSha256 = Hmac<Sha256>;

const DEFAULT_ISSUER: &str = "https://contestdojo.com/api/oidc";
const DEFAULT_API_BASE: &str = "https://api.contestdojo.com";
const SCOPE: &str = "openid profile email read:events";

const TEAM_COOKIE: &str = "team";
const STATE_COOKIE: &str = "oidc_state";
const VERIFIER_COOKIE: &str = "oidc_verifier";

/// Session lifetime for the signed `team` cookie (8 hours — enough for the round).
const SESSION_MAX_AGE_SECS: i64 = 8 * 60 * 60;
/// Short lifetime for the in-flight PKCE/state cookies.
const FLOW_MAX_AGE_SECS: i64 = 10 * 60;

/// OAuth/OIDC configuration, read from the environment at startup. `None` (see
/// [`AuthState`]) means OAuth is not configured and the sign-in routes render a
/// helpful message instead of attempting a flow.
#[derive(Clone)]
pub struct AuthConfig {
    issuer: String,
    api_base: String,
    client_id: String,
    client_secret: String,
    redirect_uri: String,
    event_id: String,
    session_secret: String,
}

/// Shared auth configuration handle provided to both the axum auth routes and
/// the Leptos server functions / websocket handler.
pub type AuthState = Option<Arc<AuthConfig>>;

impl AuthConfig {
    /// Build config from environment variables. Requires `OIDC_CLIENT_ID`,
    /// `OIDC_CLIENT_SECRET`, `OIDC_REDIRECT_URI`, and `CONTESTDOJO_EVENT_ID`.
    /// `OIDC_ISSUER` / `CONTESTDOJO_API_BASE` default to production ContestDojo.
    /// `SESSION_SECRET` signs the session cookie; a random one is generated if
    /// unset (which invalidates existing sessions on restart).
    pub fn from_env() -> AuthState {
        let client_id = std::env::var("OIDC_CLIENT_ID").ok()?;
        let client_secret = std::env::var("OIDC_CLIENT_SECRET").ok()?;
        let redirect_uri = std::env::var("OIDC_REDIRECT_URI").ok()?;
        let event_id = std::env::var("CONTESTDOJO_EVENT_ID").ok()?;

        let session_secret = std::env::var("SESSION_SECRET").unwrap_or_else(|_| {
            leptos::logging::warn!(
                "SESSION_SECRET not set; generating a random one (sessions reset on restart)"
            );
            random_token()
        });

        Some(Arc::new(Self {
            issuer: std::env::var("OIDC_ISSUER").unwrap_or_else(|_| DEFAULT_ISSUER.into()),
            api_base: std::env::var("CONTESTDOJO_API_BASE")
                .unwrap_or_else(|_| DEFAULT_API_BASE.into()),
            client_id,
            client_secret,
            redirect_uri,
            event_id,
            session_secret,
        }))
    }
}

/// The auth routes: `/auth/login`, `/auth/callback`, `/auth/logout`.
pub fn router(state: AuthState) -> Router {
    Router::new()
        .route("/auth/login", get(login))
        .route("/auth/callback", get(callback))
        .route("/auth/logout", get(logout))
        .with_state(state)
}

// --- crypto / encoding helpers ---------------------------------------------

fn random_token() -> String {
    let mut buf = [0u8; 32];
    getrandom::getrandom(&mut buf).expect("system RNG");
    URL_SAFE_NO_PAD.encode(buf)
}

fn pkce_challenge(verifier: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(verifier.as_bytes());
    URL_SAFE_NO_PAD.encode(hasher.finalize())
}

fn sign(secret: &str, payload: &str) -> String {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC accepts any key size");
    mac.update(payload.as_bytes());
    URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes())
}

fn verify(secret: &str, payload: &str, signature: &str) -> bool {
    let Ok(sig) = URL_SAFE_NO_PAD.decode(signature) else {
        return false;
    };
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC accepts any key size");
    mac.update(payload.as_bytes());
    mac.verify_slice(&sig).is_ok()
}

/// Encode a [`TeamSession`] as a tamper-proof `<base64(json)>.<hmac>` string.
fn encode_session(cfg: &AuthConfig, team: &TeamSession) -> String {
    let json = serde_json::to_string(team).expect("TeamSession serializes");
    let payload = URL_SAFE_NO_PAD.encode(json.as_bytes());
    let signature = sign(&cfg.session_secret, &payload);
    format!("{payload}.{signature}")
}

/// Verify and decode a session cookie produced by [`encode_session`].
fn decode_session(cfg: &AuthConfig, value: &str) -> Option<TeamSession> {
    let (payload, signature) = value.split_once('.')?;
    if !verify(&cfg.session_secret, payload, signature) {
        return None;
    }
    let json = URL_SAFE_NO_PAD.decode(payload).ok()?;
    serde_json::from_slice(&json).ok()
}

fn flow_cookie(name: &'static str, value: String) -> Cookie<'static> {
    let mut cookie = Cookie::new(name, value);
    cookie.set_http_only(true);
    cookie.set_same_site(SameSite::Lax);
    cookie.set_secure(true);
    cookie.set_path("/");
    cookie.set_max_age(time::Duration::seconds(FLOW_MAX_AGE_SECS));
    cookie
}

fn removal_cookie(name: &'static str) -> Cookie<'static> {
    let mut cookie = Cookie::new(name, "");
    cookie.set_path("/");
    cookie.set_max_age(time::Duration::seconds(0));
    cookie
}

fn error_page(message: &str) -> Response {
    error_page_with_identity(message, None)
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn error_page_with_identity(message: &str, signed_in_as: Option<&str>) -> Response {
    let identity = match signed_in_as {
        Some(who) => format!(
            "<p style=\"color:#6b7280;font-size:0.9rem\">You're signed in as <strong>{}</strong>. \
             If that's not the account you registered with, sign out of ContestDojo and try again.</p>",
            html_escape(who)
        ),
        None => String::new(),
    };
    Html(format!(
        "<!DOCTYPE html><html><head><meta charset=\"utf-8\"><title>Sign-in error</title>\
         <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\"></head>\
         <body style=\"font-family:system-ui,sans-serif;max-width:32rem;margin:4rem auto;padding:0 1rem;line-height:1.5\">\
         <h1 style=\"font-size:1.25rem\">Could not sign you in</h1>\
         <p>{message}</p>\
         {identity}\
         <p><a href=\"/\">Back to the puzzle round</a></p>\
         </body></html>"
    ))
    .into_response()
}

/// Decode (without verifying) the claims payload of a JWT.
fn decode_jwt_claims<T: serde::de::DeserializeOwned>(jwt: &str) -> Option<T> {
    let payload = jwt.split('.').nth(1)?;
    let bytes = URL_SAFE_NO_PAD.decode(payload).ok()?;
    serde_json::from_slice(&bytes).ok()
}

// --- route handlers ---------------------------------------------------------

async fn login(State(state): State<AuthState>) -> Response {
    let Some(cfg) = state else {
        return error_page("Sign-in is not configured on this server yet.");
    };

    let verifier = random_token();
    let csrf_state = random_token();
    let challenge = pkce_challenge(&verifier);

    let auth_url = format!(
        "{issuer}/auth?client_id={client_id}&response_type=code&redirect_uri={redirect}&scope={scope}&state={state}&code_challenge={challenge}&code_challenge_method=S256",
        issuer = cfg.issuer,
        client_id = urlencoding::encode(&cfg.client_id),
        redirect = urlencoding::encode(&cfg.redirect_uri),
        scope = urlencoding::encode(SCOPE),
        state = urlencoding::encode(&csrf_state),
        challenge = urlencoding::encode(&challenge),
    );

    let jar = CookieJar::new()
        .add(flow_cookie(STATE_COOKIE, csrf_state))
        .add(flow_cookie(VERIFIER_COOKIE, verifier));

    (jar, Redirect::to(&auth_url)).into_response()
}

#[derive(Debug, Deserialize)]
struct CallbackParams {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    #[serde(default)]
    id_token: Option<String>,
}

/// Identity claims embedded in the OIDC `id_token` (scopes `profile`, `email`).
#[derive(Debug, Deserialize)]
struct IdClaims {
    #[serde(default)]
    email: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default, rename = "type")]
    acct_type: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TeamRef {
    id: String,
    name: String,
}

#[derive(Debug, Deserialize)]
struct Registration {
    #[serde(default)]
    fname: Option<String>,
    #[serde(default)]
    lname: Option<String>,
}

#[derive(Debug, Deserialize)]
struct EventEnvelope {
    #[serde(default)]
    team: Option<TeamRef>,
    #[serde(default)]
    registration: Option<Registration>,
}

async fn callback(
    State(state): State<AuthState>,
    jar: CookieJar,
    Query(params): Query<CallbackParams>,
) -> Response {
    let Some(cfg) = state else {
        return error_page("Sign-in is not configured on this server yet.");
    };

    // Clear the in-flight cookies regardless of outcome.
    let clear = |response: Response| {
        (
            CookieJar::new()
                .add(removal_cookie(STATE_COOKIE))
                .add(removal_cookie(VERIFIER_COOKIE)),
            response,
        )
            .into_response()
    };

    if let Some(error) = params.error {
        return clear(error_page(&format!(
            "ContestDojo returned an error: {error}."
        )));
    }

    let (Some(code), Some(returned_state)) = (params.code, params.state) else {
        return clear(error_page("Missing authorization code."));
    };

    let expected_state = jar.get(STATE_COOKIE).map(|c| c.value().to_owned());
    let verifier = jar.get(VERIFIER_COOKIE).map(|c| c.value().to_owned());

    let (Some(expected_state), Some(verifier)) = (expected_state, verifier) else {
        return clear(error_page("Your sign-in session expired. Please try again."));
    };

    if expected_state != returned_state {
        return clear(error_page("State mismatch — please try signing in again."));
    }

    let http = reqwest::Client::new();

    // Step 2: exchange the code for tokens.
    let token: TokenResponse = match http
        .post(format!("{}/token", cfg.issuer))
        .form(&[
            ("grant_type", "authorization_code"),
            ("client_id", cfg.client_id.as_str()),
            ("client_secret", cfg.client_secret.as_str()),
            ("redirect_uri", cfg.redirect_uri.as_str()),
            ("code", code.as_str()),
            ("code_verifier", verifier.as_str()),
        ])
        .send()
        .await
        .and_then(|r| r.error_for_status())
    {
        Ok(resp) => match resp.json().await {
            Ok(token) => token,
            Err(err) => {
                leptos::logging::error!("token decode failed: {err}");
                return clear(error_page("Could not read the token response."));
            }
        },
        Err(err) => {
            leptos::logging::error!("token exchange failed: {err}");
            return clear(error_page("Token exchange with ContestDojo failed."));
        }
    };

    // Identity + account type come from the id_token (ContestDojo embeds
    // `email`, `name`, and `type` there; no UserInfo call is needed).
    let claims = token.id_token.as_deref().and_then(decode_jwt_claims::<IdClaims>);

    let identity: Option<String> = claims
        .as_ref()
        .and_then(|c| c.email.clone().or_else(|| c.name.clone()));

    // Only student accounts may join the puzzle round. Coaches/admins/orgs are
    // rejected up front with a clear message (they also lack a team
    // registration, but this makes the intent explicit). If the type claim is
    // absent we fall through to the team gate below.
    if let Some(acct_type) = claims.as_ref().and_then(|c| c.acct_type.as_deref()) {
        if acct_type != "student" {
            return clear(error_page_with_identity(
                &format!(
                    "Only student accounts can join the puzzle round (your account type is \"{}\"). \
                     Sign in with the student account that is registered on your team.",
                    html_escape(acct_type)
                ),
                identity.as_deref(),
            ));
        }
    }

    // Step 3: read this user's registration for the configured event.
    let resp = match http
        .get(format!(
            "{}/v1alpha1/me/events/{}",
            cfg.api_base, cfg.event_id
        ))
        .bearer_auth(&token.access_token)
        .send()
        .await
    {
        Ok(resp) => resp,
        Err(err) => {
            leptos::logging::error!("events request failed: {err}");
            return clear(error_page("Could not reach the ContestDojo API."));
        }
    };

    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();

    if status == reqwest::StatusCode::NOT_FOUND {
        return clear(error_page_with_identity(
            "You are not registered for this event on ContestDojo.",
            identity.as_deref(),
        ));
    }

    if !status.is_success() {
        leptos::logging::error!("events request returned {status}: {body}");
        return clear(error_page("Could not read your event registration."));
    }

    let envelope: EventEnvelope = match serde_json::from_str(&body) {
        Ok(envelope) => envelope,
        Err(err) => {
            leptos::logging::error!("events decode failed: {err}; body={body}");
            return clear(error_page("Could not read your event registration."));
        }
    };

    let Some(team) = envelope.team else {
        return clear(error_page_with_identity(
            "You are registered, but not assigned to a team yet. Ask your coach to add you to a team, then sign in again.",
            identity.as_deref(),
        ));
    };

    let user_name = envelope.registration.and_then(|r| match (r.fname, r.lname) {
        (Some(f), Some(l)) => Some(format!("{f} {l}")),
        (Some(f), None) => Some(f),
        (None, Some(l)) => Some(l),
        (None, None) => None,
    });

    let session = TeamSession {
        team_id: team.id,
        team_name: team.name,
        user_name,
    };

    let mut session_cookie = Cookie::new(TEAM_COOKIE, encode_session(&cfg, &session));
    session_cookie.set_http_only(true);
    session_cookie.set_same_site(SameSite::Lax);
    session_cookie.set_secure(true);
    session_cookie.set_path("/");
    session_cookie.set_max_age(time::Duration::seconds(SESSION_MAX_AGE_SECS));

    let jar = CookieJar::new()
        .add(removal_cookie(STATE_COOKIE))
        .add(removal_cookie(VERIFIER_COOKIE))
        .add(session_cookie);

    (jar, Redirect::to("/")).into_response()
}

async fn logout() -> Response {
    let jar = CookieJar::new().add(removal_cookie(TEAM_COOKIE));
    (jar, Redirect::to("/")).into_response()
}

// --- shared lookups (used by server fns and the websocket handler) ----------

/// The auth config currently in the Leptos request context, if configured.
pub fn config() -> AuthState {
    use_context::<AuthState>().flatten()
}

fn cookie_value<'a>(cookie_header: &'a str, name: &str) -> Option<&'a str> {
    cookie_header.split(';').find_map(|pair| {
        let (k, v) = pair.trim().split_once('=')?;
        (k == name).then_some(v)
    })
}

/// Resolve the signed-in team from the current request's cookies. Used by the
/// `current_team` server function and the websocket join handler.
pub async fn team_from_request() -> Option<TeamSession> {
    let cfg = config()?;
    let headers = leptos_axum::extract::<header::HeaderMap>().await.ok()?;
    let cookie_header = headers.get(header::COOKIE)?.to_str().ok()?;
    let value = cookie_value(cookie_header, TEAM_COOKIE)?;
    decode_session(&cfg, value)
}
