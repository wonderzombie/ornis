use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct BrpQueryRequest {
    pub(crate) jsonrpc: String,
    pub(crate) method: String,
    pub(crate) id: serde_json::Value,
    pub(crate) params: QueryParams,
}

impl Default for BrpQueryRequest {
    fn default() -> Self {
        Self {
            jsonrpc: "2.0".into(),
            method: String::default(),
            id: Value::default(),
            params: QueryParams::default(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(crate) struct QueryParams {
    pub(crate) data: QueryData,
    pub(crate) filter: QueryFilter,
    pub(crate) strict: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(crate) struct QueryData {
    pub(crate) components: Vec<String>,
    pub(crate) option: Vec<String>,
    pub(crate) has: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(crate) struct QueryFilter {
    with: Vec<String>,
    without: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct BrpQueryResponse {
    pub(crate) result: Vec<BrpEntity>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct BrpEntity {
    pub(crate) components: HashMap<String, Value>,
    #[serde(rename = "entity")]
    pub(crate) id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct BrpRegistryRequest {
    pub(crate) jsonrpc: String,
    pub(crate) method: String,
    pub(crate) id: serde_json::Value,
    pub(crate) params: RegistryParams,
}

impl Default for BrpRegistryRequest {
    fn default() -> Self {
        Self {
            jsonrpc: "2.0".into(),
            id: Value::default(),
            method: Default::default(),
            params: Default::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(crate) struct RegistryParams {
    pub(crate) with_crates: Vec<String>,
    pub(crate) without_crates: Vec<String>,
    pub(crate) type_limit: RegistryTypeLimit,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(crate) struct RegistryTypeLimit {
    pub with: Vec<String>,
    pub without: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(crate) struct BrpRegistryResponse {
    result: Map<String, Value>,
}
