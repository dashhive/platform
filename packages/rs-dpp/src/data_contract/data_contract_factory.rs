use anyhow::anyhow;
use serde_json::{Number, Value as JsonValue};
use std::collections::BTreeMap;
use std::convert::TryFrom;

use crate::{
    data_contract::{self, generate_data_contract_id},
    decode_protocol_entity_factory::DecodeProtocolEntityFactory,
    errors::{consensus::ConsensusError, ProtocolError},
    mocks,
    prelude::Identifier,
    util::entropy_generator,
};

use super::DataContract;

pub struct DataContractFactory<SR> {
    dpp: mocks::DashPlatformProtocol<SR>,
    _validate_data_contract: mocks::ValidateDataContract,
    decode_protocol_entity: DecodeProtocolEntityFactory,
}

impl<SR> DataContractFactory<SR> {
    pub fn new(
        dpp: mocks::DashPlatformProtocol<SR>,
        _validate_data_contract: mocks::ValidateDataContract,
        decode_protocol_entity: DecodeProtocolEntityFactory,
    ) -> Self {
        Self {
            dpp,
            _validate_data_contract,
            decode_protocol_entity,
        }
    }

    /// Create Data Contract
    pub fn create(
        &self,
        owner_id: Identifier,
        documents: JsonValue,
    ) -> Result<DataContract, ProtocolError> {
        let entropy = entropy_generator::generate();
        let data_contract_id =
            Identifier::from_bytes(&generate_data_contract_id(owner_id.to_buffer(), entropy))?;

        let mut documents_map: BTreeMap<String, JsonValue> = BTreeMap::new();
        if let JsonValue::Object(documents) = documents {
            for (document_name, value) in documents {
                documents_map.insert(document_name, value);
            }
        } else {
            return Err(ProtocolError::Generic(String::from(
                "attached documents are not in form a map",
            )));
        }

        let data_contract = DataContract {
            protocol_version: self.dpp.get_protocol_version(),
            schema: String::from(data_contract::SCHEMA),
            id: data_contract_id,
            version: 1,
            owner_id,
            documents: documents_map,
            defs: BTreeMap::new(),
            entropy,

            ..Default::default()
        };
        Ok(data_contract)
    }

    /// Create Data Contract from plain object
    pub async fn create_from_object(
        &self,
        raw_data_contract: JsonValue,
        skip_validation: bool,
    ) -> Result<DataContract, ProtocolError> {
        if !skip_validation {
            let result = self
                ._validate_data_contract
                .validate_data_contract(&raw_data_contract)
                .await;
            if !result.is_valid() {
                return Err(ProtocolError::InvalidDataContractError {
                    errors: result.errors,
                    raw_data_contract,
                });
            }
        }
        DataContract::try_from(raw_data_contract)
    }

    /// Create Data Contract from buffer
    pub async fn create_from_buffer(
        &self,
        buffer: Vec<u8>,
        skip_validation: bool,
    ) -> Result<DataContract, ProtocolError> {
        let (protocol_version, mut raw_data_contract) =
            self.decode_protocol_entity.decode_protocol_entity(buffer)?;

        match raw_data_contract {
            JsonValue::Object(ref mut m) => m.insert(
                String::from("protocolVersion"),
                JsonValue::Number(Number::from(protocol_version)),
            ),
            _ => {
                return Err(ConsensusError::SerializedObjectParsingError {
                    parsing_error: anyhow!("the '{:?}' is not a map", raw_data_contract),
                }
                .into())
            }
        };

        self.create_from_object(raw_data_contract, skip_validation)
            .await
    }

    // TODO
    //   /**
    //    * Create Data Contract Create State Transition
    //    *
    //    * @param {DataContract} dataContract
    //    * @return {DataContractCreateTransition}
    //    */
    //   createDataContractCreateTransition(dataContract) {
    //     return new DataContractCreateTransition({
    //       protocolVersion: this.dpp.getProtocolVersion(),
    //       dataContract: dataContract.toObject(),
    //       entropy: dataContract.getEntropy(),
    //     });
    //   }

    //   /**
    //    * Create Data Contract Update State Transition
    //    *
    //    * @param {DataContract} dataContract
    //    * @return {DataContractUpdateTransition}
    //    */
    //   createDataContractUpdateTransition(dataContract) {
    //     return new DataContractUpdateTransition({
    //       protocolVersion: this.dpp.getProtocolVersion(),
    //       dataContract: dataContract.toObject(),
    //     });
    //   }
}
