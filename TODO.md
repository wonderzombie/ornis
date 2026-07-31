
- Config for setting defaults (instead of hardcoding them)
    - Dump a default config?
- Ingest modules, namespaces, etc; match parameters like `Components` against this list
    - ...and prefer the "local" namespace? e.g. wanderrust

- Tab completion: press tab, try to match types we know about

**Mini-micro-fancy-token language for making certain queries easier**

Mostly `world.query`.

So the args are something like this, like in `Query`. We have QueryData and we have QueryFilter. 

QueryData is that:
- list of required components
- list of optional components
- list of whether the entity `has` a requested component

QueryFilter is that:
- list of components the entity must have
- list of compoments the entity musn't have

So let's start by saying that a required component called `Komponent` we'd just say `Komponent`. 

- `Komponent` - required
- `@Komponent` - optional
- `#Komponent` - has

- `+Komponent` - with
- `-Komponent` - without

It's hacky af but extremely practical.
