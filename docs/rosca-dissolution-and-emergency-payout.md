# ROSCA Group Dissolution and Emergency Payout Voting

> **Status:** Implemented in `contracts/ahjoor-rosca/src/lib.rs`
> (`set_dissolution_config`, `start_dissolution_vote`, `vote_dissolve_group`,
> `execute_dissolution`, `dissolve_group`, `set_emergency_payout_config`,
> `request_emergency_payout`, `vote_emergency_payout`, `execute_emergency_payout`)
> and related helpers in `contracts/ahjoor-rosca/src/internals.rs`.
> Events are defined in `contracts/ahjoor-rosca/src/events.rs`.

---

## Overview

`ahjoor-rosca` provides two distinct collective-exit mechanisms that let members
bring a group's lifecycle to a controlled end, and one mechanism that allows a
member to draw funds early without ending the group:

1. **Group dissolution** — permanently winds down the group and distributes the
   entire remaining pool to all members pro-rata. Two paths exist:
   - *Admin-initiated*: the contract admin calls `dissolve_group` directly,
     supplying an optional reason hash. No vote is required.
   - *Member-initiated*: any member opens a timed vote with
     `start_dissolution_vote`; once quorum is reached after the window closes,
     anyone may call `execute_dissolution` to trigger the actual dissolution.

2. **Emergency payout voting** — a member requests a one-time early payout of
   their full contribution amount from the pool. The request must pass a group
   vote (request → vote → execute) before any funds move. The group continues
   operating normally after an emergency payout.

> **Note:** This document covers emergency *payout* voting only. Emergency
> *reserve loans* are a separate feature documented in
> [docs/rosca-emergency-loan.md](rosca-emergency-loan.md).

| Fact | Value |
|---|---|
| Contract | `ahjoor-rosca` |
| Dissolution entry points | `set_dissolution_config`, `start_dissolution_vote`, `vote_dissolve_group`, `execute_dissolution`, `dissolve_group` |
| Emergency payout entry points | `set_emergency_payout_config`, `request_emergency_payout`, `vote_emergency_payout`, `execute_emergency_payout` |
| Dissolution storage keys | `DataKey2::DissolutionConfig`, `DataKey2::DissolutionVotes`, `DataKey2::DissolutionVoteCount`, `DataKey2::DissolutionDeadline`, `DataKey2::GroupStatus` |
| Emergency payout storage keys | `DataKey2::EmergencyPayoutConfig`, `DataKey2::EmergencyPayoutRequests`, `DataKey2::EmergencyPayoutVotes`, `DataKey2::EmergencyPayoutApproved`, `DataKey2::EmergencyPayoutCount` |

---

## Part 1: Group Dissolution

### Lifecycle

#### Path A — Admin-initiated (direct)

```text
admin calls dissolve_group(admin, reason_hash)
  ├─ checks: not dissolved, pool > 0
  ├─ sets GroupStatus::Dissolved
  ├─ distribute_pro_rata → transfer share to each member
  │    └─ emits MemberRefunded (one per member with share > 0)
  └─ emits GroupDissolved
```

#### Path B — Member-initiated (democratic vote)

```text
any member calls start_dissolution_vote(member)
  ├─ checks: not dissolved, member present, no live vote in this round
  ├─ sets DissolutionDeadline[current_round] = now + vote_window_seconds
  ├─ resets DissolutionVoteCount[current_round] = 0
  └─ emits DissolutionVoteStarted

members call vote_dissolve_group(voter, approve) [before deadline]
  ├─ checks: not dissolved, voter is member, vote not already cast
  ├─ adds voter_weight to DissolutionVoteCount[round] if approve
  └─ emits DissolutionVoteCast

— voting window expires —

anyone calls execute_dissolution()
  ├─ checks: deadline has passed, quorum met
  ├─ emits DissolutionQuorumReached
  └─ calls dissolve_group(admin, [0u8;32])
       ├─ sets GroupStatus::Dissolved
       ├─ distribute_pro_rata
       └─ emits GroupDissolved
```

### 1. Configure — `set_dissolution_config`

```rust
pub fn set_dissolution_config(
    env: Env,
    admin: Address,
    dissolution_quorum_bps: u32,
    vote_window_seconds: u64,
)
```

