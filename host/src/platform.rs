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
    let ciphertext = std::fs::read(path).ok()?;
    let plaintext = dpapi_unprotect(&ciphertext).ok()?;
    serde_json::from_slice(&plaintext).ok()
}

pub(crate) fn save_session(session: &Session) -> Result<(), String> {
    let path = session_path().ok_or("no home directory to store a session in")?;
    let plaintext = serde_json::to_vec(session).map_err(|e| e.to_string())?;
    let ciphertext = dpapi_protect(&plaintext)?;
    std::fs::write(path, ciphertext).map_err(|e| e.to_string())
}

/// Encrypts with DPAPI, scoped to the current Windows user by default
/// (no extra entropy needed, the same guarantee Windows' own Credential
/// Manager relies on): only a process running as this same user can ever
/// decrypt it back, so `session.json`'s bytes on disk are useless to
/// anything that is not.
fn dpapi_protect(plaintext: &[u8]) -> Result<Vec<u8>, String> {
    use windows::Win32::Foundation::LocalFree;
    use windows::Win32::Security::Cryptography::{CryptProtectData, CRYPT_INTEGER_BLOB};

    unsafe {
        let input = CRYPT_INTEGER_BLOB {
            cbData: plaintext.len() as u32,
            pbData: plaintext.as_ptr() as *mut u8,
        };
        let mut output = CRYPT_INTEGER_BLOB::default();

        CryptProtectData(&input, None, None, None, None, 0, &mut output)
            .map_err(|e| format!("failed to encrypt session: {e}"))?;

        let bytes = std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec();
        let _ = LocalFree(Some(windows::Win32::Foundation::HLOCAL(
            output.pbData as *mut _,
        )));
        Ok(bytes)
    }
}

fn dpapi_unprotect(ciphertext: &[u8]) -> Result<Vec<u8>, String> {
    use windows::Win32::Foundation::LocalFree;
    use windows::Win32::Security::Cryptography::{CryptUnprotectData, CRYPT_INTEGER_BLOB};

    unsafe {
        let input = CRYPT_INTEGER_BLOB {
            cbData: ciphertext.len() as u32,
            pbData: ciphertext.as_ptr() as *mut u8,
        };
        let mut output = CRYPT_INTEGER_BLOB::default();

        CryptUnprotectData(&input, None, None, None, None, 0, &mut output)
            .map_err(|e| format!("failed to decrypt session: {e}"))?;

        let bytes = std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec();
        let _ = LocalFree(Some(windows::Win32::Foundation::HLOCAL(
            output.pbData as *mut _,
        )));
        Ok(bytes)
    }
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

/// Exchanges the stored refresh token for a new session, and persists it.
/// If this itself fails (the refresh token is also expired or revoked),
/// the caller sees a plain "run `dew login`" rather than a confusing
/// error about a token refresh nobody asked for directly.
fn refresh_session(session: &Session) -> Result<Session, String> {
    let response = agent()
        .post(&format!(
            "{SUPABASE_URL}/auth/v1/token?grant_type=refresh_token"
        ))
        .set("apikey", SUPABASE_ANON_KEY)
        .set("Content-Type", "application/json")
        .send_json(serde_json::json!({ "refresh_token": session.refresh_token }))
        .map_err(|_| "session expired; run `dew login`".to_string())?;

    let token: TokenResponse = response
        .into_json()
        .map_err(|_| "session expired; run `dew login`".to_string())?;
    let refreshed = Session {
        access_token: token.access_token,
        refresh_token: token.refresh_token,
        user_id: token.user.id,
        email: token.user.email,
    };
    save_session(&refreshed)?;
    Ok(refreshed)
}

/// Every authenticated `dew-platform` request goes through this: on a 401
/// (an access token expired, typically within about an hour), it
/// transparently refreshes and retries once before giving up, so a
/// session that has gone stale does not surface as a confusing error to
/// something as simple as `dew discover`.
fn authorized_call(request: impl Fn(&str) -> ureq::Request) -> Result<ureq::Response, String> {
    let session = require_session()?;
    match request(&session.access_token).call() {
        Ok(response) => Ok(response),
        Err(ureq::Error::Status(401, _)) => {
            let refreshed = refresh_session(&session)?;
            request(&refreshed.access_token)
                .call()
                .map_err(describe_auth_error)
        }
        Err(e) => Err(describe_auth_error(e)),
    }
}

fn authorized_json<T: serde::de::DeserializeOwned>(
    request: impl Fn(&str) -> ureq::Request,
) -> Result<T, String> {
    authorized_call(request)?
        .into_json()
        .map_err(|e| e.to_string())
}

fn authorized_bytes(request: impl Fn(&str) -> ureq::Request) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    authorized_call(request)?
        .into_reader()
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    Ok(bytes)
}

