use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;

const SUCCESS_PAGE: &str = r#"<!DOCTYPE html>
<html lang="fr">
<head><meta charset="UTF-8"><title>HireLens</title></head>
<body style="font-family:system-ui;padding:48px;max-width:480px;margin:auto;text-align:center">
  <h2>✅ Connexion Google réussie !</h2>
  <p style="color:#555">Vous pouvez fermer cet onglet et retourner dans HireLens.</p>
</body>
</html>"#;

pub struct CallbackServer {
    listener: TcpListener,
    pub port: u16,
}

impl CallbackServer {
    pub fn bind() -> anyhow::Result<Self> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let port = listener.local_addr()?.port();
        Ok(Self { listener, port })
    }

    /// Blocks until the OAuth2 callback arrives. Returns `(code, state)`.
    pub fn wait_for_callback(self) -> anyhow::Result<(String, String)> {
        let (stream, _) = self.listener.accept()?;
        let mut reader = BufReader::new(&stream);
        let mut request_line = String::new();
        reader.read_line(&mut request_line)?;

        // "GET /callback?code=xxx&state=yyy HTTP/1.1"
        let path = request_line
            .split_whitespace()
            .nth(1)
            .unwrap_or("")
            .to_owned();

        let query = path.split('?').nth(1).unwrap_or("");
        let params = parse_query(query);

        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\n\r\n{}",
            SUCCESS_PAGE.len(),
            SUCCESS_PAGE
        );
        let mut writer = std::io::BufWriter::new(&stream);
        let _ = writer.write_all(response.as_bytes());
        drop(writer); // flush

        let code = params.get("code").cloned().unwrap_or_default();
        let state = params.get("state").cloned().unwrap_or_default();

        if code.is_empty() {
            let error = params.get("error").cloned().unwrap_or_else(|| "unknown error".to_owned());
            anyhow::bail!("OAuth2 callback error: {error}");
        }

        Ok((code, state))
    }
}

fn parse_query(query: &str) -> HashMap<String, String> {
    query
        .split('&')
        .filter_map(|pair| {
            let mut parts = pair.splitn(2, '=');
            let key = parts.next()?.to_owned();
            let val = percent_decode(parts.next().unwrap_or("").replace('+', " ").as_str());
            Some((key, val))
        })
        .collect()
}

fn percent_decode(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(hex) = std::str::from_utf8(&bytes[i + 1..i + 3]) {
                if let Ok(byte) = u8::from_str_radix(hex, 16) {
                    result.push(byte as char);
                    i += 3;
                    continue;
                }
            }
        }
        result.push(bytes[i] as char);
        i += 1;
    }
    result
}