**Authentication:** `admin.require_auth()`. The caller must equal the stored
`DataKey::Admin`; panics with `"Only admin can set dissolution config"` otherwise.

| Parameter | Meaning | Validation |
|---|---|---|
| `dissolution_quorum_bps` | Share of possible votes required for dissolution to succeed, in basis points. `7500` = 75%. | Must be `1000`–`10000` inclusive, else `ExtError::InvalidDissolutionConfig` (68). |
| `vote_window_seconds` | Duration of the voting window starting from `start_dissolution_vote`. | Must be non-zero, else `ExtError::InvalidDissolutionConfig` (68). |

**Defaults (if never configured):** `dissolution_quorum_bps = 7500` (75%),
`vote_window_seconds = 1_209_600` (14 days).

**Events emitted:** `DissolutionConfigUpdated { dissolution_quorum_bps, vote_window_seconds }`

### 2. Start a vote — `start_dissolution_vote`

```rust
pub fn start_dissolution_vote(env: Env, member: Address)
```

**Authentication:** `member.require_auth()`.

**Behaviour:**
1. Rejects if paused, or if `GroupStatus::Dissolved`
   (`ExtError::GroupAlreadyDissolved`, 62).
2. Rejects if `member` is not in `DataKey::Members` (`Error::NotAMember`, 8).
3. Rejects if `DissolutionDeadline[current_round] > 0` and
   `now < deadline` (`ExtError::DissolutionVoteInProgress`, 63) — prevents
   restarting a live vote. A new vote *can* be opened in a later round once the
   previous window has expired.
4. Sets `DissolutionDeadline[current_round]` = `now + vote_window_seconds`.
5. Resets `DissolutionVoteCount[current_round]` = `0`.

**Events emitted:** `DissolutionVoteStarted { round, deadline }`

### 3. Vote — `vote_dissolve_group`

```rust
pub fn vote_dissolve_group(env: Env, voter: Address, approve: bool)
```

**Authentication:** `voter.require_auth()`.

**Behaviour:**
1. Rejects if paused or dissolved.
2. Rejects non-members (`Error::OnlyMembersAllowed`, 19).
3. Panics with `"No dissolution vote in progress"` if no deadline is set for
   `current_round`.
4. Rejects votes after the deadline (`ExtError::DissolutionVoteExpired`, 65).
5. Rejects a duplicate vote from the same voter (panics
   `"Already voted on dissolution"`).
6. Reads `voter_weight` via `get_member_voting_weight`:
   - `VotingMode::Equal` — weight = 1.
   - `VotingMode::WeightedByContributions` — weight = cumulative contributions.
7. **Only approval votes accumulate.** `votes_for` in
   `DissolutionVoteCount[round]` is incremented by `voter_weight` when
   `approve == true`. Votes against are not tracked separately — the quorum
   check in `execute_dissolution` checks only `votes_for` against the required
   threshold.

**Events emitted:** `DissolutionVoteCast { round, voter, approve, votes_for }`

### 4. Execute — `execute_dissolution`

```rust
pub fn execute_dissolution(env: Env)
```

No `require_auth` — execution is permissionless once the window closes.

**Behaviour:**
1. Rejects if paused or dissolved.
2. Panics `"No dissolution vote in progress"` if no deadline is set.
3. Panics `"Voting period not ended"` if `now <= deadline` — callers must wait
   until the window has fully elapsed.
4. Computes quorum:

```
total_possible_votes = members.len()                    // Equal mode
                     OR Σ MemberContributions[member]   // WeightedByContributions

required_votes = ceil(total_possible_votes × dissolution_quorum_bps / 10_000)
               = (total_possible_votes × dissolution_quorum_bps + 9_999) / 10_000
```

5. If `votes_for < required_votes` → `ExtError::DissolutionQuorumNotMet` (64).
6. On success: emits `DissolutionQuorumReached`, then calls `dissolve_group`
   with the stored admin address and a zeroed reason hash (`[0u8; 32]`).

### 5. Dissolve — `dissolve_group`

```rust
pub fn dissolve_group(env: Env, admin: Address, reason_hash: BytesN<32>)
```

**Authentication:** `admin.require_auth()` + identity check against
`DataKey::Admin`. Callable directly by the admin (bypassing the vote), or
internally by `execute_dissolution`.

