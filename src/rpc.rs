use std::collections::HashMap;
use std::fmt::Debug;

use log::debug;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::{Map, Value};

use crate::methods::{
    BRP_GET_RESOURCES_METHOD, BRP_LIST_COMPONENTS_METHOD, BRP_LIST_RESOURCES_METHOD,
    BRP_QUERY_METHOD, BRP_REGISTRY_SCHEMA_METHOD,
};

use anyhow::Result;

const JSONRPC_VER: &str = "2.0";
const URL: &str = "http://localhost:15702";

#[derive(Debug, Clone)]
pub(crate) enum Message {
    Query(BrpQueryResponse),
    ListComponents(BrpListComponentsResponse),
    RegistrySchema(BrpRegistrySchemaResponse),
    ListResources(BrpListResourcesResponse),
    GetResources(BrpGetResourcesResponse),
}

pub trait BrpRequestExt: Serialize + Debug {
    type Response: DeserializeOwned + Debug;

    const METHOD: &'static str;
    fn into_message(resp: Self::Response) -> Message;
}

pub fn send<P: BrpRequestExt, T>(req_params: P, f: impl FnOnce(Message) -> T) -> Result<T> {
    let params = serde_json::to_value(req_params)?;
    let req = BrpRequest {
        jsonrpc: JSONRPC_VER.to_string(),
        method: P::METHOD.to_owned(),
        params,
        ..Default::default()
    };

    let client = reqwest::blocking::Client::new();
    let j = client.post(URL).json(&req).send()?.text()?;

    debug!("response: {:?}", j);

    let json = serde_json::from_str::<<P as BrpRequestExt>::Response>(&j)?;

    Ok(f(P::into_message(json)))
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

impl BrpRequestExt for QueryParams {
    type Response = BrpQueryResponse;

    const METHOD: &'static str = BRP_QUERY_METHOD;

    fn into_message(resp: Self::Response) -> Message {
        Message::Query(resp)
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
pub(crate) struct BrpRegistrySchemaResponse {
    pub(crate) result: Map<String, Value>,
}

impl BrpRequestExt for RegistryParams {
    type Response = BrpRegistrySchemaResponse;

    const METHOD: &'static str = BRP_REGISTRY_SCHEMA_METHOD;

    fn into_message(resp: Self::Response) -> Message {
        Message::RegistrySchema(resp)
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

impl BrpRequestExt for ListComponentsParams {
    type Response = BrpListComponentsResponse;

    const METHOD: &'static str = BRP_LIST_COMPONENTS_METHOD;

    fn into_message(resp: Self::Response) -> Message {
        Message::ListComponents(resp)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub(crate) struct BrpListResourcesParams;

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub(crate) struct BrpListResourcesResponse {
    pub(crate) result: Vec<String>,
}

impl BrpRequestExt for BrpListResourcesParams {
    type Response = BrpListResourcesResponse;

    const METHOD: &'static str = BRP_LIST_RESOURCES_METHOD;

    fn into_message(resp: Self::Response) -> Message {
        Message::ListResources(resp)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub(crate) struct BrpGetResourcesParams {
    pub(crate) resource: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub(crate) struct BrpGetResourcesResponse {
    pub(crate) result: Value,
}

impl BrpRequestExt for BrpGetResourcesParams {
    type Response = BrpGetResourcesResponse;

    const METHOD: &'static str = BRP_GET_RESOURCES_METHOD;

    fn into_message(resp: Self::Response) -> Message {
        Message::GetResources(resp)
    }
}
