# ROSCA Group Snapshots

The `ahjoor-rosca` contract provides on-chain group snapshots for immutable audit and state recovery. A snapshot records the group's state at the time it is taken. Snapshots are append-only: taking a later snapshot does not modify an earlier one.

## Creating a Snapshot

An administrator or group member can create a snapshot by calling:

```text
take_snapshot(caller) -> snapshot_id
```

The caller must authenticate and must be either the configured group administrator or a group member. The returned `snapshot_id` is assigned sequentially, starting at `0`.

An administrator can configure a minimum interval between snapshots with:

```text
set_min_snapshot_interval(admin, interval_ledgers)
```

When the interval is greater than zero, a snapshot taken before that many ledgers have elapsed since the previous snapshot is rejected with `SnapshotTooSoon`. The default interval is zero.

Each successful snapshot emits a `SnapshotTaken` event containing the snapshot ID, caller, and state hash.

## Snapshot Contents

A `GroupSnapshot` contains:

| Field | Meaning |
| --- | --- |
| `snapshot_id` | Sequential identifier in the append-only snapshot log. |
| `taken_at_ledger` | Ledger sequence when the snapshot was created. |
| `taken_by` | Administrator or member that created the snapshot. |
| `round_number` | Current group round. |
| `pooled_balance` | Sum of the members' contributions for the current round. |
| `member_statuses` | Current status for each group member, in member-list order. |
| `payout_order` | Current payout order. |
| `state_hash` | SHA-256 hash of the current round number, pooled balance, and payout-order length. |

The snapshot captures the contract's recorded values at that point in time. It is an audit record, not a token transfer or a replacement for the live group state.

## Reading Snapshots

Read a snapshot by ID:

```text
get_snapshot(snapshot_id) -> GroupSnapshot
```

Get the number of snapshots in the log:

```text
get_snapshot_count() -> u32
```

Get the latest snapshot for a round, if one exists:

```text
get_group_snapshot_at(group_id, round) -> Option<GroupSnapshot>
```

The contract represents one group, so `group_id` is accepted for interface consistency and does not select another group. If multiple snapshots exist for a round, the round index returns the latest one.

## Restore and Recovery Process

Snapshots do not provide an on-chain `restore_snapshot` operation. They cannot directly roll back contributions, payouts, membership, or any other live contract state.

To use a snapshot during recovery:

1. Retrieve the relevant snapshot with `get_snapshot` or `get_group_snapshot_at`.
2. Compare its round, pooled balance, member statuses, payout order, and state hash with the current contract state and off-chain records.
3. Use the snapshot as the point-in-time reference when determining the correct recovery action.
4. Apply only the contract's available administrative or member operations needed to recover the group. A snapshot remains in the append-only log as evidence of the earlier state.

The `state_hash` helps detect differences in the values included in the hash. It is not a full serialization of every contract storage entry, so it should be considered together with the snapshot fields and other audit records.
