use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum ServiceAddress {
    String(String),
}

impl ServiceAddress {
    /// Creates a new ServiceAddress from a String
    #[allow(dead_code)]
    pub fn from_string(address: String) -> Self {
        ServiceAddress::String(address)
    }

    /// Returns the address as a string reference
    pub fn as_str(&self) -> &str {
        match self {
            ServiceAddress::String(addr) => addr.as_str(),
        }
    }

    /// Attempts to extract the port from the address
    #[allow(dead_code)]
    pub fn extract_port(&self) -> Option<u16> {
        match self {
            ServiceAddress::String(addr) => {
                // Check for URL format with protocol
                if addr.contains("://") {
                    // Split by protocol and get the host part
                    let parts: Vec<&str> = addr.split("://").collect();
                    if parts.len() < 2 {
                        return None;
                    }

                    // Try to find a port in the host part
                    let host_parts: Vec<&str> = parts[1].split(':').collect();
                    if host_parts.len() < 2 {
                        return None;
                    }

                    // Parse the port section
                    host_parts[1].split('/').next()?.parse::<u16>().ok()
                } else {
                    // No protocol, check for direct host:port format
                    let parts: Vec<&str> = addr.split(':').collect();
                    if parts.len() < 2 {
                        return None;
                    }

                    parts[1].split('/').next()?.parse::<u16>().ok()
                }
            }
        }
    }

    /// Checks if the address uses a secure protocol (https, wss, etc.)
    #[allow(dead_code)]
    pub fn is_secure(&self) -> bool {
        match self {
            ServiceAddress::String(addr) => {
                addr.starts_with("https://")
                    || addr.starts_with("wss://")
                    || addr.starts_with("ftps://")
                    || addr.starts_with("sftp://")
                    || addr.starts_with("ssh://")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{from_str, to_string};

    #[test]
    fn test_from_string() {
        let address = ServiceAddress::from_string("http://localhost:8080".to_string());
        assert!(matches!(address, ServiceAddress::String(_)));
        assert_eq!(address.as_str(), "http://localhost:8080");
    }

    #[test]
    fn test_as_str() {
        let address = ServiceAddress::String("https://api.example.com:443".to_string());
        assert_eq!(address.as_str(), "https://api.example.com:443");
    }

    #[test]
    fn test_extract_port_with_protocol() {
        let address = ServiceAddress::String("http://localhost:8080".to_string());
        assert_eq!(address.extract_port(), Some(8080));

        let address = ServiceAddress::String("https://api.example.com:443".to_string());
        assert_eq!(address.extract_port(), Some(443));

        let address = ServiceAddress::String("http://localhost:8080/api/v1".to_string());
        assert_eq!(address.extract_port(), Some(8080));
    }

    #[test]
    fn test_extract_port_without_protocol() {
        let address = ServiceAddress::String("localhost:8080".to_string());
        assert_eq!(address.extract_port(), Some(8080));

        let address = ServiceAddress::String("127.0.0.1:9090".to_string());
        assert_eq!(address.extract_port(), Some(9090));
    }

    #[test]
    fn test_extract_port_none() {
        let address = ServiceAddress::String("localhost".to_string());
        assert_eq!(address.extract_port(), None);

        let address = ServiceAddress::String("http://localhost".to_string());
        assert_eq!(address.extract_port(), None);
    }

    #[test]
    fn test_is_secure() {
        let secure_addresses = vec![
            "https://api.example.com",
            "wss://websocket.example.com",
            "ftps://ftp.example.com",
            "sftp://sftp.example.com",
            "ssh://ssh.example.com",
        ];

        let insecure_addresses = vec![
            "http://api.example.com",
            "ws://websocket.example.com",
            "ftp://ftp.example.com",
            "example.com",
            "localhost:8080",
        ];

        for addr in secure_addresses {
            let service_addr = ServiceAddress::String(addr.to_string());
            assert!(
                service_addr.is_secure(),
                "Address {} should be secure",
                addr
            );
        }

        for addr in insecure_addresses {
            let service_addr = ServiceAddress::String(addr.to_string());
            assert!(
                !service_addr.is_secure(),
                "Address {} should not be secure",
                addr
            );
        }
    }

    #[test]
    fn test_serialize_deserialize() {
        let address = ServiceAddress::String("https://api.example.com:443".to_string());

        // Serialize
        let serialized = to_string(&address).expect("Failed to serialize");
        let expected = r#"{"type":"String","value":"https://api.example.com:443"}"#;
        assert_eq!(serialized, expected);

        // Deserialize
        let deserialized: ServiceAddress = from_str(&serialized).expect("Failed to deserialize");
        assert!(matches!(deserialized, ServiceAddress::String(_)));
        assert_eq!(deserialized.as_str(), "https://api.example.com:443");
    }
}
