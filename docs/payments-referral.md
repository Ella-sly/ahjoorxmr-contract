# Merchant Referral Program

The `ahjoor-payments` contract supports a **referral program** that lets an
approved merchant invite new merchants and earn a commission on the platform
fees those referred merchants generate.

This document describes referral registration, how commission is calculated and
paid out, and how to query referral state on-chain.

## Overview

A referral links a **referrer** (an existing, approved merchant) to a
**referred merchant** (a brand-new merchant) for a limited window of ledgers.
When the referred merchant settles a payment and a platform fee is collected,
the contract awards the referrer a share of that fee as commission. The
referrer can then claim the accumulated commission to their own address.

The program is gated by an admin-configured global commission rate and a
per-referral accrual window, both of which can be adjusted without changing any
referral records already on the ledger.

## Global Configuration

The admin configures the program terms with:

```text
set_referral_config(admin, commission_bps, window_ledgers)
```

- Only the contract admin can call this, and only while the contract is not
  paused.
- `commission_bps` is the global commission rate in **basis points** (1 bp =
  0.01%). It is applied to the *fee* collected on referred merchants' payments,
  not to the payment amount itself.
- `window_ledgers` is the default accrual window, in ledgers, applied to every
  referral registered afterward. A value of `0` means referrals never expire.

Both values are stored in instance storage. Existing referral records keep the
window that was current when they were registered.

## Registration

An approved merchant registers a referral with:

```text
register_referral(referrer, referred_merchant)
```

Preconditions:

- `referrer` must sign the call and must already be an **approved merchant**
  (`approve_merchant`).
- `referred_merchant` must **not** already have a merchant record. Registering a
  referral for an already-approved merchant fails with
  `ReferralAlreadyExists` (error `15`).

On success the contract stores a `ReferralRecord` keyed by the referred
merchant:

| Field | Meaning |
| --- | --- |
| `referrer` | The referrer that will earn commission. |
| `registered_at_ledger` | Ledger sequence at which the referral was created. |
| `window_ledgers` | Accrual window copied from the global config at registration. |

The contract also bumps the record's TTL and emits `ReferralRegistered`.

The intended flow is to register the referral **before** the referred merchant
is approved, then call `approve_merchant` for the new merchant.

## Reward Calculation

Commission is accrued automatically when a referred merchant's payment is
settled. During `complete_payment` (and other finalization paths), whenever a
non-zero platform fee is collected, the contract calls the internal
`accrue_referral_commission` routine:

1. It looks up the `ReferralRecord` for the payment's merchant. If there is no
   referral record, nothing accrues.
2. It checks the accrual window. If `window_ledgers > 0` and the current ledger
   is greater than `registered_at_ledger + window_ledgers`, the referral has
   expired and nothing accrues.
3. It reads the global `commission_bps`. If it is `0`, nothing accrues.
4. It computes the commission:

   ```text
   commission = fee_amount * commission_bps / 10_000
   ```

   using integer (floor) division.

5. It adds `commission` to the referrer's pending balance and lifetime earned
   total (both saturating), bumps their TTL, and emits `CommissionAccrued`.

The commission is a share of the **fee**, not of the payment amount. For
example, with a `100` bps (1%) platform fee and `1000` bps (10%) commission, a
`1000`-unit payment collects a `10`-unit fee and accrues `1` unit of commission.

Commission is paid from the contract's own balance at claim time; the platform
fee itself is still routed to the configured fee recipient as normal.

## Claiming Commission

A referrer withdraws their pending commission with:

```text
claim_referral_commission(referrer, token)
```

- `referrer` must sign the call, and the contract must not be paused.
- The contract transfers the full pending balance of `token` from the contract
  to the referrer.
- The pending balance is reset to `0`, the lifetime claimed total is increased
  by the same amount, and `CommissionClaimed` is emitted.

Claiming with no pending balance fails with `NoCommissionToClaim` (error `16`).

## Querying Referral State

The following read-only entrypoints are available:

| Function | Returns |
| --- | --- |
| `get_pending_commission(referrer)` | The referrer's current unclaimed commission. |
| `get_referral_record(referred_merchant)` | The `ReferralRecord` for a referred merchant, if any. |
| `get_referral_commission_summary(referrer)` | A `(total_earned, total_claimed)` tuple of the referrer's lifetime totals. |

`get_referral_commission_summary` returns `(0, 0)` for a referrer with no
activity, and the pending balance equals `total_earned - total_claimed`.

## Events

### ReferralRegistered

Emitted when a referral is registered.

| Field | Meaning |
| --- | --- |
| `referrer` | The merchant that will earn commission. |
| `referred_merchant` | The newly referred merchant. |

### CommissionAccrued

Emitted when commission accrues on a referred merchant's settled payment.

| Field | Meaning |
| --- | --- |
| `referrer` | The referrer receiving the commission. |
| `referred_merchant` | The merchant whose payment triggered the accrual. |
| `payment_id` | The settled payment. |
| `amount` | The commission amount accrued. |

### CommissionClaimed

Emitted when a referrer claims their accumulated commission.

| Field | Meaning |
| --- | --- |
| `referrer` | The referrer that claimed. |
| `amount` | The amount transferred to the referrer. |

## Integration Notes

- Register a referral **before** approving the referred merchant; doing it in
  the reverse order fails with `ReferralAlreadyExists`.
- Treat the accrual window as a ledger countdown, not a wall-clock timestamp.
  A short `window_ledgers` effectively caps how long a referrer earns from a
  given merchant.
- Display the pending balance via `get_pending_commission`, and the lifetime
  totals via `get_referral_commission_summary`.
- Commission is only paid from the contract's token balance; ensure the
  contract holds sufficient liquidity of the token before referrers claim.
