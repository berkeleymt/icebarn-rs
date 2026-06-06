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
use openidconnect::{
    core::{CoreAuthenticationFlow, CoreClient, CoreProviderMetadata},
    AuthorizationCode, ClientId, ClientSecret, CsrfToken, IssuerUrl, Nonce,
    OAuth2TokenResponse, PkceCodeChallenge, PkceCodeVerifier, RedirectUrl, Scope,
};
use serde::Deserialize;
use sha2::Sha256;

use crate::auth::TeamSession;

type HmacSha256 = Hmac<Sha256>;

const DEFAULT_ISSUER: &str = "https://contestdojo.com/api/oidc";
const DEFAULT_API_BASE: &str = "https://api.contestdojo.com";

const TEAM_COOKIE: &str = "team";
const STATE_COOKIE: &str = "oidc_state";
const VERIFIER_COOKIE: &str = "oidc_verifier";
const NONCE_COOKIE: &str = "oidc_nonce";

const SESSION_MAX_AGE_SECS: i64 = 8 * 60 * 60;
const FLOW_MAX_AGE_SECS: i64 = 10 * 60;

/// OIDC configuration. Provider metadata (JWKS, endpoints) is discovered at
/// startup; the client is rebuilt per-request (cheap struct construction) so
/// we avoid needing to name the openidconnect typestate generics.
#[derive(Clone)]
pub struct AuthConfig {
    metadata: CoreProviderMetadata,
    http_client: reqwest::Client,
    client_id: ClientId,
    client_secret: ClientSecret,
    redirect_uri: RedirectUrl,
    api_base: String,
    event_id: String,
    session_secret: String,
}

pub type AuthState = Option<Arc<AuthConfig>>;

impl AuthConfig {
    /// Discover OIDC provider metadata and build config from env vars.
    /// Returns `None` if required vars are missing (OAuth disabled).
    pub async fn from_env() -> AuthState {
        let client_id = std::env::var("OIDC_CLIENT_ID").ok()?;
        let client_secret = std::env::var("OIDC_CLIENT_SECRET").ok()?;
        let redirect_uri = std::env::var("OIDC_REDIRECT_URI").ok()?;
        let event_id = std::env::var("CONTESTDOJO_EVENT_ID").ok()?;
        let issuer = std::env::var("OIDC_ISSUER").unwrap_or_else(|_| DEFAULT_ISSUER.into());

        let session_secret = std::env::var("SESSION_SECRET").unwrap_or_else(|_| {
            leptos::logging::warn!("SESSION_SECRET not set; sessions will reset on restart");
            let mut buf = [0u8; 32];
            getrandom::getrandom(&mut buf).expect("system RNG");
            URL_SAFE_NO_PAD.encode(buf)
        });

        let http_client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .ok()?;

        let issuer_url = IssuerUrl::new(issuer).ok()?;
        let metadata = CoreProviderMetadata::discover_async(issuer_url, &http_client)
            .await
            .map_err(|e| leptos::logging::error!("OIDC discovery failed: {e}"))
            .ok()?;

        Some(Arc::new(Self {
            metadata,
            http_client,
            client_id: ClientId::new(client_id),
            client_secret: ClientSecret::new(client_secret),
            redirect_uri: RedirectUrl::new(redirect_uri).ok()?,
            api_base: std::env::var("CONTESTDOJO_API_BASE")
                .unwrap_or_else(|_| DEFAULT_API_BASE.into()),
            event_id,
            session_secret,
        }))
    }
}

pub fn router(state: AuthState) -> Router {
    Router::new()
        .route("/auth/login", get(login))
        .route("/auth/callback", get(callback))
        .route("/auth/logout", get(logout))
        .with_state(state)
}

// --- session cookie (HMAC-SHA256 signed, app-specific) ----------------------

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

fn encode_session(cfg: &AuthConfig, team: &TeamSession) -> String {
    let json = serde_json::to_string(team).expect("TeamSession serializes");
    let payload = URL_SAFE_NO_PAD.encode(json.as_bytes());
    let signature = sign(&cfg.session_secret, &payload);
    format!("{payload}.{signature}")
}

fn decode_session(cfg: &AuthConfig, value: &str) -> Option<TeamSession> {
    let (payload, signature) = value.split_once('.')?;
    if !verify(&cfg.session_secret, payload, signature) {
        return None;
    }
    let json = URL_SAFE_NO_PAD.decode(payload).ok()?;
    serde_json::from_slice(&json).ok()
}

// --- cookie + HTML helpers --------------------------------------------------

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

// --- route handlers ---------------------------------------------------------

async fn login(State(state): State<AuthState>) -> Response {
    let Some(cfg) = state else {
        return error_page("Sign-in is not configured on this server yet.");
    };

    let client = CoreClient::from_provider_metadata(
        cfg.metadata.clone(),
        cfg.client_id.clone(),
        Some(cfg.client_secret.clone()),
    )
    .set_redirect_uri(cfg.redirect_uri.clone());

    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

    let (auth_url, csrf_state, nonce) = client
        .authorize_url(
            CoreAuthenticationFlow::AuthorizationCode,
            CsrfToken::new_random,
            Nonce::new_random,
        )
        .add_scope(Scope::new("profile".to_string()))
        .add_scope(Scope::new("email".to_string()))
        .add_scope(Scope::new("read:events".to_string()))
        .set_pkce_challenge(pkce_challenge)
        .url();

    let jar = CookieJar::new()
        .add(flow_cookie(STATE_COOKIE, csrf_state.secret().clone()))
        .add(flow_cookie(VERIFIER_COOKIE, pkce_verifier.secret().clone()))
        .add(flow_cookie(NONCE_COOKIE, nonce.secret().clone()));

    (jar, Redirect::to(auth_url.as_str())).into_response()
}

