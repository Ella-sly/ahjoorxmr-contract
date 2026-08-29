# ROSCA Savings Milestone Rewards

Savings milestone rewards give a member token incentives as their personal
savings goal advances. A goal owner defines percentage milestones and a reward
rate for each milestone. When a recorded contribution crosses a configured
threshold, the contract can transfer a reward to the goal owner from the
contract reward pool.

Savings contributions themselves are tracking records: calling
`contribute_to_savings_goal` does not transfer the contributed amount from the
member. Only a successfully funded milestone reward moves tokens.

For the full savings-goal lifecycle, see [ROSCA Savings Goals](rosca-savings-goals.md).

## Milestone Thresholds

The goal owner adds milestones with:

```text
add_savings_goal_milestones(goal_id, milestones)
```

The fields that control an on-chain token reward are:

| Field | Purpose |
|---|---|
| `milestone_id` | Identifies the milestone and its position in the claimed-reward bitmask. |
| `percentage` | Completion threshold, from 1 through 100. |
| `amount` | Required positive milestone metadata; reward triggering uses `percentage`, not this value. |
| `reward_bps` | Reward rate in basis points of the contribution that crosses the threshold. A value of `0` disables the token reward. |

`name`, `description`, `reward_type`, `reward_value`, and
`celebration_event` are descriptive fields and do not alter the token payout
calculation.

For each contribution, the contract calculates integer completion percentages:

```text
percentage_before = floor((current_amount - contribution_amount) * 100 / target_amount)
percentage_now    = floor(current_amount * 100 / target_amount)
```

A milestone is crossed when:

```text
percentage_before < milestone.percentage
and percentage_now >= milestone.percentage
```

Consequently, a single contribution can cross several milestones. Each newly
crossed milestone with a nonzero `reward_bps` is evaluated during that same
call. Percentage calculations use integer division, so fractional percentages
are rounded down.

## Reward Formula and Examples

Each crossed milestone is calculated independently:

```text
reward_amount = floor(contribution_amount * reward_bps / 10,000)
```

The calculation uses the contribution that crosses the milestone, not the
goal's cumulative balance or `Milestone.reward_value`.

For example, a goal has a target of 1,000 tokens and a 25% milestone configured
with `reward_bps = 1,000` (10%). A contribution of 250 crosses the threshold:

```text
reward_amount = 250 * 1,000 / 10,000 = 25 tokens
```

If one contribution of 600 crosses both 25% and 50% milestones, and each has
`reward_bps = 500` (5%), each milestone pays 30 tokens. The total pool debit is
60 tokens.

Integer division rounds down. If the calculated value is zero, no token is
transferred, but the milestone is still recorded as claimed.

## Funding and Automatic Distribution

The contract admin makes tokens available with:

```text
fund_savings_reward_pool(admin, amount)
```

The admin must authenticate, must match the admin stored at initialization,
and must provide a positive amount. The call transfers the configured ROSCA
token from the admin to the contract and increases `DataKey::RewardPool`.
`get_savings_reward_pool()` returns this stored pool balance.

There is no separate user claim transaction. Distribution happens inside
`contribute_to_savings_goal`:

1. The contributing address authenticates and submits a positive contribution.
2. The contract validates the goal state, expiry, and target cap, then records
   the new cumulative amount.
3. The contract checks every reward-enabled milestone for a threshold crossing.
4. For each newly crossed milestone, it calculates the reward from that
   contribution.
5. If the pool covers the full reward, the contract transfers the configured
   ROSCA token directly to the goal owner, decreases the pool, and emits
   `MilestoneReached(group_id, member, milestone_pct, reward_amount)`.
6. The contract records the milestone as claimed whether the transfer succeeds
   or is skipped for a zero reward or insufficient pool.

The pool does not make partial payments. If its balance is below the calculated
reward, that reward is skipped, the contribution remains successful, and the
milestone cannot be retried later after the pool is replenished.

## Security, Limits, and Guards

- **Admin-controlled funding:** only the authenticated contract admin can fund
  the pool. Funding transfers existing tokens into the contract; this feature
  does not mint tokens.
- **Owner-controlled milestones:** adding milestones requires authentication by
  the goal owner. Milestone percentages must be from 1 to 100 and `amount` must
  be positive.
- **One evaluation per milestone bit:** claimed status is stored in
  `DataKey3::SavingsMilestonesClaimed(goal_id, goal_owner)` as a `u64` bitmask.
  Once its bit is set, later contributions cannot pay that bit again.
- **Bitmask limit:** the bit position is `milestone_id % 64`. IDs separated by
  a multiple of 64 share a bit, so deployments should use unique milestone IDs
  in the range 0–63 for each goal.
- **No reward-rate cap:** the current milestone validation does not cap
  `reward_bps`. Administrators and clients should treat 10,000 as 100% and
  reject unintended rates above that value before submission.
- **Pool cap:** a reward transfers only when the stored pool balance is at least
  the full calculated amount. Insufficient funds do not revert the contribution
  or create a later claim.
- **Contribution guards:** contribution amounts must be positive, must not push
  the goal above its target, and are rejected for completed, abandoned, failed,
  or expired goals.
- **Transfer behavior:** the contract has no manual milestone-claim entry point
  and no explicit reentrancy-lock field. Reward distribution occurs within the
  authenticated Soroban contract invocation and its token transfer is part of
  the same atomic transaction.

Claim status can be inspected with:

```text
get_savings_milestones_claimed(goal_id, member) -> u64
```

Bit `N` being set means the bit assigned to milestone ID `N` has already been
consumed. This record may indicate a successful payout, a zero-value calculated
reward, or a reward skipped because the pool was too small.