**Behaviour:**
1. Rejects if already dissolved (`ExtError::GroupAlreadyDissolved`, 62).
2. Reads the contract's live token balance. If `total_pool <= 0` →
   `ExtError::NoFundsToDistribute` (66).
3. Sets `DataKey2::GroupStatus = GroupStatus::Dissolved`.
4. Calls `internals::distribute_pro_rata` — see below.
5. Emits `GroupDissolved { round, reason_hash, total_pool, member_count }`.

---

### How Remaining Funds Are Distributed on Dissolution

Fund distribution is handled by `internals::distribute_pro_rata`
(`contracts/ahjoor-rosca/src/internals.rs`). It distributes the entire
contract token balance to members in proportion to their **cumulative historical
contributions** (`DataKey::MemberCollected`), not in equal shares.

```
total_contributions = Σ MemberCollected[member] for all members

for each member:
    share = MemberCollected[member] × total_pool / total_contributions
    if share > 0:
        transfer share to member
        emit MemberRefunded { member, amount: share,
                              contribution: MemberCollected[member],
                              total_pool }

rounding_dust = total_pool − Σ shares
if rounding_dust > 0:
    transfer to DataKey2::FeeRecipient  (or members[0] if no fee recipient)
```

Key points:

- A member who contributed more over the group's history receives a larger
  share.
- Integer truncation means the last few stratum of funds (dust) go to the
  configured fee recipient, or to the first member in the roster if none is set.
- **Edge case:** if `total_contributions == 0` (all members joined but nobody
  ever contributed via `contribute()`), the pro-rata loop is skipped entirely
  and no funds are distributed. The `NoFundsToDistribute` guard checks the
  on-chain balance, not `MemberCollected`, so a group with an externally funded
  balance but zero tracked contributions would pass that guard but still
  distribute nothing to members.

---

### Vote Thresholds and Relationship to Per-Proposal Quorum

Dissolution has its **own independent quorum system** — it is not governed by
the per-`ProposalType` quorum configuration described in
[docs/rosca-governance-quorum.md](rosca-governance-quorum.md). The comparison
is:

| Mechanism | Quorum source | Default quorum | Configurable range | Entry point |
|---|---|---|---|---|
| **Dissolution vote** | `DataKey2::DissolutionConfig.dissolution_quorum_bps` | 75% (7 500 bps) | 10%–100% (1 000–10 000 bps) | `set_dissolution_config` |
| **Emergency payout** | `DataKey2::EmergencyPayoutConfig.emergency_quorum_bps` | 66.67% (6 667 bps) | 10%–100% | `set_emergency_payout_config` |
| **Standard governance proposals** | `DataKey2::QuorumConfig[ProposalType]` or `DataKey::QuorumPercentage` | 51% | Per-type override via `set_quorum_per_type` | See [rosca-governance-quorum.md](rosca-governance-quorum.md) |

All three mechanisms use the same ceiling-division formula and respect the same
`VotingMode` (`Equal` or `WeightedByContributions`).

The dissolution quorum intentionally defaults higher (75%) than the standard
governance default (51%) because dissolving the group is an irreversible action.

---

## Part 2: Emergency Payout Voting

### Lifecycle

```text
member calls request_emergency_payout(member, reason_hash)
  ├─ checks: not paused, not dissolved, member present
  ├─ checks: no existing request for (round, member)
  ├─ checks: not already paid in this cycle
  ├─ checks: per-cycle cap not exhausted
  └─ emits EmergencyPayoutRequested

members call vote_emergency_payout(voter, requester, approve) [before deadline]
  ├─ checks: voter is member, not self-vote, no duplicate vote, not expired
  └─ emits EmergencyPayoutVoteCast

anyone calls execute_emergency_payout(requester) [before deadline]
  ├─ checks: quorum met (votes_for + votes_against ≥ required_votes)
  ├─ if votes_for ≤ votes_against:
  │    └─ emits EmergencyPayoutRejected (no transfer, no error)
  └─ if votes_for > votes_against:
       ├─ transfers ContributionAmt to requester
       ├─ marks requester as paid for this round
       └─ emits EmergencyPayoutExecuted
```

### 1. Configure — `set_emergency_payout_config`

