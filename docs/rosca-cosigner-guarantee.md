# Cosigner Guarantee in ROSCA

This document describes the co-signer guarantee in the `ahjoor-rosca` contract: how a member nominates a guarantor, how that guarantor accepts, how the admin-configured grace window works after a missed contribution, and how the co-signer covers the default.

> Implementation references: `contracts/ahjoor-rosca/src/lib.rs` (`set_co_signer_window`, `get_co_signer_window`, `set_co_signer`, `accept_co_signer`, `co_signer_contribute`, `remove_co_signer`, `get_co_signer_record`, and the defaulter loop in `finalize_round`), `contracts/ahjoor-rosca/src/types.rs` (`CoSignerStatus`, `CoSignerRecord`), `contracts/ahjoor-rosca/src/events.rs`, and `contracts/ahjoor-rosca/src/test_cosigner_guarantee.rs`.

---

## 1. Overview

A ROSCA member can designate another address as a **co-signer**. After the co-signer accepts, a missed contribution does not immediately apply the default penalty. Instead, `finalize_round` opens a ledger-based grace window. During that window the co-signer can pay the contract on the member's behalf; the member is recorded as paid and the window is cleared. If the window expires unused, the next `finalize_round` applies the default penalty as usual.

The co-signer does **not** have to be a registered group member. Tokens for coverage are transferred from the co-signer's account, not from the member's.

The feature is inactive until an admin sets a non-zero window. `get_co_signer_window()` returns `0` when the value has never been stored, and a window of `0` skips the grace path entirely.

---

## 2. Storage and Types

```rust
#[contracttype]
pub enum CoSignerStatus {
    Pending = 0,   // set by member, not yet accepted
    Active = 1,    // accepted by co-signer
}

#[contracttype]
pub struct CoSignerRecord {
    pub co_signer: Address,
    pub status: CoSignerStatus,
}
```

| Key | Storage | Contents |
| :--- | :--- | :--- |
| `DataKey4::CoSigners` | instance | `Map<Address, CoSignerRecord>` — member → designation |
| `DataKey4::CoSignerWindowLedgers` | instance | `u32` — grace length in ledgers (admin-configured) |
| `DataKey3::CoSignerWindowStart` | instance | `Map<Address, u32>` — member → ledger sequence when the window opened |

The co-signer map is keyed by **member address** on contract instance storage, not by `(group_id, member)`. `group_id` is still passed into the public functions and included in events.

---

## 3. Admin-Configured Grace Window

```rust
pub fn set_co_signer_window(env: Env, admin: Address, window_ledgers: u32)
pub fn get_co_signer_window(env: Env) -> u32
```

- `set_co_signer_window` requires the contract not to be paused, `admin.require_auth()`, and `admin` to match the stored `DataKey::Admin`. A mismatched admin panics with `NotAMember` (`1008`).
- The value is stored under `DataKey4::CoSignerWindowLedgers`.
- `get_co_signer_window` returns that value, or `0` if unset. Tests in `test_view_functions.rs` cover the default and a round-trip of `500`.

A window of `0` means: on default, do not open a co-signer window; apply the penalty immediately even if an Active co-signer exists.

---

## 4. Nomination

```rust
pub fn set_co_signer(env: Env, member: Address, group_id: u32, co_signer: Address)
```

- **Authentication:** `member.require_auth()`.
- **Membership:** `member` must be in `DataKey::Members`, otherwise `NotAMember` (`1008`).
- **Uniqueness:** if `CoSigners` already contains `member`, the call panics with `CoSignerAlreadySet` (`1072`). The existing record must be removed first.
- The new record is stored with `status = Pending`.
- Emits `CoSignerSet` with `(group_id, member, co_signer)`.

The designated `co_signer` is not required to be a group member and is not authenticated on this call.

---

## 5. Acceptance

```rust
pub fn accept_co_signer(env: Env, co_signer: Address, group_id: u32, member: Address)
```

- **Authentication:** `co_signer.require_auth()`.
- If `member` has no record, panics `NoCoSignerFound` (`1073`).
- If `record.co_signer != co_signer`, panics `NotTheCoSigner` (`1075`).
- Sets `status = Active` and emits `CoSignerAccepted` with `(group_id, member, co_signer)`.

Until acceptance, the designation cannot cover a default. `co_signer_contribute` panics with `CoSignerNotAccepted` (`1074`) while status is `Pending`. A pending designation also does **not** open a grace window (see below).

---

## 6. Default Handling and the Grace Window

When `finalize_round` processes defaulters (members who did not pay this round), it loads `CoSignerWindowLedgers`, `CoSigners`, and `CoSignerWindowStart`.

