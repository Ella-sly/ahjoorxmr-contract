# Customer Spend Limits and Cooling-Off Cancellation in ahjoor-payments

> **Status:** Implemented in `contracts/ahjoor-payments/src/lib.rs`
> (`set_customer_spend_limit`, `set_default_spend_limit`,
> `remove_customer_spend_limit`, `set_max_cooling_off_ledgers`,
> `create_payment_with_cooling_off`, `cancel_during_cooling_off`).
> Events are defined in `contracts/ahjoor-payments/src/events.rs`.
> Test coverage lives in `contracts/ahjoor-payments/src/test_spending_limit.rs`
> and `contracts/ahjoor-payments/src/test_cooling_off.rs`.

---

## Overview

`ahjoor-payments` ships two independent customer-safety features:

1. **Customer spend limits** — a merchant sets a maximum amount a customer
   may spend within a rolling time window. Every payment is checked at
   `complete_payment` time; payments that would push the customer over the cap
   are rejected before funds move.

2. **Cooling-off cancellation** — a merchant creates a payment with a
   cooling-off period (measured in ledgers). After the payment settles and
   enters `CoolingOff` status, the customer has a window in which they can
   cancel and receive a full refund, without needing merchant approval.

The two features are independent and may be used together or separately.

| Fact | Value |
|---|---|
| Contract | `ahjoor-payments` |
| Spend-limit entry points | `set_customer_spend_limit`, `set_default_spend_limit`, `remove_customer_spend_limit` |
| Cooling-off entry points | `set_max_cooling_off_ledgers`, `create_payment_with_cooling_off`, `cancel_during_cooling_off` |
| Spend-limit storage keys | `DataKey2::CustomerSpendLimit(merchant, customer)`, `DataKey2::DefaultSpendLimit(merchant)`, `DataKey2::CustomerSpendWindowState(merchant, customer)` |
| Cooling-off storage keys | `DataKey2::MaxCoolingOffLedgers`, `DataKey2::CoolingOffConfig(payment_id)` |

---

## Part 1: Customer Spend Limits

Spend limits let a merchant cap how much a single customer can spend within a
rolling time window. There are three tiers; at payment completion time the
contract resolves the effective limit using this priority order:

```
1. Per-customer override  (set_customer_spend_limit)
2. Tier-based limit       (set_tier_spending_limit, keyed on BuyerTrustTierLevel)
3. Merchant default       (set_default_spend_limit)
4. No limit               (check passes immediately)
```

### How the spend-limit window is measured

The window is a **rolling window anchored to the first payment that opens it**,
not a fixed calendar period. The window state is stored as
`CustomerSpendWindow { window_start: u64, spent: i128 }` in persistent storage
at `DataKey2::CustomerSpendWindowState(merchant, customer)`.

At every `complete_payment` call the enforcement helper:
1. Reads `now = env.ledger().timestamp()`.
2. If `now >= window_start + window_seconds`, resets the state to
   `{ window_start: now, spent: 0 }`.
3. Computes `new_total = spent + payment_amount`.
4. If `new_total > limit.amount`, rejects the payment.
5. Otherwise saves `spent = new_total`.

**When the window resets:** the anchor moves to the timestamp of the first
payment *in the new window*, not to the original `window_start + window_seconds`
boundary. Each new window starts from the moment the customer makes their first
payment after the previous window expired.

> [!NOTE]
> The spend check happens at `complete_payment` time, not at
> `create_payment` time. A payment can be created even if its amount would
> breach the limit; it will only be rejected when the merchant attempts to
> complete it.

> [!NOTE]
> Cancelling a payment during the cooling-off period does **not** reverse the
> spend counter. If a payment enters `CoolingOff` status, the customer's
> window-spent total has already been updated. The amount remains counted even
> after a successful refund via `cancel_during_cooling_off`.

### 1. Set a per-customer limit — `set_customer_spend_limit`

```rust
pub fn set_customer_spend_limit(
    env: Env,
    merchant: Address,
    customer: Address,
    amount: i128,
    window_seconds: u64,
)
```

**Authentication:** `merchant.require_auth()` — only the merchant can set limits
on their customers.

