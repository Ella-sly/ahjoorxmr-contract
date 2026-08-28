# ROSCA Round-Skip Mechanism (`ahjoor-rosca`)

This document describes the **round-skip** mechanism in the `ahjoor-rosca` smart contract: how members can temporarily skip contributing to a specific round without defaulting, the fee and cycle limit rules governing skips, and how skipping impacts payout recipient selection, pot balances, and cycle audit records.

> Implementation references:
> - Contract Entrypoint: [`contracts/ahjoor-rosca/src/lib.rs`](file:///home/muhammad-mustapha/dev/muhammad/ahjoorxmr-contract/contracts/ahjoor-rosca/src/lib.rs) (`request_skip`, `close_round`, `finalize_round`, `init`)
> - Internal Settlement Logic: [`contracts/ahjoor-rosca/src/internals.rs`](file:///home/muhammad-mustapha/dev/muhammad/ahjoorxmr-contract/contracts/ahjoor-rosca/src/internals.rs) (`select_payout_recipient`, `record_cycle_snapshot`)
> - State & Storage Types: [`contracts/ahjoor-rosca/src/types.rs`](file:///home/muhammad-mustapha/dev/muhammad/ahjoorxmr-contract/contracts/ahjoor-rosca/src/types.rs) (`DataKey2::SkipFee`, `DataKey2::MaxSkipsPerCycle`, `DataKey2::SkipRequests`, `DataKey2::MemberSkips`)
> - Event Definitions: [`contracts/ahjoor-rosca/src/events.rs`](file:///home/muhammad-mustapha/dev/muhammad/ahjoorxmr-contract/contracts/ahjoor-rosca/src/events.rs) (`RoundSkipRequested`)
> - Error Definitions: [`contracts/ahjoor-rosca/src/errors.rs`](file:///home/muhammad-mustapha/dev/muhammad/ahjoorxmr-contract/contracts/ahjoor-rosca/src/errors.rs) and [Error Codes Reference](errors.md)
> - Test Suite: [`contracts/ahjoor-rosca/src/test_skip.rs`](file:///home/muhammad-mustapha/dev/muhammad/ahjoorxmr-contract/contracts/ahjoor-rosca/src/test_skip.rs)

---

## 1. Overview

In rotating savings and credit associations (ROSCAs), members commit to contributing a fixed token amount during every round. However, unpredictable cash flow disruptions or temporary liquidity constraints can make it difficult for a member to deposit their contribution on time.

The **round-skip mechanism** allows an active ROSCA member to formally excuse themselves from a round's deposit obligation by submitting a skip request before the round deadline.

### Skipping vs. Defaulting

| Attribute | Defaulting (Unexcused Missed Deposit) | Round Skip (`request_skip`) |
| :--- | :--- | :--- |
| **Defaulter Record** | Added to `DataKey::Defaulters` | Excluded from `DataKey::Defaulters` |
| **Default Counter** | Increments `DataKey::DefaultCount` strike | No default strike recorded (`default_count` unchanged) |
| **Suspension Risk** | Suspended if default strikes reach `max_defaults` | No suspension risk |
| **Penalty Charge** | Incurs `penalty_amount` deduction | Incurs `skip_fee` upfront (if configured) |
| **Pot Impact** | Pot is reduced by the missing contribution | Pot includes the non-refundable `skip_fee` |
| **Payout Eligibility** | Delinquent members can be excluded | Skipped member is bypassed if scheduled for payout this round |
| **Cycle Bonus** | Disqualified from perfect cycle bonus | Disqualified from perfect cycle bonus (`CycleBonusAmount`) |

### Configuration Parameters

The skip mechanism is configured during ROSCA initialization via `RoscaConfig`:

```rust
pub struct RoscaConfig {
    // ...
    pub skip_fee: i128,             // Fee charged in base token to skip a round
    pub max_skips_per_cycle: u32,   // Maximum allowed skips per member per cycle
    // ...
}
```

- **`skip_fee` (`i128`)**: The fee charged in the group's base token for each skip request. If set to `0`, skips are free. If `> 0`, the fee is debited immediately from the member and held in the contract balance, directly augmenting the round's pooled payout.
- **`max_skips_per_cycle` (`u32`)**: The ceiling on how many rounds a single member can skip within a single cycle.

### Storage Layout

Skip state is persisted in instance storage under `DataKey2`:

| Storage Key | Storage Type | Rust Type | Description |
| :--- | :--- | :--- | :--- |
| `DataKey2::SkipFee` | Instance | `i128` | The configured token fee required to request a skip. |
| `DataKey2::MaxSkipsPerCycle` | Instance | `u32` | The maximum skips allowed per member per cycle. |
| `DataKey2::SkipRequests` | Instance | `Map<(Address, u32), bool>` | Maps `(member, round)` to boolean indicating if a skip was requested. |
| `DataKey2::MemberSkips` | Instance | `Map<(Address, u32), u32>` | Maps `(member, cycle_index)` to the count of skips used in that cycle. |

---

## 2. Skip Eligibility

A member can successfully request a round skip only if all of the following state constraints and conditions are satisfied:

```
                      +-----------------------------+
                      | Member calls request_skip   |
                      +--------------+--------------+
                                     |
                                     v
                        [ Contract unpaused? ] --------No-----> Revert: ContractPaused (1025)
                                     | Yes
                                     v
                         [ Caller authorized? ] -------No-----> Revert: Auth Error
                                     | Yes
                                     v
                         [ Is active member? ] --------No-----> Revert: NotAMember (1008)
                                     | Yes
                                     v
                        [ round >= current_round? ] ---No-----> Revert: RoundDeadlinePassed (1006)
                                     | Yes
                                     v
                 [ If round == current: before deadline? ] -No-> Revert: ContributionWindowClosed (1033)
                                     | Yes
                                     v
                 [ If round == current: not yet paid? ] ----No-> Revert: AlreadyContributed (1009)
                                     | Yes
                                     v
                        [ Already skipped round? ] ----Yes----> Revert: AlreadySkipped (1055)
                                     | No
                                     v
                    [ Member skips < max_skips? ] -----No-----> Revert: SkipLimitReached (1054)
                                     | Yes
                                     v
                    [ Transfer skip_fee successful? ] -No-----> Revert: Token transfer failure
                                     | Yes
                                     v
                         +-----------------------+
                         | Record Skip & Emit    |
                         +-----------------------+
```

### Eligibility Requirements

1. **Contract Operational Status**: The ROSCA contract must not be paused (`internals::check_not_paused(&env)`). If paused, reverts with `ContractPaused` (`1025`).
2. **Caller Authentication**: The caller must provide valid cryptographic authorization matching `member` (`member.require_auth()`).
3. **Active Membership**: The address must exist in the group's registered member roster (`DataKey::Members`). Non-members revert with `NotAMember` (`1008`).
4. **Round Timing and Temporal Validity**:
   - **Past Rounds Disallowed**: The requested `round` must be greater than or equal to `DataKey::CurrentRound`. Attempting to skip an already elapsed round reverts with `RoundDeadlinePassed` (`1006`).
   - **Contribution Window Open**: For the active round (`round == current_round`), the request must arrive before the round deadline (`env.ledger().timestamp() <= deadline`). If the deadline has expired, the contribution window is closed and the call reverts with `ContributionWindowClosed` (`1033`).
5. **No Prior Contribution for the Round**: If `round == current_round`, the member must not already be recorded in `DataKey::PaidMembers`. A member cannot contribute first and then request a skip for the same round; doing so reverts with `AlreadyContributed` (`1009`).
6. **No Duplicate Skip Request**: The member must not have already submitted a skip for the given round. If `SkipRequests.get((member, round))` is already `true`, the call reverts with `AlreadySkipped` (`1055`).
7. **Within Per-Cycle Allowance**: The member must not have exhausted their skip quota for the target cycle (`cycle_index = round / payout_order.len()`). If `current_skips >= max_skips_per_cycle`, the call reverts with `SkipLimitReached` (`1054`).
8. **Skip Fee Solvency**: If `skip_fee > 0`, the member must hold sufficient balance and allowance in the group's base token. The contract immediately transfers `skip_fee` tokens from `member` to the contract address.

---

## 3. Request Flow (`request_skip`)

### Function Signature

```rust
pub fn request_skip(env: Env, member: Address, round: u32)
```

### Step-by-Step Execution Lifecycle

1. **Authorization and State Guards:**
   The contract verifies that execution is not paused, enforces `member.require_auth()`, and verifies the caller is a registered group member.

2. **Timing Validation:**
   The contract fetches `current_round` and the active deadline (`RoundDeadlineTimestamp` if `UseTimestampSchedule` is active, otherwise `RoundDeadline`). It validates that `round >= current_round`, and if `round == current_round`, confirms `env.ledger().timestamp() <= deadline`.

3. **Contribution Check:**
   If `round == current_round`, the contract inspects `DataKey::PaidMembers`. If the member has already contributed, execution aborts with `Error::AlreadyContributed`.

4. **Cycle Calculation and Allowance Check:**
   The contract calculates the cycle index for the specified round:
   $$\text{cycle\_index} = \left\lfloor \frac{\text{round}}{\text{payout\_order.len()}} \right\rfloor$$
   It checks `DataKey2::MemberSkips` for `(member, cycle_index)`. If `current_skips >= max_skips_per_cycle`, execution aborts with `ExtError::SkipLimitReached`.

5. **Fee Transfer:**
   If `skip_fee > 0`, the contract invokes the Soroban token client for the group's base token (`DataKey::Token`) to transfer `skip_fee` tokens from `member` to `env.current_contract_address()`.

6. **State Storage Updates:**
   - Marks `SkipRequests.set((member, round), true)`.
   - Increments `MemberSkips.set((member, cycle_index), current_skips + 1)`.
   - Persists the updated maps to instance storage.

7. **Event Emission:**
   Emits the `RoundSkipRequested` contract event.

8. **Storage TTL Extension:**
   Extends instance storage TTL (`INSTANCE_LIFETIME_THRESHOLD`, `INSTANCE_BUMP_AMOUNT`).

---

## 4. Downstream Effects on Group Settlement & Progression

Submitting a skip request alters several core mechanisms during subsequent round progression and finalization:

### 1. Exemption from Defaulter Status

During `finalize_round` and `close_round`:
```rust
let mut defaulters = Vec::new(&env);
for member in members.iter() {
    let has_skipped = skip_requests
        .get((member.clone(), current_round))
        .unwrap_or(false);
    if !paid_members.contains(&member) && !exited_members.contains(&member) && !has_skipped {
        defaulters.push_back(member);
    }
}
```
- Members who skipped are **not** appended to `DataKey::Defaulters`.
- They do **not** receive an increment to their `DataKey::DefaultCount`.
- They do **not** trigger default penalties or delinquency suspensions.

### 2. Payout Recipient Selection

When determining who receives the pot during `finalize_round`:
```rust
let mut recipient_idx = (current_round % payout_order.len()) as u32;
let mut attempts = 0;
while attempts < payout_order.len() {
    let potential_recipient = payout_order.get(recipient_idx).unwrap();
    let has_skipped = skip_requests.get((potential_recipient.clone(), current_round)).unwrap_or(false);
    if !suspended_members.contains(&potential_recipient)
        && !exited_members.contains(&potential_recipient)
        && !has_skipped
    {
        break;
    }
    recipient_idx = (recipient_idx + 1) % payout_order.len();
    attempts += 1;
}
```
- If the member scheduled for payout in `payout_order` has skipped the current round (`has_skipped == true`), the selection loop automatically bypasses them and advances to the next eligible member in the order.

### 3. Pot Accumulation from Skip Fees

The `skip_fee` paid by skipping members remains in the contract's token account. During `finalize_round`, the total pot paid to the round's recipient is calculated from the contract's base token balance (minus protocol fees). Consequently, skip fees directly supplement the pot received by the active recipient.

### 4. Cycle Audit Records

When `finalize_round` snapshots the cycle state (`internals::record_cycle_snapshot`), it aggregates all addresses that skipped the round:
```rust
let mut skippers: Vec<Address> = Vec::new(env);
for (key, skipped) in skip_requests.iter() {
    let (addr, round_num) = key;
    if skipped && round_num == current_round {
        skippers.push_back(addr);
    }
}
```
The list of skipping members is stored in the immutable `CycleRecord` and can be retrieved via `client.get_cycle_record(cycle_number)`.

### 5. Disqualification from Perfect Cycle Bonus

If the group is configured with a cycle completion bonus (`DataKey4::CycleBonusAmount`), the contract verifies at the end of each cycle whether members completed all rounds without defaults and without skips:
```rust
if defaults == 0 && !had_skip {
    qualifying.push_back(member);
}
```
Requesting one or more skips in a cycle disqualifies that member from sharing in the cycle bonus reward pool for that cycle.

---

## 5. Per-Cycle Skip Limits

Ahjoor ROSCAs operate in recurring cycles. Each cycle consists of $N$ rounds, where $N$ equals the number of slots in `payout_order` (`cycle_len = payout_order.len()`).

### Cycle Calculation

Given a target round number $R$ and group size $N$:

$$\text{cycle\_index} = \left\lfloor \frac{R}{N} \right\rfloor$$

For example, in a 4-member group ($N = 4$):
- **Cycle 0:** Rounds 0, 1, 2, 3
- **Cycle 1:** Rounds 4, 5, 6, 7
- **Cycle 2:** Rounds 8, 9, 10, 11

### Quota Tracking and Enforcement

- Skips are tracked per member per cycle in `DataKey2::MemberSkips` keyed by `(Address, cycle_index)`.
- When a member submits `request_skip(member, round)`, the contract retrieves `current_skips` for that `(member, cycle_index)`.
- If `current_skips >= max_skips_per_cycle`, the call is rejected with `ExtError::SkipLimitReached` (`1054`).
- When the ROSCA advances into a new cycle ($R \ge (C+1) \times N$), the cycle index increments. Since `MemberSkips` is indexed by cycle number, the member starts with `0` recorded skips for the new cycle, resetting their available allowance to `max_skips_per_cycle`.
- **Advance Booking:** A member may request a skip for a future round in an upcoming cycle (e.g. Round 5 while currently in Round 1). The contract derives `cycle_index = 1` from the target round and tracks that skip against Cycle 1's allowance.

---

## 6. Constraints & Edge Cases Matrix

| Situation | Contract Behaviour | Error Code |
| :--- | :--- | :--- |
| **Caller is not a registered member** | Rejects request; non-members cannot interact with round state. | `NotAMember` (`1008`) |
| **Contract is paused** | Rejects request via `internals::check_not_paused`. | `ContractPaused` (`1025`) |
| **Skip requested for past round (`round < current_round`)** | Rejects request; retroactive skips are not allowed. | `RoundDeadlinePassed` (`1006`) |
| **Skip requested after round deadline** | Rejects request if `round == current_round` and deadline has elapsed. | `ContributionWindowClosed` (`1033`) |
| **Member already paid contribution this round** | Rejects request; paid members cannot convert contribution into a skip. | `AlreadyContributed` (`1009`) |
| **Duplicate skip request for same round** | Rejects request if already marked in `DataKey2::SkipRequests`. | `AlreadySkipped` (`1055`) |
| **Member reached cycle limit (`current_skips >= max_skips`)** | Rejects request; skip quota exhausted for this cycle. | `SkipLimitReached` (`1054`) |
| **`skip_fee > 0` but insufficient balance/allowance** | Rejects request on base token contract transfer. | Host error |
| **`max_skips_per_cycle` configured as `0`** | Rejects all skip requests; skipping is disabled for the group. | `SkipLimitReached` (`1054`) |
| **Scheduled recipient skips their payout round** | Contract skips that member and awards payout to the next eligible member in `payout_order`. | N/A (Success) |
| **Skipping member in cycle with `CycleBonusAmount`** | Member avoids default penalties but does not qualify for the cycle bonus. | N/A (Settlement rule) |

---

## 7. Events Reference

### `RoundSkipRequested`

Emitted immediately when a member successfully submits a skip request.

```rust
#[contractevent]
#[derive(Clone, Debug)]
pub struct RoundSkipRequested {
    pub member: Address,
    pub round: u32,
    pub fee_paid: i128,
}
```

| Field | Type | Description |
| :--- | :--- | :--- |
| `member` | `Address` | Stellar address of the member requesting the skip. |
| `round` | `u32` | The round sequence number being skipped. |
| `fee_paid` | `i128` | The token fee amount debited from the member for the skip. |

---

## 8. Error Codes Reference

| Soroban Error Enum | Contract Code | Consolidated Code ([`errors.md`](errors.md)) | Error Name | Trigger Condition |
| :--- | :--- | :--- | :--- | :--- |
| `Error` | `6` | `1006` | `RoundDeadlinePassed` | Target `round` is less than `current_round`. |
| `Error` | `8` | `1008` | `NotAMember` | Caller is not a registered member of the ROSCA group. |
| `Error` | `9` | `1009` | `AlreadyContributed` | Member has already contributed to the current round. |
| `Error` | `25` | `1025` | `ContractPaused` | Contract is currently paused by the admin. |
| `Error` | `33` | `1033` | `ContributionWindowClosed` | Current round deadline timestamp has already passed. |
| `ExtError` | `54` | `1054` | `SkipLimitReached` | Member has reached or exceeded `max_skips_per_cycle` for the cycle. |
| `ExtError` | `55` | `1055` | `AlreadySkipped` | Member has already requested a skip for the target round. |

---

## 9. Function Reference

### `request_skip`

Submits an authorized skip request for a specific round.

```rust
pub fn request_skip(env: Env, member: Address, round: u32)
```

- **Caller:** `member` (must sign transaction via `member.require_auth()`).
- **Arguments:**
  - `member`: The `Address` of the skipping member.
  - `round`: The `u32` sequence number of the round to skip.
- **Returns:** `()` on success; panics with error code on failure.
