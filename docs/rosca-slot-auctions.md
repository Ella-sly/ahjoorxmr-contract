# ROSCA Slot Auctions

This document provides a comprehensive specification of the slot auction mechanisms in the `ahjoor-rosca` smart contract.

---

## 1. Overview

In an Ahjoor ROSCA (Rotating Savings and Credit Association) group, members receive the pooled contribution pot according to a pre-defined payout order (`PayoutOrder`). Slot auctions allow members to bid for an earlier payout position by offering a token payment. The winning bid amount is distributed back to the remaining active group members as a dividend bonus.

The `ahjoor-rosca` contract implements two distinct slot-auction mechanisms:

1. **Plain Slot Auction (Open-Bid):** A single-phase, transparent open auction automatically opened at the start of each ROSCA cycle where bids are publicly visible on-chain.
2. **Sealed-Bid Slot Auction (Commit-Reveal):** A multi-phase, anti-sniping auction where bids remain cryptographically hidden during a commit phase, verified during a reveal phase, and resolved against a configurable minimum reserve price.

---

## 2. Plain Slot Auction (Open-Bid)

### What It Is
The plain slot auction is an open bidding mechanism. Bidders deposit base tokens into the contract to indicate their desired slot index in the group's payout order. Bids are immediately visible on-chain to all participants.

### How an Auction Starts
A plain slot auction is enabled via the group configuration (`RoscaConfig.auction_enabled = true`).

- **Automatic Opening:** When a new ROSCA cycle begins (i.e. when `new_round % payout_order.len() == 0` during `reset_round_state`), the contract automatically opens an auction.
- **Window Initialization:** The contract calculates the open deadline:
  $$\text{AuctionOpenUntil} = \text{ledger\_timestamp} + \text{auction\_window\_ledgers}$$
- **State Setup:** Leftover bids from previous auctions are cleared (`DataKey3::AuctionBids`), and `DataKey3::AuctionRound` is set to the new round.

### Bid Submission and Updates
Members submit and manage bids using two contract functions:

#### 1. Placing a Bid
```rust
pub fn place_slot_bid(env: Env, bidder: Address, desired_slot: u32, bid_amount: i128)
```
- **Requirements & Validations:**
  - `auction_enabled` must be `true`.
  - Auction must be open (`AuctionOpenUntil > 0`) and the current ledger timestamp must not exceed `AuctionOpenUntil`.
  - `bid_amount` must be greater than `0`.
  - `bidder` must be an active group member (`DataKey::Members`).
  - `desired_slot` must be a valid slot index (`desired_slot < payout_order.len()`).
- **Collateral & Replacement:** The caller transfers `bid_amount` of base tokens to the contract. If the bidder already had an active bid, the previous deposit is atomically refunded to the bidder before the new deposit is taken.

#### 2. Updating an Existing Bid
```rust
pub fn update_slot_bid(env: Env, bidder: Address, desired_slot: u32, new_bid_amount: i128)
```
- **Requirements & Validations:** Same window, slot index, and member validations as `place_slot_bid`.
- **Atomic Replacement:** If no existing bid is found for `bidder`, the function panics with `ExtError2::NoBidFound`. Otherwise, the prior bid deposit is refunded, the new deposit is transferred, and the recorded `SlotBid` entry is updated.

### Bid Visibility
Plain auction bids are **fully public**:
- Every bid placement or update emits the `slot_bid_placed` event.
- Current bids can be inspected at any time using the read function `get_slot_bids(env)`.

### Bidding Window & Expiry
- The duration of the bidding window is governed by `auction_window_ledgers` (duration in seconds).
- Any attempt to call `place_slot_bid` or `update_slot_bid` after `env.ledger().timestamp() > AuctionOpenUntil` fails with `ExtError2::AuctionWindowClosed`.

### Querying Auction State
- `get_slot_bids(env) -> Vec<SlotBid>`: Returns all active bids placed during the current bidding window.
- `get_auction_status(env) -> (bool, u64, u64, u32)`: Returns `(auction_enabled, auction_window_ledgers, auction_open_until, auction_round)`.

