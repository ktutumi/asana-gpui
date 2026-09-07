use anyhow::{Context, Result, bail, ensure};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::Utc;
use rand::RngCore;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
use url::Url;

pub const CREDENTIAL_SERVICE: &str = "asana-gpui.oauth";
pub const DEFAULT_REDIRECT: &str = "http://127.0.0.1:18787/callback";
const TOKEN_ENDPOINT: &str = "https://app.asana.com/-/oauth_token";

// Deliberately no Debug: credentials must never appear in logs or panic diagnostics.
#[derive(Clone, Serialize, Deserialize)]
pub struct Session {
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: Option<i64>,
}

impl Session {
    pub fn from_environment_token() -> Result<Self> {
        let access_token = std::env::var("ASANA_API_KEY").context("ASANA_API_KEY is not set.")?;
        ensure!(!access_token.trim().is_empty(), "ASANA_API_KEY is empty.");
        Ok(Self {
            client_id: String::new(),
            client_secret: String::new(),
            redirect_uri: DEFAULT_REDIRECT.into(),
            access_token,
            refresh_token: String::new(),
            expires_at: None,
        })
    }

    pub fn needs_refresh(&self) -> bool {
        self.expires_at
            .is_some_and(|expiry| expiry <= Utc::now().timestamp() + 60)
    }

    pub fn refresh(&mut self, http: &Client) -> Result<()> {
        ensure!(
            !self.refresh_token.is_empty(),
            "Your session has expired. Connect with Asana again."
        );
        let token = request_token(
            http,
            TOKEN_ENDPOINT,
            &[
                ("grant_type", "refresh_token"),
                ("client_id", &self.client_id),
                ("client_secret", &self.client_secret),
                ("refresh_token", &self.refresh_token),
            ],
        )?;
        self.apply_token(token)
    }

    fn apply_token(&mut self, token: TokenResponse) -> Result<()> {
        ensure!(
            !token.access_token.is_empty()
                && token.token_type.eq_ignore_ascii_case("bearer")
                && token.expires_in > 0,
            "Asana returned an invalid token response."
        );
        self.access_token = token.access_token;
        if let Some(refresh_token) = token.refresh_token.filter(|s| !s.is_empty()) {
            self.refresh_token = refresh_token;
        }
        self.expires_at = Some(Utc::now().timestamp().saturating_add(token.expires_in));
        Ok(())
    }
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    token_type: String,
    expires_in: i64,
    refresh_token: Option<String>,
}

pub fn http_client() -> Result<Client> {
    Client::builder()
        .timeout(Duration::from_secs(30))
        .connect_timeout(Duration::from_secs(10))
        .redirect(reqwest::redirect::Policy::none())
        .user_agent("asana-gpui/0.1")
        .build()
        .context("Could not initialize a secure connection.")
}

fn request_token(http: &Client, endpoint: &str, form: &[(&str, &str)]) -> Result<TokenResponse> {
    let response = http.post(endpoint).form(form).send().map_err(|_| {
        anyhow::anyhow!(
            "Could not reach Asana authentication. Check your connection and try again."
        )
    })?;
    ensure!(
        response.status().is_success(),
        "Asana authentication failed (HTTP {}). Check the client settings or connect again.",
        response.status().as_u16()
    );
    response
        .json()
        .map_err(|_| anyhow::anyhow!("Asana returned an unreadable token response."))
}

pub struct PendingAuth {
    session: Session,
    state: String,
    verifier: String,
    listener: Option<TcpListener>,
    started: Instant,
    pub cancelled: Arc<AtomicBool>,
    pub authorize_url: String,
}

