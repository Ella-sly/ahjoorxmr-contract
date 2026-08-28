# Seller Veto Mechanism (`ahjoor-escrow`)

This document describes the **seller veto** mechanism in the `ahjoor-escrow` contract:
how the seller of an escrow can raise a veto to delay an imminent fund release, the
cooldown window that limits how often a veto can be raised, and how the admin can
override a veto.

> Implementation references: `contracts/ahjoor-escrow/src/lib.rs`
> (`raise_seller_veto`, `admin_override_veto`, `set_veto_cooldown_seconds`, …) and the
> test suite `contracts/ahjoor-escrow/src/test_seller_veto.rs`.

## Overview

A **seller veto** is a seller-initiated signal that temporarily blocks the release of
escrow funds. Its purpose is to give the seller a window to flag a problem (for example,
a disputed delivery) before the buyer or arbiter can release the escrow balance to the
seller.

A veto is recorded by storing the ledger timestamp at which it was raised
(`SellerVetoLastTimestamp`). While that timestamp plus the cooldown window is still in
the future, any attempt to release the escrow is rejected, and any attempt by the seller
to raise a *new* veto is also rejected.

| Fact | Value |
| --- | --- |
| Primary entry point | `raise_seller_veto(seller, escrow_id)` |
| Admin override | `admin_override_veto(admin, escrow_id)` |
| Cooldown storage key | `SellerVetoLastTimestamp(escrow_id)` |
| Default cooldown | 7 days (`DEFAULT_VETO_COOLDOWN_SECONDS = 7 * 24 * 60 * 60` seconds) |
| Default override window | 48 hours (`DEFAULT_VETO_OVERRIDE_WINDOW_SECONDS = 48 * 60 * 60` seconds) |

## When a veto can be raised (trigger conditions)

A veto can be raised by calling `raise_seller_veto(seller, escrow_id)`. The call must
satisfy **all** of the following conditions, otherwise it panics:

1. **Caller is the seller.** The `seller` argument must equal the `seller` field of the
   escrow. Otherwise it fails with `OnlyEscrowSellerCanRaiseVeto` (error `26`).
2. **Escrow is open/active.** The escrow status must be an open status (e.g. `Active`),
   checked via `is_open_escrow_status`. Otherwise it fails with `EscrowIsNotActive`.
3. **Cooldown has elapsed.** The current ledger timestamp must be at or past the
   previously recorded veto timestamp plus the cooldown window. If not, it fails with
   `VetoCooldownActive` (error `27`).

On success, the contract stores `SellerVetoLastTimestamp(escrow_id) = now` and emits the
`SellerVetoRaised` event.

```rust
// Seller raises a veto on an active escrow.
client.raise_seller_veto(&seller, &escrow_id);
```

### Effect on fund release

While a veto is active (`now < SellerVetoLastTimestamp + VetoCooldownSeconds`), the
`release_escrow` entry point is blocked and fails with `SellerVetoActive` (error `38`),
regardless of who calls it (buyer or arbiter). Once the cooldown window elapses, the
veto is no longer considered active and release is permitted again without any explicit
clearing step.

## The cooldown window

The cooldown window limits how frequently the seller can raise vetoes. It is a single
global value (in seconds) stored at the instance level under `VetoCooldownSeconds`.

- **Default:** 7 days.
- **Behavior:** After a veto is raised at time `T`, the seller cannot raise another veto
  until `T + VetoCooldownSeconds`. The same window determines how long the veto blocks
  `release_escrow`.
- **Configuration:** Only the admin can change it:

  ```rust
  client.set_veto_cooldown_seconds(&admin, &new_window_seconds);
  let current = client.get_veto_cooldown_seconds();
  ```

- Setting the window to `0` effectively disables the cooldown (not recommended, as it lets
  the seller re-veto without bound).

The cooldown check (in `raise_seller_veto`) is:

```rust
if env.ledger().timestamp() < last_ts + cooldown {
    panic_with_error!(&env, EscrowErrorExt4::VetoCooldownActive);
}
```

### Why the cooldown exists

Without a cooldown, a seller could repeatedly raise vetoes to indefinitely stall a
legitimate release. The cooldown ensures that a veto is a deliberate, bounded action: it
blocks release for at most one cooldown window, after which release proceeds unless the
seller escalates through a dispute or the admin intervenes.

## Admin override

The admin can override an active seller veto with
`admin_override_veto(admin, escrow_id)`:

1. **Caller must be the admin.** Enforced via `require_admin`.
2. **Escrow must be open/active.** Otherwise it fails with `EscrowIsNotActive`.
3. **Effect:** the contract resets `SellerVetoLastTimestamp(escrow_id) = now`, starting a
   *fresh* cooldown window, and emits the `VetoOverridden` event.

