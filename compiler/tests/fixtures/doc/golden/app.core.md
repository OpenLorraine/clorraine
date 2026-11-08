# app.core

Core application types.

## Entry

```clorraine
export struct Entry<T>
```

A stored value.

## Failure

```clorraine
export enum Failure : u8
```

A failure code.

## Token

```clorraine
export handle Token freedBy releaseToken
```

An owned handle.

**Related:** [releaseToken](#releasetoken)

## DEFAULT_COUNT

```clorraine
export const DEFAULT_COUNT: u32 = 4
```

The default entry count.

## current

```clorraine
export let current: i32 = 0
```

The current entry.

## releaseToken

```clorraine
export func releaseToken(take token: Token): void
```

Releases a token.

**Related:** [Token](#token)

## makeEntry

```clorraine
export func makeEntry<T>(take value: T): Entry<T>
```

Creates an entry.

The value moves into the entry.

**Related:** [Entry](#entry)

## describe

```clorraine
export func describe(
    borrow entry: Entry<i32>
): void
```

Describes an entry.

**Related:** [Entry](#entry)
