use cloudflare::endpoints::dns::{
    CreateDnsRecord, CreateDnsRecordParams, DnsContent, DnsRecord, ListDnsRecords,
    ListDnsRecordsParams, UpdateDnsRecord, UpdateDnsRecordParams,
};

use cloudflare::framework::auth::Credentials;
use cloudflare::framework::Environment::Production;
use cloudflare::framework::HttpApiClient;
use cloudflare::framework::HttpApiClientConfig;
use serde_derive::{Deserialize, Serialize};

use crate::dns_providers::{self, DnsBackend, DnsBackendError, DnsRecordType};

pub struct CloudFlareBackend {
    zone_identifier: String,
    client: HttpApiClient,
    zone: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct CloudFlareBackendConfig {
    pub api_token: String,
    pub zone_identifier: String,
    pub zone: String,
}

impl From<DnsRecord> for dns_providers::DnsRecord {
    fn from(value: DnsRecord) -> Self {
        let (record_type, data) = match value.content {
            DnsContent::A { content } => (DnsRecordType::A, content.to_string()),
            DnsContent::AAAA { content } => (DnsRecordType::AAAA, content.to_string()),
            _ => (DnsRecordType::Other, "".to_string()),
        };
        let non_fqdn = value.name.split('.').next().unwrap();
        dns_providers::DnsRecord {
            kind: record_type,
            name: non_fqdn.to_string(),
            data,
        }
    }
}

impl From<CloudFlareBackendConfig> for CloudFlareBackend {
    fn from(value: CloudFlareBackendConfig) -> Self {
        // let credentials = Credentials::UserAuthKey {
        //     email: value.email,
        //     key: value.api_key,
        // };
        let credentials = Credentials::UserAuthToken {
            token: value.api_token,
        };
        let config = HttpApiClientConfig::default();
        let client = HttpApiClient::new(credentials, config, Production).unwrap();
        CloudFlareBackend {
            zone_identifier: value.zone_identifier,
            client,
            zone: value.zone,
        }
    }
}

impl CloudFlareBackend {
    fn _get_zone_records_internal(
        &self,
    ) -> Result<Vec<DnsRecord>, crate::dns_providers::DnsBackendError> {
        let params = ListDnsRecordsParams::default();
        let list_records_request = ListDnsRecords {
            zone_identifier: &self.zone_identifier,
            params,
        };
        match self.client.request(&list_records_request) {
            Ok(resp) => {
                let res_vec = resp.result;
                debug!(
                    "Fetched records from cloudflare for domain {}: {:?}",
                    &self.zone_identifier, res_vec
                );
                Ok(res_vec)
            }
            Err(e) => Err(DnsBackendError {
                message: format!("Dns backend error {e:?}"),
            }),
        }
    }
}

impl DnsBackend for CloudFlareBackend {
    fn zone(&self) -> String {
        self.zone.clone()
    }
    fn get_zone_records(
        &self,
    ) -> Result<Vec<crate::dns_providers::DnsRecord>, crate::dns_providers::DnsBackendError> {
        let records = self._get_zone_records_internal()?;
        Ok(records.into_iter().map(|x| x.into()).collect())
    }
    fn create_record(&self, record: dns_providers::DnsRecord) -> Result<(), DnsBackendError> {
        let content = match record.kind {
            DnsRecordType::A => DnsContent::A {
                content: record.data.parse().unwrap(),
            },
            DnsRecordType::AAAA => DnsContent::AAAA {
                content: record.data.parse().unwrap(),
            },
            _ => {
                return Err(DnsBackendError {
                    message: "Record type not supported".to_string(),
                })
            }
        };
        let params = CreateDnsRecordParams {
            ttl: Some(3600),
            priority: None,
            // TODO: Make proxied configurable
            proxied: Some(false),
            name: &record.name,
            content,
        };
        let create_record_request = CreateDnsRecord {
            zone_identifier: &self.zone_identifier,
            params,
        };
        match self.client.request(&create_record_request) {
            Ok(_) => {
                info!("Created record {create_record_request:?}");
                Ok(())
            }
            Err(e) => Err(DnsBackendError {
                message: format!("Dns backend error while creating {}: {:?}", record.name, e),
            }),
        }
    }
    fn update_record(
        &self,
        record: &dns_providers::DnsRecord,
        new_data: &str,
    ) -> Result<(), DnsBackendError> {
        let records = self._get_zone_records_internal()?;
        let existing_record = records
            .iter()
            .find(|x| x.name.split('.').next().unwrap() == record.name)
            .ok_or(DnsBackendError {
                message: "Tried to update a nonexistant record".to_string(),
            })?;

        let content = match record.kind {
            DnsRecordType::A => DnsContent::A {
                content: new_data.parse().unwrap(),
            },
            DnsRecordType::AAAA => DnsContent::AAAA {
                content: new_data.parse().unwrap(),
            },
            _ => {
                return Err(DnsBackendError {
                    message: "Record type not supported".to_string(),
                })
            }
        };

        let params = UpdateDnsRecordParams {
            ttl: Some(3600),
            proxied: Some(false),
            name: &record.name,
            content,
        };

        let update_record_request = UpdateDnsRecord {
            zone_identifier: &self.zone_identifier,
            identifier: &existing_record.id,
            params,
        };
        match self.client.request(&update_record_request) {
            Ok(_) => {
                info!("Updated record {update_record_request:?}");
                Ok(())
            }
            Err(e) => Err(DnsBackendError {
                message: format!("Dns backend error while updating {}: {:?}", record.name, e),
            }),
        }
    }
}
