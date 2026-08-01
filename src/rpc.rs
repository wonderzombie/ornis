use std::collections::HashMap;

use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::{Map, Value};

use crate::{
    JSONRPC_VER, Message, URL,
    methods::{BRP_LIST_COMPONENTS_METHOD, BRP_QUERY_METHOD, BRP_REGISTRY_SCHEMA_METHOD},
};

use anyhow::Result;

trait BrpRequestExt {
    type Response: DeserializeOwned;

    const METHOD: &'static str;
    fn into_message(resp: Self::Response) -> Message;
}

fn send<R: BrpRequestExt + Serialize>(req: R) -> Result<Message> {
    let params = serde_json::to_value(req)?;
    let req = BrpRequest {
        jsonrpc: JSONRPC_VER.to_string(),
        method: R::METHOD.to_owned(),
        params: params,
        ..Default::default()
    };

    let client = reqwest::blocking::Client::new();
    let j = client.post(URL).json(&req).send()?.json()?;

    Ok(R::into_message(j))
}

fn decode<T: DeserializeOwned>(value: Value, wrap: impl FnOnce(T) -> Message) -> Result<Message> {
    Ok(wrap(serde_json::from_value::<T>(value)?))
}

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

impl BrpRequestExt for BrpQueryResponse {
    type Response = Self;

    const METHOD: &'static str = BRP_QUERY_METHOD;

    fn into_message(resp: Self::Response) -> Message {
        Message::QueryResults(resp)
    }
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

impl BrpRequestExt for BrpRegistryResponse {
    type Response = Self;

    const METHOD: &'static str = BRP_REGISTRY_SCHEMA_METHOD;

    fn into_message(resp: Self::Response) -> Message {
        Message::LoadRegistry(resp)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub(crate) struct ListComponentsParams {
    pub(crate) entity: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub(crate) struct BrpListComponentsResponse {
    pub(crate) result: Vec<String>,
}

impl BrpRequestExt for BrpListComponentsResponse {
    type Response = Self;

    const METHOD: &'static str = BRP_LIST_COMPONENTS_METHOD;

    fn into_message(resp: Self::Response) -> Message {
        Message::ComponentsList(resp)
    }
}
