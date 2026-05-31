#![allow(unused)]

pub const BRP_GET_COMPONENTS_METHOD: &str = "world.get_components";

/// The method path for a `world.query` request.
pub const BRP_QUERY_METHOD: &str = "world.query";

/// The method path for a `world.spawn_entity` request.
pub const BRP_SPAWN_ENTITY_METHOD: &str = "world.spawn_entity";

/// The method path for a `world.insert_components` request.
pub const BRP_INSERT_COMPONENTS_METHOD: &str = "world.insert_components";

/// The method path for a `world.remove_components` request.
pub const BRP_REMOVE_COMPONENTS_METHOD: &str = "world.remove_components";

/// The method path for a `world.despawn_entity` request.
pub const BRP_DESPAWN_COMPONENTS_METHOD: &str = "world.despawn_entity";

/// The method path for a `world.reparent_entities` request.
pub const BRP_REPARENT_ENTITIES_METHOD: &str = "world.reparent_entities";

/// The method path for a `world.list_components` request.
pub const BRP_LIST_COMPONENTS_METHOD: &str = "world.list_components";

/// The method path for a `world.mutate_components` request.
pub const BRP_MUTATE_COMPONENTS_METHOD: &str = "world.mutate_components";

/// The method path for a `world.get_components+watch` request.
pub const BRP_GET_COMPONENTS_AND_WATCH_METHOD: &str = "world.get_components+watch";

/// The method path for a `world.list_components+watch` request.
pub const BRP_LIST_COMPONENTS_AND_WATCH_METHOD: &str = "world.list_components+watch";

/// The method path for a `world.get_resources` request.
pub const BRP_GET_RESOURCE_METHOD: &str = "world.get_resources";

/// The method path for a `world.insert_resources` request.
pub const BRP_INSERT_RESOURCE_METHOD: &str = "world.insert_resources";

/// The method path for a `world.remove_resources` request.
pub const BRP_REMOVE_RESOURCE_METHOD: &str = "world.remove_resources";

/// The method path for a `world.mutate_resources` request.
pub const BRP_MUTATE_RESOURCE_METHOD: &str = "world.mutate_resources";

/// The method path for a `world.list_resources` request.
pub const BRP_LIST_RESOURCES_METHOD: &str = "world.list_resources";

/// The method path for a `world.trigger_event` request.
pub const BRP_TRIGGER_EVENT_METHOD: &str = "world.trigger_event";

/// The method path for a `registry.schema` request.
pub const BRP_REGISTRY_SCHEMA_METHOD: &str = "registry.schema";

/// The method path for a `rpc.discover` request.
pub const RPC_DISCOVER_METHOD: &str = "rpc.discover";
