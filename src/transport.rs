use crate::{Cli, CliResult};
use serde_json::{json, Value};
use std::time::Duration;

pub(crate) struct ApiClient {
    base: String,
    client: reqwest::blocking::Client,
    username: Option<String>,
    password: Option<String>,
    timeout: Duration,
}

impl ApiClient {
    pub(crate) fn new(
        cli: &Cli,
        selected_target: Option<&str>,
        username: Option<String>,
        password: Option<String>,
    ) -> CliResult<Self> {
        let target = selected_target.ok_or((
            "TARGET_REQUIRED",
            "provide --target URL, AWTRIX_URL, or select a named/default device profile".into(),
        ))?;
        let parsed = reqwest::Url::parse(target)
            .map_err(|_| ("ARGUMENT", "target must be an absolute HTTP URL".into()))?;
        if !matches!(parsed.scheme(), "http" | "https")
            || parsed.host_str().is_none()
            || !parsed.username().is_empty()
            || parsed.password().is_some()
            || parsed.query().is_some()
            || parsed.fragment().is_some()
        {
            return Err(("ARGUMENT", "target must be an absolute HTTP URL".into()));
        }
        let base = target.trim_end_matches('/').to_string();
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_millis(cli.timeout.max(1)))
            .build()
            .map_err(|_| ("TRANSPORT", "could not create HTTP client".into()))?;
        Ok(Self {
            base,
            client,
            username,
            password,
            timeout: Duration::from_millis(cli.timeout.max(1)),
        })
    }

    pub(crate) fn get(&self, path: &str) -> CliResult<Value> {
        self.get_with_timeout(path, self.timeout)
    }

    pub(crate) fn get_with_timeout(&self, path: &str, timeout: Duration) -> CliResult<Value> {
        let mut request = self
            .client
            .get(format!("{}{path}", self.base))
            .timeout(timeout.min(self.timeout));
        if let Some(username) = &self.username {
            request = request.basic_auth(username, self.password.as_deref());
        }
        let response = request.send().map_err(|error| {
            if error.is_timeout() {
                ("TIMEOUT", "request timed out".to_string())
            } else {
                ("TRANSPORT", "could not reach the HTTP target".to_string())
            }
        })?;
        if response.status() == reqwest::StatusCode::UNAUTHORIZED {
            return Err((
                "AUTHENTICATION",
                "device rejected HTTP Basic credentials".into(),
            ));
        }
        if !response.status().is_success() {
            return Err((
                "HTTP",
                format!("device returned HTTP {}", response.status().as_u16()),
            ));
        }
        response
            .json::<Value>()
            .map_err(|_| ("INVALID_RESPONSE", "device returned invalid JSON".into()))
    }

    pub(crate) fn raw_get(&self, path: &str) -> CliResult<String> {
        let response = self
            .authorized(self.client.get(format!("{}{path}", self.base)))
            .send()
            .map_err(|e| {
                if e.is_timeout() {
                    ("TIMEOUT", "request timed out".into())
                } else {
                    ("TRANSPORT", "could not reach the HTTP target".into())
                }
            })?;
        if response.status() == reqwest::StatusCode::UNAUTHORIZED {
            return Err((
                "AUTHENTICATION",
                "device rejected HTTP Basic credentials".into(),
            ));
        }
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(("HTTP", "script not found".into()));
        }
        if !response.status().is_success() {
            return Err((
                "HTTP",
                format!("device returned HTTP {}", response.status().as_u16()),
            ));
        }
        String::from_utf8(
            response
                .bytes()
                .map_err(|_| ("INVALID_RESPONSE", "could not read script source".into()))?
                .to_vec(),
        )
        .map_err(|_| ("INVALID_RESPONSE", "script source is not UTF-8".into()))
    }
    pub(crate) fn raw_bytes_get_with_timeout(
        &self,
        path: &str,
        max_bytes: u64,
        timeout: Duration,
    ) -> CliResult<Vec<u8>> {
        let response = self
            .authorized(
                self.client
                    .get(format!("{}{path}", self.base))
                    .timeout(timeout.min(self.timeout)),
            )
            .send()
            .map_err(|error| {
                if error.is_timeout() {
                    ("TIMEOUT", "request timed out".into())
                } else {
                    ("TRANSPORT", "could not reach the HTTP target".into())
                }
            })?;
        if response.status() == reqwest::StatusCode::UNAUTHORIZED {
            return Err((
                "AUTHENTICATION",
                "device rejected HTTP Basic credentials".into(),
            ));
        }
        if !response.status().is_success() {
            return Err((
                "HTTP",
                format!("device returned HTTP {}", response.status().as_u16()),
            ));
        }
        if response
            .content_length()
            .is_some_and(|length| length > max_bytes)
        {
            return Err((
                "RESPONSE_TOO_LARGE",
                "framebuffer response exceeds the safe size limit".into(),
            ));
        }
        use std::io::Read;
        let mut limited = response.take(max_bytes.saturating_add(1));
        let mut bytes = Vec::new();
        limited.read_to_end(&mut bytes).map_err(|_| {
            (
                "TRANSPORT",
                "failed while reading framebuffer response".into(),
            )
        })?;
        if bytes.len() as u64 > max_bytes {
            return Err((
                "RESPONSE_TOO_LARGE",
                "framebuffer response exceeds the safe size limit".into(),
            ));
        }
        Ok(bytes)
    }
    pub(crate) fn resource_request(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<reqwest::blocking::Body>,
        content_type: Option<&str>,
    ) -> CliResult<Value> {
        let mut request =
            self.authorized(self.client.request(method, format!("{}{path}", self.base)));
        if let Some(body) = body {
            request = request.body(body);
        }
        if let Some(content_type) = content_type {
            request = request.header(reqwest::header::CONTENT_TYPE, content_type);
        }
        let response = request.send().map_err(|e| {
            if e.is_timeout() {
                ("TIMEOUT", "request timed out".into())
            } else {
                ("TRANSPORT", "could not reach the HTTP target".into())
            }
        })?;
        if response.status() == reqwest::StatusCode::UNAUTHORIZED {
            return Err((
                "AUTHENTICATION",
                "device rejected HTTP Basic credentials".into(),
            ));
        }
        if !response.status().is_success() {
            return Err((
                "HTTP",
                format!("device returned HTTP {}", response.status().as_u16()),
            ));
        }
        let bytes = response
            .bytes()
            .map_err(|_| ("TRANSPORT", "failed reading device response".into()))?;
        if bytes.is_empty() {
            return Ok(json!({"ok":true}));
        }
        serde_json::from_slice(&bytes)
            .map_err(|_| ("INVALID_RESPONSE", "device returned invalid JSON".into()))
    }
    pub(crate) fn multipart_post(
        &self,
        path: &str,
        form: reqwest::blocking::multipart::Form,
    ) -> CliResult<Value> {
        let response = self
            .authorized(
                self.client
                    .post(format!("{}{path}", self.base))
                    .multipart(form),
            )
            .send()
            .map_err(|e| {
                if e.is_timeout() {
                    ("TIMEOUT", "request timed out".into())
                } else {
                    ("TRANSPORT", "could not reach the HTTP target".into())
                }
            })?;
        if response.status() == reqwest::StatusCode::UNAUTHORIZED {
            return Err((
                "AUTHENTICATION",
                "device rejected HTTP Basic credentials".into(),
            ));
        }
        if !response.status().is_success() {
            return Err((
                "HTTP",
                format!("device returned HTTP {}", response.status().as_u16()),
            ));
        }
        let bytes = response
            .bytes()
            .map_err(|_| ("TRANSPORT", "failed reading device response".into()))?;
        if bytes.is_empty() {
            return Ok(json!({"ok":true}));
        }
        serde_json::from_slice(&bytes)
            .map_err(|_| ("INVALID_RESPONSE", "device returned invalid JSON".into()))
    }
    pub(crate) fn raw_put(&self, path: &str, source: &str) -> CliResult<Value> {
        let response = self
            .authorized(
                self.client
                    .put(format!("{}{path}", self.base))
                    .header(reqwest::header::CONTENT_TYPE, "text/plain")
                    .body(source.to_owned()),
            )
            .send()
            .map_err(|e| {
                if e.is_timeout() {
                    ("TIMEOUT", "request timed out".into())
                } else {
                    (
                        "TRANSPORT",
                        "write result is unknown after transport failure".into(),
                    )
                }
            })?;
        self.script_response(response)
    }
    /// Send an authenticated JSON mutation and decode the official JSON response.
    pub(crate) fn mutate(
        &self,
        method: reqwest::Method,
        path: &str,
        body: &Value,
    ) -> CliResult<Value> {
        self.mutate_with_timeout(method, path, body, self.timeout)
    }
    pub(crate) fn mutate_with_timeout(
        &self,
        method: reqwest::Method,
        path: &str,
        body: &Value,
        timeout: Duration,
    ) -> CliResult<Value> {
        let mut request = self
            .client
            .request(method, format!("{}{path}", self.base))
            .timeout(timeout.min(self.timeout));
        if let Some(username) = &self.username {
            request = request.basic_auth(username, self.password.as_deref());
        }
        if !body.is_null() {
            request = request.json(body);
        }
        let response = request.send().map_err(|e| {
            if e.is_timeout() {
                ("TIMEOUT", "request timed out".into())
            } else {
                (
                    "TRANSPORT",
                    "mutation result is unknown after transport failure".into(),
                )
            }
        })?;
        if response.status() == reqwest::StatusCode::INSUFFICIENT_STORAGE {
            // The enabled-route OpenAPI explicitly says 507 is applied in memory but not saved.
            // Discard the server body: its untrusted details must not leak to CLI output.
            return Err(("HTTP_507", "device returned HTTP 507".into()));
        }
        self.script_response(response)
    }
    pub(crate) fn conditional_put(
        &self,
        name: &str,
        expected: &Value,
        source: &str,
    ) -> CliResult<Value> {
        let response = self
            .authorized(
                self.client
                    .put(format!("{}/api/v1/apps/script-update/{name}", self.base))
                    .json(&json!({"expected_source":expected,"source":source})),
            )
            .send()
            .map_err(|e| {
                if e.is_timeout() {
                    ("TIMEOUT", "write result is unknown after timeout".into())
                } else {
                    (
                        "TRANSPORT",
                        "write result is unknown after transport failure".into(),
                    )
                }
            })?;
        if response.status() == reqwest::StatusCode::CONFLICT {
            return Err((
                "CONFLICT",
                "remote source differs from the supplied reference; no overwrite performed".into(),
            ));
        }
        self.script_response(response)
    }
    fn authorized(
        &self,
        request: reqwest::blocking::RequestBuilder,
    ) -> reqwest::blocking::RequestBuilder {
        if let Some(username) = &self.username {
            request.basic_auth(username, self.password.as_deref())
        } else {
            request
        }
    }
    fn script_response(&self, response: reqwest::blocking::Response) -> CliResult<Value> {
        if response.status() == reqwest::StatusCode::UNAUTHORIZED {
            return Err((
                "AUTHENTICATION",
                "device rejected HTTP Basic credentials".into(),
            ));
        }
        if !response.status().is_success() {
            return Err((
                "HTTP",
                format!("device returned HTTP {}", response.status().as_u16()),
            ));
        }
        response
            .json()
            .map_err(|_| ("INVALID_RESPONSE", "device returned invalid JSON".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;
    use std::{
        io::{Read, Write},
        net::TcpListener,
        thread,
    };
    use tiny_http::{Response, Server};

    fn client_with_timeout(target: &str, timeout_ms: &str) -> ApiClient {
        let cli = Cli::try_parse_from([
            "awtrix-cli",
            "--target",
            target,
            "--timeout",
            timeout_ms,
            "device",
            "state",
        ])
        .unwrap();
        ApiClient::new(&cli, Some(target), None, None).unwrap()
    }

    fn client(target: &str) -> ApiClient {
        client_with_timeout(target, "1000")
    }

    #[test]
    fn framebuffer_read_allows_exact_limit_and_rejects_known_excess() {
        for (body, expected_ok) in [(vec![1, 2, 3], true), (vec![1, 2, 3, 4], false)] {
            let server = Server::http("127.0.0.1:0").unwrap();
            let target = format!("http://{}", server.server_addr());
            let api = client(&target);
            let worker = thread::spawn(move || {
                server
                    .recv()
                    .unwrap()
                    .respond(Response::from_data(body))
                    .unwrap();
            });
            let result = api.raw_bytes_get_with_timeout("/frame", 3, Duration::from_secs(1));
            worker.join().unwrap();
            assert_eq!(result.is_ok(), expected_ok);
            if let Err((code, message)) = result {
                assert_eq!(code, "RESPONSE_TOO_LARGE");
                assert_eq!(message, "framebuffer response exceeds the safe size limit");
            }
        }
    }

    #[test]
    fn framebuffer_streamed_excess_without_content_length_is_rejected() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let target = format!("http://{}", listener.local_addr().unwrap());
        let api = client(&target);
        let worker = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0; 1024];
            let _ = stream.read(&mut request).unwrap();
            stream
                .write_all(b"HTTP/1.1 200 OK\r\nConnection: close\r\n\r\n1234")
                .unwrap();
        });
        let result = api.raw_bytes_get_with_timeout("/frame", 3, Duration::from_secs(1));
        worker.join().unwrap();
        assert_eq!(result.unwrap_err().0, "RESPONSE_TOO_LARGE");
    }

    #[test]
    fn request_timeout_override_preserves_zero_and_is_clamped_to_client_minimum() {
        let server = Server::http("127.0.0.1:0").unwrap();
        let target = format!("http://{}", server.server_addr());
        let api = client_with_timeout(&target, "1000");
        let worker = thread::spawn(move || {
            assert!(server
                .recv_timeout(Duration::from_millis(100))
                .unwrap()
                .is_none());
        });
        assert_eq!(
            api.get_with_timeout("/timeout", Duration::ZERO)
                .unwrap_err()
                .0,
            "TIMEOUT"
        );
        worker.join().unwrap();

        let server = Server::http("127.0.0.1:0").unwrap();
        let target = format!("http://{}", server.server_addr());
        let api = client_with_timeout(&target, "10");
        let worker = thread::spawn(move || {
            let request = server
                .recv_timeout(Duration::from_secs(2))
                .unwrap()
                .expect("minimum-timeout request");
            thread::sleep(Duration::from_millis(50));
            let _ = request.respond(Response::from_string("{}"));
        });
        let result = api.get_with_timeout("/timeout", Duration::from_secs(1));
        worker.join().unwrap();
        assert_eq!(result.unwrap_err().0, "TIMEOUT");
    }

    #[test]
    fn configured_timeout_has_one_millisecond_minimum() {
        let api = client_with_timeout("http://127.0.0.1:1", "0");
        assert_eq!(api.timeout, Duration::from_millis(1));
    }
}