impl PendingAuth {
    pub fn new(
        client_id: String,
        client_secret: String,
        redirect_uri: String,
        scopes: String,
    ) -> Result<Self> {
        ensure!(
            !client_id.trim().is_empty() && !client_secret.trim().is_empty(),
            "Enter a client ID and client secret, or set ASANA_CLIENT_ID and ASANA_CLIENT_SECRET."
        );
        let redirect = validate_redirect(&redirect_uri)?;
        let listener = if redirect_uri == "urn:ietf:wg:oauth:2.0:oob" {
            None
        } else {
            let port = redirect
                .port()
                .context("The callback URL needs an explicit port.")?;
            let listener = TcpListener::bind(("127.0.0.1", port))
                .context("The callback port is busy. Close another Asana login and try again.")?;
            listener.set_nonblocking(true)?;
            Some(listener)
        };
        let state = random_secret();
        let verifier = random_secret();
        let challenge = pkce_challenge(&verifier);
        let mut authorize_url = Url::parse("https://app.asana.com/-/oauth_authorize")?;
        authorize_url.query_pairs_mut().extend_pairs([
            ("client_id", client_id.as_str()),
            ("redirect_uri", redirect_uri.as_str()),
            ("response_type", "code"),
            ("state", &state),
            ("code_challenge_method", "S256"),
            ("code_challenge", &challenge),
            ("scope", scopes.trim()),
        ]);
        Ok(Self {
            session: Session {
                client_id,
                client_secret,
                redirect_uri,
                access_token: String::new(),
                refresh_token: String::new(),
                expires_at: None,
            },
            state,
            verifier,
            listener,
            started: Instant::now(),
            cancelled: Arc::new(AtomicBool::new(false)),
            authorize_url: authorize_url.into(),
        })
    }

    pub fn is_manual(&self) -> bool {
        self.listener.is_none()
    }

    pub fn wait(mut self) -> Result<Session> {
        let listener = self
            .listener
            .take()
            .context("Paste the authorization code to finish signing in.")?;
        while self.started.elapsed() < Duration::from_secs(300) {
            ensure!(
                !self.cancelled.load(Ordering::Relaxed),
                "Sign-in cancelled."
            );
            match listener.accept() {
                Ok((mut stream, _)) => {
                    if let Some(code) = self.read_callback(&mut stream)? {
                        return self.exchange(&code);
                    }
                }
                Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(50))
                }
                Err(_) => bail!("Could not receive the Asana callback. Try connecting again."),
            }
        }
        bail!("Sign-in timed out after five minutes. Connect again.")
    }

    fn read_callback(&self, stream: &mut TcpStream) -> Result<Option<String>> {
        stream.set_read_timeout(Some(Duration::from_millis(500)))?;
        stream.set_write_timeout(Some(Duration::from_secs(1)))?;
        let mut bytes = Vec::new();
        let mut chunk = [0u8; 1024];
        while bytes.len() < 8192 && !bytes.windows(4).any(|w| w == b"\r\n\r\n") {
            match stream.read(&mut chunk) {
                Ok(0) | Err(_) => break,
                Ok(n) => bytes.extend_from_slice(&chunk[..n]),
            }
        }
        let raw = String::from_utf8_lossy(&bytes);
        let mut line = raw.lines().next().unwrap_or("").split_whitespace();
        let method = line.next().unwrap_or("");
        let target = line.next().unwrap_or("");
        if method != "GET" || !target.starts_with('/') || target.starts_with("//") {
            reply(stream, "400 Bad Request", "Invalid callback.");
            return Ok(None);
        }
        let redirect = Url::parse(&self.session.redirect_uri)?;
        let callback = redirect.join(target)?;
        if callback.path() != redirect.path() {
            reply(stream, "404 Not Found", "Not found.");
            return Ok(None);
        }
        let code = match callback_code(&callback, &self.state) {
            Ok(code) => code,
            Err(_) => {
                reply(
                    stream,
                    "400 Bad Request",
                    "This sign-in response is invalid. Return to Asana GPUI and try again.",
                );
                return Ok(None);
            }
        };
        if callback.query_pairs().any(|(key, _)| key == "error") {
            reply(
                stream,
                "200 OK",
                "Sign-in was cancelled. Return to Asana GPUI.",
            );
            bail!("Asana authorization was denied. Connect again when ready.");
        }
        reply(
            stream,
            "200 OK",
            "Authorization received. Return to Asana GPUI to finish connecting. You may close this tab.",
        );
        Ok(Some(code))
    }

    pub fn exchange(mut self, code: &str) -> Result<Session> {
        ensure!(
            !self.cancelled.load(Ordering::Relaxed)
                && self.started.elapsed() < Duration::from_secs(300),
            "Sign-in expired or was cancelled. Connect again."
        );
        let code = code.trim();
        ensure!(
            !code.is_empty() && code.len() <= 4096 && !code.chars().any(char::is_whitespace),
            "Paste the authorization code shown by Asana."
        );
        let http = http_client()?;
        let token = request_token(
            &http,
            TOKEN_ENDPOINT,
            &[
                ("grant_type", "authorization_code"),
                ("client_id", &self.session.client_id),
                ("client_secret", &self.session.client_secret),
                ("redirect_uri", &self.session.redirect_uri),
                ("code", code),
                ("code_verifier", &self.verifier),
            ],
        )?;
        self.session.apply_token(token)?;
        ensure!(
            !self.session.refresh_token.is_empty(),
            "Asana did not return a refresh token. Connect again."
        );
        Ok(self.session)
    }
}

