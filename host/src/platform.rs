//! The account this `dew.exe` is signed in as, and the deployed
//! marketplace it talks to. Identity is a native email-and-password form
//! calling Supabase's own direct password grant, not a browser-and-PKCE
//! redirect: see `.artifacts/project/decisions/adr-016-supabase-auth-hosted-first.md`'s
//! 2026-09-23 amendment in the `dew-platform` repository for why. A
//! session, once obtained, is the bearer token every request this module
//! makes to the deployed platform carries.
//!
//! `SUPABASE_ANON_KEY` below is not a secret. It is the publishable key
//! meant to ship inside a client, the same way a web app embeds it in
//! JavaScript that reaches every visitor's browser; the thing that keeps
//! an account safe is the password, never this key.

#![cfg(windows)]

use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::time::Duration;

const SUPABASE_URL: &str = "https://yeijewkdjiiixjthinib.supabase.co";
const SUPABASE_ANON_KEY: &str = "sb_publishable_kfzM8P-uVqW6gDlb_7oJCw_Q7aNdLnl";
const DEW_PLATFORM_URL: &str = "https://dew-platform.fly.dev";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub access_token: String,
    pub refresh_token: String,
    pub user_id: String,
    pub email: String,
}

fn session_path() -> Option<PathBuf> {
    Some(crate::installed::dew_dir()?.join("session.json"))
}

pub fn load_session() -> Option<Session> {
    let path = session_path()?;
    let text = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

fn save_session(session: &Session) -> Result<(), String> {
    let path = session_path().ok_or("no home directory to store a session in")?;
    let text = serde_json::to_string_pretty(session).map_err(|e| e.to_string())?;
    std::fs::write(path, text).map_err(|e| e.to_string())
}

pub fn clear_session() {
    if let Some(path) = session_path() {
        let _ = std::fs::remove_file(path);
    }
}

fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(10))
        .build()
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: String,
    user: TokenUser,
}

#[derive(Deserialize)]
struct TokenUser {
    id: String,
    email: String,
}

/// Signs in an existing, already-confirmed account. Supabase's direct
/// password grant, over TLS; the password is never written to disk or to
/// any log this codebase produces, only sent in this one request body.
pub fn login(email: &str, password: &str) -> Result<Session, String> {
    let response = agent()
        .post(&format!("{SUPABASE_URL}/auth/v1/token?grant_type=password"))
        .set("apikey", SUPABASE_ANON_KEY)
        .set("Content-Type", "application/json")
        .send_json(serde_json::json!({ "email": email, "password": password }))
        .map_err(describe_auth_error)?;

    let token: TokenResponse = response.into_json().map_err(|e| e.to_string())?;
    let session = Session {
        access_token: token.access_token,
        refresh_token: token.refresh_token,
        user_id: token.user.id,
        email: token.user.email,
    };
    save_session(&session)?;
    Ok(session)
}

/// Creates a new account. Supabase's project requires email confirmation
/// before a session is issued (checked against the real project, not
/// assumed), so this cannot return a `Session` the way `login` does: the
/// person has to click the link Supabase emails them, then run
/// `dew login` themselves.
pub fn signup(email: &str, password: &str) -> Result<(), String> {
    agent()
        .post(&format!("{SUPABASE_URL}/auth/v1/signup"))
        .set("apikey", SUPABASE_ANON_KEY)
        .set("Content-Type", "application/json")
        .send_json(serde_json::json!({ "email": email, "password": password }))
        .map_err(describe_auth_error)?;
    Ok(())
}

fn describe_auth_error(error: ureq::Error) -> String {
    match error {
        ureq::Error::Status(code, response) => {
            let body = response.into_string().unwrap_or_default();
            if body.trim().is_empty() {
                format!("rejected with status {code}")
            } else {
                format!("{code}: {body}")
            }
        }
        ureq::Error::Transport(transport) => format!("offline or network error: {transport}"),
    }
}

/// One entry in `GET /discover`'s listing: `dew-platform`'s own
/// `PublicPackage`, which deliberately carries no storage key, so nothing
/// this client learns from discovery can be used to reach a package's
/// bytes on its own; `fetch_package` below is the only path to those.
#[derive(Debug, Deserialize)]
pub struct DiscoveredPackage {
    pub owner_user_id: String,
    pub applet_id: String,
    pub uploaded_at: String,
}

fn require_session() -> Result<Session, String> {
    load_session().ok_or_else(|| "not signed in; run `dew login`".to_string())
}

