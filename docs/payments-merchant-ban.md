# Merchant Ban and Suspension Flow

The `ahjoor-payments` contract supports two administrator-controlled merchant restrictions:

- **Suspended**: temporarily pauses payment activity for a specified duration.
- **Banned**: permanently restricts the merchant until an administrator reinstates it or an appeal is approved and completed.

## Triggers and Effects

Only the contract administrator can trigger either restriction. The administrator supplies a 32-byte `reason_hash` so the reason can be referenced without storing the underlying off-chain material on-chain.

Suspend a merchant for a positive duration in seconds:

```text
suspend_merchant(admin, merchant, reason_hash, duration_seconds)
```

While the suspension has not expired, the merchant cannot create or receive payments. After the ledger timestamp passes the stored expiry, payment authorization no longer blocks the merchant for that suspension; the merchant must still satisfy the normal approval or open-mode checks.

Ban a merchant:

```text
ban_merchant(admin, merchant, reason_hash)
```

A banned merchant cannot create or receive payments, and the contract clears the merchant's approved flag. The ban has no automatic expiry.

An administrator can directly clear either restriction with:

```text
reinstate_merchant(admin, merchant)
```

This sets the merchant status to `Active` and restores the approved flag.

The current status is available through:

```text
get_merchant_status(merchant) -> MerchantStatus
```

Possible statuses are `Active`, `Suspended`, and `Banned`.

## Appeal Process

Only a banned merchant can submit an appeal, and the merchant must authorize the call:

```text
submit_appeal(merchant, reason_hash)
```

The contract stores one `MerchantAppeal` record containing the merchant, appeal reason hash, submission timestamp, status, and cooling-off end timestamp. A merchant cannot submit another appeal while an existing appeal is `Pending` or `ApprovedCoolingOff`.

The administrator reviews a pending appeal and chooses one of two paths.

### Approved Appeal

The administrator approves the appeal:

```text
approve_appeal(admin, merchant)
```

The appeal moves to `ApprovedCoolingOff`, but the merchant remains banned. The contract sets `cooling_off_until` using the configured appeal period, or the default of seven days (`604800` seconds) when no period has been configured.

After the timestamp has elapsed, anyone can complete reinstatement:

```text
complete_reinstatement(merchant)
```

The appeal moves to `ApprovedReinstated`, the merchant status becomes `Active`, and the approved flag is restored. Calling this before `cooling_off_until` fails.

### Rejected Appeal

The administrator rejects a pending appeal:

```text
reject_appeal(admin, merchant)
```

The appeal moves to `Rejected`, and the merchant remains banned. The merchant cannot submit another appeal until the rejection cooldown has elapsed.

## Re-appeal Cooldown

The administrator configures the rejection cooldown in seconds with:

```text
set_appeal_rejection_cooldown(admin, seconds)
```

The configured value is used for future rejected appeals. If it has not been configured, the default cooldown is 30 days (`2592000` seconds). After rejection, the contract stores the timestamp when the merchant may appeal again. A new `submit_appeal` call before that timestamp fails with `Appeal cooldown has not elapsed`.

Once the cooldown has elapsed, the banned merchant can submit a new appeal. Approval cooling-off and rejection re-appeal cooldown are separate periods: an approved appeal waits for reinstatement, while a rejected appeal waits before another appeal may be submitted.

## Appeal Statuses

| Status | Meaning |
| --- | --- |
| `Pending` | Submitted and awaiting administrator review. |
| `ApprovedCoolingOff` | Approved, but the reinstatement timestamp has not yet elapsed. |
| `ApprovedReinstated` | Cooling-off completed and the merchant was reinstated. |
| `Rejected` | Rejected; the rejection cooldown controls when another appeal may be submitted. |

Read the current appeal record with:

```text
get_merchant_appeal(merchant) -> Option<MerchantAppeal>
```