### Resolution & Winner Determination
```rust
pub fn resolve_slot_auction(env: Env)
```
- **Execution:** Admin-triggered (`admin.require_auth()`).
- **Timing Guard:** Must be called after the bidding window has closed (`env.ledger().timestamp() > AuctionOpenUntil`). Calling while the window is still open panics with `ExtError2::AuctionWindowClosed`.
- **No Bids Placed:** If `AuctionBids` is empty, `resolve_slot_auction` acts as a no-op, clears `AuctionOpenUntil = 0`, and leaves the payout order unchanged.
- **Winning Criteria:**
  - **Highest Bid:** The bid with the maximum `amount` wins.
  - **Tie-Breaking:** If multiple bids have equal highest amounts, the bid with the earliest submission timestamp (`placed_at`) wins.

### Post-Resolution Operations
Upon resolving a plain slot auction:
1. **Losing Bids Refunded:** Bidders who did not win receive a 100% refund of their deposited bid amounts.
2. **Payout Order Swap:** The winner is moved into `desired_slot` in `PayoutOrder`. The member previously occupying `desired_slot` is moved to the winner's former slot position.
3. **Dividend Bonus Distribution:** The winning `bid_amount` is divided equally among all eligible non-winning active members (excluding the winner, exited members, and suspended members):
   $$\text{bonus\_per\_member} = \frac{\text{winning\_bid}}{\text{eligible\_count}}$$
   The calculated bonus is transferred immediately in base tokens to each eligible member.
4. **Cleanup & Event Emission:** Auction state is reset (`AuctionOpenUntil = 0`, `AuctionBids` cleared) and `slot_auction_resolved` is emitted.

---

## 3. Sealed-Bid Slot Auction (Commit-Reveal)

### What Makes It Different
The sealed-bid slot auction (`#375`) prevents bid sniping and front-running by separating bidding into two sequential phases: a **Commit Phase** and a **Reveal Phase**. Bidders submit a cryptographic commitment hash during the commit phase and reveal their actual bid parameters during the reveal phase. It also introduces a configurable **minimum reserve price**.

### Configuration and Opening

#### 1. Configuring Sealed Auction
```rust
pub fn configure_sealed_slot_auction(
    env: Env,
    admin: Address,
    commit_duration: u64,
    reveal_duration: u64,
    min_reserve: i128,
)
```
- Admin-only. Sets commit phase duration (seconds), reveal phase duration (seconds), and the minimum reserve price (`min_reserve`).
- Saves configuration in `SealedAuctionState` (`DataKey3::SealedAuction`).

#### 2. Opening Sealed Auction
```rust
pub fn open_sealed_slot_auction(env: Env, admin: Address, round: u32)
```
- Admin-only. Opens a sealed auction for the specified ROSCA `round`.
- Calculates deadlines:
  $$\text{commit\_until} = \text{now} + \text{commit\_duration}$$
  $$\text{reveal\_until} = \text{commit\_until} + \text{reveal\_duration}$$
- Initializes storage keys `DataKey3::SealedCommitters(round)` and `DataKey3::SealedRevealedBids(round)`.
- Emits the `sealed_auction_opened` event.

### Commit Phase (Bid Submission)
```rust
pub fn commit_slot_bid(env: Env, bidder: Address, commit_hash: BytesN<32>, deposit: i128)
```
- **Timing:** Valid only while `env.ledger().timestamp() <= commit_until` and `state.open == true`.
- **Identity-Bound Hash:** `commit_hash` must equal the SHA-256 hash of the bidder's address, bid amount, and a secret 32-byte salt:
  $$\text{commit\_hash} = \text{sha256}(\text{bidder.to\_xdr()} \mathbin{\Vert} \text{bid\_amount.to\_be\_bytes()} \mathbin{\Vert} \text{salt})$$
  Binding `bidder.to_xdr()` into the hash prevents malicious actors from copying another user's commitment hash.
- **Deposit Collateral:** The bidder transfers `deposit` base tokens to the contract upfront. `deposit` serves as collateral and sets the upper bound for the bid that can be revealed later.
- **Constraint:** One commitment per bidder per round (`DataKey3::SlotBidCommit(round, bidder)`).

