use crate::config::{Config, ParsedDomainConfig};
use crate::dns_providers::{update_records, DnsBackendError};
use crate::providers::cloudflare::CloudFlareBackend;
use crate::providers::digitalocean::DigitalOceanBackend;
use default_net::get_default_interface;
use get_if_addrs::{get_if_addrs, Interface};
use reqwest;
use std::net::{IpAddr, Ipv4Addr};
use std::str::FromStr;

#[derive(Debug, PartialEq)]
pub enum SystemAddress {
    V4(SystemV4Address),
    V6(SystemV6Address),
}

impl From<SystemAddress> for IpAddr {
    fn from(addr: SystemAddress) -> IpAddr {
        match addr {
            SystemAddress::V4(addr) => addr.address,
            SystemAddress::V6(addr) => addr.address,
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct SystemAddresses {
    pub v4_addresses: Vec<SystemV4Address>,
    pub v6_addresses: Vec<SystemV6Address>,
}

#[derive(Debug, PartialEq, Clone)]

pub struct SystemV4Address {
    pub interface: String,
    pub address: IpAddr,
}

#[derive(Debug, PartialEq, Clone)]
pub struct SystemV6Address {
    pub interface: String,
    pub address: IpAddr,
}

impl SystemAddresses {
    fn new(interfaces: Vec<Interface>, external_address: Option<Ipv4Addr>) -> Self {
        let mut ipv4_addr: Vec<SystemV4Address> = interfaces
            .clone()
            .into_iter()
            .filter(|x| x.addr.ip().is_ipv4())
            .map(|x| SystemV4Address {
                interface: x.name,
                address: x.addr.ip(),
            })
            .collect();

        let ipv6_addr: Vec<SystemV6Address> = interfaces
            .into_iter()
            .filter(|x| x.addr.ip().is_ipv6())
            .map(|x| SystemV6Address {
                interface: x.name,
                address: x.addr.ip(),
            })
            .collect();

        if let Some(external_ip) = external_address {
            ipv4_addr.push(SystemV4Address {
                interface: "external".to_string(),
                address: IpAddr::from(external_ip),
            });
        };

        Self {
            v4_addresses: ipv4_addr,
            v6_addresses: ipv6_addr,
        }
    }
}

pub fn update_dns(config: Config) {
    let local_interfaces = get_if_addrs().expect("Could not fetch local interface IPs");

    let default_interface = get_default_interface().expect("Failed to get default interface");

    info!("Detected default interface as {}", default_interface.name);

    let external_ipv4: Option<Ipv4Addr> = match config.settings {
        Some(settings) => match settings.external_ipv4_check_url {
            Some(url) => {
                info!("Making request to {} for IPV4 address discovery", &url);
                let req_body = reqwest::blocking::get(url).unwrap().text().unwrap();
                info!(
                    "Got response from external IP discovery service: {}",
                    req_body
                );
                let ip_address = Ipv4Addr::from_str(&req_body)
                    .expect("Couldn't parse IP from external discovery service");
                info!("Discovered external IPv4 address: {:?}", ip_address);
                Some(ip_address)
            }
            None => None,
        },
        None => None,
    };

    let system_interfaces = SystemAddresses::new(local_interfaces, external_ipv4);
    info!("System IPs: {:?}", system_interfaces);

    for domain in config.domains {
        info!("Running for domain {}", domain.name);
        let parsed_domain = domain.parse_config(&default_interface);
        update_domain_dns(parsed_domain, &system_interfaces).unwrap();
    }
}

fn update_domain_dns(
    domain: ParsedDomainConfig,
    system_interfaces: &SystemAddresses,
) -> Result<(), DnsBackendError> {
    info!("Starting update of {}", domain.name);
    let desired_records = domain.records;
    if let Some(digitalocean_config) = domain.digital_ocean_backend {
        let backend = DigitalOceanBackend::new(digitalocean_config.api_key, domain.name)
            .expect("Failed to setup digitalocean client");
        update_records(backend, desired_records, system_interfaces)?;

        Ok(())
    } else if let Some(cloudflare_config) = domain.cloudflare_backend {
        let backend = CloudFlareBackend::from(cloudflare_config);
        update_records(backend, desired_records, system_interfaces)?;

        Ok(())
    } else {
        Err(DnsBackendError {
            message: format!("No dns backend configured for: {}", domain.name),
        })
    }
}