pub fn validate_redirect(raw: &str) -> Result<Url> {
    let url = Url::parse(raw).context("Enter a valid redirect URL.")?;
    if raw == "urn:ietf:wg:oauth:2.0:oob" {
        return Ok(url);
    }
    ensure!(
        url.scheme() == "http"
            && url.host_str() == Some("127.0.0.1")
            && url.port().is_some_and(|p| p > 0)
            && url.username().is_empty()
            && url.password().is_none()
            && url.query().is_none()
            && url.fragment().is_none()
            && url.path() != "/",
        "Use http://127.0.0.1:PORT/callback or urn:ietf:wg:oauth:2.0:oob."
    );
    Ok(url)
}

fn callback_code(url: &Url, expected_state: &str) -> Result<String> {
    let states: Vec<_> = url.query_pairs().filter(|(k, _)| k == "state").collect();
    ensure!(
        states.len() == 1 && states[0].1 == expected_state,
        "The OAuth state did not match."
    );
    if url.query_pairs().any(|(k, _)| k == "error") {
        return Ok(String::new());
    }
    let codes: Vec<_> = url.query_pairs().filter(|(k, _)| k == "code").collect();
    ensure!(
        codes.len() == 1 && !codes[0].1.is_empty(),
        "The OAuth response has no unique authorization code."
    );
    Ok(codes[0].1.to_string())
}

fn reply(stream: &mut TcpStream, status: &str, body: &str) {
    let _ = write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Type: text/plain; charset=utf-8\r\nCache-Control: no-store\r\nContent-Security-Policy: default-src 'none'\r\nReferrer-Policy: no-referrer\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    );
}

fn random_secret() -> String {
    let mut bytes = [0u8; 32];
    rand::rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}
fn pkce_challenge(verifier: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pkce_rfc7636_and_callback_validation() {
        assert_eq!(
            pkce_challenge("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );
        for raw in [
            "http://example.com:18787/callback",
            "http://127.0.0.1:18787/callback?x=1",
            "http://user@localhost:18787/callback",
            "http://localhost:0/callback",
            "http://localhost:18787/callback",
        ] {
            assert!(validate_redirect(raw).is_err());
        }
        assert!(validate_redirect(DEFAULT_REDIRECT).is_ok());
        assert!(validate_redirect("urn:ietf:wg:oauth:2.0:oob").is_ok());
        let parse = |query: &str| Url::parse(&format!("{DEFAULT_REDIRECT}?{query}")).unwrap();
        assert_eq!(
            callback_code(&parse("state=expected&code=sample"), "expected").unwrap(),
            "sample"
        );
        for query in [
            "state=wrong&code=sample",
            "code=sample",
            "state=expected&state=wrong&code=sample",
            "state=expected&code=a&code=b",
        ] {
            assert!(callback_code(&parse(query), "expected").is_err());
        }
        assert!(callback_code(&parse("state=wrong&error=denied"), "expected").is_err());
    }

    #[test]
    fn refresh_keeps_existing_refresh_token() {
        let mut session = Session {
            client_id: String::new(),
            client_secret: String::new(),
            redirect_uri: DEFAULT_REDIRECT.into(),
            access_token: "old".into(),
            refresh_token: "refresh".into(),
            expires_at: Some(0),
        };
        assert!(session.needs_refresh());
        session
            .apply_token(TokenResponse {
                access_token: "new".into(),
                token_type: "bearer".into(),
                expires_in: 3600,
                refresh_token: None,
            })
            .unwrap();
        assert_eq!(session.refresh_token, "refresh");
        assert!(!session.needs_refresh());
    }
}
