# ROSCA Group Charter Acknowledgement

> **Status:** Implemented in `contracts/ahjoor-rosca/src/charter.rs` (core logic)
> and `contracts/ahjoor-rosca/src/lib.rs` (public entry points and gated call
> sites). Events are defined in `contracts/ahjoor-rosca/src/events.rs`.
> Test coverage lives in `contracts/ahjoor-rosca/src/test_charter.rs`.

---

## Overview

`ahjoor-rosca` allows a group admin to publish a **group charter** — an
off-chain rules document anchored on-chain by a hash and a URI. Once a charter
exists, members must explicitly acknowledge its current version before they can
join the group or make a contribution. A charter update after the group has been
activated goes through a governance proposal and vote rather than taking effect
immediately.

| Fact | Value |
|---|---|
| Contract | `ahjoor-rosca` |
| Core implementation | `contracts/ahjoor-rosca/src/charter.rs` |
| Entry points | `set_group_charter`, `acknowledge_charter`, `get_group_charter`, `has_acknowledged_charter`, `get_acknowledged_charter_version` |
| Charter storage key | `DataKey5::GroupCharter` (instance) |
| Per-member ack key | `DataKey5::CharterAck(Address)` (persistent) |
| Pending update key | `DataKey5::PendingCharter(proposal_id)` (persistent) |
| Charter voting window | `CHARTER_VOTING_WINDOW_SECONDS = 259_200` (3 days) |

---

## Data Types

### `GroupCharter`

```rust
pub struct GroupCharter {
    pub version:        u32,        // starts at 1, increments on every update
    pub charter_hash:   BytesN<32>, // hash of the off-chain document
    pub uri:            String,     // IPFS or other URI pointing to the document
    pub set_at_ledger:  u32,        // ledger sequence when this version was applied
}
```

Stored in **instance** storage at `DataKey5::GroupCharter`.

### Per-member acknowledgement

Stored in **persistent** storage at `DataKey5::CharterAck(member)` as a `u32`
version number — the version the member last acknowledged. Defaults to `0` if
the member has never acknowledged any version.

---

## Lifecycle

### Before group activation (no contributions yet)

```text
admin calls set_group_charter(admin, charter_hash, uri)
  ├─ applies charter directly (version = 1, or previous version + 1)
  ├─ emits CharterSet { version, charter_hash, uri }
  └─ returns None

member calls acknowledge_charter(member, version)
  ├─ validates version == current charter.version
  ├─ stores CharterAck(member) = version
  └─ emits CharterAcknowledged { member, version }

member calls contribute / add_member / join_with_invite / …
  ├─ require_charter_acknowledged(member) → passes
  └─ proceeds normally
```

### After group activation (first contribution recorded)

```text
admin calls set_group_charter(admin, new_charter_hash, uri)
  ├─ opens a CharterUpdate governance proposal (voting window = 3 days)
  ├─ stores PendingCharter(proposal_id) = (new_charter_hash, uri)
  ├─ emits CharterUpdateProposed { proposal_id, proposer, charter_hash }
  └─ returns Some(proposal_id)   — current charter version is UNCHANGED

members vote on the proposal via vote_on_proposal / execute_proposal

— if proposal passes —
execute_proposal calls execute_charter_update(proposal_id)
  ├─ reads PendingCharter(proposal_id), removes it
  ├─ applies new charter (version = old version + 1)
  └─ emits CharterSet { version, charter_hash, uri }

— all existing members now have stale ack versions —

each member must call acknowledge_charter(member, new_version)
before their next contribution or join action
```

---

## Entry Points

### 1. `set_group_charter`

```rust
pub fn set_group_charter(
    env: Env,
    admin: Address,
    charter_hash: BytesN<32>,
    uri: String,
) -> Option<u32>
```

**Authentication:** `admin.require_auth()` + `Self::require_admin(&env, &admin)`.

**Behaviour — two paths:**

**Path A — Group not yet activated:**
- `charter::apply_charter` is called directly.
- Computes `version = current_charter.version + 1`, or `1` if no charter
  exists yet.
- Stores the new `GroupCharter` in instance storage.
- **Re-setting before activation bumps the version with each call.**
- Returns `None`.
- **Events emitted:** `CharterSet { version, charter_hash, uri }`.

**Path B — Group is activated (first contribution has been recorded):**
- `charter::propose_charter_update` opens a `CharterUpdate` governance
  proposal with a 3-day voting deadline.
- The `(charter_hash, uri)` tuple is stored at
  `DataKey5::PendingCharter(proposal_id)` in persistent storage (TTL extended).
- The **current charter version is not changed yet.**
- Returns `Some(proposal_id)`.
- **Events emitted:** `ProposalCreated { … }` and
  `CharterUpdateProposed { proposal_id, proposer, charter_hash }`.

**Group activation** is determined by `DataKey4::GroupActivationEmitted` —
a boolean that is set to `true` after the first `contribute` call.

---

### 2. `acknowledge_charter`

```rust
pub fn acknowledge_charter(env: Env, member: Address, version: u32)
```

**Authentication:** `member.require_auth()`.

**Validation (error codes):**

| Condition | Error |
|---|---|
| No charter has been set | `ExtError2::CharterNotSet` (136) |
| `version != charter.version` | `ExtError2::CharterVersionMismatch` (137) |

The caller must supply the **exact current version number**. Attempting to
acknowledge an old or future version is always rejected.

**State mutations:**
- Writes `version` to persistent storage at `DataKey5::CharterAck(member)`.
- Extends the key's TTL (threshold 100 000, bump 120 000 ledgers).