For each defaulter, if `co_signer_window > 0` and a record exists:

| Co-signer status | Window already open? | Behaviour |
| :--- | :--- | :--- |
| `Active` | no | Store `CoSignerWindowStart[member] = current ledger sequence`. Skip the default penalty this round. |
| `Active` | yes, and `sequence < start + window` | Still inside the window. Skip the penalty again. |
| `Active` | yes, and `sequence >= start + window` | Clear the start entry, emit `CoSignerWinExpired` (`CoSignerWindowExpired` is the *call* error; the event symbol is `CoSignerWinExpired`), then fall through and apply the default penalty. |
| `Pending` | n/a | No window. Apply the default penalty immediately. |

Penalty application increments `DefaultCount`, emits `defaulted`, updates the credit score, and may suspend the member after `max_defaults`.

`test_pending_cosigner_skipped_on_default` asserts that a Pending co-signer causes `default_count` to increase on `finalize_round` and that `co_signer_contribute` still fails.

---

## 7. Coverage Payout

```rust
pub fn co_signer_contribute(
    env: Env,
    co_signer: Address,
    group_id: u32,
    member: Address,
    token: Address,
    amount: i128,
)
```

The co-signer pays the contract **during an open window**. The contribution is recorded as the member's.

Checks, in order:

1. `co_signer.require_auth()`.
2. Record exists (`NoCoSignerFound`).
3. Caller is the designated co-signer (`NotTheCoSigner`).
4. Status is `Active` (`CoSignerNotAccepted`).
5. `CoSignerWindowStart` has an entry for `member` (`CoSignerWindowNotOpen`, `1076`) — the member must already have defaulted and had a window opened.
6. `env.ledger().sequence() < start + co_signer_window`. Otherwise `CoSignerWindowExpired` (`1077`).

Then:

- `token::Client::transfer` moves `amount` from `co_signer` to the contract.
- `member` is appended to `PaidMembers` if not already present.
- The member's window start entry is removed.
- Emits `CoSignerContributed` with `(group_id, member, co_signer, amount)`.

`test_cosigner_honours_contribution` covers a successful pay after `finalize_round`. `test_window_expiry_triggers_member_penalty` advances 600 ledgers past a 500-ledger window and expects `co_signer_contribute` to fail.

This entrypoint does not compare `amount` to the group's contribution size and does not require `token` to be the group base token. Callers should pass the group's contribution token and amount.

---

## 8. Removal

```rust
pub fn remove_co_signer(env: Env, member: Address, group_id: u32)
```

- **Authentication:** `member.require_auth()`.
- **Between rounds only:** if `PaidMembers` is non-empty, panics `CannotChangeMidRound` (`1016`).
- If no record exists, panics `NoCoSignerFound`.
- Removes the member's entry from `CoSigners`.

`test_remove_cosigner_clears_designation` checks that a new `set_co_signer` succeeds after removal. No dedicated remove event is emitted.

---

## 9. Queries

```rust
pub fn get_co_signer_record(env: Env, group_id: u32, member: Address) -> Option<CoSignerRecord>
```

Returns the stored record, or `None` if the member has never nominated a co-signer (or it was removed). `group_id` is unused for the lookup.

```rust
pub fn get_co_signer_window(env: Env) -> u32
```

See [Admin-Configured Grace Window](#3-admin-configured-grace-window).

---

## 10. Events and Errors

### Events

| Symbol | Payload |
| :--- | :--- |
| `CoSignerSet` | `(group_id, member, co_signer)` |
| `CoSignerAccepted` | `(group_id, member, co_signer)` |
| `CoSignerContributed` | `(group_id, member, co_signer, amount)` |
| `CoSignerWinExpired` | `(group_id, member)` |

### Errors

Numeric codes are the ROSCA offset (`1000 + variant`) published in [Contract Error Codes](errors.md).

| Code | Name | When |
| :--- | :--- | :--- |
| 1072 | `CoSignerAlreadySet` | `set_co_signer` while a record already exists |
| 1073 | `NoCoSignerFound` | Accept, contribute, or remove with no record |
| 1074 | `CoSignerNotAccepted` | Contribute while status is `Pending` |
| 1075 | `NotTheCoSigner` | Caller is not the designated address |
| 1076 | `CoSignerWindowNotOpen` | Contribute before a window was opened on default |
| 1077 | `CoSignerWindowExpired` | Contribute after `start + window` ledgers |
| 1008 | `NotAMember` | Nominator is not a member; also used if `set_co_signer_window` is not the stored admin |
| 1016 | `CannotChangeMidRound` | `remove_co_signer` while `PaidMembers` is non-empty |
