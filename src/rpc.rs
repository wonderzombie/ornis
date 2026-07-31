use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct BrpRequest {
    pub(crate) jsonrpc: String,
    pub(crate) method: String,
    pub(crate) id: serde_json::Value,
    pub(crate) params: serde_json::Value,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub(crate) struct QueryParams {
    pub(crate) data: QueryData,
    pub(crate) filter: QueryFilter,
    pub(crate) strict: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub(crate) struct QueryData {
    pub(crate) components: Vec<String>,
    pub(crate) option: Vec<String>,
    pub(crate) has: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub(crate) struct QueryFilter {
    pub(crate) with: Vec<String>,
    pub(crate) without: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct BrpQueryResponse {
    pub(crate) result: Vec<BrpEntity>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct BrpEntity {
    pub(crate) components: HashMap<String, Option<Value>>,
    #[serde(rename = "entity")]
    pub(crate) id: i64,
    pub(crate) has: Option<HashMap<String, bool>>,
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

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub(crate) struct BrpRegistryResponse {
    pub(crate) result: Map<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub(crate) struct ListComponentsParams {
    pub(crate) entity: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub(crate) struct BrpListComponentsResponse {
    pub(crate) result: Vec<String>,
}