### Reveal Phase (Bid Unveiling)
```rust
pub fn reveal_slot_bid(
    env: Env,
    bidder: Address,
    desired_slot: u32,
    bid_amount: i128,
    salt: BytesN<32>,
)
```
- **Timing:** Valid only during the reveal phase (`commit_until < env.ledger().timestamp() <= reveal_until`). Calling before `commit_until` panics with `"Reveal phase has not opened yet"`. Calling after `reveal_until` panics with `ExtError2::AuctionWindowClosed`.
- **Cryptographic Verification:**
  - Computes $\text{sha256}(\text{bidder.to\_xdr()} \mathbin{\Vert} \text{bid\_amount.to\_be\_bytes()} \mathbin{\Vert} \text{salt})$ and verifies it matches stored `commit_hash`. Panics with `"Revealed values do not match commitment"` on mismatch.
  - Verifies `bid_amount > 0` and `bid_amount <= commit.deposit`.
  - Verifies `desired_slot < payout_order.len()`.
- **State Update:** Marks `commit.revealed = true` and appends the validated `SlotBid` to `DataKey3::SealedRevealedBids(round)`. Emits `slot_bid_revealed`.

### Settlement & Winner Determination
```rust
pub fn settle_sealed_slot_auction(env: Env)
```
- **Execution:** Admin-triggered (`admin.require_auth()`).
- **Timing Guard:** Must be called strictly after the reveal phase has closed (`env.ledger().timestamp() > reveal_until`). Calling earlier panics with `ExtError2::AuctionWindowClosed`.
- **Minimum Reserve Enforcement:**
  - To qualify as a winner, a revealed bid must **strictly exceed** `min_reserve` ($\text{bid\_amount} > \text{min\_reserve}$).
  - If no revealed bid exceeds `min_reserve`, no winner is selected and the slot is left unallocated.
- **Winning Criteria:**
  - The highest valid revealed bid above `min_reserve` wins.
  - Tie-breaking: Earliest reveal timestamp (`placed_at`).

### Post-Settlement Refunds & Forfeiture
1. **Revealed Losing Bidders:** Fully refunded their deposited collateral (`commit.deposit`).
2. **Winner Settlement:** Refunded $\text{commit.deposit} - \text{winning\_amount}$ (net charge equals `winning_amount`). Moved into `desired_slot` in `PayoutOrder` (swapped with existing occupant).
3. **Unrevealed Committers (Forfeiture):** Committers who failed to reveal their bids before `reveal_until` **forfeit their deposit**. Forfeited deposits remain in contract balance.
4. **Dividend Bonus Distribution:** `winning_amount` is divided equally among non-winning active members ($\text{winning\_amount} / \text{eligible\_count}$) and paid out immediately.
5. **Auction Close:** `state.open = false` is saved and `sealed_auction_settled` is emitted.

### Querying Sealed Auction State
- `get_sealed_auction(env) -> Option<SealedAuctionState>`: Returns live configuration and phase status.
- `get_sealed_revealed_bids(env, round) -> Vec<SlotBid>`: Returns list of successfully revealed bids for `round`.

---

## 4. Comparison Table

| Feature / Attribute | Plain Slot Auction (Open-Bid) | Sealed-Bid Slot Auction (Commit-Reveal) |
| :--- | :--- | :--- |
| **Bid Visibility** | Publicly visible on-chain immediately upon placement (`get_slot_bids`). | Hidden during commit phase; revealed only during reveal phase (`get_sealed_revealed_bids`). |
| **Submission Process** | Direct placement via `place_slot_bid` / `update_slot_bid`. | Two-step: `commit_slot_bid` (hash + deposit) then `reveal_slot_bid` (values + salt). |
| **Phases & Windows** | Single bidding window (`auction_window_ledgers`). | Two sequential phases: Commit phase (`commit_duration`) followed by Reveal phase (`reveal_duration`). |
| **Minimum Reserve Price** | None (any positive bid amount is valid). | Configurable `min_reserve` (winning bid must strictly exceed reserve). |
| **Tie-Breaking Rule** | Earliest bid submission timestamp (`placed_at`). | Earliest reveal timestamp (`placed_at`). |
| **Unrevealed Bids** | N/A (bids are submitted directly). | Deposits of unrevealed commitments are **forfeited** to the contract. |
| **Refund Logic** | Losers refunded 100%; Winner charged exact `bid_amount`. | Revealed losers refunded 100%; Winner refunded `deposit - winning_amount`. |
| **Bonus Distribution** | Winning bid distributed equally to non-winning active members. | Winning bid distributed equally to non-winning active members. |
| **Payout Order Swap** | Winner swapped into `desired_slot` with existing occupant. | Winner swapped into `desired_slot` with existing occupant. |
| **Trigger Mechanism** | Automatically opened at cycle start; resolved by admin. | Manually configured & opened per round by admin; settled by admin. |
| **Read APIs** | `get_slot_bids`, `get_auction_status` | `get_sealed_auction`, `get_sealed_revealed_bids` |

