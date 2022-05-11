use serde_derive::{Deserialize, Serialize};

pub trait BackendConfig {}

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub settings: Option<Settings>,
    pub domains: Vec<DomainConfig>,
}

#[derive(Serialize, Deserialize)]
pub struct Settings {
    pub external_ipv4_check_url: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct DomainConfig {
    pub name: String,
    pub digital_ocean_backend: Option<DigitalOceanBackendConfig>,
    pub records: Option<Vec<Record>>,
}

#[derive(Serialize, Deserialize)]
pub struct DigitalOceanBackendConfig {
    pub api_key: String,
}

impl BackendConfig for DigitalOceanBackendConfig {}

#[derive(Serialize, Deserialize)]
pub struct Record {
    pub name: String,
    pub record_type: String,
    pub interface: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_settings_deserialize() {
        let config: Settings = toml::from_str(
            r#"
        external_ipv4_check_url = "test_v4"
        "#,
        )
        .unwrap();

        assert_eq!(config.external_ipv4_check_url, Some("test_v4".to_string()));
    }

    #[test]
    fn test_digitalocean_settings_deserialize() {
        let config: DigitalOceanBackendConfig = toml::from_str(
            r#"
        api_key = "test"
        "#,
        )
        .unwrap();

        assert_eq!(config.api_key, "test".to_string());
    }

    #[test]
    fn test_do_backend_deserialize() {
        let config: Config = toml::from_str(
            r#"
        [[domains]]
        name = "example.com"

            [domains.digital_ocean_backend]
            api_key = "test"
        "#,
        )
        .unwrap();

        let do_backend = config.domains[0].digital_ocean_backend.as_ref().unwrap();
        assert_eq!(do_backend.api_key, "test".to_string());
    }

    #[test]
    fn test_domain_record_deserialize() {
        let config: Config = toml::from_str(
            r#"
        [[domains]]
        name = "example.com"
        
            [[domains.records]]
            name = "testhost"
            record_type = "A"
            interface = "eth0"
        "#,
        )
        .unwrap();
        assert_eq!(config.domains.len(), 1);
        assert_eq!(config.domains[0].name, "example.com".to_string());
        let record = &config.domains[0].records.as_ref().unwrap()[0];
        assert_eq!(record.name, "testhost");
        assert_eq!(record.record_type, "A");
        assert_eq!(record.interface, "eth0");
    }
}
