---
title: "TLDraw Sync: Real-Time Collaboration Architecture"
description: A survey of tldraw's sync architecture, data model, protocol, and conflict resolution.
tags:
  - internal
  - wg
  - research
format: md
---

# TLDraw Sync: Real-Time Collaboration Architecture

> Research document covering tldraw's sync engine — architecture, data model,
> protocol, and conflict resolution strategy.
>
> **Source repo:** `tldraw/tldraw` (main branch, ~2024-2026)
> **Key packages:** `@tldraw/store`, `@tldraw/sync-core`, `@tldraw/sync`

---

## 1. Architecture Overview

TLDraw sync uses a **server-authoritative, push/pull/rebase** model — not a
true CRDT. The architecture is closer to a centralized version control system
(git-like optimistic rebase) than to a peer-to-peer CRDT mesh.

### Package Layering

```
@tldraw/store          — Generic record store with typed IDs, diffs, and history
@tldraw/sync-core      — Protocol types, TLSyncRoom (server), TLSyncClient, storage interfaces
@tldraw/sync           — React hook (useSync) that wires TLSyncClient to a TLStore
@tldraw/tlschema       — Schema definitions, migrations, record types for tldraw shapes
```

### Topology

```
  ┌──────────┐    WebSocket    ┌─────────────┐    WebSocket    ┌──────────┐
  │  Client A │◄──────────────►│ TLSyncRoom  │◄──────────────►│ Client B │
  │ TLSyncCli-│                │  (server)   │                │ TLSyncCli-│
  │   ent     │                │             │                │   ent     │
  │           │                │  Storage    │                │           │
  │  TLStore  │                │  (InMemory/ │                │  TLStore  │
  │  (local)  │                │   SQLite)   │                │  (local)  │
  └──────────┘                 └─────────────┘                └──────────┘
```

- **One `TLSyncRoom` per document** — this is enforced as a hard invariant.
  On Cloudflare, Durable Objects guarantee single-instance-per-room.
- **Server holds authoritative state** in a pluggable `TLSyncStorage` backend.
- **Clients hold optimistic local state** and rebase against the server.

---

## 2. Data Model: Records and Store

### `@tldraw/store` — The Record Store

Everything in tldraw is a **record** — a flat JSON object identified by a typed
ID string (e.g. `shape:abc123`, `page:page1`, `instance_presence:xyz`).

```ts
interface BaseRecord<TypeName extends string> {
  id: ID<this>
  typeName: TypeName
}

// Example
interface TLShape extends BaseRecord<'shape'> {
  x: number
  y: number
  props: { ... }
}
```

Key design decisions:

- **Flat record map** — the store is `Map<string, Record>`, not a tree.
  Parent-child relationships are expressed via fields on the records themselves.
- **Typed IDs** — IDs carry their record type in the TypeScript type system
  (`ID<TLShape>` is a branded string like `"shape:abc123"`).
- **Scoped record types** — each record type has a `scope`:
  - `'document'` — persisted and synced (shapes, pages, etc.)
  - `'presence'` — ephemeral, not persisted (cursors, selections)
- **History tracking** — the store emits `RecordsDiff` on every change,
  capturing `added`, `updated` (with `[from, to]` pairs), and `removed`.
- **`mergeRemoteChanges(fn)`** — applies changes from remote without
  triggering the `'user'` source listener (prevents echo loops).

### `RecordsDiff<R>`

The reversible diff format used internally:

```ts
interface RecordsDiff<R> {
  added: Record<string, R>;
  updated: Record<string, [from: R, to: R]>;
  removed: Record<string, R>;
}
```

This is the **internal** diff — verbose but reversible. It's what the client
uses for undo/redo and speculative rebase.

---

## 3. Diff & Patch: The Network Format

### `NetworkDiff<R>` — compact, non-reversible

For wire transmission, tldraw converts `RecordsDiff` into a compact
`NetworkDiff` that doesn't carry the "from" state:

