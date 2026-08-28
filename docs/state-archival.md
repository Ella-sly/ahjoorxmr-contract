# State Archival Troubleshooting

Stellar/Soroban uses **State Archival** to keep the network's storage footprint
bounded. Every contract instance and contract-data entry has a Time To Live
(TTL), measured in ledgers. If an entry's TTL is not periodically extended, its
TTL eventually reaches `0` and the entry is archived (or, for `Temporary`
storage, permanently deleted).

This guide expands on the [State Archival & TTL](../readMe.md#state-archival--ttl)
section of the main README with concrete steps for checking whether an Ahjoor
contract (or one of its entries) has been archived, restoring it via the Stellar
CLI, and preventing it from happening again.

## How Ahjoor Handles State Archival

Ahjoor's contracts proactively extend TTL on every write path. Each
state-mutating entrypoint calls `extend_ttl()` on the storage it touches so
active contract state is never silently archived mid-operation:

- `Instance` storage (contract configuration, admin, flags) is bumped on every
  write path using the contract's `INSTANCE_LIFETIME_THRESHOLD` /
  `INSTANCE_BUMP_AMOUNT` constants.
- `Persistent` storage (member balances, escrow records, KYB records, etc.) is
  bumped per entry with `PERSISTENT_LIFETIME_THRESHOLD` /
  `PERSISTENT_BUMP_AMOUNT`.

For long periods of inactivity, the `ahjoor-rosca` contract also exposes a
manual `bump_storage()` entrypoint that extends the contract instance TTL
without needing any other state change:

```text
bump_storage()
```

There are three storage types with different archival behaviour:

| Storage type | Archival behaviour |
| --- | --- |
| `Instance`   | Archived when TTL reaches `0`; can be restored. |
| `Persistent` | Archived when TTL reaches `0`; can be restored. |
| `Temporary`  | Permanently deleted when TTL reaches `0`; **cannot** be restored. |

## Checking Archived Status

There is no single "is this archived?" flag exposed by the contract. In
practice, archival is detected by attempting to read state:

### 1. Attempt a read-only invocation

Try calling a read entrypoint (for example the ROSCA contract's `get_state`):

```bash
stellar contract invoke \
  --id <CONTRACT_ID> \
  --source alice \
  --network testnet \
  -- get_state
```

- If the invocation returns normally, the contract instance is live.
- If the invocation fails with an archival / missing-entry error, the instance
  (or a persistent entry the call reads) has been archived and must be restored.

### 2. Read a specific entry

To inspect a single persistent entry (e.g. a member or payment record), use
`stellar contract read`:

```bash
stellar contract read \
  --id <CONTRACT_ID> \
  --key <KEY> \
  --durability persistent \
  --source alice \
  --network testnet
```

An archived entry cannot be read and will surface as a missing entry. Replace
`<KEY>` with the storage key (symbol) you want to inspect.

## Restoring a Contract

When a contract instance (or its backing WASM code) has been archived, restore
it with:

```bash
stellar contract restore \
  --id <CONTRACT_ID> \
  --source alice \
  --network testnet
```

When no key is specified, `stellar contract restore` restores the contract
instance itself (and its code if needed).

### Restoring a specific persistent entry

If only a single persistent entry is archived while the contract instance is
still live, restore just that entry by passing its storage key:

```bash
stellar contract restore \
  --id <CONTRACT_ID> \
  --key <KEY> \
  --durability persistent \
  --source alice \
  --network testnet
```

`--durability` accepts `persistent` (default) or `temporary`. Only `persistent`
and `instance` entries can be restored; `temporary` entries are deleted
permanently and cannot be recovered.

After restoration, the entry's TTL is reset to the network's minimum. Call
`bump_storage()` (or otherwise extend the TTL) immediately so the contract is
not archived again in the short term.

## Preventing Future Archival

### Manual `bump_storage()`

Call the ROSCA contract's `bump_storage()` entrypoint to extend instance TTL
during idle periods:

```bash
stellar contract invoke \
  --id <CONTRACT_ID> \
  --source alice \
  --network testnet \
  -- bump_storage
```

**Recommended frequency:** at least once every **30 days** of inactivity.
Groups that call `contribute` or other state-writing functions regularly do not
need to call it manually — those interactions bump storage automatically.

### CLI-level TTL extension

To extend TTL directly without invoking a contract function, use
`stellar contract extend`:

```bash
stellar contract extend \
  --id <CONTRACT_ID> \
  --ledgers-to-extend 120000 \
  --source alice \
  --network testnet
```

This is equivalent to the TTL bump the contract performs internally and is
useful when the contract instance is still live but you want to push its TTL
out further. The `--ledgers-to-extend` value is subject to the network's maximum
TTL.

## What Is Preserved vs. Lost

- **Preserved and restorable:** contract code, contract instance data, and all
  `persistent` entries (members, balances, escrow records, KYB records, etc.).
- **Permanently lost when archived:** `temporary` entries. Do not rely on
  `temporary` storage for any data that must survive long idle periods.

For a deeper treatment of the protocol behaviour, see the Stellar documentation
on [State Archival](https://developers.stellar.org/docs/learn/fundamentals/contract-development/storage/state-archival).