```rust
pub fn set_emergency_payout_config(
    env: Env,
    admin: Address,
    emergency_quorum_bps: u32,
    vote_window_seconds: u64,
    max_emergency_per_cycle: u32,
)
```

**Authentication:** `admin.require_auth()` + identity check; panics
`"Only admin can set emergency payout config"` on mismatch.

| Parameter | Meaning | Validation |
|---|---|---|
| `emergency_quorum_bps` | Quorum threshold in basis points. | `1000`–`10000` inclusive, else `ExtError::InvalidEmergencyConfig` (67). Boundary values 1 000 and 10 000 are accepted. |
| `vote_window_seconds` | Voting window length. | Non-zero, else `ExtError::InvalidEmergencyConfig` (67). |
| `max_emergency_per_cycle` | Maximum number of emergency payouts that may execute per cycle. | Non-zero, else `ExtError::InvalidEmergencyConfig` (67). |

**Defaults (if never configured):**

| Field | Default |
|---|---|
| `emergency_quorum_bps` | `6667` (66.67%) |
| `vote_window_seconds` | `604_800` (7 days) |
| `max_emergency_per_cycle` | `1` |

**Events emitted:** `EmergencyPayoutConfigUpdated { emergency_quorum_bps, vote_window_seconds, max_emergency_per_cycle }`

### 2. Request — `request_emergency_payout`

```rust
pub fn request_emergency_payout(env: Env, member: Address, reason_hash: BytesN<32>)
```

**Authentication:** `member.require_auth()`.

**Behaviour:**
1. Rejects if paused, or if `GroupStatus::Dissolved`
   (`ExtError::GroupAlreadyDissolved`, 62).
2. Rejects non-members (`Error::NotAMember`, 8).
3. Rejects a second request for `(current_round, member)`
   (`ExtError::EmergencyPayoutRequested`, 57).
4. Rejects if the member already received an emergency payout in the current
   *cycle* (`ExtError::EmergencyPayoutAlreadyExecuted`, 60).
   Cycle index = `current_round / payout_order.len()`.
5. Rejects if the cycle has already reached `max_emergency_per_cycle`
   (`ExtError::EmergencyPayoutLimitReached`, 61).
6. Stores `EmergencyPayoutRequest { requester, reason_hash, created_at,
   deadline: now + vote_window_seconds, votes_for: 0, votes_against: 0,
   executed: false }` in `DataKey2::EmergencyPayoutRequests[(round, member)]`.

The `reason_hash` is an opaque `BytesN<32>` (e.g. the hash of an off-chain
document); the contract stores it but does not interpret it.

**Events emitted:** `EmergencyPayoutRequested { requester, round, reason_hash, deadline }`

### 3. Vote — `vote_emergency_payout`

```rust
pub fn vote_emergency_payout(env: Env, voter: Address, requester: Address, approve: bool)
```

**Authentication:** `voter.require_auth()`.

**Behaviour:**
1. Rejects if paused or dissolved.
2. Rejects non-members (`Error::OnlyMembersAllowed`, 19).
3. Rejects self-voting — the requester cannot vote on their own request
   (panics `"Cannot vote on your own emergency payout request"`).
4. Rejects if no request exists for `(current_round, requester)`.
5. Rejects if `request.executed == true`.
6. Rejects if `now > request.deadline`
   (`ExtError::EmergencyPayoutVoteExpired`, 59).
7. Rejects a duplicate vote from the same voter.
8. Adds `get_member_voting_weight(voter)` to `votes_for` or `votes_against`.
   Vote weight follows `VotingMode` (1 per member in `Equal`; cumulative
   contributions in `WeightedByContributions`).

Unlike the dissolution vote, **both** `votes_for` and `votes_against` are
accumulated — the final outcome requires both a quorum check and a majority
check.

**Events emitted:** `EmergencyPayoutVoteCast { requester, round, voter, approve, votes_for, votes_against }`

### 4. Execute — `execute_emergency_payout`

```rust
pub fn execute_emergency_payout(env: Env, requester: Address)
```

No `require_auth` — permissionless execution; the outcome is determined solely
by the recorded votes.

