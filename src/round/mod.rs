use leptos::prelude::*;
use serde::{Deserialize, Serialize};

#[cfg(feature = "ssr")]
pub mod server;

/// Which phase the (global, synchronized) puzzle round is currently in. The
/// same phase applies to every team — there is a single round shared by all.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RoundPhase {
    /// The round has not been started yet. Boards are locked.
    NotStarted,
    /// The round is live. Boards are editable until `end_at_ms` passes.
    Running,
    /// The round is over (time expired or stopped by staff). Boards are locked.
    Ended,
}

/// A snapshot of the global round, sent to clients so they can render the
/// countdown and decide whether boards should be locked.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoundState {
    pub phase: RoundPhase,
    /// Unix epoch milliseconds at which the round ends. Only meaningful while
    /// [`RoundPhase::Running`].
    pub end_at_ms: Option<i64>,
    /// The server's clock (Unix epoch ms) when this snapshot was produced, so
    /// the client can correct for clock skew when counting down.
    pub server_now_ms: i64,
}

impl RoundState {
    /// Whether puzzle boards should be locked (read-only) for solvers. Boards
    /// are only editable while the round is running and time has not yet run
    /// out.
    pub fn locked(&self) -> bool {
        match self.phase {
            RoundPhase::Running => match self.end_at_ms {
                Some(end) => self.server_now_ms >= end,
                None => false,
            },
            RoundPhase::NotStarted | RoundPhase::Ended => true,
        }
    }

    /// Whether the actual puzzles should be shown at all. Before the round
    /// starts solvers may only read the rules and worked examples; the puzzles
    /// themselves stay hidden until staff start the round.
    pub fn show_puzzles(&self) -> bool {
        !matches!(self.phase, RoundPhase::NotStarted)
    }
}

/// An admin-issued change to the global round.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum RoundAction {
    /// Start the round, ending `duration_secs` seconds from now.
    Start { duration_secs: i64 },
    /// Immediately end the round (locks all boards).
    Stop,
    /// Reset back to the not-started state.
    Reset,
}

/// Read the current global round state. Safe to call from anyone.
#[server]
pub async fn get_round() -> Result<RoundState, ServerFnError> {
    let pool = use_context::<sqlx::PgPool>()
        .ok_or_else(|| ServerFnError::new("database unavailable"))?;
    Ok(server::snapshot(&pool).await)
}

/// Apply an admin action to the global round. Requires the admin password.
#[server]
pub async fn set_round(password: String, action: RoundAction) -> Result<RoundState, ServerFnError> {
    let pool = use_context::<sqlx::PgPool>()
        .ok_or_else(|| ServerFnError::new("database unavailable"))?;
    server::set_with_auth(&pool, &password, action).await
}
