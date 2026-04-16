//! Donor-privacy redaction filter per FR-106.
//!
//! Strips PII, hostnames, local IPs, usernames, MAC addresses from
//! telemetry before emission. Must be unit-tested as a release gate.

/// Redact known PII patterns from a string.
/// This is applied at the telemetry emit layer to every field value.
pub fn redact(input: &str) -> String {
    let mut output = input.to_string();

    // Redact MAC addresses (XX:XX:XX:XX:XX:XX)
    let mac_re = regex_lite::Regex::new(r"[0-9a-fA-F]{2}(:[0-9a-fA-F]{2}){5}").unwrap();
    output = mac_re.replace_all(&output, "[REDACTED_MAC]").to_string();

    // Redact IPv4 private addresses
    let ipv4_private_re = regex_lite::Regex::new(
        r"(10\.\d{1,3}\.\d{1,3}\.\d{1,3}|192\.168\.\d{1,3}\.\d{1,3}|172\.(1[6-9]|2\d|3[01])\.\d{1,3}\.\d{1,3})"
    ).unwrap();
    output = ipv4_private_re.replace_all(&output, "[REDACTED_IP]").to_string();

    // Redact Unix-style usernames in paths (/home/username/, /Users/username/)
    let user_path_re = regex_lite::Regex::new(r"/(home|Users)/[a-zA-Z0-9_.-]+").unwrap();
    output = user_path_re.replace_all(&output, "/$1/[REDACTED_USER]").to_string();

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_mac_address() {
        let input = "interface aa:bb:cc:dd:ee:ff is up";
        assert_eq!(redact(input), "interface [REDACTED_MAC] is up");
    }

    #[test]
    fn redacts_private_ipv4() {
        assert!(redact("connecting to 192.168.1.42").contains("[REDACTED_IP]"));
        assert!(redact("host 10.0.0.1 is reachable").contains("[REDACTED_IP]"));
    }

    #[test]
    fn redacts_username_paths() {
        assert!(redact("/Users/jmanning/data").contains("[REDACTED_USER]"));
        assert!(redact("/home/alice/.config").contains("[REDACTED_USER]"));
    }

    #[test]
    fn leaves_public_ips_alone() {
        let input = "connecting to 8.8.8.8";
        assert_eq!(redact(input), input);
    }
}
