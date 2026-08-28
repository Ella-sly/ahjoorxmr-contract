# Refund Contract (ahjoor-refund)

This document describes the refund contract (`contracts/ahjoor-refund`) behavior, who can trigger refunds, and the typical flow for requesting/approving/claiming refunds.

## When refunds apply
- Cancelled escrow: when an escrow is cancelled before funds are released.
- Failed round: if a ROSCA round fails to execute (e.g., not enough contributions).
- Overpayment: participant deposited more than required or duplicate payments.

Refund issuance is typically created when an upstream contract (e.g., `ahjoor-escrow` or `ahjoor-rosca`) determines funds must be returned. The refund record may be created automatically by the originating contract or by an explicit call to the refund contract.

## Who can trigger a refund
- Admin: can create or approve refunds in exceptional cases or to resolve disputes.
- Participant: a participant can request a refund for their own payment (see `request_refund`).
- Automatic: originating contracts may create refund records automatically on cancellation or failure.

## Key functions

- `request_refund(refund_id)` — participant requests a refund for a specific payment or escrow. Creates a refund record in `Pending` status when caller is the beneficiary.

- `approve_refund(refund_id)` — admin approves a pending refund. Moves refund to `Approved` state and records timestamp and approved amount. Only callable by admin or an authorized arbiter.

- `claim_refund(refund_id)` — participant claims funds for an approved refund. Transfers funds to claimant and marks refund `Claimed`.

- `create_refund(refund_id, owner, amount, metadata)` — (internal / called by originating contracts) create a refund record (used for automatic creation on cancellation).

- `auto_reject_stale_refund(refund_id)` — permissionlessly auto-rejects unhandled refund requests after the configured auto-reject window (plus extensions) has elapsed, returning escrowed tokens to the customer.

- `extend_refund_deadline(admin, refund_id, extra_seconds)` — admin-only function to extend the auto-reject deadline for an active refund request.

- `get_refund(refund_id) -> Refund` — view refund record and status (`Pending`, `Approved`, `Claimed`, `Cancelled`, `Rejected`, `CounterOffered`).


## Typical flow

1. Escrow is cancelled (or round fails / overpayment detected).
2. Originating contract calls `create_refund(...)` on `ahjoor-refund` (automatic) OR participant calls `request_refund(refund_id)`.
3. Admin (or arbiter) reviews and calls `approve_refund(refund_id)` to approve the refund.
4. Participant calls `claim_refund(refund_id)` to withdraw the approved amount.

Alternate shorter flow (automatic approval): some flows can be configured so that refund creation includes an initial `Approved` status, allowing `claim_refund` directly after creation.

## Time limits and expirations

- Approval-to-claim window: the contract may enforce a time window (e.g., 30 days) within which the claimant must call `claim_refund` after approval. After the window expires the refund may be moved to `Expired` and require admin re-approval.
- Claim timelock: some refunds may include an optional timelock preventing claims until a given epoch (useful for dispute cooling-off).
- Stale refund auto-reject window: unhandled requests in `Requested` status beyond the configured window (plus any admin-granted extensions) can be auto-rejected permissionlessly.
- Customer cancel window: participants can auto-cancel unhandled requests after the cancellation window elapses.

Check the contract configuration constants for the exact timeouts used in the deployed instance.

## Refund Deadline Boundary Enforcement