---

## 5. End-to-End Auction Lifecycle

### Plain Slot Auction Lifecycle

```
[ Cycle Starts (new_round % len == 0) ]
                 │
                 ▼
[ Auction Opens (AuctionOpenUntil = now + window) ]
                 │
                 ▼
[ Bidders Call place_slot_bid() / update_slot_bid() ]
                 │
                 ▼
[ Bidding Window Closes (now > AuctionOpenUntil) ]
                 │
                 ▼
[ Admin Calls resolve_slot_auction() ]
                 │
  ┌──────────────┴──────────────┐
  ▼                             ▼
[ Bids Exist ]            [ No Bids ]
  │                             │
  ├─ Highest Bid Wins           └─ Clear Window (No-op)
  ├─ Tie-break: Earliest
  ├─ Swap Winner in PayoutOrder
  ├─ Refund Losing Bidders
  └─ Distribute Winning Bid as Dividend Bonus to Active Non-Winners
```

---

### Sealed-Bid Slot Auction Lifecycle

```
[ Admin Configures Auction (configure_sealed_slot_auction) ]
                 │
                 ▼
[ Admin Opens Auction for Round (open_sealed_slot_auction) ]
                 │
                 ▼
[ COMMIT PHASE: Members Call commit_slot_bid() (Hash + Deposit) ]
                 │
                 ▼
[ Commit Phase Closes (now > commit_until) ]
                 │
                 ▼
[ REVEAL PHASE: Members Call reveal_slot_bid() (Amount + Salt + Slot) ]
                 │
                 ▼
[ Reveal Phase Closes (now > reveal_until) ]
                 │
                 ▼
[ Admin Calls settle_sealed_slot_auction() ]
                 │
  ┌──────────────┴─────────────────────────┐
  ▼                                        ▼
[ Valid Revealed Bid > Reserve ]    [ No Revealed Bid > Reserve ]
  │                                        │
  ├─ Highest Revealed Bid Wins             ├─ Refund All Revealed Deposits
  ├─ Winner Swapped in PayoutOrder         ├─ Forfeit Unrevealed Commitments
  ├─ Winner Refunded Deposit - Bid         └─ Slot Left Unallocated
  ├─ Revealed Losers Refunded 100%
  ├─ Unrevealed Deposits Forfeited
  └─ Distribute Winning Bid as Bonus to Active Non-Winners
```

---

## 6. Contract Code & Error Codes Reference

### Data Structures (`types.rs`)

```rust
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SlotBid {
    pub bidder: Address,
    pub desired_slot: u32,
    pub amount: i128,
    pub placed_at: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SealedAuctionState {
    pub enabled: bool,
    pub commit_duration: u64,
    pub reveal_duration: u64,
    pub min_reserve: i128,
    pub round: u32,
    pub commit_until: u64,
    pub reveal_until: u64,
    pub open: bool,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SealedCommit {
    pub commit_hash: BytesN<32>,
    pub deposit: i128,
    pub revealed: bool,
}
```

### Relevant Error Codes & Messages

| Error / Message | Context | Description |
| :--- | :--- | :--- |
| `AuctionNotEnabled` | Plain Auction | Plain auction feature is not enabled in group configuration. |
| `AuctionNotOpen` | Plain Auction | No plain auction is currently open. |
| `AuctionWindowClosed` | Both Auctions | Attempted bid, update, reveal, or premature resolution before window closed / after window expired. |
| `InvalidSlotIndex` | Plain Auction | `desired_slot` exceeds length of `PayoutOrder`. |
| `NoBidFound` | Plain Auction | `update_slot_bid` called by a member who has no existing bid. |
| `"Reveal phase has not opened yet"` | Sealed Auction | `reveal_slot_bid` called while commit phase is still active. |
| `"Revealed values do not match commitment"` | Sealed Auction | Revealed `(bid_amount, salt)` SHA-256 hash does not match `commit_hash`. |
| `"Revealed bid exceeds committed deposit"` | Sealed Auction | Revealed bid amount is greater than the collateral deposit submitted during commit. |
