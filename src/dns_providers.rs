use crate::config::Record;
use crate::update::{SystemAddress, SystemAddresses, SystemV4Address, SystemV6Address};
use digitalocean::api::{Domain, DomainRecord};
use digitalocean::request::Executable;
use digitalocean::DigitalOcean;
use std::fmt;
use std::net::IpAddr;

#[derive(Debug)]
pub struct DnsRecord {
    pub kind: String,
    pub name: String,
    pub data: String,
}

#[derive(std::fmt::Debug, PartialEq)]
pub struct DnsBackendError {
    pub message: String,
}

impl fmt::Display for DnsBackendError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

// pub trait DnsBackend {
//     fn get_zone(self, zone: String) -> Result<DnsZoneRecords, DnsBackendError>;
//     fn create_record(record: DnsRecord) -> Result<(), DnsBackendError>;
//     // fn update_record(record: DnsRecord) -> Result<(), DnsBackendError>;
// }

pub struct DigitalOceanBackend {
    client: DigitalOcean,
    zone: String,
}

impl DigitalOceanBackend {
    pub fn new(api_key: String, zone: String) -> Result<Self, digitalocean::error::Error> {
        match DigitalOcean::new(api_key) {
            Ok(client) => Ok(Self { client, zone }),
            Err(e) => Err(e),
        }
    }
}

impl DigitalOceanBackend {
    pub fn get_zone(&self) -> Result<Vec<DomainRecord>, DnsBackendError> {
        match Domain::get(&self.zone).records().execute(&self.client) {
            Ok(results) => {
                debug!(
                    "Fetched records from digital ocean for domain {}: {:?}",
                    &self.zone, results
                );
                Ok(results)
            }
            Err(e) => Err(DnsBackendError {
                message: format!("Dns backend error: {:?}", e),
            }),
        }
    }

    pub fn create_record(&self, record: DnsRecord) -> Result<(), DnsBackendError> {
        let result = Domain::get(&self.zone)
            .records()
            .create(record.kind, record.name, record.data)
            .execute(&self.client);
        match result {
            Ok(_) => Ok(()),
            Err(e) => Err(DnsBackendError {
                message: format!("Failed to create DNS record: {:?}", e),
            }),
        }
    }

    pub fn update_record(
        &self,
        record: &DomainRecord,
        new_data: &String,
    ) -> Result<(), DnsBackendError> {
        let result = Domain::get(&self.zone)
            .records()
            .update(*record.id())
            .data(new_data)
            .execute(&self.client);
        match result {
            Ok(_) => Ok(()),
            Err(e) => Err(DnsBackendError {
                message: format!("Failed to update DNS record: {:?}", e),
            }),
        }
    }
}

pub fn update_records(
    backend: DigitalOceanBackend,
    desired_records: Vec<Record>,
    system_interfaces: &SystemAddresses,
) -> Result<(), DnsBackendError> {
    let current_records = backend.get_zone()?;
    for desired_record in desired_records {
        let zone_name = &backend.zone;
        let mut matching_records: Vec<DomainRecord> = current_records
            .clone()
            .into_iter()
            .filter(|x| x.name() == &desired_record.name && x.kind() == &desired_record.record_type)
            .collect();
        let interface = find_matching_interface(&desired_record, &system_interfaces)?;
        if matching_records.len() > 1 {
            warn!(
                "Multiple records found for {}.{}, not updating",
                &desired_record.name, zone_name
            );
        } else if matching_records.is_empty() {
            // Cloning some values for logging :( Should probably fix this. Done is better than perfect
            let rec_kind = desired_record.record_type.clone();
            let rec_name = desired_record.name.clone();
            let rec_data = IpAddr::from(interface).to_string();
            let new_record = DnsRecord {
                kind: desired_record.record_type,
                name: desired_record.name,
                data: rec_data.clone(),
            };
            backend.create_record(new_record)?;
            info!(
                "Created new {} record: {}.{} = {}",
                rec_kind, rec_name, &backend.zone, rec_data
            );
        } else if matching_records.len() == 1 {
            if let Some(record) = matching_records.pop() {
                let current_ip = IpAddr::from(interface).to_string();
                if record.data() != &current_ip.to_string() {
                    backend.update_record(&record, &current_ip)?;
                    info!(
                        "Updated {} record {}.{}: old {} new {}",
                        record.kind(),
                        record.name(),
                        &backend.zone,
                        record.data(),
                        current_ip
                    );
                } else {
                    info!(
                        "{} record {}.{} already up to date",
                        record.kind(),
                        record.name(),
                        &backend.zone
                    );
                }
            }
        }
    }
    Ok(())
}