```rust
// Admin overrides the seller's veto.
client.admin_override_veto(&admin, &escrow_id);
```

### What override actually does

Overriding does **not** instantly unlock release. Because the timestamp is reset to `now`,
the release block remains in effect for a full new cooldown window (`now < now + cooldown`
is always true). In other words:

- The seller **cannot immediately re-veto** — the restarted cooldown prevents a rapid
  re-raise (`VetoCooldownActive`).
- `release_escrow` becomes possible again only after the *new* window has elapsed, just
  like a normal veto expiry. This is verified by
  `test_release_blocked_by_veto_cleared_by_override`, which advances time past the cooldown
  after the override before the release succeeds.

Admin override is the mechanism for resolving a veto that should not keep blocking
release: the admin resets the timer, and once the window passes, normal flow resumes.

### Related override window & hard override

The contract also exposes a secondary, stricter override path governed by the *override
window* (`VetoOverrideWindow`, default 48 hours):

- `set_veto_override_window(admin, window_seconds)` — sets the window. Must be positive
  (`WindowSecondsMustBePositive`, error `31`); only the admin may call it.
- `cancel_seller_veto(seller, escrow_id)` — the seller may cancel their own veto, but only
  before the override window has elapsed since the veto (`VetoWindowElapsed`, error `33`).
  Emits `SellerVetoCancelled`.
- `override_veto(admin, escrow_id, reason_hash)` — the admin may hard-override a veto **only
  after** the override window has elapsed (`VetoWindowNotElapsed`, error `36`), provided no
  active dispute exists (`ActiveDisputeExists`, error `35`). It records the `reason_hash`
  immutably and releases the funds to the buyer. Only the admin may call it
  (`OnlyAdminCanOverrideSellerVeto`, error `34`).

## Events

| Event | Emitted by | Notes |
| --- | --- | --- |
| `SellerVetoRaised` | `raise_seller_veto` | Carries `escrow_id`, `seller`, `veto_timestamp`. |
| `VetoOverridden` | `admin_override_veto` | Carries `escrow_id`, `admin`, `overridden_at`. |
| `SellerVetoCancelled` | `cancel_seller_veto` | Carries `escrow_id`, `seller`. |

## Error codes

| Code | Name | Raised when |
| --- | --- | --- |
| 26 | `OnlyEscrowSellerCanRaiseVeto` | Non-seller calls `raise_seller_veto`. |
| 27 | `VetoCooldownActive` | A veto is raised within the cooldown window. |
| 30 | `OnlyAdminCanSetVetoOverrideWindow` | Non-admin sets the override window. |
| 31 | `WindowSecondsMustBePositive` | Override window set to `0`. |
| 33 | `VetoWindowElapsed` | Seller cancels a veto after the override window. |
| 34 | `OnlyAdminCanOverrideSellerVeto` | Non-admin calls `override_veto`. |
| 35 | `ActiveDisputeExists` | `override_veto` called while a dispute is active. |
| 36 | `VetoWindowNotElapsed` | `override_veto` called before the override window. |
| 38 | `SellerVetoActive` | `release_escrow` blocked by an active veto. |
| 39 | `SellerVetoActive2` | `release_escrow` blocked by a separate active veto record. |

## End-to-end example

```text
1. Escrow is Active with seller = S.
2. S calls raise_seller_veto(escrow_id).
     → SellerVetoLastTimestamp = now; SellerVetoRaised emitted.
     → For the next 7 days, release_escrow fails with SellerVetoActive.
3. S tries raise_seller_veto again immediately.
     → Fails with VetoCooldownActive (cooldown not elapsed).
4. Time advances 7 days + 1 second.
     → Veto is no longer active; release_escrow now succeeds.
5. Alternatively, admin calls admin_override_veto(escrow_id) at step 2.
     → SellerVetoLastTimestamp reset to now; VetoOverridden emitted.
     → Seller still cannot re-veto (new cooldown), and release is allowed
       only after this new window elapses.
```

## Function reference

| Function | Caller | Purpose |
| --- | --- | --- |
| `raise_seller_veto(seller, escrow_id)` | Seller | Raise a veto; blocks release until cooldown elapses. |
| `admin_override_veto(admin, escrow_id)` | Admin | Reset the veto cooldown timer. |
| `set_veto_cooldown_seconds(admin, seconds)` | Admin | Set the global cooldown window (seconds). |
| `get_veto_cooldown_seconds()` | Anyone | Read the current cooldown window. |
| `set_veto_override_window(admin, window_seconds)` | Admin | Set the secondary override window (seconds). |
| `cancel_seller_veto(seller, escrow_id)` | Seller | Cancel own veto within the override window. |
| `override_veto(admin, escrow_id, reason_hash)` | Admin | Hard-override after the window; releases to buyer. |