**Validation:**
- Contract must not be paused.
- `amount <= 0` → panics `"Spend limit amount must be positive"`.
- `window_seconds == 0` → panics `"Window seconds must be positive"`.

**State mutations:**
- Writes `SpendLimit { amount, window_seconds }` to persistent storage at
  `DataKey2::CustomerSpendLimit(merchant, customer)`.
- Extends the key's TTL.

**Events emitted:** `CustomerSpendLimitSet { merchant, customer, amount, window_seconds }`

---

### 2. Set a merchant-wide default — `set_default_spend_limit`

```rust
pub fn set_default_spend_limit(env: Env, merchant: Address, amount: i128, window_seconds: u64)
```

**Authentication:** `merchant.require_auth()`.

**Validation:** Same as `set_customer_spend_limit` (positive amount, non-zero
window).

**State mutations:**
- Writes `SpendLimit { amount, window_seconds }` to persistent storage at
  `DataKey2::DefaultSpendLimit(merchant)`.
- Extends the key's TTL.

This limit applies to every customer of this merchant who has no individual
override and no tier-based limit. It does not affect customers who already have
a per-customer limit set.

**Events emitted:** `DefaultSpendLimitSet { merchant, amount, window_seconds }`

---

### 3. Remove a per-customer limit — `remove_customer_spend_limit`

```rust
pub fn remove_customer_spend_limit(env: Env, merchant: Address, customer: Address)
```

**Authentication:** `merchant.require_auth()`.

**State mutations:**
- Removes the `DataKey2::CustomerSpendLimit(merchant, customer)` key from
  persistent storage.
- The window-state entry at `DataKey2::CustomerSpendWindowState` is **not**
  cleared. If the limit is re-added later, the old window state (if still within
  its TTL) will resume.

After removal the customer falls through to tier-based or default limits (if
any are configured).

**Events emitted:** `CustomerSpendLimitRemoved { merchant, customer }`

---

### Query Functions

| Function | Returns |
|---|---|
| `get_customer_spend_limit(merchant, customer)` | `Option<SpendLimit>` — the per-customer override, or `None`. |
| `get_default_spend_limit(merchant)` | `Option<SpendLimit>` — the merchant default, or `None`. |

---

## Part 2: Cooling-Off Cancellation

The cooling-off feature gives customers a right-of-withdrawal window after a
payment completes. During the window the payment status is `CoolingOff` instead
of `Completed`. The customer (and only the customer) may cancel and receive a
full refund before the window expires.

### Cooling-off lifecycle

```text
admin calls set_max_cooling_off_ledgers(admin, max_ledgers)
  └─ stores max as DataKey2::MaxCoolingOffLedgers (instance storage)

customer calls create_payment_with_cooling_off(customer, merchant, amount,
                                               token, cooling_off_ledgers)
  ├─ validates cooling_off_ledgers ≤ max_cooling_off_ledgers (if max > 0)
  ├─ creates a normal Payment (status: Pending, funds escrowed)
  └─ stores CoolingOffConfig { cooling_off_ledgers, cooling_off_expiry_ledger: 0 }

merchant calls complete_payment(payment_id)
  ├─ runs normal settlement: fee deduction, net transfer to merchant
  ├─ detects CoolingOffConfig on this payment
  ├─ sets payment.status = PaymentStatus::CoolingOff
  ├─ sets cooling_off_expiry_ledger = sequence() + cooling_off_ledgers
  └─ emits PaymentInCoolingOff { payment_id, customer, expiry_ledger }

 — cooling-off window is open —

customer calls cancel_during_cooling_off(customer, payment_id)   [optional]
  ├─ checks: status == CoolingOff, sequence() < expiry_ledger
  ├─ transfers payment.amount from contract to customer (full refund)
  ├─ sets payment.status = PaymentStatus::CancelledInCoolingOff
  └─ emits CoolingOffCancellation + PaymentStatusChanged

 — if not cancelled —

window expires: payment stays CoolingOff until the merchant or their
backend marks it Completed via normal settlement or withdrawal logic.
```

### 1. Set the platform maximum — `set_max_cooling_off_ledgers`