/// Lists every public package across every account. Requires a session
/// the same way every other platform request does, even though the
/// listing itself is not scoped to the caller: `dew-platform` gates every
/// route behind a verified token, discovery included.
pub fn discover() -> Result<Vec<DiscoveredPackage>, String> {
    let session = require_session()?;
    agent()
        .get(&format!("{DEW_PLATFORM_URL}/discover"))
        .set("Authorization", &format!("Bearer {}", session.access_token))
        .call()
        .map_err(describe_auth_error)?
        .into_json()
        .map_err(|e| e.to_string())
}

/// Fetches a public package's raw bytes by its owner and applet id
/// together, exactly the shape `dew-platform`'s own milestone 18 route
/// takes: never by applet id alone, since that still cannot say whose
/// package is meant.
pub fn fetch_package(owner_user_id: &str, applet_id: &str) -> Result<Vec<u8>, String> {
    let session = require_session()?;
    let response = agent()
        .get(&format!(
            "{DEW_PLATFORM_URL}/packages/{owner_user_id}/{applet_id}"
        ))
        .set("Authorization", &format!("Bearer {}", session.access_token))
        .call()
        .map_err(describe_auth_error)?;

    let mut bytes = Vec::new();
    response
        .into_reader()
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    Ok(bytes)
}

/// Fetches one of the CALLER'S OWN packages by applet id alone.
/// `dew-platform` resolves the owner from the bearer token itself for
/// this route, so it is only ever the caller's own published package,
/// never a stranger's -- `dew sync` restores what an account published
/// itself, and it is honest about not yet solving restoring someone
/// else's public package by id alone, the same namespacing question
/// ADR-015 already deferred.
pub fn fetch_own_package(applet_id: &str) -> Result<Vec<u8>, String> {
    let session = require_session()?;
    let response = agent()
        .get(&format!("{DEW_PLATFORM_URL}/packages/{applet_id}"))
        .set("Authorization", &format!("Bearer {}", session.access_token))
        .call()
        .map_err(describe_auth_error)?;

    let mut bytes = Vec::new();
    response
        .into_reader()
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    Ok(bytes)
}

/// One entry in `GET /sync`'s listing. `dew-platform` also returns
/// `added_at`, not read here: nothing this command does needs it.
#[derive(Debug, Deserialize)]
pub struct SyncedApplet {
    pub applet_id: String,
}

/// Lists the signed-in account's synced applet ids.
pub fn list_synced() -> Result<Vec<SyncedApplet>, String> {
    let session = require_session()?;
    agent()
        .get(&format!("{DEW_PLATFORM_URL}/sync"))
        .set("Authorization", &format!("Bearer {}", session.access_token))
        .call()
        .map_err(describe_auth_error)?
        .into_json()
        .map_err(|e| e.to_string())
}

/// Adds an applet id to the signed-in account's synced list. Idempotent
/// on `dew-platform`'s own side, so calling this for an id already
/// synced is a harmless no-op, not an error.
pub fn add_synced(applet_id: &str) -> Result<(), String> {
    let session = require_session()?;
    agent()
        .put(&format!("{DEW_PLATFORM_URL}/sync/{applet_id}"))
        .set("Authorization", &format!("Bearer {}", session.access_token))
        .call()
        .map_err(describe_auth_error)?;
    Ok(())
}

/// Reads an email and a password from the terminal. The password is
/// masked; `rpassword` is this module's only dependency with no protocol
/// or crypto role, present purely for that.
///
/// `rpassword` reads the console device directly on Windows rather than
/// redirected stdin, so a scripted or CI invocation of `dew login` would
/// otherwise hang waiting on a console that is not there. `DEW_LOGIN_EMAIL`
/// and `DEW_LOGIN_PASSWORD`, set together, skip the prompt entirely for
/// exactly that case.
pub fn prompt_credentials() -> Result<(String, String), String> {
    if let (Ok(email), Ok(password)) = (
        std::env::var("DEW_LOGIN_EMAIL"),
        std::env::var("DEW_LOGIN_PASSWORD"),
    ) {
        return Ok((email, password));
    }

    print!("email: ");
    std::io::stdout().flush().map_err(|e| e.to_string())?;
    let mut email = String::new();
    std::io::stdin()
        .read_line(&mut email)
        .map_err(|e| e.to_string())?;

    let password = rpassword::prompt_password("password: ")
        .map_err(|e| format!("failed to read password: {e}"))?;

    Ok((email.trim().to_string(), password))
}
