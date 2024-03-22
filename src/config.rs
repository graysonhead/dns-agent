use crate::{
    dns_providers::DnsRecordType,
    providers::{cloudflare::CloudFlareBackendConfig, digitalocean::DigitalOceanBackendConfig},
};
use default_net::Interface;
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
    pub cloudflare_backend: Option<CloudFlareBackendConfig>,
    pub records: Vec<Record>,
}

#[derive(Serialize, Deserialize)]
pub struct ParsedDomainConfig {
    pub name: String,
    pub digital_ocean_backend: Option<DigitalOceanBackendConfig>,
    pub cloudflare_backend: Option<CloudFlareBackendConfig>,
    pub records: Vec<ParsedRecord>,
}

impl DomainConfig {
    pub fn parse_config(&self, default_interface: &Interface) -> ParsedDomainConfig {
        let parsed_records = self
            .records
            .iter()
            .map(|conf_record| {
                let interface: String = conf_record
                    .interface
                    .clone()
                    .unwrap_or(default_interface.name.clone());
                ParsedRecord {
                    name: conf_record.name.to_string(),
                    record_type: conf_record.record_type.as_str().into(),
                    interface,
                }
            })
            .collect();
        ParsedDomainConfig {
            name: self.name.clone(),
            digital_ocean_backend: self.digital_ocean_backend.clone(),
            cloudflare_backend: self.cloudflare_backend.clone(),
            records: parsed_records,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct Record {
    pub name: String,
    pub record_type: String,
    pub interface: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ParsedRecord {
    pub name: String,
    pub record_type: DnsRecordType,
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
        let record = &config.domains[0].records[0];
        assert_eq!(record.name, "testhost");
        assert_eq!(record.record_type, "A");
        assert_eq!(record.interface, Some("eth0".to_string()));
    }
}
