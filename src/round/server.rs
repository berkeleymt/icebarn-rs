use std::{
    sync::OnceLock,
    time::{SystemTime, UNIX_EPOCH},
};

use leptos::prelude::ServerFnError;
use sqlx::PgPool;
use tokio::sync::Mutex;

use super::{RoundAction, RoundPhase, RoundState};

/// The persisted shape of the round. `end_at_ms` is only set while running.
#[derive(Clone, Copy)]
struct Stored {
    phase: RoundPhase,
    end_at_ms: Option<i64>,
}

impl Default for Stored {
    fn default() -> Self {
        Self {
            phase: RoundPhase::NotStarted,
            end_at_ms: None,
        }
    }
}

/// In-memory cache of the canonical round, shared by the server functions and
/// the realtime websocket handler (same process). `None` until first load from
/// the database.
static CACHE: OnceLock<Mutex<Option<Stored>>> = OnceLock::new();

fn cache() -> &'static Mutex<Option<Stored>> {
    CACHE.get_or_init(|| Mutex::new(None))
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn phase_to_str(phase: RoundPhase) -> &'static str {
    match phase {
        RoundPhase::NotStarted => "not_started",
        RoundPhase::Running => "running",
        RoundPhase::Ended => "ended",
    }
}

fn phase_from_str(s: &str) -> RoundPhase {
    match s {
        "running" => RoundPhase::Running,
        "ended" => RoundPhase::Ended,
        _ => RoundPhase::NotStarted,
    }
}

/// Create the round table if it doesn't exist. Called once at startup.
pub async fn init(pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS round (\
            id INT PRIMARY KEY, \
            phase TEXT NOT NULL, \
            end_at_ms BIGINT\
        )",
    )
    .execute(pool)
    .await?;
    Ok(())
}

async fn load_from_db(pool: &PgPool) -> Stored {
    match sqlx::query_as::<_, (String, Option<i64>)>(
        "SELECT phase, end_at_ms FROM round WHERE id = 1",
    )
    .fetch_optional(pool)
    .await
    {
        Ok(Some((phase, end_at_ms))) => Stored {
            phase: phase_from_str(&phase),
            end_at_ms,
        },
        Ok(None) => Stored::default(),
        Err(err) => {
            leptos::logging::warn!("error loading round state: {err}");
            Stored::default()
        }
    }
}

async fn save_to_db(pool: &PgPool, stored: Stored) {
    if let Err(err) = sqlx::query(
        "INSERT INTO round (id, phase, end_at_ms) VALUES (1, $1, $2) \
         ON CONFLICT (id) DO UPDATE SET phase = $1, end_at_ms = $2",
    )
    .bind(phase_to_str(stored.phase))
    .bind(stored.end_at_ms)
    .execute(pool)
    .await
    {
        leptos::logging::warn!("error saving round state: {err}");
    }
}

/// Read the cached round, loading from the database on first access.
async fn cached(pool: &PgPool) -> Stored {
    let mut guard = cache().lock().await;
    match *guard {
        Some(stored) => stored,
        None => {
            let stored = load_from_db(pool).await;
            *guard = Some(stored);
            stored
        }
    }
}

/// A snapshot of the round for clients, with lazy expiry: a running round whose
/// end time has passed is reported as ended.
pub async fn snapshot(pool: &PgPool) -> RoundState {
    let stored = cached(pool).await;
    let now = now_ms();
    let phase = match (stored.phase, stored.end_at_ms) {
        (RoundPhase::Running, Some(end)) if now >= end => RoundPhase::Ended,
        (phase, _) => phase,
    };
    RoundState {
        phase,
        end_at_ms: stored.end_at_ms,
        server_now_ms: now,
    }
}

/// Whether boards are currently locked. Used by the realtime handler to reject
/// edits server-side once the round is not (or no longer) running.
pub async fn is_locked(pool: &PgPool) -> bool {
    snapshot(pool).await.locked()
}

/// Constant-time string comparison to avoid leaking the admin password length
/// or contents via timing.
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

/// Apply an admin action after verifying the password against `ADMIN_PASSWORD`.
pub async fn set_with_auth(
    pool: &PgPool,
    password: &str,
    action: RoundAction,
) -> Result<RoundState, ServerFnError> {
    let expected = std::env::var("ADMIN_PASSWORD").map_err(|_| {
        ServerFnError::new("Admin controls are not configured on this server (set ADMIN_PASSWORD).")
    })?;

    if expected.is_empty() || !constant_time_eq(password.as_bytes(), expected.as_bytes()) {
        return Err(ServerFnError::new("Incorrect admin password."));
    }

    let now = now_ms();
    let stored = match action {
        RoundAction::Start { duration_secs } => {
            let duration_secs = duration_secs.max(0);
            Stored {
                phase: RoundPhase::Running,
                end_at_ms: Some(now + duration_secs * 1000),
            }
        }
        RoundAction::Stop => Stored {
            phase: RoundPhase::Ended,
            end_at_ms: None,
        },
        RoundAction::Reset => Stored::default(),
    };

    save_to_db(pool, stored).await;
    *cache().lock().await = Some(stored);

    Ok(snapshot(pool).await)
}
