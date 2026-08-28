# ROSCA Governance Quorum Requirements

This document provides a comprehensive specification of governance quorum rules, per-`ProposalType` quorum overrides, resolution logic, and administrative controls in the `ahjoor-rosca` smart contract.

---

## 1. Overview & Quorum Concept

In Ahjoor ROSCA governance, **quorum** represents the minimum participation threshold (expressed in basis points, where $100 \text{ bps} = 1\%$) that must be reached by combined member votes (`votes_for + votes_against`) before a proposal can be evaluated for approval or rejection.

### How Quorum Affects Proposals
When a proposal voting window expires and `execute_proposal` is invoked:
1. **Quorum Verification:** The contract calculates the total votes cast (`total_votes = votes_for + votes_against`) and compares it against the required threshold (`required_votes`).
2. **Rejection for Insufficient Quorum:** If `total_votes < required_votes`, the proposal is immediately marked as `ProposalStatus::Rejected` with the failure reason `Symbol("insufficient_quorum")`, regardless of whether the majority voted in favor.
3. **Majority Decision:** Only if `total_votes >= required_votes` does the contract evaluate vote totals. The proposal passes and is marked `ProposalStatus::Approved` (or `Executed`) if `votes_for > votes_against`.

### Global Default vs. Per-Type Overrides
- **Global Default (`QuorumPercentage`):** Stored in instance storage under `DataKey::QuorumPercentage`. Initialized to `51` (51%).
- **Per-Type Override (`QuorumConfig`):** Stored in instance storage under `DataKey2::QuorumConfig` as a map (`Map<ProposalType, u32>`). Allows administrators to set custom quorum requirements for specific proposal types (e.g., emergency freezes, rule changes, or member removals).

---

## 2. ProposalType Reference

The `ProposalType` enum in `types.rs` defines all supported governance proposal categories.

### Complete `ProposalType` Quorum Table

| ProposalType | Discriminant | Purpose & Description | Default Quorum (BPS) | Default Quorum (%) | Per-Type Override Available | Primary Creation API |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| `PenaltyAppeal` | `0` | Appeal default penalties assessed against a member. Executing resets default count and removes member from suspension. | `5,100` | 51% | Yes | `create_proposal` |
| `RuleChange` | `1` | Propose updating ROSCA operational parameters (such as group quorum percentage). | `5,100` | 51% | Yes | `create_proposal` / `propose_treasury_round` |
| `MemberRemoval` | `2` | Remove an inactive or non-compliant member from both group membership and payout order. | `5,100` | 51% | Yes | `create_proposal` |
| `MaxMembersUpdate` | `3` | Update the group's maximum allowable member cap (`MaxMembers`). | `5,100` | 51% | Yes | `create_proposal` |
| `Reinstatement` | `4` | Request reinstatement for a suspended member after resolving default obligations. | `5,100` | 51% | Yes | `request_reinstatement` / `create_proposal` |
| `MemberFreeze` | `5` | Member-initiated emergency freeze of ROSCA operations. Pre-seeded in `init` with a higher safety threshold. | `6,700` | 67% | Yes | `propose_member_freeze` / `create_proposal` |

> [!NOTE]
> All standard proposal types default to `5,100` bps (51%) derived from the global `QuorumPercentage`, except `MemberFreeze`, which is pre-configured during contract initialization (`init`) to require `6,700` bps (67%) for enhanced emergency protection.

---

## 3. Special Governance Mechanisms

Beyond the standard `ProposalType` proposals, the ROSCA contract implements two specialized governance workflows with dedicated quorum parameters:

| Workflow | Configuration Struct | Default Quorum | Configuration Function | Trigger Function |
| :--- | :--- | :--- | :--- | :--- |
| **Emergency Payout** | `EmergencyPayoutConfig` | `6,667` bps (66.67%) | `set_emergency_payout_config` | `propose_emergency_payout` |
| **Group Dissolution** | `DissolutionConfig` | `7,500` bps (75.00%) | `set_dissolution_config` | `propose_group_dissolution` |

---

## 4. Per-Type Override Mechanism

Administrators can customize the required quorum for any individual `ProposalType` without altering the global default quorum percentage.

### Admin Configuration API

```rust
pub fn set_quorum_per_type(
    env: Env,
    admin: Address,
    proposal_type: ProposalType,
    quorum_bps: u32,
)
```

#### Authorization & Security Guards
- **Authentication:** Requires `admin.require_auth()`.
- **Identity Verification:** The `admin` parameter must match the stored contract administrator (`DataKey::Admin`). If not, the transaction panics with `"Only admin can set quorum per type"`.
- **Range Validation:** The `quorum_bps` parameter must satisfy $100 \le \text{quorum\_bps} \le 10000$ (1% to 100%). Panics with `"Quorum must be between 1% and 100%"` if outside this range.

#### Storage & Event Emission
- Overrides are saved in instance storage under `DataKey2::QuorumConfig` (`Map<ProposalType, u32>`).
- Emits the `emit_quorum_config_updated(&env, proposal_type, quorum_bps)` event.

#### Proposal Snapshot Isolation
When a proposal is created (via `create_proposal`, `propose_member_freeze`, or `request_reinstatement`), the effective quorum is resolved and stored permanently on the `Proposal` struct in the `required_quorum` field.

> [!IMPORTANT]
> Updating a quorum requirement via `set_quorum_per_type` applies exclusively to **future proposals** created after the update. Existing active proposals retain their `required_quorum` snapshot taken at creation time.

---

## 5. Resolution & Effective Quorum Calculation

