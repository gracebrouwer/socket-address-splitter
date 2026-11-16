//! Split a `host:port` string into its host and port components.
//!
//! The parser understands the three common shapes:
//!
//! * `host:port`
//! * `[ipv6]:port`
//! * `[ipv6%zone]:port`
//!
//! It deliberately does not resolve hostnames, validate IP addresses, or
//! accept a port as a service name. The caller gets the two pieces back as
//! borrowed string slices so no allocation is needed.

use std::fmt;

/// The error returned when a socket address cannot be split.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SplitError {
    /// The input was empty.
    Empty,
    /// A bracketed address was missing the closing `]`.
    MissingClosingBracket,
    /// The `]` was not followed by a `:`.
    MissingPortSeparator,
    /// The port part was empty.
    EmptyPort,
    /// The port part was not a valid `u16`.
    InvalidPort,
    /// A non-bracketed address contained more than one `:`.
    AmbiguousColon,
    /// The host part was empty.
    EmptyHost,
}

impl fmt::Display for SplitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SplitError::Empty => write!(f, "input is empty"),
            SplitError::MissingClosingBracket => write!(f, "missing closing bracket"),
            SplitError::MissingPortSeparator => write!(f, "missing ':' after closing bracket"),
            SplitError::EmptyPort => write!(f, "port is empty"),
            SplitError::InvalidPort => write!(f, "port is not a valid u16"),
            SplitError::AmbiguousColon => write!(f, "more than one ':' in a non-bracketed address"),
            SplitError::EmptyHost => write!(f, "host is empty"),
        }
    }
}

impl std::error::Error for SplitError {}

/// Split `input` into a host slice and a port number.
///
/// The returned host slice borrows from `input`; no allocation is performed.
///
/// # Examples
///
/// ```
/// use socket_address_splitter::split;
///
/// assert_eq!(split("example.org:80").unwrap(), ("example.org", 80));
/// assert_eq!(split("[::1]:443").unwrap(), ("::1", 443));
/// assert_eq!(split("[fe80::1%eth0]:9").unwrap(), ("fe80::1%eth0", 9));
/// ```
pub fn split(input: &str) -> Result<(&str, u16), SplitError> {
    if input.is_empty() {
        return Err(SplitError::Empty);
    }

    if input.starts_with('[') {
        split_bracketed(input)
    } else {
        split_plain(input)
    }
}

fn split_plain(input: &str) -> Result<(&str, u16), SplitError> {
    // A bare IPv6 address has many colons and no way to tell where the port
    // starts, so we reject it. The caller must use brackets for IPv6.
    let colon = input.find(':').ok_or(SplitError::MissingPortSeparator)?;
    if input[colon + 1..].contains(':') {
        return Err(SplitError::AmbiguousColon);
    }

    let host = &input[..colon];
    if host.is_empty() {
        return Err(SplitError::EmptyHost);
    }
    let port = parse_port(&input[colon + 1..])?;
    Ok((host, port))
}

fn split_bracketed(input: &str) -> Result<(&str, u16), SplitError> {
    let close = input.find(']').ok_or(SplitError::MissingClosingBracket)?;
    let host = &input[1..close];
    if host.is_empty() {
        return Err(SplitError::EmptyHost);
    }

    let rest = &input[close + 1..];
    let port_str = rest
        .strip_prefix(':')
        .ok_or(SplitError::MissingPortSeparator)?;
    let port = parse_port(port_str)?;
    Ok((host, port))
}

fn parse_port(s: &str) -> Result<u16, SplitError> {
    if s.is_empty() {
        return Err(SplitError::EmptyPort);
    }
    s.parse::<u16>().map_err(|_| SplitError::InvalidPort)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_host_and_port() {
        assert_eq!(split("example.org:80"), Ok(("example.org", 80)));
    }

    #[test]
    fn plain_host_with_dashes_and_dots() {
        assert_eq!(split("my-host.local:8080"), Ok(("my-host.local", 8080)));
    }

    #[test]
    fn bracketed_ipv6() {
        assert_eq!(split("[::1]:443"), Ok(("::1", 443)));
    }

    #[test]
    fn bracketed_ipv6_with_zone() {
        assert_eq!(split("[fe80::1%eth0]:9"), Ok(("fe80::1%eth0", 9)));
    }

    #[test]
    fn bracketed_ipv6_with_zone_and_zero_port() {
        assert_eq!(split("[fe80::1%eth0]:0"), Ok(("fe80::1%eth0", 0)));
    }

    #[test]
    fn port_at_upper_bound() {
        assert_eq!(split("host:65535"), Ok(("host", 65535)));
    }

    #[test]
    fn empty_input() {
        assert_eq!(split(""), Err(SplitError::Empty));
    }

    #[test]
    fn missing_port_separator_plain() {
        assert_eq!(split("host"), Err(SplitError::MissingPortSeparator));
    }

    #[test]
    fn missing_port_separator_bracketed() {
        assert_eq!(split("[::1]"), Err(SplitError::MissingPortSeparator));
    }

    #[test]
    fn missing_closing_bracket() {
        assert_eq!(split("[::1:443"), Err(SplitError::MissingClosingBracket));
    }

    #[test]
    fn empty_port() {
        assert_eq!(split("host:"), Err(SplitError::EmptyPort));
        assert_eq!(split("[::1]:"), Err(SplitError::EmptyPort));
    }

    #[test]
    fn invalid_port() {
        assert_eq!(split("host:notaport"), Err(SplitError::InvalidPort));
        assert_eq!(split("host:65536"), Err(SplitError::InvalidPort));
        assert_eq!(split("host:-1"), Err(SplitError::InvalidPort));
    }

    #[test]
    fn ambiguous_colons() {
        assert_eq!(split("::1:443"), Err(SplitError::AmbiguousColon));
        assert_eq!(split("a:b:c"), Err(SplitError::AmbiguousColon));
    }

    #[test]
    fn empty_host() {
        assert_eq!(split(":80"), Err(SplitError::EmptyHost));
        assert_eq!(split("[]:80"), Err(SplitError::EmptyHost));
    }

    #[test]
    fn trailing_data_after_port_is_rejected() {
        assert_eq!(split("host:80x"), Err(SplitError::InvalidPort));
        assert_eq!(split("[::1]:80x"), Err(SplitError::InvalidPort));
    }

    #[test]
    fn error_display_is_stable() {
        assert_eq!(SplitError::Empty.to_string(), "input is empty");
        assert_eq!(SplitError::InvalidPort.to_string(), "port is not a valid u16");
    }
}
