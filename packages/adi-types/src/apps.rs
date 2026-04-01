//! Application configuration types.

use serde::{Deserialize, Serialize};

/// Portal application configuration.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct PortalConfig {
    /// HTTP port for the portal.
    pub http_port: u16,
}

impl PortalConfig {
    /// Creates a new portal config with the given port.
    pub fn new(http_port: u16) -> Self {
        Self { http_port }
    }
}

/// Explorer application configuration.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct ExplorerConfig {
    /// HTTP port for the explorer.
    pub http_port: u16,
}

impl ExplorerConfig {
    /// Creates a new explorer config with the given port.
    pub fn new(http_port: u16) -> Self {
        Self { http_port }
    }
}

/// Applications configuration from configs/apps.yaml.
///
/// # Example YAML
/// ```yaml
/// portal:
///   http_port: 3030
/// explorer:
///   http_port: 3010
/// ```
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct Apps {
    /// Portal configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub portal: Option<PortalConfig>,

    /// Explorer configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub explorer: Option<ExplorerConfig>,
}

impl Apps {
    /// Creates a new apps configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates apps with default ports (portal: 3030, explorer: 3010).
    #[must_use]
    pub fn with_defaults() -> Self {
        Self {
            portal: Some(PortalConfig::new(3030)),
            explorer: Some(ExplorerConfig::new(3010)),
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_apps_deserialize() {
        let yaml = r#"
portal:
  http_port: 3030
explorer:
  http_port: 3010
"#;
        let apps: Apps = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(apps.portal.as_ref().map(|p| p.http_port), Some(3030));
        assert_eq!(apps.explorer.as_ref().map(|e| e.http_port), Some(3010));
    }

    #[test]
    fn test_apps_with_defaults() {
        let apps = Apps::with_defaults();
        assert_eq!(apps.portal.as_ref().map(|p| p.http_port), Some(3030));
        assert_eq!(apps.explorer.as_ref().map(|e| e.http_port), Some(3010));
    }
}
