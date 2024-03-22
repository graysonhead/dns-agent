use serde::{Deserialize, Serialize};

use crate::config::ParsedRecord;
use crate::update::{SystemAddress, SystemAddresses, SystemV4Address, SystemV6Address};

use std::fmt;
use std::net::IpAddr;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DnsRecordType {
    A,
    AAAA,
    TXT,
    NS,
    SRV,
    Other,
}

impl From<&str> for DnsRecordType {
    fn from(value: &str) -> Self {
        match value {
            "A" => DnsRecordType::A,
            "AAAA" => DnsRecordType::AAAA,
            "TXT" => DnsRecordType::TXT,
            "NS" => DnsRecordType::NS,
            "SRV" => DnsRecordType::SRV,
            _ => DnsRecordType::Other,
        }
    }
}

impl From<DnsRecordType> for String {
    fn from(value: DnsRecordType) -> Self {
        match value {
            DnsRecordType::A => "A".to_string(),
            DnsRecordType::AAAA => "AAAA".to_string(),
            DnsRecordType::TXT => "TXT".to_string(),
            DnsRecordType::NS => "NS".to_string(),
            DnsRecordType::SRV => "SRV".to_string(),
            DnsRecordType::Other => "Other".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsRecord {
    pub kind: DnsRecordType,
    pub name: String,
    pub data: String,
}

impl DnsRecord {
    fn kind(&self) -> DnsRecordType {
        self.kind.clone()
    }
    pub fn name(&self) -> String {
        self.name.to_string()
    }
    fn data(&self) -> String {
        self.data.to_string()
    }
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

pub trait DnsBackend {
    fn zone(&self) -> String;
    fn get_zone_records(&self) -> Result<Vec<DnsRecord>, DnsBackendError>;
    fn create_record(&self, record: DnsRecord) -> Result<(), DnsBackendError>;
    fn update_record(&self, record: &DnsRecord, new_data: &str) -> Result<(), DnsBackendError>;
}

pub fn update_records<T>(
    backend: T,
    desired_records: Vec<ParsedRecord>,
    system_interfaces: &SystemAddresses,
) -> Result<(), DnsBackendError>
where
    T: DnsBackend,
{
    let current_records = backend.get_zone_records()?;
    for desired_record in desired_records {
        let zone_name = &backend.zone();
        let mut matching_records: Vec<DnsRecord> = current_records
            .clone()
            .into_iter()
            .filter(|x| x.name() == desired_record.name && x.kind() == desired_record.record_type)
            .collect();
        let interface = find_matching_interface(&desired_record, system_interfaces)?;
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
                "Created new {:?} record: {}.{} = {}",
                rec_kind, rec_name, &zone_name, rec_data
            );
        } else if matching_records.len() == 1 {
            if let Some(record) = matching_records.pop() {
                let current_ip = IpAddr::from(interface).to_string();
                if record.data() != current_ip {
                    backend.update_record(&record, &current_ip)?;
                    info!(
                        "Updated {:?} record {}.{}: old {} new {}",
                        record.kind(),
                        record.name(),
                        &zone_name,
                        record.data(),
                        current_ip
                    );
                } else {
                    info!(
                        "{:?} record {}.{} already up to date",
                        record.kind(),
                        record.name(),
                        &zone_name
                    );
                }
            }
        }
    }
    Ok(())
}

pub fn find_matching_interface(
    record: &ParsedRecord,
    system_interfaces: &SystemAddresses,
) -> Result<SystemAddress, DnsBackendError> {
    let v4_addresses = system_interfaces.v4_addresses.clone();
    let v6_addresses = system_interfaces.v6_addresses.clone();
    match record.record_type {
        DnsRecordType::A => {
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
        DnsRecordType::AAAA => {
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
                "{:?} is not a valid record type, try \"A\" or \"AAAA\"",
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
        let record = ParsedRecord {
            name: "test_record".to_string(),
            record_type: DnsRecordType::A,
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
        let record = ParsedRecord {
            name: "test_record".to_string(),
            record_type: DnsRecordType::AAAA,
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
        let record = ParsedRecord {
            name: "test_record".to_string(),
            record_type: DnsRecordType::A,
            interface: "eth0".to_string(),
        };
        let interface = SystemV4Address {
            interface: "eth1".to_string(),
            address: IpAddr::from(std::net::Ipv4Addr::new(10, 1, 1, 1)),
        };
        let interfaces = SystemAddresses {
            v4_addresses: vec![interface],
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
