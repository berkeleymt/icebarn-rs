use leptos::prelude::*;
use serde::{Deserialize, Serialize};

#[cfg(feature = "ssr")]
pub mod server;

/// The authenticated user's team, resolved from their ContestDojo event
/// registration and persisted in a signed session cookie. This is what the
/// frontend uses to auto-join the team's multiplayer room — replacing the old
/// manual room-password flow.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeamSession {
    /// ContestDojo team id; also the key of the team's multiplayer room.
    pub team_id: String,
    /// Human-readable team name, shown in the lobby.
    pub team_name: String,
    /// The signed-in user's display name, if known.
    #[serde(default)]
    pub user_name: Option<String>,
}

/// Read the signed-in team from the session cookie. Returns `None` when the
/// user is not signed in (or OAuth isn't configured).
#[server]
pub async fn current_team() -> Result<Option<TeamSession>, ServerFnError> {
    Ok(server::team_from_request().await)
}
