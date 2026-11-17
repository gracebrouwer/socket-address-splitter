# socket-address-splitter

Split a `host:port` string into its host and port parts, including bracketed IPv6 literals and zone identifiers.

```rust
use socket_address_splitter::split;

let (host, port) = split("[fe80::1%eth0]:8080").unwrap();
assert_eq!(host, "fe80::1%eth0");
assert_eq!(port, 8080);
```

## Why this exists

Most configuration and command-line parsers hand you a combined `host:port` string. Splitting it sounds trivial until IPv6 shows up: `::1:443` is ambiguous because both the address and the port use `:`. The standard way to disambiguate is to wrap the address in brackets, as in `[::1]:443`, and to allow a zone identifier after a `%` inside those brackets (`[fe80::1%eth0]:443`).

This crate does exactly that one job. It returns borrowed slices, performs no allocation, and does not resolve hostnames or validate that the host is an IP address. The trade-off is that a bare IPv6 literal without brackets is rejected as ambiguous rather than guessed at. If you have `::1:443`, you must write `[::1]:443`.

## The edge you will hit

A non-bracketed address with more than one colon is rejected with `SplitError::AmbiguousColon`. This is intentional: guessing where the port begins in `::1:443` would silently produce the wrong address on some inputs.

## API

- `split(&str) -> Result<(&str, u16), SplitError>`
- `SplitError` variants: `Empty`, `MissingClosingBracket`, `MissingPortSeparator`, `EmptyPort`, `InvalidPort`, `AmbiguousColon`, `EmptyHost`.