```ts
interface NetworkDiff<R> {
  [id: string]: RecordOp<R>;
}

type RecordOp<R> =
  | ["put", R] // full record replacement or creation
  | ["patch", ObjectDiff] // partial property update
  | ["remove"]; // deletion
```

### `ObjectDiff` — property-level diffing

```ts
interface ObjectDiff {
  [key: string]: ValueOp;
}

type ValueOp =
  | ["put", value] // replace value
  | ["delete"] // remove key
  | ["patch", ObjectDiff] // nested object diff
  | ["append", value[] | string, offset]; // append to array/string
```

Key behaviors in `diffRecord()`:

- **Nested keys** `props` and `meta` are always diff'd recursively (not replaced wholesale).
- **Arrays**: If same length, patches up to `len/5` elements; if longer, uses `append` op.
- **Strings**: If `nextValue.startsWith(prevValue)`, emits an `append` op (protocol v8+).
- **Everything else**: deep equality check → `put` if different.

The `append` op is significant — it allows efficient incremental sync of text
content and array growth without sending the full value.

---

## 4. Sync Protocol

### Protocol Version

Current: **v8** (`TLSYNC_PROTOCOL_VERSION = 8`). Backward compat is handled
with shims (v5→v6→v7→v8 normalization in `handleConnectRequest`).

### Message Types

**Client → Server:**

| Type      | Purpose                                                  |
| --------- | -------------------------------------------------------- |
| `connect` | Handshake with schema, protocol version, lastServerClock |
| `push`    | Send local changes (document diff + presence op)         |
| `ping`    | Keep-alive                                               |

**Server → Client:**

| Type                    | Purpose                                                           |
| ----------------------- | ----------------------------------------------------------------- |
| `connect`               | Handshake response with full/partial diff, schema, serverClock    |
| `patch`                 | Broadcast of changes from other clients                           |
| `push_result`           | Ack for a client's push: `commit`, `discard`, or `rebaseWithDiff` |
| `pong`                  | Keep-alive response                                               |
| `data`                  | Batched array of `patch` and `push_result` messages               |
| `custom`                | Application-defined messages                                      |
| `incompatibility_error` | Legacy error (deprecated, replaced by WS close codes)             |

### Connection Handshake

```
Client                          Server
  |                               |
  |--- connect {                  |
  |      protocolVersion,         |
  |      schema,                  |
  |      lastServerClock,         |
  |      connectRequestId         |
  |    } ─────────────────────►   |
  |                               |  (validate version, migrate schema)
  |                               |  (compute diff since lastServerClock)
  |   ◄────────────────────────   |
  |    connect {                  |
  |      hydrationType:           |
  |        'wipe_all' |           |
  |        'wipe_presence',       |
  |      diff: NetworkDiff,       |
  |      schema,                  |
  |      serverClock,             |
  |      isReadonly               |
  |    }                          |
  |                               |
```

- `hydrationType: 'wipe_presence'` — client keeps its document state, server
  sends only changes since `lastServerClock`. (Normal reconnect.)
- `hydrationType: 'wipe_all'` — client must discard all local state and
  hydrate from scratch. (Happens when tombstone history is too old.)

### Push/Ack Cycle

```
Client                          Server
  |                               |
  |--- push {                     |
  |      clientClock: 5,          |
  |      diff: { ... },           |
  |      presence: [op, data]     |
  |    } ─────────────────────►   |
  |                               |  (validate, migrate up, apply to storage)
  |                               |  (broadcast to other clients)
  |   ◄────────────────────────   |
  |    push_result {              |
  |      clientClock: 5,          |
  |      serverClock: 42,         |
  |      action: 'commit' |       |
  |              'discard' |      |
  |              { rebaseWithDiff }|
  |    }                          |
```

Three possible outcomes:

- **`commit`** — server accepted the diff exactly as sent.
- **`discard`** — server ignored the diff (no effective changes).
- **`rebaseWithDiff`** — server modified the records (validation, normalization)
  and returns the actual diff the client should use instead.

### Presence

Presence records (cursors, selections) are:

