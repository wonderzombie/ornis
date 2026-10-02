# Ornis

It's an `iced`-based (GUI) client for reading Bevy state at runtime via [Bevy Remote Protocol][brp]. It is
useful for interactive debugging and testing.

It is like a glorified command line running entirely independently of Bevy: it's out of process and
does not depend on any Bevy crates. It's possible BRP could diverge in a way that breaks `ornis`.
That has been an acceptable trade-off for me up to this point.

You can see just how thin a wrapper this is over JSONRPC by checking out BRP's [built-in methods][brp-methods]
and comparing method names to `ornis` command names.

Compare & contrast with [bevy_inspector_egui][bevy-inp-egui], which I also like & use.

[brp]: https://docs.rs/bevy/latest/bevy/remote/index.html
[bevy-insp-egui]: https://docs.rs/bevy-inspector-egui/latest/bevy_inspector_egui/
[brp-methods]: https://docs.rs/bevy/latest/bevy/remote/index.html#built-in-methods

## About

### Why?

It's useful if you want run queries using approximations of `QueryData` and `QueryFilter` without writing new code for any of the particular components.

- Works when your game's UI is unresponsive or nonexistent.
- Search the type registry (substring match).
- Supports a nano-language for such as `With<T>`, `Without<T>`, `Has<T>`, and `Option<T>`.
- Command line history — but only in-memory, so no persistence across invocations.
- Inspect the type registry as reported by Bevy.
- Out of process and no stateful connection: just leave it running, reload the type registry when needed. 
- I wanted to try `iced` and `serde_json`.
- I really like Bevy.

### Caveats

- It's not configurable yet. Fetching the type registry requires an allow/deny list that's unfortunately hardcoded.
- Error handling is very poor still. `ornis` doesn't surface errors hardly at all. If you want to know what went wrong, `ornis` logs the error response from BRP to the console. Otherwise you'll see a generic JSON error. This is a consequence of rolling our own types.
- Searching the type registry is a substring match. It's VERY basic so as to cast a wide net. It also means that there are two entries for each type: `foo::bar::Baz` and `Baz` (except lowercase).
- Crates that don't use reflection have limited support. Things like `Has<T>` and `Option<T>` work but without reflection that's about it.
- Undoubtedly there are other bugs, and not just because this is my first time with `iced`. :P

### Requirements

If you want your own types to show up, they must derive `Reflect`, like:

```rust
#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct Health {
    pub max: u32,
    pub curr: u32,
}
```

You can still query for types from crates that don't support this as long as you're not asking BRP to materialize them (into JSON).

## Quick start

You'll need to clone this repo and use `cargo run`. 

If/when it loads correctly and if your game is running, you'll see something like:

```
registered info for {ntypes} types
```

Type `?` for something approximating help.

If your game wasn't running you'll see a complaint about being unable to connect. Invoke `rl` in the tool and it will load the types. 

`sr` to search the type registry and ensure the types you want are there.

`lreg` if you just want to dump the type registry. It will have a lot of things.

### Querying for components

**Query components** like `NameOrEntity` and `Visibility`:

```
q nameorentity visibility
```

It's just a substring match so the preceding might not work if you have a type of your own that's called `Visibility`.

You can use a longer path if needed.

**Exclude components:**

```
q nameorentity visibility +sprite -dead
```

This is like `Query<(NameOrEntity, &Visibility), (With<Sprite>, Without<Dead>)>`. 

**List an entity's components**

Copy-paste an entity ID `12345` and list components on the entity:

```
l 12345
```

**Examine an entity's components**

Ask to see values of components on an entity:

```
gcom 12345 nameorentity transform sprite
```

### mini-language for queries

`Foo` -> `&Foo`  
`@Foo` -> `Option<&Foo>`  
`#Foo` -> `Has<Foo>`  
`+Foo` -> `With<Foo>`  
`-Foo` -> `Without<Foo>`  

### Querying the type registry

Not querying per se but rather an in-memory map of all-lowercase types.

**Reload the type registry**:

After you make changes to your code, esp when adding new types, you needn't relaunch `ornis`:

```
rl
```

You can also use this if you invoked `ornis` without your game running and `ornis` complains.

**See all types in the type registry**:

And I do mean **all types** that are on the allow list:

```
lrg
```

**See substring match for types in registry**:

```
sreg foo
```

This will show `foo::bar::Baz` *and* `quux::Foo` if they are in the type registry provided by BRP.

### Inspecting resources

This is even less polished if you can believe it.

**List resources:**

```
lsres 
```

**Show resources:**

Doesn't work presently.

```
gres myresource
```

## Commands list / cheat sheet

Ornis commands:

- q, wq, world.query, query (WorldQuery)
  query for entities which have one or more component

```
q name visibility
```

- lc, l, world.list_components, list_components (ListComponents)
  list components w/ data on a single entity

```
l 12345
```

- lrg, lreg, listreg (ListRegistry)
  show types reported by bevy remote protocol
- sr, sreg, searchreg (SearchRegistry)
  query registry of types via substring match

```
sr mycomponent
```

- rl, rlreg, reloadreg (LoadRpcSchema)
  reload the registry from bevy

```
rl
```

- ?, help (PrintHelp)
  print this help
- lrs, lsres, listres (ListResources)
  list resources
- grs, gres, getres (GetResources)
  list resources
- gcs, gcom (GetComponents)
  read components for an entity

```
gcom 12345 name visibility transform
```

- cell (GetCell)
  show information about entities at a given cell

An empty result quite often means that the type in question hasn't been configured for reflection, meaning the remote protocol can't send a representation of it.
For `Component` or `Resource`, this can mean deriving Reflect and adding `#[reflect(Component)]` or `Resource`.
However, it won't work if either datatype includes a type which is NOT suitable for reflection or serde.

## Notes

**CRATE_PATH** 

`CRATE_PATH` is how I am able to omit my crate's name before every path:

```
q ::gamestate::worldclock
```

It's hardcoded but needn't be.

**send_rpc_schema_request()**

This is where we allow/deny crates:

```rust
fn send_rpc_schema_request() -> Result<Message> {
    let params = rpc::RegistryParams {
        with_crates: vec![
            "bevy_app".into(),
            "bevy_ecs".into(),
            "bevy_camera".into(),
            "bevy_picking".into(),
            "bevy_state".into(),
            "bevy_ui".into(),
            "bevy_northstar".into(), // this should not be hardcoded
            "wanderrust".into(), // nor this
        ],
        without_crates: vec!["glam".into()],
        ..Default::default()
    };

    send(params)
}
```

