# Ornis

It's an `iced`-based client for the Bevy Remote Protocol. It's very thinly veiled.

## Commands

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

### mini-language for queries

`Foo` -> `&Foo`
`@Foo` -> `Option<&Foo>`
`#Foo` -> `Has<Foo>`
`+Foo` -> `With<Foo>`
`-Foo` -> `Without<Foo>`