- Stored **in-memory only** on the server (`PresenceStore` — not in `TLSyncStorage`).
- Sent as part of `push` messages alongside document diffs.
- **Not persisted** — wiped on reconnect (hence `wipe_presence` hydration type).
- Scoped to a **session** — each session gets a unique `presenceId`.

### Message Batching

Server debounces data messages at **60 fps** (`DATA_MESSAGE_DEBOUNCE_INTERVAL = 1000/60`).
Multiple `patch` and `push_result` messages are batched into a single
`{ type: 'data', data: [...] }` frame.

---

## 5. Conflict Resolution: Optimistic Rebase

TLDraw uses **optimistic concurrency with server-authoritative rebase** — not
CRDTs, not OT, not last-write-wins.

### Client-Side Mechanics (`TLSyncClient`)

The client maintains:

- `speculativeChanges: RecordsDiff<R>` — accumulated unconfirmed local changes
- `pendingPushRequests: TLPushRequest[]` — in-flight pushes awaiting server ack
- `unsentChanges` — buffered changes not yet sent

**Rebase algorithm** (runs at ~30fps when collaborative, ~1fps when solo):

```
1. Flush store history
2. Undo speculative changes (apply reverse diff)
3. Apply all incoming server events in order:
   - For 'patch': apply the NetworkDiff
   - For 'push_result':
     - 'commit': apply the original push diff as confirmed
     - 'discard': drop the push
     - 'rebaseWithDiff': apply the server's corrected diff instead
4. Re-apply remaining pending pushes + unsent changes
5. The resulting delta becomes the new speculativeChanges
```

This is essentially the same pattern as `git rebase`:

- Undo your local commits
- Fast-forward to the server's state
- Re-apply your commits on top

### Server-Side Conflict Resolution (`TLSyncRoom`)

The server is the **single source of truth**. When it receives a push:

1. **Migrate up** — if client is on an older schema version, migrate the
   records up to the current server schema.
2. **Validate** — run the record type's `props` validator.
3. **Apply** — write to storage via a transaction.
4. **Diff** — compute the actual diff between old and new state.
5. **Respond** — if the applied diff matches the push exactly → `commit`.
   If it differs (server normalized data) → `rebaseWithDiff`.
   If no changes resulted → `discard`.
6. **Broadcast** — send the actual diff to all other connected clients,
   migrating down to each client's schema version if needed.

### Schema Migration During Sync

A critical feature: clients on different schema versions can collaborate.
The server:

- Migrates incoming records **up** from the client's version.
- Migrates outgoing diffs **down** to each client's version.
- Each session tracks its `serializedSchema` and `requiresDownMigrations` flag.

---

## 6. Storage Layer

### Interface: `TLSyncStorage<R>`

```ts
interface TLSyncStorage<R> {
  transaction<T>(callback, opts?): TLSyncStorageTransactionResult<T, R>;
  getClock(): number;
  onChange(callback): () => void;
  getSnapshot?(): RoomSnapshot;
}
```

Transactions are **synchronous** — no async allowed. This simplifies
consistency guarantees (no need for distributed locks).

### Clock System

- **`documentClock`** — monotonically incrementing counter. Bumped on every
  write transaction. Used for change tracking.
- Each document record stores its `lastChangedClock`.
- **Tombstones** — deleted record IDs mapped to their deletion clock.
  Used to inform reconnecting clients of deletions.
- **`tombstoneHistoryStartsAtClock`** — pruning boundary. If a client's
  `lastServerClock` is older than this, they must do a full `wipe_all` resync.
- Tombstones pruned when count > 5000 (with 1000 buffer).

### Implementations

1. **`InMemorySyncStorage`** — Default. Uses `AtomMap` (reactive maps from
   `@tldraw/state`). Data lost on process restart. Supports `onChange` callback
   for external persistence.

2. **`SQLiteSyncStorage`** — Production-recommended. Persists to SQLite.
   Supports Cloudflare Durable Objects (`DurableObjectSqliteSyncWrapper`)
   and Node.js (`NodeSqliteWrapper` for `better-sqlite3` or `node:sqlite`).

