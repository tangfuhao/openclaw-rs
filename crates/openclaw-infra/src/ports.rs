use std::net::TcpListener;

/// Check if a port is available for binding.
pub fn is_port_available(port: u16) -> bool {
    TcpListener::bind(("127.0.0.1", port)).is_ok()
}

/// Find an available port starting from the given port.
pub fn find_available_port(start: u16) -> Option<u16> {
    (start..=65535).find(|&port| is_port_available(port))
}

/// Get the default gateway port (18789), or find the next available.
pub fn default_gateway_port() -> u16 {
    const DEFAULT_PORT: u16 = 18789;
    if is_port_available(DEFAULT_PORT) {
        DEFAULT_PORT
    } else {
        find_available_port(DEFAULT_PORT + 1).unwrap_or(DEFAULT_PORT)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_port() {
        let port = find_available_port(49152);
        assert!(port.is_some());
    }
}