#[derive(Debug, Deserialize)]
struct CallbackParams {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
}

/// ContestDojo UserInfo; we only need the custom `type` field (standard claims
/// come from the verified id_token).
#[derive(Deserialize)]
struct UserInfo {
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

    let clear = |response: Response| {
        (
            CookieJar::new()
                .add(removal_cookie(STATE_COOKIE))
                .add(removal_cookie(VERIFIER_COOKIE))
                .add(removal_cookie(NONCE_COOKIE)),
            response,
        )
            .into_response()
    };

    if let Some(error) = params.error {
        return clear(error_page(&format!(
            "ContestDojo returned an error: {}.",
            html_escape(&error)
        )));
    }

    let (Some(code), Some(returned_state)) = (params.code, params.state) else {
        return clear(error_page("Missing authorization code."));
    };

    let expected_state = jar.get(STATE_COOKIE).map(|c| c.value().to_owned());
    let verifier = jar.get(VERIFIER_COOKIE).map(|c| c.value().to_owned());
    let nonce = jar.get(NONCE_COOKIE).map(|c| c.value().to_owned());

    let (Some(expected_state), Some(verifier), Some(nonce)) =
        (expected_state, verifier, nonce)
    else {
        return clear(error_page(
            "Your sign-in session expired. Please try again.",
        ));
    };

    if expected_state != returned_state {
        return clear(error_page(
            "State mismatch — please try signing in again.",
        ));
    }

    let client = CoreClient::from_provider_metadata(
        cfg.metadata.clone(),
        cfg.client_id.clone(),
        Some(cfg.client_secret.clone()),
    )
    .set_redirect_uri(cfg.redirect_uri.clone());

    // Exchange the authorization code for tokens (PKCE + client_secret_post).
    let exchange = match client.exchange_code(AuthorizationCode::new(code)) {
        Ok(req) => req,
        Err(err) => {
            leptos::logging::error!("token endpoint not configured: {err}");
            return clear(error_page("Sign-in is misconfigured on this server."));
        }
    };
    let token_response = match exchange
        .set_pkce_verifier(PkceCodeVerifier::new(verifier))
        .request_async(&cfg.http_client)
        .await
    {
        Ok(resp) => resp,
        Err(err) => {
            leptos::logging::error!("token exchange failed: {err}");
            return clear(error_page("Token exchange with ContestDojo failed."));
        }
    };

    // Verify the id_token: RS256 signature via JWKS, plus iss/aud/exp/nonce.
    let id_token = match token_response.extra_fields().id_token() {
        Some(t) => t,
        None => return clear(error_page("No ID token in the response.")),
    };

    let claims = match id_token.claims(&client.id_token_verifier(), &Nonce::new(nonce)) {
        Ok(c) => c,
        Err(err) => {
            leptos::logging::error!("id_token verification failed: {err}");
            return clear(error_page("Could not verify your identity token."));
        }
    };

    let identity: Option<String> = claims
        .email()
        .map(|e| e.to_string())
        .or_else(|| {
            claims
                .name()
                .and_then(|n| n.get(None))
                .map(|n| n.to_string())
        });

    // The `type` claim (student/coach/admin) is ContestDojo-specific. Fetch it
    // from the UserInfo endpoint, already authenticated via the access token.
    let acct_type: Option<String> = match cfg.metadata.userinfo_endpoint() {
        Some(url) => {
            match cfg
                .http_client
                .get(url.url().as_str())
                .bearer_auth(token_response.access_token().secret())
                .send()
                .await
            {
                Ok(r) => r.json::<UserInfo>().await.ok().and_then(|u| u.acct_type),
                Err(_) => None,
            }
        }
        None => None,
    };

    // Student-only gate.
    if let Some(ref t) = acct_type {
        if t != "student" {
            return clear(error_page_with_identity(
                &format!(
                    "Only student accounts can join the puzzle round \
                     (your account type is \"{}\"). Sign in with the student \
                     account that is registered on your team.",
                    html_escape(t)
                ),
                identity.as_deref(),
            ));
        }
    }

    // Read this user's registration for the configured event.
    let resp = match cfg
        .http_client
        .get(format!(
            "{}/v1alpha1/me/events/{}",
            cfg.api_base, cfg.event_id
        ))
        .bearer_auth(token_response.access_token().secret())
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
            "You are registered, but not assigned to a team yet. \
             Ask your coach to add you to a team, then sign in again.",
            identity.as_deref(),
        ));
    };

    let user_name = envelope
        .registration
        .and_then(|r| match (r.fname, r.lname) {
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
        .add(removal_cookie(NONCE_COOKIE))
        .add(session_cookie);

    (jar, Redirect::to("/")).into_response()
}

async fn logout() -> Response {
    let jar = CookieJar::new().add(removal_cookie(TEAM_COOKIE));
    (jar, Redirect::to("/")).into_response()
}

// --- shared lookups (used by server fns and the websocket handler) ----------

pub fn config() -> AuthState {
    use_context::<AuthState>().flatten()
}

fn cookie_value<'a>(cookie_header: &'a str, name: &str) -> Option<&'a str> {
    cookie_header.split(';').find_map(|pair| {
        let (k, v) = pair.trim().split_once('=')?;
        (k == name).then_some(v)
    })
}

pub async fn team_from_request() -> Option<TeamSession> {
    let cfg = config()?;
    let headers = leptos_axum::extract::<header::HeaderMap>().await.ok()?;
    let cookie_header = headers.get(header::COOKIE)?.to_str().ok()?;
    let value = cookie_value(cookie_header, TEAM_COOKIE)?;
    decode_session(&cfg, value)
}