### `RoomSnapshot`

The serialization format for persisting room state:

```ts
interface RoomSnapshot {
  clock?: number;
  documentClock?: number;
  documents: Array<{ state: UnknownRecord; lastChangedClock: number }>;
  tombstones?: Record<string, number>;
  tombstoneHistoryStartsAtClock?: number;
  schema?: SerializedSchema;
}
```

---

## 7. Server Wrapper: `TLSocketRoom`

`TLSocketRoom` (in `TLSocketRoom.ts`) is the public-facing server class that
wraps `TLSyncRoom` and handles:

- WebSocket lifecycle
- Session management
- Storage configuration
- Snapshot extraction for persistence

`TLSyncRoom` (internal) handles:

- Connection handshake
- Push processing
- Broadcast to connected sessions
- Session pruning (idle timeout, awaiting removal)
- Schema migration per-session

### Session States

```
AwaitingConnectMessage → Connected → AwaitingRemoval → (removed)
```

- `AwaitingConnectMessage`: socket open but no handshake yet (10s timeout).
- `Connected`: actively syncing.
- `AwaitingRemoval`: socket closed, waiting for reconnect (10s grace period).

---

## 8. Client Integration: `useSync` Hook

The React hook creates and manages:

1. A `ClientWebSocketAdapter` (reconnecting WebSocket wrapper)
2. A `TLStore` with schema, assets, and user configuration
3. A `TLSyncClient` that bridges the socket ↔ store
4. A presence derivation that reactively computes cursor/selection state

Returns `RemoteTLStoreWithStatus`:

- `{ status: 'loading' }` — connecting
- `{ status: 'synced-remote', connectionStatus, store }` — active
- `{ status: 'error', error }` — failed

---

## 9. Pros and Cons

### Pros

| Aspect                    | Detail                                                                          |
| ------------------------- | ------------------------------------------------------------------------------- |
| **Simplicity**            | No CRDT library needed. Record-level granularity is natural for canvas objects. |
| **Server authority**      | Single source of truth eliminates divergence. Easy to reason about consistency. |
| **Schema migrations**     | Built-in version skew handling — clients on different versions can collaborate. |
| **Efficient diffs**       | Property-level patching with append ops minimizes wire traffic.                 |
| **Predictable conflicts** | Rebase model is well-understood (git analogy). Server always wins.              |
| **Low latency**           | Optimistic local application + 60fps server batching = responsive UI.           |
| **Flexible storage**      | Pluggable backend (in-memory, SQLite, custom).                                  |
| **Presence separation**   | Ephemeral presence data kept out of persistent storage.                         |

### Cons

| Aspect                         | Detail                                                                                                                                            |
| ------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Server required**            | No peer-to-peer or offline-first without a server. Single point of failure per room.                                                              |
| **No true CRDT**               | Concurrent edits to the same record field → last-write-wins via server. No automatic merge of, e.g., concurrent text edits within a single field. |
| **Record-level granularity**   | Two users editing different properties of the same shape may conflict. The rebase resolves this, but the loser's change can be lost.              |
| **Single-room-single-process** | Must guarantee exactly one `TLSyncRoom` per document globally. Requires Durable Objects or similar coordination.                                  |
| **No partial sync**            | Reconnect sends all changes since `lastServerClock` (or full state if too old). No sub-document subscriptions.                                    |
| **Synchronous transactions**   | Storage layer must be synchronous (no async DB calls in transactions).                                                                            |
| **No offline persistence**     | Client doesn't persist optimistic state. If browser tab closes during offline, speculative changes are lost.                                      |
| **Tombstone growth**           | Deleted records tracked as tombstones with clock values. Requires periodic pruning.                                                               |

---

## 10. Key Constants

