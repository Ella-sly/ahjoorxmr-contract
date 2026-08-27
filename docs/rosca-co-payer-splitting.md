# Co-Payer Contribution Splitting in ROSCA

This document explains the co-payer contribution splitting flow in the `ahjoor-rosca` contract. The feature lets a single ROSCA member register one or more co-payers who collectively cover that member's contribution obligation for a round.

## What This Feature Is

Co-payer splitting applies to one member's slot only:

- The member keeps their place in the ROSCA group.
- The group's membership does not change.
- The registered co-payers fund that member's required contribution amount in exact portions.

This is distinct from the separate group-split feature documented in `docs/rosca-group-split.md` and exercised in `test_group_split.rs`.

- Co-payer splitting covers one member's payment obligation.
- Group split divides an entire ROSCA group into two sub-groups with a proposal and confirmation flow.

If you are looking for member reassignment, confirmation windows, or creation of two resulting groups, use the group-split documentation instead. This page covers only contribution sharing for a single member's slot.

## Actors and Responsibilities

- **Member**: Registers and revokes the co-payer configuration for their own slot.
- **Co-payers**: Provide the token amounts assigned to them in the registered split.
- **Contract**: Stores the split, enforces the exact total required for that member, and records the member as paid only after the whole split contribution succeeds.

Only the member can register or revoke splits for their own slot.

## Data Model

Each co-payer entry is stored as:

```rust
pub struct CoPayerSplit {
    pub co_payer: Address,
    pub amount: i128,
}
```

The contract stores exact token amounts, not percentage values. Any "split ratio" is therefore implicit:

```text
co_payer_ratio = split.amount / member_required_contribution
```

For example, if a member owes `100` tokens and registers splits of `60` and `40`, the effective ratio is `60% / 40%`.

## 1. Register Co-Payers for a Member Slot

The member registers the split list with:

```text
register_co_payer_splits(member, splits)
```

### Registration rules

The call succeeds only when all of the following are true:

- `member` authorizes the transaction.
- `member` is a current ROSCA member.
- `member` has not exited the group.
- No co-payer split is already stored for that member.
- Every split amount is positive.
- The sum of all split amounts exactly equals the member's required contribution.

If a split is already registered, the contract rejects the call with `CopayerSplitsAlreadySet`. The member must first remove the existing configuration with:

```text
revoke_co_payer_splits(member)
```

### How the required amount is determined

The contract calculates the member's required contribution from the configured base contribution and the member's tier basis points:

```text
required = base_amount * tier_bps / 10_000
```

- If the member has no tier override, `tier_bps` defaults to `10_000`, meaning 100% of the base amount.
- Because the contract compares the registered total against this computed `required` amount, the split is enforced at registration time.

### Registration event

On success, the contract emits:

```text
CoPayerSplitRegistered(member, co_payer_count, total_split_amount)
```

Clients can also query the current configuration with:

```text
get_co_payer_splits(member)
```

This returns the stored list or an empty list when none is registered.

## 2. How the Split Ratio Is Enforced

The split is enforced in two layers:

### Layer 1: Registration-time total enforcement

At registration, the contract adds every `split.amount` and requires:

```text
sum(split.amounts) == member_required_contribution
```

If the totals do not match, the call fails with `CopayerAmountsMismatch`.

This means the contract will not store underfunded or overfunded configurations for the member's slot.

### Layer 2: Contribution-time exact transfer amounts

When the member later uses:

```text
contribute_split(member, token)
```

the contract loads the stored split list and iterates over every co-payer entry. For each entry, it attempts to transfer exactly the stored `amount` for that co-payer.

The contract does not recalculate or rebalance the split during contribution. Whatever amounts were registered are the amounts attempted during execution.

## 3. Contribution Execution Flow

`contribute_split(member, token)` is the function that turns the registered split into an actual round contribution.

Before any transfers are attempted, the contract verifies:

- the ROSCA is not paused or frozen;
- the group is active;
- `member` authorizes the call;
- `member` is still in the group and has not exited;
- `member` has not already contributed for the current round; and
- `token` is approved for the ROSCA.

If no split is registered for the member, the call fails with `NoCopayersRegistered`.

If the checks pass, the contract:

1. Loads the stored co-payer list.
2. Iterates over each registered co-payer.
3. Attempts the exact transfer amount for that co-payer.
4. Emits `CoPayerContributed(member, co_payer, amount, round)` for each successful per-co-payer transfer.
5. Sums the transferred amounts.
6. Marks the member as paid for the current round.
7. Records the member's total contribution amount.
8. Emits the standard contribution event for the member.

The member is treated as the contributor of record for the round even though the tokens came from the registered co-payers.

## 4. What Happens If a Co-Payer Fails To Contribute

The contract requires the full registered split to succeed in one `contribute_split` transaction.

If any co-payer cannot satisfy their part of the split, the transaction reverts and the contribution is not partially accepted. In practical terms:

- the member is not marked as paid for the round;
- the round contribution is not recorded for that member; and
- remaining co-payers are not treated as having completed the obligation on their own.

Typical failure causes include:

- no split is registered for the member (`NoCopayersRegistered`);
- the member already contributed this round (`AlreadyContributed`);
- the token is not approved (`TokenNotApproved`);
- a co-payer transfer fails because the token transfer conditions are not satisfied; or
- the member or group is no longer eligible to use the feature, such as `NotAMember` or `MemberHasExited`.

The contract does not implement a fallback that automatically reassigns an unpaid co-payer's share to another co-payer. It also does not partially mark the member as paid based on whichever co-payers succeeded before the failure condition was hit. The expected recovery path is to fix the underlying issue and retry, or revoke and register a new split arrangement.

## 5. Revocation and Reconfiguration

A member can remove an existing split arrangement with:

```text
revoke_co_payer_splits(member)
```

This call:

- requires member authorization;
- requires that the caller is still a ROSCA member; and
- fails with `NoCopayersRegistered` if nothing is stored.

On success, the contract deletes the stored co-payer configuration and emits:

```text
CoPayerSplitRevoked(member)
```

After revocation, the member can register a different co-payer layout for future split contributions.

## Errors and Client Handling

| Error | Meaning | Recommended handling |
| --- | --- | --- |
| `CopayerSplitsAlreadySet` | The member already has a split configuration. | Query the current split, then revoke before registering a replacement. |
| `CopayerAmountsMismatch` | The split total does not exactly equal the member's required contribution. | Recompute the split amounts so they sum exactly to the required amount. |
| `NoCopayersRegistered` | No split exists for the member. | Register a split first, or fall back to the normal contribution path. |
| `AmountMustBePositive` | One or more split amounts are zero or negative. | Submit only positive amounts. |
| `AlreadyContributed` | The member has already been recorded as paid for the current round. | Treat the split contribution as no longer available for that round. |
| `NotAMember` | The specified member is not an active group member. | Refresh membership state before calling. |
| `MemberHasExited` | The member has exited the ROSCA. | Do not allow new split registration or split contribution for that slot. |
| `TokenNotApproved` | The provided token is not allowed for the ROSCA. | Retry with an approved token. |

## Events

Clients can monitor these events to track the lifecycle:

- `CoPayerSplitRegistered(member, co_payer_count, total_split_amount)` for new registrations.
- `CoPayerContributed(member, co_payer, amount, round)` for each successful co-payer transfer.
- `CoPayerSplitRevoked(member)` when a member removes the split configuration.

For auditability, index these events together with `get_co_payer_splits(member)` and the standard contribution records for the affected round.
