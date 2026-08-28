# Weighted Voting in Ahjoor-ROSCA Governance

This document describes the weighted voting mechanism in the `ahjoor-rosca` contract. It explains how member vote weight is derived, how votes are cast and aggregated (including delegation), how proposals are tallied against dynamic quorum thresholds, and how weighted voting compares to the standard one-member-one-vote model.

---

## 1. Overview & Motivation

Governance in `ahjoor-rosca` allows group members to create, vote on, and execute proposals that govern circle operations and membership rules. Supported proposal types include:

- `PenaltyAppeal`: Appeal against penalty fees assessed due to missed contributions.
- `RuleChange`: Modifications to contract parameters and group operational rules.
- `MemberRemoval`: Expelling a defaulting or bad-faith participant from the group.
- `MaxMembersUpdate`: Expanding or contracting the maximum membership limit.
- `Reinstatement`: Restoring a suspended member to active status after penalty clearance.
- `MemberFreeze`: Emergency member-initiated pause on group operations.

Ahjoor supports two distinct voting modes configurable at group creation via `VotingMode`:

1. **`VotingMode::Equal` (One-Member-One-Vote):** Every registered member has an equal voting power of `1`, regardless of financial contribution.
2. **`VotingMode::WeightedByContributions` (Weighted Voting):** A member's voting power is proportional to their cumulative token contributions in the active round. This aligns decision-making power with financial stake and commitment within the savings circle.

---

## 2. Configuration & Storage