/// Like `authorized_call`, but for a request that sends a body: the
/// caller does the `.send_*()` itself and hands back its `Result`,
/// since `ureq::Request` has no one method that fits every body shape.
fn authorized_send<T: serde::de::DeserializeOwned>(
    send: impl Fn(&str) -> Result<ureq::Response, ureq::Error>,
) -> Result<T, String> {
    let session = require_session()?;
    let response = match send(&session.access_token) {
        Ok(response) => response,
        Err(ureq::Error::Status(401, _)) => {
            let refreshed = refresh_session(&session)?;
            send(&refreshed.access_token).map_err(describe_auth_error)?
        }
        Err(e) => return Err(describe_auth_error(e)),
    };
    response.into_json().map_err(|e| e.to_string())
}

/// Lists every public package across every account. Requires a session
/// the same way every other platform request does, even though the
/// listing itself is not scoped to the caller: `dew-platform` gates every
/// route behind a verified token, discovery included.
pub fn discover() -> Result<Vec<DiscoveredPackage>, String> {
    authorized_json(|token| {
        agent()
            .get(&format!("{DEW_PLATFORM_URL}/discover"))
            .set("Authorization", &format!("Bearer {token}"))
    })
}

/// Fetches a public package's raw bytes by its owner and applet id
/// together, exactly the shape `dew-platform`'s own milestone 18 route
/// takes: never by applet id alone, since that still cannot say whose
/// package is meant.
pub fn fetch_package(owner_user_id: &str, applet_id: &str) -> Result<Vec<u8>, String> {
    authorized_bytes(|token| {
        agent()
            .get(&format!(
                "{DEW_PLATFORM_URL}/packages/{owner_user_id}/{applet_id}"
            ))
            .set("Authorization", &format!("Bearer {token}"))
    })
}

/// Fetches one of the CALLER'S OWN packages by applet id alone.
/// `dew-platform` resolves the owner from the bearer token itself for
/// this route, so it is only ever the caller's own published package,
/// never a stranger's -- `dew sync` restores what an account published
/// itself, and it is honest about not yet solving restoring someone
/// else's public package by id alone, the same namespacing question
/// ADR-015 already deferred.
pub fn fetch_own_package(applet_id: &str) -> Result<Vec<u8>, String> {
    authorized_bytes(|token| {
        agent()
            .get(&format!("{DEW_PLATFORM_URL}/packages/{applet_id}"))
            .set("Authorization", &format!("Bearer {token}"))
    })
}

/// `dew-platform`'s own `Package`, returned on a successful publish.
#[derive(Debug, Deserialize)]
pub struct PublishedPackage {
    pub applet_id: String,
    pub public: bool,
}

/// Uploads a `.dewpkg`'s raw bytes. The applet id is never named here:
/// `dew-platform` reads it out of the package's own `dew.toml`, so what a
/// package claims to be and what it is addressed by can never disagree.
pub fn publish(bytes: Vec<u8>, public: bool) -> Result<PublishedPackage, String> {
    let query = if public { "?public=true" } else { "" };
    // `ureq::Error` is large; boxing it everywhere `authorized_send` might
    // be used would be more machinery than the one call site needs.
    #[allow(clippy::result_large_err)]
    authorized_send(|token| {
        agent()
            .post(&format!("{DEW_PLATFORM_URL}/packages{query}"))
            .set("Authorization", &format!("Bearer {token}"))
            .set("Content-Type", "application/octet-stream")
            .send_bytes(&bytes)
    })
}

/// One entry in `GET /sync`'s listing. `dew-platform` also returns
/// `added_at`, not read here: nothing this command does needs it.
#[derive(Debug, Deserialize)]
pub struct SyncedApplet {
    pub applet_id: String,
}

/// Lists the signed-in account's synced applet ids.
pub fn list_synced() -> Result<Vec<SyncedApplet>, String> {
    authorized_json(|token| {
        agent()
            .get(&format!("{DEW_PLATFORM_URL}/sync"))
            .set("Authorization", &format!("Bearer {token}"))
    })
}

/// Adds an applet id to the signed-in account's synced list. Idempotent
/// on `dew-platform`'s own side, so calling this for an id already
/// synced is a harmless no-op, not an error.
pub fn add_synced(applet_id: &str) -> Result<(), String> {
    authorized_call(|token| {
        agent()
            .put(&format!("{DEW_PLATFORM_URL}/sync/{applet_id}"))
            .set("Authorization", &format!("Bearer {token}"))
    })?;
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