```rust
pub fn set_max_cooling_off_ledgers(env: Env, admin: Address, max_ledgers: u32)
```

**Authentication:** `Self::require_admin(&env, &admin)` — contract admin only.

**Behaviour:** Writes `max_ledgers` to instance storage at
`DataKey2::MaxCoolingOffLedgers`. The default (`DEFAULT_MAX_COOLING_OFF_LEDGERS`)
is `0`, which means cooling-off is **disabled by default** until the admin
explicitly enables it by setting a positive value.

**No events emitted.**

---

### 2. Create a payment with cooling-off — `create_payment_with_cooling_off`

```rust
pub fn create_payment_with_cooling_off(
    env: Env,
    customer: Address,
    merchant: Address,
    amount: i128,
    token: Address,
    cooling_off_ledgers: u32,
) -> u32
```

**Authentication:** Delegates to `create_payment`; `customer.require_auth()` is
enforced there along with token-whitelist and merchant-approval checks.

**Validation:**
- If `max_cooling_off_ledgers > 0` and `cooling_off_ledgers > max`, panics with
  `Error::CoolingOffExceedsMax` (29).

**State mutations:**
- Creates a `Payment` record (status `Pending`) via the standard `create_payment`
  path; customer funds are escrowed in the contract.
- If `cooling_off_ledgers > 0`, writes
  `CoolingOffConfig { cooling_off_ledgers, cooling_off_expiry_ledger: 0 }` to
  persistent storage at `DataKey2::CoolingOffConfig(payment_id)`.
  `cooling_off_expiry_ledger` is set to zero at creation and populated only
  when `complete_payment` is called.

**Events emitted:** `PaymentCreated { … }` (from the delegated `create_payment`
call).

---

### 3. Cancel during cooling-off — `cancel_during_cooling_off`

```rust
pub fn cancel_during_cooling_off(env: Env, customer: Address, payment_id: u32)
```

**Authentication:** `customer.require_auth()` — only the payment's customer may
cancel. The contract also checks `payment.customer == customer`; panics with
`"Only the payment customer can cancel"` on mismatch.

**Validation:**
- `payment.status != PaymentStatus::CoolingOff` → `Error::NotInCoolingOff` (28).
- `sequence() >= cooling_off_config.cooling_off_expiry_ledger` →
  `Error::CoolingOffExpired` (27).

**State mutations:**
- Calls `token::transfer(contract → customer, payment.amount)` — the full
  recorded payment amount is returned to the customer.
- Sets `payment.status = PaymentStatus::CancelledInCoolingOff`.
- Sets `payment.refunded_amount = payment.amount`.

**Events emitted:**
- `CoolingOffCancellation { payment_id, customer, refund_amount }` — carries
  the amount refunded.
- `PaymentStatusChanged { payment_id, old_status: CoolingOff, new_status: CancelledInCoolingOff }`.

---

### What the merchant sees when a payment is cancelled during cooling-off

Understanding the fund flow helps explain the merchant-visible effects:

1. When `complete_payment` is called, the full normal settlement runs first:
   fees are deducted and the **net amount is transferred to the merchant**
   (or enqueued in the merchant's withdrawal queue).
2. The payment status then transitions to `CoolingOff` (instead of `Completed`).
3. If the customer later cancels, `cancel_during_cooling_off` refunds
   `payment.amount` from the **contract's own balance** — not from the
   merchant's wallet.

Merchant-facing implications:

- The merchant's withdrawal queue **is not reversed** on cancellation. The net
  settlement entry placed there by `complete_payment` remains.
- The settlement stats (`MerchantStats`, global counters) incremented during
  `complete_payment` are also not reversed.
- The only on-chain signal the merchant receives is the generic
  `PaymentStatusChanged { CoolingOff → CancelledInCoolingOff }` event.
  There is no dedicated merchant-notification callback.
- Because the refund draws from the contract balance, the contract must hold
  sufficient funds (via merchant collateral or rolling reserves) to honour the
  refund. A refund will fail at the token-transfer level if the contract is
  underfunded.

---

## Configuration Reference

| Parameter | Entry point | Default | Constraint |
|---|---|---|---|
| Per-customer limit amount | `set_customer_spend_limit` | (none) | `> 0` |
| Per-customer limit window | `set_customer_spend_limit` | (none) | `> 0` seconds |
| Merchant-default limit amount | `set_default_spend_limit` | (none) | `> 0` |
| Merchant-default limit window | `set_default_spend_limit` | (none) | `> 0` seconds |
| Max cooling-off ledgers | `set_max_cooling_off_ledgers` | `0` (disabled) | `u32` |

---

## Events

| Event | Emitted by | Fields |
|---|---|---|
| `CustomerSpendLimitSet` | `set_customer_spend_limit` | `merchant`, `customer`, `amount`, `window_seconds` |
| `CustomerSpendLimitRemoved` | `remove_customer_spend_limit` | `merchant`, `customer` |
| `CustomerSpendLimitExceeded` | `complete_payment` (internal check) | `merchant`, `customer`, `attempted`, `cap` |
| `DefaultSpendLimitSet` | `set_default_spend_limit` | `merchant`, `amount`, `window_seconds` |
| `PaymentInCoolingOff` | `complete_payment` (on cooling-off transition) | `payment_id`, `customer`, `expiry_ledger` |
| `CoolingOffCancellation` | `cancel_during_cooling_off` | `payment_id`, `customer`, `refund_amount` |
| `PaymentStatusChanged` | `cancel_during_cooling_off` | `payment_id`, `old_status`, `new_status` |

---

## Error Cases

| Code | Name | Raised by | Description |
|---|---|---|---|
| 23 | `CustomerSpendLimitExceeded` | `complete_payment` | Cumulative spend in the current window would exceed the cap. |
| 27 | `CoolingOffExpired` | `cancel_during_cooling_off` | The ledger sequence is past `cooling_off_expiry_ledger`. |
| 28 | `NotInCoolingOff` | `cancel_during_cooling_off` | The payment status is not `CoolingOff`. |
| 29 | `CoolingOffExceedsMax` | `create_payment_with_cooling_off` | Requested cooling-off period exceeds the platform maximum. |

---

## Function Reference

| Function | Caller | Purpose |
|---|---|---|
| `set_customer_spend_limit(merchant, customer, amount, window_seconds)` | Merchant | Sets or updates a per-customer spend cap for this merchant. |
| `set_default_spend_limit(merchant, amount, window_seconds)` | Merchant | Sets a fallback spend cap applied to customers with no individual override. |
| `remove_customer_spend_limit(merchant, customer)` | Merchant | Removes a per-customer cap; customer falls through to default or tier limits. |
| `get_customer_spend_limit(merchant, customer)` | Anyone | Returns the per-customer `SpendLimit` if set. |
| `get_default_spend_limit(merchant)` | Anyone | Returns the merchant-default `SpendLimit` if set. |
| `set_max_cooling_off_ledgers(admin, max_ledgers)` | Admin | Sets the platform-wide maximum cooling-off period. |
| `create_payment_with_cooling_off(customer, merchant, amount, token, cooling_off_ledgers)` | Customer | Creates a payment and attaches a cooling-off config to it. |
| `cancel_during_cooling_off(customer, payment_id)` | Customer | Cancels the payment and refunds the full amount while the cooling-off window is open. |

---

## Storage Reference

| Key | Storage | Data |
|---|---|---|
| `DataKey2::CustomerSpendLimit(merchant, customer)` | Persistent | `SpendLimit { amount, window_seconds }` — per-customer cap |
| `DataKey2::DefaultSpendLimit(merchant)` | Persistent | `SpendLimit { amount, window_seconds }` — merchant default cap |
| `DataKey2::CustomerSpendWindowState(merchant, customer)` | Persistent | `CustomerSpendWindow { window_start: u64, spent: i128 }` |
| `DataKey2::MaxCoolingOffLedgers` | Instance | `u32` — platform maximum cooling-off period in ledgers |
| `DataKey2::CoolingOffConfig(payment_id)` | Persistent | `CoolingOffConfig { cooling_off_ledgers: u32, cooling_off_expiry_ledger: u32 }` |
