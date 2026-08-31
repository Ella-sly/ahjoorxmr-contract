# Escrow Dispute Timeout Handling

This document focuses specifically on the dispute-timeout mechanism in the `ahjoor-escrow` contract: how the timeout window is determined, and what happens when it is reached. It complements [Escrow Dispute Flow](escrow-dispute-flow.md), which covers the full dispute lifecycle.

Behavior described here is exercised by the tests in `contracts/ahjoor-escrow/src/test_dispute_timeout.rs`.

## Timeout Window

- **Deadline start**: recorded as `DataKey::DisputeDeadlineStart(escrow_id)` — the ledger timestamp at the moment `dispute_escrow` is called.
- **Effective timeout duration**, in priority order:
  1. **Per-escrow override**: set via `create_escrow_w_timeout(..., dispute_timeout_seconds)`, persisted on `escrow.extensions.dispute_timeout_seconds`.
  2. **Global default**: `604,800` seconds (7 days), configurable by the admin via `update_default_dispute_timeout(admin, timeout_seconds)` and readable via `get_default_dispute_timeout()`.
- **Expiration**: the deadline is considered passed once `current_ledger_timestamp - deadline_start >= effective_timeout`.

## Enforcing the Timeout

`enforce_dispute_timeout(escrow_id)` is the public entry point that applies the default outcome once the window has elapsed:

- **Permissionless**: it is a plain `pub fn` with no `require_auth()` call — any address (buyer, seller, admin, or an off-chain keeper bot) can invoke it.
- **Preconditions**, all of which panic if unmet:
  - Contract is not paused.
  - The escrow exists.
  - A dispute record exists for the escrow and `dispute.resolved == false`.
  - Escrow status is `Disputed` or `PartiallyDisputed` (panics `"Escrow is not disputed"` otherwise).
  - The deadline has actually passed (panics `"Dispute timeout deadline has not passed yet"` otherwise).

## Default Resolution Outcome

When `enforce_dispute_timeout` succeeds, the outcome is determined by a configurable **default winner**, `DisputeDefaultWinner`:

- **Global default**: set by the admin via `set_default_dispute_winner(admin, winner)`, read via `get_default_dispute_winner()`. Values are `Buyer` (`0`, the default if never configured) or `Seller` (`1`).
- **Per-escrow override**: set at creation time via `dispute_default_winner` on `EscrowCreateRequest` (used with `create_escrow_v2`), persisted on `escrow.extensions.dispute_default_winner`.

Fund distribution then follows the winner:

- **Buyer wins**: the disputed amount is refunded to the buyer(s); escrow status becomes `Refunded`.
- **Seller wins**: the disputed amount is released to the seller; escrow status becomes `Released`.
- **Partial disputes**: if the dispute only covered part of the total (`PartiallyDisputed`), the undisputed portion was already sent to the seller when `dispute_escrow` was called — only the remaining locked portion is moved to the default winner on timeout.

Additional side effects on enforcement:

- Any minted receipt NFT for the escrow is burned (`burn_receipt_if_exists`).
- The dispute record is marked `resolved = true`.
- The assigned arbiter's `ArbiterTimeoutCount` is incremented by 1, tracking missed deadlines for reputation purposes (see `get_arbiter_timeout_count`).
- Events `DisputeTimedOut(escrow_id, arbiter, default_winner, elapsed_seconds)` and `ArbitersTimeoutPenaltyApplied(arbiter, new_timeout_count)` are emitted.

## Notes

- Once an escrow reaches a terminal status (`Resolved`, `Released`, `Refunded`) — whether via `resolve_dispute` or via timeout enforcement — it cannot be disputed again.
- Reassigning or removing an arbiter near the deadline does **not** pause, reset, or extend `DisputeDeadlineStart`; the timer runs from the original `dispute_escrow` call regardless of arbiter changes.
