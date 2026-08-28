# Merchant KYB Verification

The `ahjoor-payments` contract supports **KYB (Know Your Business)** verification
for merchants. KYB lets an administrator record a merchant's off-chain
verification result on-chain and then gate payment creation on it, so only
verified merchants can accept payments.

This document covers the full KYB lifecycle: enabling enforcement, submitting a
verification, the payment-creation gate, on-chain status checks, renewal, and
revocation.

## Data Model

A merchant's KYB verification is stored as a `MerchantKYB` record keyed by the
merchant's address:

| Field | Meaning |
| --- | --- |
| `kyb_hash` | 32-byte hash (`BytesN<32>`) of the off-chain verification documents/result. |
| `expiry_ledger` | Ledger sequence after which the verification is considered expired. |
| `jurisdiction` | Country code / jurisdiction string the merchant is verified under (e.g. `"NG"`, `"GH"`). |
| `revoked` | Whether the verification has been administratively revoked. |

The read-only status returned to callers is a `KYBStatus`:

| Field | Meaning |
| --- | --- |
| `verified` | `true` when a record exists, is not revoked, and has not expired. |
| `expiry_ledger` | The record's expiry ledger (`0` when no record exists). |
| `jurisdiction` | The merchant's jurisdiction (empty when no record exists). |

## Enabling KYB Enforcement

KYB gating is a global toggle, off by default. The admin enables it with:

```text
set_kyb_enforcement(admin, enabled)
```

- Only the contract admin can call this.
- When enabled, every `create_payment` performs the KYB check described below.
- When disabled, payments are created without any KYB check.

The enforcement flag is stored in instance storage. For backward compatibility
across contract upgrades, the flag is written to both the current key and the
legacy key; reads prefer the current key and fall back to the legacy one.

## Submission and Verification

The admin records a merchant's verification result with:

```text
set_merchant_kyb(admin, merchant, kyb_hash, expiry_ledger, jurisdiction)
```

- Only the contract admin can call this.
- `kyb_hash` is the 32-byte hash of the merchant's verification documents.
- `expiry_ledger` is the ledger sequence at which the verification lapses.
- `jurisdiction` is a short string describing where the merchant is verified.

This stores a fresh `MerchantKYB` record (with `revoked = false`) in persistent
storage and bumps the record's TTL so it is not archived. It also emits
`MerchantKYBSet`.

## Payment-Creation Gating

When `create_payment` runs and KYB enforcement is enabled, the contract checks
the destination merchant's record in this order:

1. If no `MerchantKYB` record exists → fails with `KYBVerificationRequired` (error `33`).
2. If the record is revoked (`revoked == true`) → fails with `KYBVerificationRequired` (error `33`).
3. If the current ledger is greater than `expiry_ledger` → fails with `MerchantKYBExpired` (error `72`).

A payment is created only when the merchant has a live, unrevoked, unexpired
verification. When enforcement is disabled, these checks are skipped entirely.

## Checking Status On-Chain

Anyone can query a merchant's current status with:

```text
get_merchant_kyb_status(merchant) -> KYBStatus
```

The returned `KYBStatus.verified` is computed as:

```text
verified = !revoked && current_ledger <= expiry_ledger
```

Example CLI call:

```bash
stellar contract invoke \
  --id <PAYMENTS_CONTRACT_ID> \
  --source alice \
  --network testnet \
  -- get_merchant_kyb_status --merchant <MERCHANT_ADDRESS>
```

## Renewal

When a verification is about to expire (or has expired), the admin renews it
with:

```text
renew_merchant_kyb(admin, merchant, new_kyb_hash, new_expiry_ledger, jurisdiction)
```

- Only the contract admin can call this.
- The renewal overwrites the previous `kyb_hash`, `expiry_ledger`, and
  `jurisdiction`, and resets `revoked` to `false`.
- Renewing an expired record makes the merchant eligible for payments again
  immediately (provided enforcement remains enabled).

Renewal also re-bumps the record's TTL and emits `MerchantKYBSet`.

## Revocation

The admin can revoke a merchant's verification at any time with:

```text
revoke_merchant_kyb(admin, merchant)
```

- Only the contract admin can call this.
- The existing record is kept in persistent storage but marked `revoked = true`.
- A revoked merchant fails the payment-creation gate with
  `KYBVerificationRequired` even if the record's `expiry_ledger` is in the
  future.
- The record's TTL is re-bumped so the revocation persists.

Revocation emits `MerchantKYBRevoked`.

To clear a revocation, the admin records a fresh verification via
`set_merchant_kyb` (or renews with `renew_merchant_kyb`), both of which reset
`revoked` to `false`.

## Events

### MerchantKYBSet

Emitted when a merchant's verification is first recorded (`set_merchant_kyb`) or
renewed (`renew_merchant_kyb`).

| Field | Meaning |
| --- | --- |
| `merchant` | The verified merchant address. |
| `kyb_hash` | The recorded verification hash. |
| `expiry_ledger` | The ledger at which the verification expires. |
| `jurisdiction` | The merchant's jurisdiction. |

### MerchantKYBRevoked

Emitted when a merchant's verification is revoked (`revoke_merchant_kyb`).

| Field | Meaning |
| --- | --- |
| `merchant` | The revoked merchant address. |

## Integration Notes

- Enable `set_kyb_enforcement` only after you have recorded verifications for the
  merchants you intend to allow, otherwise all payment creation fails.
- Surface `get_merchant_kyb_status` in merchant dashboards so they can see
  their `verified` state, `expiry_ledger`, and `jurisdiction`.
- Treat `KYBVerificationRequired` (missing/revoked) and `MerchantKYBExpired`
  (expired) as distinct outcomes so you can prompt for verification vs. renewal.
- `expiry_ledger` is a ledger sequence, not a wall-clock timestamp — map it to a
  time for display only.