**Behaviour:**
1. Rejects if paused or dissolved.
2. Rejects if no request exists, if already executed
   (`ExtError::EmergencyPayoutAlreadyExecuted`, 60), or if `now > deadline`
   (`ExtError::EmergencyPayoutVoteExpired`, 59).
3. Computes quorum:

```
total_possible_votes = members.len()                    // Equal mode
                     OR Σ MemberContributions[member]   // WeightedByContributions

required_votes = ceil(total_possible_votes × emergency_quorum_bps / 10_000)

total_votes = votes_for + votes_against
```

   If `total_votes < required_votes` →
   `ExtError::EmergencyPayoutQuorumNotMet` (58).

4. If `votes_for <= votes_against` (majority did not approve): emits
   `EmergencyPayoutRejected { requester, round, reason: "votes_failed" }` and
   returns. No funds move, no error is raised.

5. On success (quorum met **and** majority approved):
   - Marks `request.executed = true`.
   - Records `EmergencyPayoutApproved[(cycle_index, requester)] = true`.
   - Increments `EmergencyPayoutCount[cycle_index]`.
   - Transfers `DataKey::ContributionAmt` (the group's single-round
     contribution amount) from the contract to `requester`.
   - Adds `requester` to `DataKey::PaidMembers` — this replaces the member's
     normal round payout rather than stacking on top of it.

**Events emitted:** `EmergencyPayoutExecuted { requester, round, payout_amount }`

---

## Configuration Reference

### Dissolution

| Field | Storage key | Default | Range |
|---|---|---|---|
| `dissolution_quorum_bps` | `DataKey2::DissolutionConfig` | 7 500 (75%) | 1 000–10 000 bps |
| `vote_window_seconds` | `DataKey2::DissolutionConfig` | 1 209 600 (14 days) | > 0 |

### Emergency Payout

| Field | Storage key | Default | Range |
|---|---|---|---|
| `emergency_quorum_bps` | `DataKey2::EmergencyPayoutConfig` | 6 667 (66.67%) | 1 000–10 000 bps |
| `vote_window_seconds` | `DataKey2::EmergencyPayoutConfig` | 604 800 (7 days) | > 0 |
| `max_emergency_per_cycle` | `DataKey2::EmergencyPayoutConfig` | 1 | ≥ 1 |

---

## Events

| Event | Emitted by | Fields |
|---|---|---|
| `DissolutionConfigUpdated` | `set_dissolution_config` | `dissolution_quorum_bps`, `vote_window_seconds` |
| `DissolutionVoteStarted` | `start_dissolution_vote` | `round`, `deadline` |
| `DissolutionVoteCast` | `vote_dissolve_group` | `round`, `voter`, `approve`, `votes_for` |
| `DissolutionQuorumReached` | `execute_dissolution` | `round`, `votes_for` |
| `GroupDissolved` | `dissolve_group` | `round`, `reason_hash`, `total_pool`, `member_count` |
| `MemberRefunded` | `internals::distribute_pro_rata` | `member`, `amount`, `contribution`, `total_pool` |
| `EmergencyPayoutConfigUpdated` | `set_emergency_payout_config` | `emergency_quorum_bps`, `vote_window_seconds`, `max_emergency_per_cycle` |
| `EmergencyPayoutRequested` | `request_emergency_payout` | `requester`, `round`, `reason_hash`, `deadline` |
| `EmergencyPayoutVoteCast` | `vote_emergency_payout` | `requester`, `round`, `voter`, `approve`, `votes_for`, `votes_against` |
| `EmergencyPayoutRejected` | `execute_emergency_payout` | `requester`, `round`, `reason` |
| `EmergencyPayoutExecuted` | `execute_emergency_payout` | `requester`, `round`, `payout_amount` |

> [!NOTE]
> The `EmergencyPayoutApproved` event struct is defined in `events.rs` but no
> current entry point emits it; a successful payout emits
> `EmergencyPayoutExecuted` directly.

---

## Error Cases

