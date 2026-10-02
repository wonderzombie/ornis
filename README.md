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

Type `q somecomponent` to see a list of entities with that component. You can use an entity ID for subsequent commands like `l {id}` (list components on an entity) or `g {id} componenta componentb` if you copy-paste the ID in place of `{id}` in those examples.

`sr` to search the type registry and ensure the types you want are there. If not, `ornis` should have generated `ornis.toml` in the source root. You can adjust `with_crates` and `without_crates` to ensure (only) the crates you want appear in the `ornis` type registry.

`lreg` if you just want to dump the type registry. It will have a lot of things!

If you set a namespace, either through `setns` or via the config, you can sidestep duplicate names
like `foo::bar::Baz` and `quux::zap::Baz`. This works for `WorldQuery` and `ListComponents`.

When your namespace is set to `foo`, the following will not be confused with `Baz` defined in `quux::zap`:

```
> q ::bar::Baz
```

This is just string shenanigans based on the type registry, so this does NOT set such as a global flag that limits packages to this.

For that, you would want `with_crates` or `without_crates`. See also [registry.schema](https://docs.rs/bevy/latest/bevy/remote/index.html#registryschema) in the BRP docs.

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

This will show `foo::bar::Baz` _and_ `quux::Foo` if they are in the type registry provided by BRP.

### Inspecting resources

This is even less polished if you can believe it.

**List resources:**

```
lsres
```

**Show resources:**

This works now:

```
gres myresource
```

## Commands list / cheat sheet

_This is the built-in help verbatim._

Ornis commands:

- q, wq, world.query, query (WorldQuery)
  query for entities which have one or more component:
  > q name transform +sprite
- lc, l, world.list_components, list_components (ListComponents)
  list components w/ data on a single entity
- lt, lrg, lreg, listreg (ListRegistry)
  show types reported by bevy remote protocol
- sr, sreg, searchreg (SearchRegistry)
  query registry of types via substring match
- rl, rlreg, reloadreg (LoadRpcSchema)
  reload the registry from bevy
- ?, help (PrintHelp)
  print this help
- lsrs, lrs, lsres, listres (ListResources)
  list resources
- grs, gres, getres (GetResources)
  display a resource
- g, gc, gcs, gcom (GetComponents)
  read components for an entity:
  > g [entity_id] mycomponent myothercomponent
- cfg, showcfg (ShowConfig)
  show currently used ornis configuration
- rlcfg (ReloadConfig)
  reload config from disk
- setns (SetNamespace)
  set the in-memory configuration's namespace

An empty result quite often means that the type in question hasn't been configured for reflection, meaning the remote protocol can't send a representation of it.
For `Component` or `Resource`, this can mean deriving Reflect and adding `#[reflect(Component)]` or `Resource`.
However, it won't work if either datatype includes a type which is NOT suitable for reflection or serde.

## Configuration

This is the default config which `ornis` will create for you if you don't have one already:

```toml
current_ns = "wanderrust"
with_crates = [
    "bevy_app",
    "bevy_ecs",
    "bevy_camera",
    "bevy_picking",
    "bevy_state",
    "bevy_ui",
    "bevy_northstar",
    "wanderrust",
]
without_crates = []
max_results = 25
```

You can use `showcfg` to see what config you're using and `rlcfg` to reload it from disk.

If you changed `with_crates` or `without_crates`, you should reload the type registry with `rl`. Nothing will combust but `ornis` won't know about any new types so you won't be able to use the all lowercase shorthand, so you'd have to type `some::path::Foo` instead of just `foo`.
