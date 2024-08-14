use crate::dns_providers::{DnsBackend, DnsBackendError, DnsRecord};
use digitalocean::api::{Domain, DomainRecord};
use digitalocean::request::Executable;
use digitalocean::DigitalOcean;
use serde_derive::{Deserialize, Serialize};

pub struct DigitalOceanBackend {
    client: DigitalOcean,
    zone: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct DigitalOceanBackendConfig {
    pub api_key: String,
}

impl DigitalOceanBackend {
    pub fn new(api_key: String, zone: String) -> Result<Self, digitalocean::error::Error> {
        match DigitalOcean::new(api_key) {
            Ok(client) => Ok(Self { client, zone }),
            Err(e) => Err(e),
        }
    }

    fn _get_records_internal(&self) -> Result<Vec<DomainRecord>, DnsBackendError> {
        match Domain::get(&self.zone).records().execute(&self.client) {
            Ok(records) => {
                debug!(
                    "Fetched records from digital ocean for domain {}: {:?}",
                    &self.zone, records
                );
                let results = records.to_vec();
                Ok(results)
            }
            Err(e) => Err(DnsBackendError {
                message: format!("Dns backend error: {:?}", e),
            }),
        }
    }
}

impl DnsBackend for DigitalOceanBackend {
    fn zone(&self) -> String {
        self.zone.clone()
    }
    fn get_zone_records(&self) -> Result<Vec<DnsRecord>, DnsBackendError> {
        let records = &self._get_records_internal()?;
        Ok(records.iter().map(|x| DnsRecord::from(x.clone())).collect())
    }

    fn create_record(&self, record: DnsRecord) -> Result<(), DnsBackendError> {
        let result = Domain::get(&self.zone)
            .records()
            .create(record.kind.into(), record.name, record.data)
            .execute(&self.client);
        match result {
            Ok(_) => Ok(()),
            Err(e) => Err(DnsBackendError {
                message: format!("Failed to create DNS record: {:?}", e),
            }),
        }
    }

    #[allow(suspicious_double_ref_op)]
    fn update_record(&self, record: &DnsRecord, new_data: &str) -> Result<(), DnsBackendError> {
        let current_records = self._get_records_internal()?;
        let existing_record = current_records
            .iter()
            .find(|x| {
                // Intentionally cloning the reference here, because closures are weird
                let local_record = x.clone();
                let conv = DnsRecord::from(local_record.clone());
                conv.name() == record.name && conv.kind == record.kind
            })
            .ok_or(DnsBackendError {
                message: "Tried to update a nonexistant record".to_string(),
            })?;
        let result = Domain::get(&self.zone)
            .records()
            .update(*existing_record.id())
            .data(new_data)
            .execute(&self.client);
        match result {
            Ok(_) => Ok(()),
            Err(e) => Err(DnsBackendError {
                message: format!(
                    "Failed to update DNS record {existing_record:?} with value {new_data}: {:?}",
                    e
                ),
            }),
        }
    }
}

impl From<DomainRecord> for DnsRecord {
    fn from(value: DomainRecord) -> Self {
        DnsRecord {
            kind: value.kind().as_str().into(),
            name: value.name().to_string(),
            data: value.data().to_string(),
        }
    }
}