| Code | Name | Raised by | Description |
|---|---|---|---|
| 8 | `NotAMember` | `request_emergency_payout`, `start_dissolution_vote` | Caller is not in the member list. |
| 19 | `OnlyMembersAllowed` | `vote_emergency_payout`, `vote_dissolve_group` | Voter is not a member. |
| 57 | `EmergencyPayoutRequested` | `request_emergency_payout` | A request already exists for this member in this round. |
| 58 | `EmergencyPayoutQuorumNotMet` | `execute_emergency_payout` | Total votes cast are below the required quorum. |
| 59 | `EmergencyPayoutVoteExpired` | `vote_emergency_payout`, `execute_emergency_payout` | The vote window has closed. |
| 60 | `EmergencyPayoutAlreadyExecuted` | `request_emergency_payout`, `execute_emergency_payout` | The member already received an emergency payout this cycle or the payout was already executed. |
| 61 | `EmergencyPayoutLimitReached` | `request_emergency_payout` | `max_emergency_per_cycle` has been exhausted for this cycle. |
| 62 | `GroupAlreadyDissolved` | All dissolution and emergency payout entry points | The group has already been dissolved. |
| 63 | `DissolutionVoteInProgress` | `start_dissolution_vote` | A live dissolution vote already exists for this round. |
| 64 | `DissolutionQuorumNotMet` | `execute_dissolution` | `votes_for` is below the required quorum after the window closed. |
| 65 | `DissolutionVoteExpired` | `vote_dissolve_group` | Vote attempted after the dissolution deadline. |
| 66 | `NoFundsToDistribute` | `dissolve_group` | The contract's token balance is zero or negative. |
| 67 | `InvalidEmergencyConfig` | `set_emergency_payout_config` | Quorum, window, or per-cycle cap is out of range. |
| 68 | `InvalidDissolutionConfig` | `set_dissolution_config` | Quorum or window is out of range. |

---

## Function Reference

| Function | Caller | Purpose |
|---|---|---|
| `set_dissolution_config(admin, quorum_bps, vote_window_seconds)` | Admin | Configures dissolution quorum and voting window. |
| `start_dissolution_vote(member)` | Member | Opens a time-bounded dissolution vote for the current round. |
| `vote_dissolve_group(voter, approve)` | Member | Casts a weighted vote on the open dissolution proposal. |
| `execute_dissolution()` | Anyone (after deadline) | Tallies votes; if quorum met, triggers dissolution. |
| `dissolve_group(admin, reason_hash)` | Admin (or `execute_dissolution`) | Marks the group dissolved and distributes the pool pro-rata. |
| `set_emergency_payout_config(admin, quorum_bps, vote_window_seconds, max_per_cycle)` | Admin | Configures quorum, voting window, and per-cycle cap for emergency payouts. |
| `request_emergency_payout(member, reason_hash)` | Member | Opens a voted emergency payout request for the current round. |
| `vote_emergency_payout(voter, requester, approve)` | Member | Casts a weighted vote on a pending emergency payout request. |
| `execute_emergency_payout(requester)` | Anyone (before deadline) | Tallies votes and, on approval, transfers `ContributionAmt` to the requester. |

---

## Storage Reference

| Key | Storage | Data |
|---|---|---|
| `DataKey2::DissolutionConfig` | Instance | `DissolutionConfig { dissolution_quorum_bps, vote_window_seconds }` |
| `DataKey2::DissolutionVotes` | Instance | `Map<(round, voter), bool>` — per-member vote record |
| `DataKey2::DissolutionVoteCount` | Instance | `Map<round, i128>` — cumulative `votes_for` per round |
| `DataKey2::DissolutionDeadline` | Instance | `Map<round, u64>` — vote window deadline per round |
| `DataKey2::GroupStatus` | Instance | `GroupStatus` enum (`Active`, `Dissolved`, `Merged`, `Split`, `Completed`) |
| `DataKey2::EmergencyPayoutConfig` | Instance | `EmergencyPayoutConfig { emergency_quorum_bps, vote_window_seconds, max_emergency_per_cycle }` |
| `DataKey2::EmergencyPayoutRequests` | Instance | `Map<(round, requester), EmergencyPayoutRequest>` |
| `DataKey2::EmergencyPayoutVotes` | Instance | `Map<(round, requester, voter), bool>` — who has voted |
| `DataKey2::EmergencyPayoutApproved` | Instance | `Map<(cycle_index, requester), bool>` — executed payouts per cycle |
| `DataKey2::EmergencyPayoutCount` | Instance | `Map<cycle_index, u32>` — count of executed payouts per cycle |