pub fn find_matching_interface(
    record: &Record,
    system_interfaces: &SystemAddresses,
) -> Result<SystemAddress, DnsBackendError> {
    let v4_addresses = system_interfaces.v4_addresses.clone();
    let v6_addresses = system_interfaces.v6_addresses.clone();
    match record.record_type.as_str() {
        "A" => {
            let mut matched_interface: Vec<SystemV4Address> = v4_addresses
                .into_iter()
                .filter(|x| x.interface == record.interface)
                .collect();
            match matched_interface.pop() {
                Some(interface) => Ok(SystemAddress::V4(interface)),
                None => Err(DnsBackendError {
                    message: format!(
                        "Couldn't find interface {} in system interfaces",
                        record.interface
                    ),
                }),
            }
        }
        "AAAA" => {
            let mut matched_interface: Vec<SystemV6Address> = v6_addresses
                .into_iter()
                .filter(|x| x.interface == record.interface)
                .collect();
            match matched_interface.pop() {
                Some(interface) => Ok(SystemAddress::V6(interface)),
                None => Err(DnsBackendError {
                    message: format!(
                        "Couldn't find interface {} in system interfaces",
                        record.interface
                    ),
                }),
            }
        }
        _ => Err(DnsBackendError {
            message: format!(
                "{} is not a valid record type, try \"A\" or \"AAAA\"",
                record.record_type
            ),
        }),
    }
}

#[cfg(test)]
mod tests {
    use std::net::Ipv6Addr;

    use super::*;

    #[test]
    fn test_find_matching_interface_match_v4() {
        let record = Record {
            name: "test_record".to_string(),
            record_type: "A".to_string(),
            interface: "eth0".to_string(),
        };
        let interface = SystemV4Address {
            interface: "eth0".to_string(),
            address: IpAddr::from(std::net::Ipv4Addr::new(10, 1, 1, 1)),
        };
        let interfaces = SystemAddresses {
            v4_addresses: vec![interface.clone()],
            v6_addresses: Vec::new(),
        };

        let result = find_matching_interface(&record, &interfaces);
        assert_eq!(result.unwrap(), SystemAddress::V4(interface))
    }

    #[test]
    fn test_find_matching_interface_match_v6() {
        let record = Record {
            name: "test_record".to_string(),
            record_type: "AAAA".to_string(),
            interface: "eth0".to_string(),
        };
        let interface = SystemV6Address {
            interface: "eth0".to_string(),
            address: IpAddr::from(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1)),
        };
        let interfaces = SystemAddresses {
            v4_addresses: Vec::new(),
            v6_addresses: vec![interface.clone()],
        };
        let result = find_matching_interface(&record, &interfaces);
        assert_eq!(result.unwrap(), SystemAddress::V6(interface))
    }

    #[test]
    fn test_find_matching_interface_no_match() {
        let record = Record {
            name: "test_record".to_string(),
            record_type: "A".to_string(),
            interface: "eth0".to_string(),
        };
        let interface = SystemV4Address {
            interface: "eth1".to_string(),
            address: IpAddr::from(std::net::Ipv4Addr::new(10, 1, 1, 1)),
        };
        let interfaces = SystemAddresses {
            v4_addresses: vec![interface.clone()],
            v6_addresses: Vec::new(),
        };

        let result = find_matching_interface(&record, &interfaces);
        assert_eq!(
            result,
            Err(DnsBackendError {
                message: "Couldn't find interface eth0 in system interfaces".to_string()
            })
        )
    }
}