| Constant                         | Value           | Purpose                                        |
| -------------------------------- | --------------- | ---------------------------------------------- |
| `TLSYNC_PROTOCOL_VERSION`        | 8               | Wire protocol version                          |
| `DATA_MESSAGE_DEBOUNCE_INTERVAL` | ~16ms (1000/60) | Server message batching                        |
| `COLLABORATIVE_MODE_FPS`         | 30              | Client sync rate with collaborators            |
| `SOLO_MODE_FPS`                  | 1               | Client sync rate when alone                    |
| `PING_INTERVAL`                  | 5000ms          | Client→server keepalive                        |
| `SESSION_IDLE_TIMEOUT`           | (configurable)  | Server prunes idle sessions                    |
| `SESSION_START_WAIT_TIME`        | 10000ms         | Time to wait for connect message               |
| `SESSION_REMOVAL_WAIT_TIME`      | 10000ms         | Grace period before removing cancelled session |
| `MAX_TOMBSTONES`                 | 5000            | Trigger tombstone pruning                      |
| `TOMBSTONE_PRUNE_BUFFER_SIZE`    | 1000            | Extra tombstones pruned beyond threshold       |

---

## 11. Source References

| File                                                   | Description                                                    |
| ------------------------------------------------------ | -------------------------------------------------------------- |
| `packages/store/src/lib/Store.ts`                      | Core record store with history, diffs, and listeners           |
| `packages/sync-core/src/lib/protocol.ts`               | Protocol message type definitions                              |
| `packages/sync-core/src/lib/diff.ts`                   | `NetworkDiff`, `ObjectDiff`, `diffRecord`, `applyObjectDiff`   |
| `packages/sync-core/src/lib/TLSyncRoom.ts`             | Server-side room: session management, push handling, broadcast |
| `packages/sync-core/src/lib/TLSyncClient.ts`           | Client-side sync: rebase, push queue, presence                 |
| `packages/sync-core/src/lib/TLSyncStorage.ts`          | Storage interface, transaction types, snapshot loading         |
| `packages/sync-core/src/lib/InMemorySyncStorage.ts`    | In-memory storage with tombstone pruning                       |
| `packages/sync-core/src/lib/SQLiteSyncStorage.ts`      | SQLite-backed persistent storage                               |
| `packages/sync-core/src/lib/TLSocketRoom.ts`           | Public server wrapper class                                    |
| `packages/sync/src/useSync.ts`                         | React hook for client-side integration                         |
| `packages/sync-core/src/lib/ClientWebSocketAdapter.ts` | Reconnecting WebSocket with chunking                           |
| `packages/sync-core/src/lib/RoomSession.ts`            | Session state machine and timeouts                             |

---

## 12. Relevance to Grida

### What could be borrowed

| TLDraw Concept                          | Grida Equivalent      | Notes                                    |
| --------------------------------------- | --------------------- | ---------------------------------------- |
| Record-based flat store                 | Grida node store      | Natural fit for canvas objects           |
| `NetworkDiff` with patch/put/remove ops | Wire diff format      | Efficient for scene graph changes        |
| Property-level `ObjectDiff` with append | Fine-grained sync     | Good for text content in shapes          |
| Server-authoritative rebase model       | —                     | Simpler than CRDT for structured records |
| Schema migration during sync            | —                     | Critical for versioned deployments       |
| Presence as ephemeral separate scope    | Cursor/selection sync | Keeps persistence layer clean            |
| Tombstone-based deletion tracking       | —                     | Simple clock-based change detection      |
| 60fps server batching                   | —                     | Prevents message flood                   |

### What would differ

| Aspect             | TLDraw                  | Grida Consideration                                     |
| ------------------ | ----------------------- | ------------------------------------------------------- |
| Rendering          | DOM/SVG (JS)            | Skia/Rust (WASM) — store lives in different process     |
| Data format        | JSON records            | FlatBuffers (.grida format) — need serialization bridge |
| Storage            | JS in-memory / SQLite   | Supabase (PostgreSQL) — async, not synchronous          |
| Offline            | None                    | Grida may want offline-first with local persistence     |
| Scale              | Per-room single process | May need multi-process for large documents              |
| Text collaboration | Append ops on strings   | May need richer text CRDT (e.g., Yjs for rich text)     |
