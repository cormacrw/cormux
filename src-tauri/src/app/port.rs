//! Where a dev server says it listens, read from what it printed.

use crate::process::strip_ansi;

pub fn detect_port(output: &str) -> Option<u16> {
    let output = strip_ansi(output);
    for marker in ["localhost:", "127.0.0.1:", "0.0.0.0:"] {
        for (index, _) in output.match_indices(marker) {
            let digits: String = output[index + marker.len()..]
                .chars()
                .take_while(|c| c.is_ascii_digit())
                .collect();
            if let Ok(port) = digits.parse() {
                return Some(port);
            }
        }
    }
    None
}

/// Whether the server announced `port` with an `https://` URL, so links to it should too.
pub fn serves_https(output: &str, port: u16) -> bool {
    let output = strip_ansi(output);
    ["localhost", "127.0.0.1", "0.0.0.0"]
        .iter()
        .any(|host| output.contains(&format!("https://{host}:{port}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spots_an_https_banner() {
        assert!(serves_https(
            "  \x1b[32m➜\x1b[39m  Local:   \x1b[36mhttps://localhost:\x1b[1m5173\x1b[22m/",
            5173
        ));
        assert!(!serves_https("Local: http://localhost:5173/", 5173));
        assert!(!serves_https(
            "https://localhost:4000/ and localhost:5173",
            5173
        ));
    }

    #[test]
    fn reads_port_from_dev_server_banner() {
        assert_eq!(detect_port("Local:   http://127.0.0.1:5173/\n"), Some(5173));
        assert_eq!(detect_port("ready on localhost:3000"), Some(3000));
        assert_eq!(
            detect_port(
                "  \x1b[32m➜\x1b[39m  \x1b[1mLocal\x1b[22m:   \x1b[36mhttp://localhost:\x1b[1m5173\x1b[22m/\x1b[39m"
            ),
            Some(5173)
        );
        assert_eq!(
            detect_port("see localhost: docs\nhttp://localhost:4000/"),
            Some(4000)
        );
    }
}