### Quorum Resolution Order
When a proposal is initialized, the contract resolves `required_quorum` using the following precedence:

$$\text{effective\_quorum\_bps} = \begin{cases} \text{QuorumConfig}[proposal\_type] & \text{if per-type override exists in } DataKey2::QuorumConfig \\ \text{DataKey}::QuorumPercentage \times 100 & \text{otherwise (default: } 5100 \text{ bps)} \end{cases}$$

### Required Votes Formula
During proposal execution (`execute_proposal`), the required votes threshold is computed based on the active `VotingMode`:

1. **Equal Voting Mode (`VotingMode::Equal`):**
   $$\text{total\_possible\_votes} = |\text{Members}|$$

2. **Weighted Voting Mode (`VotingMode::WeightedByContributions`):**
   $$\text{total\_possible\_votes} = \sum_{m \in \text{Members}} \text{MemberContributions}[m]$$

3. **Required Vote Threshold Calculation (Ceiling Division):**
   $$\text{required\_votes} = \left\lceil \frac{\text{total\_possible\_votes} \times \text{required\_quorum\_bps}}{10000} \right\rceil$$

   *In Rust source code:* `((total_possible_votes * proposal.required_quorum as i128) + 9999) / 10000`

---

## 6. Governance Quorum Examples

The following examples demonstrate effective quorum resolution for a ROSCA group with **10 active members** operating under `VotingMode::Equal` ($\text{total\_possible\_votes} = 10$).

### Example 1: Standard Proposal with Default Quorum
- **ProposalType:** `RuleChange`
- **Override Status:** Unset
- **Global Quorum:** 51% (`QuorumPercentage = 51`)
- **Effective Quorum:** $51 \times 100 = 5,100 \text{ bps}$
- **Required Votes:** $\lceil (10 \times 5100) / 10000 \rceil = \lceil 5.1 \rceil = 6 \text{ votes}$

### Example 2: Emergency Freeze Proposal (Pre-Seeded Default)
- **ProposalType:** `MemberFreeze`
- **Override Status:** Pre-seeded in `init` to 67% (`6_700` bps)
- **Effective Quorum:** $6,700 \text{ bps}$ (67%)
- **Required Votes:** $\lceil (10 \times 6700) / 10000 \rceil = \lceil 6.7 \rceil = 7 \text{ votes}$

### Example 3: Lowered Quorum for Penalty Appeals
- **Action:** Administrator calls `set_quorum_per_type(admin, ProposalType::PenaltyAppeal, 1000)` (10%).
- **ProposalType:** `PenaltyAppeal`
- **Effective Quorum:** $1,000 \text{ bps}$ (10%)
- **Required Votes:** $\lceil (10 \times 1000) / 10000 \rceil = \lceil 1.0 \rceil = 1 \text{ vote}$

### Example 4: Supermajority Requirement for Member Removal
- **Action:** Administrator calls `set_quorum_per_type(admin, ProposalType::MemberRemoval, 7500)` (75%).
- **ProposalType:** `MemberRemoval`
- **Effective Quorum:** $7,500 \text{ bps}$ (75%)
- **Required Votes:** $\lceil (10 \times 7500) / 10000 \rceil = \lceil 7.5 \rceil = 8 \text{ votes}$

---

## 7. Administrative Guidance

### Inspecting Current Quorum Configurations

Administrators and group members can query quorum parameters using on-chain view functions:

#### 1. Query Effective Quorum for a Specific ProposalType
```rust
pub fn get_quorum_for_type(env: Env, proposal_type: ProposalType) -> u32
```
- Returns the active quorum requirement for `proposal_type` in basis points (e.g., `5100` for 51%, `6700` for 67%).
- Resolves per-type overrides first, falling back to `QuorumPercentage * 100` if no override exists.

#### 2. Query Global Default Quorum Percentage
```rust
pub fn get_quorum_percentage(env: Env) -> u32
```
- Returns the global default quorum percentage (e.g., `51` for 51%).

### Best Practices for Governance Administration
1. **Calibrate Quorum to Group Size:** In large ROSCA groups, setting excessively high quorum requirements (e.g., >80%) may result in proposal deadlocks due to inactive voters.
2. **Protect Critical Actions:** Use `set_quorum_per_type` to require supermajority thresholds (e.g., 67%–75%) for disruptive actions like `MemberRemoval` or `MemberFreeze`.
3. **Lower Thresholds for Time-Sensitive Appeals:** Lower quorum thresholds (e.g., 10%–25%) for `PenaltyAppeal` to allow defaulted members prompt resolution without requiring full group turnout.

---

## 8. Source Code & Test Reference

- **Types & Enums:** `contracts/ahjoor-rosca/src/types.rs` (`ProposalType`, `Proposal`)
- **Initialization & Defaults:** `contracts/ahjoor-rosca/src/lib.rs` (`init`)
- **Override Function:** `contracts/ahjoor-rosca/src/lib.rs` (`set_quorum_per_type`)
- **Query Function:** `contracts/ahjoor-rosca/src/lib.rs` (`get_quorum_for_type`)
- **Proposal Creation & Quorum Snapshot:** `contracts/ahjoor-rosca/src/lib.rs` (`create_proposal`, `propose_member_freeze`, `request_reinstatement`)
- **Execution & Vote Tallying:** `contracts/ahjoor-rosca/src/lib.rs` (`execute_proposal`)
- **Unit & Integration Tests:** `contracts/ahjoor-rosca/src/test_quorum.rs`, `contracts/ahjoor-rosca/src/test_view_functions.rs`