The `ahjoor-refund` contract enforces deterministic timeout and deadline boundaries across every refund lifecycle stage. Aligned with the cross-contract timeout convention (#553), all permissionless-after-timeout functions enforce an **exclusive boundary** rule.

### Boundary Convention and Exact Expiration Rules

In Soroban ledger timekeeping, all timeout checks are evaluated against the current ledger timestamp (`now = env.ledger().timestamp()`):

- **Exclusive Boundary Invariant:** A timeout or deadline window is considered elapsed **only** when the current ledger timestamp is strictly greater than the computed deadline (`now > deadline`).
- **Exact-Boundary Evaluation (`now == deadline`):** If the current ledger timestamp exactly matches the deadline timestamp, the window has **not yet elapsed**. Any attempt to trigger a timeout transition at the exact boundary panics and reverts (`now <= deadline` blocks).
- **Rationale:** The exclusive boundary ensures that participants and merchants are guaranteed the full duration of their allotted review or cancellation periods, preventing race conditions or premature execution at the final second of a window.

#### Summary of Boundary Checks

| Function | Compared Deadline / Expiry | Blocking Condition | Eligible When | Caller Permissions |
|---|---|---|---|---|
| `auto_reject_stale_refund` | `requested_at + auto_reject_window + extension` | `now <= deadline` | `now > deadline` | Permissionless (Anyone) |
| `auto_approve_refund` | `requested_at + dispute_window` | `now <= threshold` | `now > threshold` | Permissionless (Anyone) |
| `auto_cancel_expired_request` | `requested_at + cancel_window` | `now <= threshold` | `now > threshold` | Permissionless (Anyone) |
| `settle_expired_counter_offer` | `offer.expiry` | `now <= expiry` | `now > expiry` | Permissionless (Anyone) |

---

### Deadline Computation

Deadlines are calculated on-chain at request creation or negotiation initiation using ledger state and configuration parameters:

1. **Initial Expiration Timestamp Calculation (`request_refund`):**
   - When a customer submits a refund request, the contract records the ledger timestamp:
     ```rust
     refund.requested_at = env.ledger().timestamp();
     ```
   - **Stale Auto-Reject Deadline:**
     $$\text{Deadline}_{\text{auto-reject}} = \text{requested\_at} + \text{auto\_reject\_window} + \text{extension}$$
     - `requested_at`: Unix timestamp (in seconds) when the refund request was created.
     - `auto_reject_window`: Global duration (in seconds) retrieved from instance storage (`DataKey::AutoRejectWindow`). Configured during initialization (`RefundInitConfig`) or updated by admin via `set_auto_reject_window`.
     - `extension`: Per-refund extension seconds stored in persistent storage (`DataKey::RefundDeadlineExtension(refund_id)`), defaulting to `0`.
   - **Dispute Auto-Approval Deadline:**
     $$\text{Deadline}_{\text{dispute}} = \text{requested\_at} + \text{dispute\_window}$$
   - **Customer Auto-Cancel Deadline:**
     $$\text{Deadline}_{\text{cancel}} = \text{requested\_at} + \text{customer\_cancel\_window\_seconds}$$
   - **Ledger Sequence Deadlines:**
     - Merchant response deadline: `env.ledger().sequence() + merchant_response_window`
     - Primary review deadline: `env.ledger().sequence() + primary_review_window`
     - Auto-approval deadline: `env.ledger().sequence() + auto_deadline_window`

2. **Counter-Offer Expiration Timestamp (`counter_offer_refund`):**
   - When a merchant submits a counter-offer, the expiration timestamp is calculated as:
     $$\text{Expiry}_{\text{counter-offer}} = \text{now} + \text{counter\_offer\_expiry\_seconds}$$
     where `counter_offer_expiry_seconds` defaults to 48 hours (172,800 seconds) unless modified by admin.

---

### Deadline Extension (`extend_refund_deadline`)

The contract provides an administrative mechanism to extend the auto-reject deadline for specific refund requests that require ongoing off-chain review or mediation.

#### Caller Permissions and Constraints
- **Admin Only:** Only the designated contract admin address can execute this function (`require_admin`).
- **Contract State:** Contract execution must not be paused (`require_not_paused`).
- **Refund Status:** The target refund must exist and currently be in `RefundStatus::Requested`. If the refund is in any other state (e.g., `Approved`, `Rejected`, `CounterOffered`, `Cancelled`), the call panics with `"Refund is not in requested status"`.

#### Parameters
- `admin: Address` — The contract administrator address authorizing the extension.
- `refund_id: u32` — The unique ID of the pending refund request.
- `extra_seconds: u64` — The additional duration (in seconds) to add to the existing extension.

#### Storage and Behavior Mechanics
- **Additive Extensions:** Extensions are cumulative. The function reads any prior extension from persistent storage (`DataKey::RefundDeadlineExtension(refund_id)`), adds `extra_seconds`, and persists the updated sum:
  ```rust
  let key = DataKey::RefundDeadlineExtension(refund_id);
  let current_extension: u64 = env.storage().persistent().get(&key).unwrap_or(0);
  env.storage().persistent().set(&key, &(current_extension + extra_seconds));
  ```
- **TTL Bumping:** Automatically bumps persistent storage TTL for the extension record and refreshes contract instance TTL.
- **Impact on Auto-Reject:** Deferrals directly increase the threshold tested in `auto_reject_stale_refund`, allowing multiple successive extensions if complex reviews require additional time.

---

### Auto-Rejection of Stale Refunds (`auto_reject_stale_refund`)

When a refund request remains in `Requested` status without merchant resolution beyond the configured auto-reject window (and any administrative extensions), it becomes eligible for permissionless auto-rejection.

#### Trigger Conditions and Boundary Verification
- **Permission:** Anyone can call `auto_reject_stale_refund(refund_id)` once the boundary is crossed (permissionless crank).
- **Prerequisites:**
  - Contract is not paused.
  - Refund exists and is in `RefundStatus::Requested` (panics with `"Refund is not in requested status"` otherwise).
  - Auto-reject window is configured in contract storage.
- **Boundary Check:**
  $$\text{now} > \text{refund.requested\_at} + \text{auto\_reject\_window} + \text{extension}$$
  If `now <= deadline`, the transaction panics with `"Auto-reject window has not elapsed"`.

#### Execution Lifecycle and Side Effects
When executed successfully, `auto_reject_stale_refund` performs the following atomic operations:
1. **Escrow Refund Return:** Transfers the escrowed refund token amount held by the contract back to the customer (`token::Client::transfer(&contract, &refund.customer, &refund.amount)`).
2. **Status Transition:** Updates refund state to `RefundStatus::Rejected` and sets `refund.rejected_at = Some(now)`.
3. **Queue Eviction:** Removes the refund ID from the pending queue via `remove_from_pending_queue(&env, refund_id)`.
4. **Fraud & Merchant Metrics:**
   - Increments the customer fraud score by `+1` with reason symbol `"auto_rejected"`.
   - Records the rejection in merchant audit statistics via `update_stats_on_reject(&env, &refund.merchant)`.
5. **Event Emission:** Emits the `RefundAutoRejected` event containing `refund_id` and `elapsed_seconds` (`now - refund.requested_at`).
6. **Storage Maintenance:** Persists updated refund state and extends TTL for both persistent refund storage and instance storage.


## Escalation

Refund escalation is part of the refund dispute flow and is used when the initial review window expires without a final decision.

- **What triggers escalation:** a refund must first be in an escalatable state (`Requested`, `EvidenceSubmitted`, or `EvidencePeriodExpired`), and the primary review deadline must have passed. The contract rejects escalation before that deadline with `PrimaryDeadlineNotPassed`.
- **Who handles escalated refunds:** the configured senior arbiter handles escalated refunds. The arbiter address and the senior review window are set by the admin with `set_senior_arbiter` and `set_senior_review_window`.
- **What the senior arbiter can do:** the senior arbiter resolves the dispute with `resolve_escalated_refund(refund_id, approved, resolution_hash)`. If `approved` is `true`, the refund is processed and the customer receives the refund amount, minus any configured fee. If `approved` is `false`, the refund is marked rejected and the customer is returned the escrowed funds.
- **How the final outcome is enforced on-chain:** escalation moves the refund into `EscalatedToSenior` and stores the senior review deadline on-chain. Resolution can only be submitted by the configured senior arbiter, and the contract enforces the outcome by updating refund status and transferring tokens from the contract to the customer and, if configured, the fee recipient.
- **Missed senior deadline:** if `auto_approve_on_senior_miss` is enabled, anyone can call `trigger_senior_auto_approve(refund_id)` after the senior deadline passes. This finalizes the refund on-chain, marks it `Processed`, and records the auto-approval source as `senior_miss`.

## Events and error handling

- Events: `RefundRequested`, `RefundCreated`, `RefundApproved`, `RefundClaimed`, `RefundExpired`, `RefundCancelled`.
- Common errors: `NotAuthorized`, `InvalidRefundState`, `RefundNotFound`, `ClaimWindowExpired`, `InsufficientBalance`.

## Example (CLI)

Request a refund (participant):

```bash
stellar contract invoke --id <REFUND_CONTRACT_ID> --network testnet -- request_refund --refund-id <ID>
```

Approve (admin):

```bash
stellar contract invoke --id <REFUND_CONTRACT_ID> --network testnet -- approve_refund --refund-id <ID>
```

Claim (participant):

```bash
stellar contract invoke --id <REFUND_CONTRACT_ID> --network testnet -- claim_refund --refund-id <ID>
```

## Abuse Score

The refund contract tracks an **Abuse Score** per customer to prevent spam and dispute abuse.

### What actions increase the score
- **Refund Rejection:** When an admin rejects a refund request (`reject_refund`), the customer's score increases by `+10`.
- **Rapid Submission:** Submitting multiple refund requests within a very short timeframe (configured by `rapid_submission_window`) adds an immediate penalty of `+5` to the score.
- **Flagged Abuse:** If an admin explicitly flags a refund as abusive (`flag_refund_abuse`), it adds an elevated penalty (an additional `+10` on top of the standard rejection penalty).

### Thresholds and Restrictions
- The contract maintains an `abuse_block_threshold` (e.g., typically `30`).
- If a customer's score reaches or exceeds this threshold, they are temporarily blocked from submitting new refund requests (returning a `CustomerBlockedForAbuse` error).
- The block duration is determined by `block_duration_ledgers`. Once this period elapses (or the score decays below the threshold), the customer can request refunds again.

### Score Decay and Resets
- **Decay over time:** The abuse score decays automatically as ledgers advance. By default, the score halves (`5000 bps` factor) every `10,000` ledgers. Both the decay period and the decay factor are configurable by the admin (`set_abuse_score_decay_params`).
- **Manual Reset:** An admin can manually reset a customer's abuse score to zero using `reset_customer_abuse_score`.

## Counter-Offer Negotiation

When a customer requests a refund (`Requested` status), the merchant may respond with a counter-offer — a partial refund amount — instead of approving or rejecting the full request. This negotiation flow is implemented via the counter-offer system.

### How a buyer requests a refund and a merchant counters

1. **Customer requests refund:** The customer calls `request_refund()` with the full refund amount. The refund enters `Requested` status.
2. **Merchant counter-offers:** The merchant calls `counter_offer_refund(refund_id, amount)` to propose a lower amount. The refund moves to `CounterOffered` status and a `CounterOffer` record is stored with an expiry timestamp.
   - Only the refund's merchant can counter-offer.
   - The counter-offer amount must be positive and cannot exceed the original refund amount.
   - Only one counter-offer is permitted per refund (a second attempt panics with `Refund is not in Requested state`).

### How many rounds of negotiation are allowed

The negotiation is **single-round**: the merchant submits exactly one counter-offer. If the customer rejects or the offer expires, the refund escalates to admin review (`UnderAppeal`). There is no multi-round back-and-forth.

### How the flow resolves

The counter-offer resolves in one of four ways:

#### 1. Customer acceptance
The customer calls `accept_counter_offer(refund_id)`. The counter-offer amount is transferred immediately and the refund is marked `Processed`. If the offer has already expired when acceptance is attempted, it auto-escalates to admin instead.

#### 2. Customer rejection
The customer calls `reject_counter_offer(refund_id)`. The counter-offer record is removed and the refund is escalated to `UnderAppeal` for admin review.

#### 3. Expiry escalation
Anyone can call `check_counter_offer_expiry(refund_id)` after the offer's expiry timestamp passes. If expired, the refund escalates to `UnderAppeal` for admin review. The admin also has the option to call `settle_expired_counter_offer(refund_id)` which applies the contract's default resolution on expiry:
- **Accept original** (default): the original refund amount is paid out and the refund is `Processed`.
- **Reject**: the escrowed funds are returned to the customer and the refund is `Rejected`.

The admin can toggle the default resolution by setting the `CounterOfferDefaultResolution` configuration flag.

#### 4. Admin override via settle
The admin can configure the expiry window with `set_counter_offer_expiry_seconds(admin, seconds)` (default: 48 hours). The admin also controls the `CounterOfferDefaultResolution` flag (default: `true` = accept original on expiry).

### Key configuration constants

| Constant | Default | Description |
|---|---|---|
| `counter_offer_refund` expiry | 48 hours | Window for customer to respond to a counter-offer |
| `CounterOfferDefaultResolution` | `true` (accept original) | What happens on expiry — pay original amount or reject |

### Events

- `RefundCounterOffered` — emitted when a merchant submits a counter-offer.
- `RefundCounterAccepted` — emitted when the customer accepts the counter-offer.
- `RefundCounterRejected` — emitted when the customer rejects the counter-offer.
- `CounterOfferExpired` — emitted when a counter-offer expires and is settled.

## Notes for integrators

- Originating contracts should set refund `owner` and `amount` precisely to avoid disputes.
- Admin approvals should be auditable — consider storing `approver` and `approved_at` on the refund record.
- If automatic approvals are enabled, ensure checks are in place to prevent double refunds.

---

See `contracts/ahjoor-refund` for on-chain implementation details and exact function signatures.