Voting mode is defined as an enum in `types.rs`:

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[contracttype]
pub enum VotingMode {
    Equal = 0,
    WeightedByContributions = 1,
}
```

### Initializing Voting Mode

When initializing a ROSCA group via `init(...)`, the desired mode is specified within `RoscaConfig`:

```rust
client.init(
    &admin,
    &members,
    &contribution_amount,
    &token_address,
    &round_duration,
    &RoscaConfig {
        voting_mode: VotingMode::WeightedByContributions,
        // ...other config options
    },
    &None,
);
```

The selected mode is persisted in contract instance storage under `DataKey2::VotingMode`.

---

## 3. Derivation of Member Vote Weight

Member voting weight is queried on-chain via `get_member_voting_weight`:

```rust
pub fn get_member_voting_weight(env: Env, member: Address) -> i128
```

### Computation Rules

- **`VotingMode::Equal`:**
  Always returns `1i128` for any active member.

- **`VotingMode::WeightedByContributions`:**
  Returns the member's cumulative contribution amount recorded for the current active round (`DataKey::MemberContributions`):
  
  $$\text{VotingWeight}(m) = \text{MemberContributions}[m]$$

### Key Weight Characteristics

1. **Current Round Contributions Only:** Vote weight is derived from contributions made in the *current active round*. It is tracked as members deposit tokens via `contribute(...)`.
2. **Zero-Contribution Weight:** If a member has not contributed any tokens to the active round, their voting weight is `0`.
3. **Impact of Member Tiers:** For groups with tiered contributions (`MemberTiers`), members in higher tiers (e.g., `20_000` bps = 2x base contribution) can contribute more tokens to the round, thereby gaining higher voting weight relative to standard-tier members.
4. **Round Reset & Auto-Reinvestment:** When a round closes and advances to the next round (`reset_round_state`), `MemberContributions` is reset to empty. However, if a payout recipient has enabled automatic reinvestment (`set_reinvest_preference`), the reinvested payout is immediately credited to their `MemberContributions` in the new round, granting them immediate voting weight.

---

## 4. Casting Votes & Weight Application

Members cast votes by calling:

```rust
pub fn vote_on_proposal(env: Env, voter: Address, proposal_id: u32, vote_for: bool)
```

### Validation & Zero-Weight Protection

1. **Membership & Authentication:** `voter` must be a registered member of the group and must authenticate the transaction (`voter.require_auth()`).
2. **Proposal State:** The proposal must be in `ProposalStatus::Pending` and the current timestamp must be $\le$ `proposal.deadline`.
3. **Double-Voting Prevention:** Each member can only vote once per proposal.
4. **Insufficient Weight Guard:** In `VotingMode::WeightedByContributions`, if `get_member_voting_weight(voter) == 0`, the transaction fails immediately with:
   ```
   ExtError::InsufficientWeight (Error #56)
   ```
   Members with zero contributions in the active round cannot cast a vote.

### Vote Accumulation

When a valid vote is cast:
- If `vote_for == true`: `proposal.votes_for += voter_weight`
- If `vote_for == false`: `proposal.votes_against += voter_weight`

### Voting Delegation Integration

The contract supports voting delegation through general delegation (`vote_delegations`) or contribution-weight delegation (`DataKey3::ContribDelegations` via `delegate_contribution_vote`):

- **Direct Vote Blocking:** A delegator with an active delegation cannot call `vote_on_proposal` directly (reverts with `CannotVoteWithActiveDelegation`).
- **Proxy Weight Aggregation:** When the designated delegate votes, the delegator's voting weight is dynamically computed (`get_member_voting_weight`) and automatically bundled into the delegate's vote direction (`votes_for` or `votes_against`).

### Emitted Events

- In `VotingMode::WeightedByContributions`:
  ```rust
  WeightedVoteCast {
      member: Address,
      proposal_id: u32,
      weight: i128,
  }
  ```
- In `VotingMode::Equal`:
  ```rust
  VoteCast {
      proposal_id: u32,
      voter: Address,
      vote_for: bool,
  }
  ```

---

## 5. Proposal Tallying & Outcome Determination

Once the voting deadline passes (`ledger.timestamp > proposal.deadline`), any member or admin can trigger final tallying and execution by calling:

```rust
pub fn execute_proposal(env: Env, proposal_id: u32)
```

### Step 1: Quorum Calculation

Quorum represents the minimum participation required for a vote to be legally binding. In `ahjoor-rosca`, quorum is evaluated against the **total possible voting weight**:

- **In `VotingMode::Equal`:**
  $$\text{total\_possible\_votes} = \text{members.len()}$$
- **In `VotingMode::WeightedByContributions`:**
  $$\text{total\_possible\_votes} = \sum_{m \in \text{members}} \text{contributions}(m)$$

The required votes threshold is computed based on `proposal.required_quorum` (in basis points, default `5100` = 51%, or custom per-type quorum set via `set_quorum_per_type`):

$$\text{required\_votes} = \left\lfloor \frac{(\text{total\_possible\_votes} \times \text{required\_quorum}) + 9999}{10000} \right\rfloor$$

$$\text{total\_votes} = \text{proposal.votes\_for} + \text{proposal.votes\_against}$$

If $\text{total\_votes} < \text{required\_votes}$, the proposal is rejected:
- Status transitions to `ProposalStatus::Rejected`.
- Emits `ProposalRejected` with reason `"insufficient_quorum"`.

### Step 2: Majority Rule Determination

If quorum is met, the outcome is determined by simple majority:

- **Rejected (`votes_for <= votes_against`):**
  - Status becomes `ProposalStatus::Rejected`.
  - Emits `ProposalRejected` with reason `"votes_failed"`.
- **Approved (`votes_for > votes_against`):**
  - Status transitions to `ProposalStatus::Approved` then `ProposalStatus::Executed` upon completing proposal side effects (e.g. updating parameters, removing member, or executing freeze).
  - *Note:* `ProposalType::Reinstatement` remains `Approved` until the suspended member executes `reinstate_member`.

---

## 6. Comparison: Weighted Voting vs. Equal Voting

| Feature / Dimension | `VotingMode::Equal` (One-Member-One-Vote) | `VotingMode::WeightedByContributions` (Weighted Voting) |
| :--- | :--- | :--- |
| **Weight per Member** | Constant `1` for each member | Dynamic `i128` equal to member's round contribution |
| **Zero-Contribution Member** | Can vote with weight `1` | Cannot vote (panics with `ExtError::InsufficientWeight`) |
| **Tier / Stake Scaling** | No scaling; all tiers have equal voice | Scales with member contribution tier and deposit amount |
| **Total Possible Votes** | Total count of registered members | Sum of all contributions deposited in active round |
| **Quorum Denominator** | Member count (e.g. 51% of 10 members = 6 votes) | Total deposited token volume (e.g. 51% of 3,000 tokens = 1,530 weight) |
| **Vote Event Emitted** | `VoteCast { proposal_id, voter, vote_for }` | `WeightedVoteCast { member, proposal_id, weight }` |
| **Governance Dynamic** | Pure democratic consensus | Stakeholder-aligned capital governance |

---

## 7. Other Voting Flows Utilizing Member Weight

The `get_member_voting_weight` function is also applied across other on-chain ROSCA governance mechanisms:

1. **Emergency Payout Votes (`vote_emergency_payout`):**
   When a member requests emergency payout relief, other members vote to approve or deny. Votes cast are weighted by `get_member_voting_weight(voter)`.
2. **Group Dissolution Votes (`vote_dissolution`):**
   When group dissolution is initiated, members vote to approve dissolution. The vote accumulation `votes_for` applies each voter's `get_member_voting_weight(voter)`.

---

## 8. Practical Example Walkthrough

Consider a 3-member ROSCA group initialized with `VotingMode::WeightedByContributions` and a base contribution of `1,000` tokens:

1. **Setup & Contributions:**
   - **Member 1** is assigned Tier `20_000` bps (2x tier) and contributes `200` tokens.
   - **Member 2** is on standard Tier (`10_000` bps) and contributes `100` tokens.
   - **Member 3** has contributed `0` tokens so far this round.

2. **Derived Voting Weights:**
   - $\text{Weight}(\text{Member 1}) = 200$
   - $\text{Weight}(\text{Member 2}) = 100$
   - $\text{Weight}(\text{Member 3}) = 0$
   - $\text{Total Possible Votes} = 200 + 100 + 0 = 300$

3. **Proposal & Voting:**
   - A proposal is created with default 51% quorum (`required_quorum = 5100`).
   - `Member 3` attempts to vote $\rightarrow$ Reverts with `InsufficientWeight` (Error #56).
   - `Member 2` votes AGAINST $\rightarrow$ `votes_against = 100`.
   - `Member 1` votes FOR $\rightarrow$ `votes_for = 200`.

4. **Tallying on Execution:**
   - $\text{Total Votes Cast} = 200 + 100 = 300$.
   - $\text{Required Quorum} = \lfloor (300 \times 5100 + 9999) / 10000 \rfloor = 153$.
   - Quorum Check: $300 \ge 153$ (Quorum **passed**).
   - Majority Check: $\text{votes\_for } (200) > \text{votes\_against } (100)$ (Outcome **passed**).
   - The proposal is marked `ProposalStatus::Executed`.