**Events emitted:** `CharterAcknowledged { member, version }`

> Any address — including a prospective member who has not yet joined — may
> call `acknowledge_charter`. This allows members to pre-acknowledge before
> being added via `add_member` or `join_with_invite`.

---

### 3. `get_group_charter` (read-only)

```rust
pub fn get_group_charter(env: Env) -> Option<GroupCharter>
```

Returns the full `GroupCharter` struct if one has been set, `None` otherwise.
No authentication or state mutations.

---

### 4. `has_acknowledged_charter` (read-only)

```rust
pub fn has_acknowledged_charter(env: Env, member: Address) -> bool
```

Returns:
- `true` — if no charter exists (groups without a charter are always
  considered acknowledged).
- `true` — if `CharterAck(member) == charter.version`.
- `false` — otherwise, including members whose stored ack version is `0`
  (never acknowledged).

---

### 5. `get_acknowledged_charter_version` (read-only)

```rust
pub fn get_acknowledged_charter_version(env: Env, member: Address) -> u32
```

Returns the version the member last acknowledged, or `0` if they have never
acknowledged any version.

---

## Charter Versioning Rules

| Scenario | Result |
|---|---|
| First `set_group_charter` call (group not activated) | `version = 1`, applied immediately. |
| Second `set_group_charter` call before first contribution | `version = 2`, applied immediately. Each call increments the version. |
| `set_group_charter` after the first contribution | Opens a `CharterUpdate` governance proposal. Version unchanged until proposal executes. |
| Proposal executes (vote passes) | `version = old_version + 1`, `CharterSet` emitted. |
| Do old acknowledgements survive a version bump? | **No.** After a version bump every member's `CharterAck` holds the old version number. `has_acknowledged_charter` returns `false` for all members until they call `acknowledge_charter` with the new version. |
| Only the current version can be acknowledged | `acknowledge_charter` rejects any version that is not exactly equal to `charter.version`. |

---

## Entry Points Gated on Charter Acknowledgement

`charter::require_charter_acknowledged(env, member)` is an internal guard that
panics with `ExtError2::CharterNotAcknowledged` (138) if a charter exists and
the member has not acknowledged its current version. The following public entry
points call this guard:

| Entry Point | Member checked |
|---|---|
| `contribute` | The contributing member |
| `add_member` | The new member being added |
| `prepay_rounds` | The prepaying member |
| `join_with_invite` | The invitee |
| `contribute_as_proxy` | The beneficiary member (not the proxy) |
| `join_group_tiered` | The joining member |
| `contribute_via_proxy` | The beneficiary member (not the proxy) |
| `contribute_split` | The contributing member |

**Proxy call sites check the beneficiary member's acknowledgement, not the
proxy wallet's.** A proxy can only contribute on behalf of a member who has
already acknowledged the current charter.

The guard is a **no-op** when no charter has been set. Groups that never call
`set_group_charter` behave identically to before the feature existed.

---

## Configuration Reference

| Parameter | Entry point | Default |
|---|---|---|
| Charter hash + URI | `set_group_charter` | (none — group starts without a charter) |
| Charter voting window | Hardcoded constant | `CHARTER_VOTING_WINDOW_SECONDS = 259_200` (3 days) |
| Per-member ack TTL | Hardcoded in `charter.rs` | threshold 100 000 / bump 120 000 ledgers |

The charter voting window is not configurable per-group; it uses the same
hardcoded 3-day constant for all `CharterUpdate` governance proposals.

---

## Events

| Event | Emitted by | Fields |
|---|---|---|
| `CharterSet` | `set_group_charter` (direct apply) and `execute_proposal` (governance) | `version: u32`, `charter_hash: BytesN<32>`, `uri: String` |
| `CharterUpdateProposed` | `set_group_charter` (post-activation path) | `proposal_id: u32`, `proposer: Address`, `charter_hash: BytesN<32>` |
| `CharterAcknowledged` | `acknowledge_charter` | `member: Address`, `version: u32` |

---

## Error Cases

| Code | Name | Raised by | Description |
|---|---|---|---|
| 136 | `ExtError2::CharterNotSet` | `acknowledge_charter` | No charter has been set for this group. |
| 137 | `ExtError2::CharterVersionMismatch` | `acknowledge_charter` | The supplied version does not match the current charter version. |
| 138 | `ExtError2::CharterNotAcknowledged` | Any gated entry point | The member has not acknowledged the current charter version. |

---

## Function Reference

| Function | Caller | Purpose |
|---|---|---|
| `set_group_charter(admin, charter_hash, uri)` | Admin | Publishes a charter (direct) or proposes a charter update (post-activation). Returns `None` or `Some(proposal_id)`. |
| `acknowledge_charter(member, version)` | Member (or prospective member) | Records that the caller accepts the current charter version. |
| `get_group_charter()` | Anyone | Returns the current `GroupCharter` or `None`. |
| `has_acknowledged_charter(member)` | Anyone | Returns `true` if the member has acknowledged the current version (or if no charter exists). |
| `get_acknowledged_charter_version(member)` | Anyone | Returns the version last acknowledged by the member (`0` if never). |

---

## Storage Reference

| Key | Storage | Data |
|---|---|---|
| `DataKey5::GroupCharter` | Instance | `GroupCharter { version, charter_hash, uri, set_at_ledger }` |
| `DataKey5::CharterAck(Address)` | Persistent | `u32` — version number last acknowledged by this address |
| `DataKey5::PendingCharter(proposal_id)` | Persistent | `(BytesN<32>, String)` — pending charter hash and URI for an open proposal; removed on execution |
