# ROSCA Reinvestment Flow (`ahjoor-rosca`)

This document describes the **reinvestment** feature in the `ahjoor-rosca` contract:
how a member can elect to roll their round payout forward into the next round
instead of withdrawing it, and the constraints that govern that behaviour.

> Implementation references: `contracts/ahjoor-rosca/src/lib.rs`
> (`set_reinvest_preference`, `get_reinvest_preference`), `contracts/ahjoor-rosca/src/internals.rs`
> (payout/reinvest application), and the test suite `contracts/ahjoor-rosca/src/test_reinvest.rs`.

## Overview

In a ROSCA, each round has one payout recipient who receives the pooled contributions
(the "pot"). By default that payout is transferred to the recipient. With **reinvestment**,
the recipient opts to leave their payout inside the contract so it counts as their
contribution for the *next* round — effectively compounding their stake and skipping a
manual deposit.

A reinvestment preference is recorded per member in the `ReinvestPreference` map
(instance storage). It is a boolean flag: `true` = reinvest future payouts, `false` = receive
payouts normally (the default for any member not present in the map).

## Setting the preference

```rust
client.set_reinvest_preference(&member, &true);
let on = client.get_reinvest_preference(&member); // -> true
```

`set_reinvest_preference(member, reinvest)` enforces the following conditions:

1. **Contract is not paused.** Enforced via `check_not_paused`.
2. **Caller is a member.** `member` must be in the group's member list, otherwise it
   fails with `NotAMember`.
3. **Caller authorizes.** `member.require_auth()` is required.
4. **Before the contribution deadline.** The preference may only be toggled while the
   current round's contribution window is still open. The deadline is taken from
   `RoundDeadlineTimestamp` when `UseTimestampSchedule` is enabled, otherwise from
   `RoundDeadline` (ledger-based). If `env.ledger().timestamp() > deadline`, the call
   fails with `ContributionWindowClosed` (error `#33`).

Because of rule 4, a member must decide to reinvest **before** the round they expect to
be paid closes. The preference persists across rounds until toggled again (and can be
turned off before a later round's deadline).

`get_reinvest_preference(member)` returns the stored flag, or `false` if the member has
never set one.

## What happens at payout time

When a round is finalized and a payout recipient is selected, the contract checks the
recipient's reinvest preference. If `true`, the payout is **not** transferred out. Instead:

- The full `payout_amount` (pot balance minus any protocol fee) is treated as reinvested.
- A `PayoutReinvested` event is emitted carrying `member`, `round`, and `amount`.
- This only applies to the **base token** (`token_addr == base_token`). Non-base approved
  tokens are still paid out normally even if the member has reinvest enabled.

After the round state is reset, the reinvested amount is applied to the next round:

- It is recorded as the member's contribution for the next round
  (`next_contributions.set(recipient, reinvested_amount)`).
- If `reinvested_amount >= member_required` (the member's tier-based required
  contribution), the member is added to the next round's paid members, so they are
  considered to have already fulfilled that round's obligation.
- If `reinvested_amount < member_required`, the member still owes the difference and can
  top it up via a normal contribution. The `get_member_contribution_status` helper exposes
  the `paid` amount and a (possibly negative) `remaining` top-up figure.

No protocol fee is charged on the reinvested portion differently from a normal payout;
fees are calculated on the pot balance the same way, and only the net
(`balance - fee`) is reinvested.

## Worked example

Setup: 2 members, base contribution `100` each, `RoundRobin` payout. Both contribute `100`
in round 0, so the pot is `200`.

```
1. member1 calls set_reinvest_preference(member1, true)  // before deadline
2. Round 0 finalizes; member1 is the payout recipient.
   - member1 does NOT receive 200.
   - PayoutReinvested(member1, round=0, amount=200) emitted.
   - member1 balance stays 900 (1000 - own 100 contribution).
3. Round state resets to round 1.
   - member1 is recorded as a paid member for round 1.
   - member1's round-1 contribution status: paid = 200, remaining = -100
     (reinvested amount exceeds the 100 required, i.e. overpaid by 100).
```

The member has therefore compounded their position: the payout they would have withdrawn
is now working for them in the next round.

## Constraints & edge cases

| Situation | Behaviour |
| --- | --- |
| Non-member calls `set_reinvest_preference` | Fails with `NotAMember`. |
| Called after the round's contribution deadline | Fails with `ContributionWindowClosed` (`#33`). |
| Contract paused | `check_not_paused` rejects the call. |
| Reinvest on, but payout is in a non-base token | Payout is transferred normally; reinvestment only applies to the base token. |
| Reinvested amount ≥ required contribution | Member auto-fulfills next round (added to paid members). |
| Reinvested amount < required contribution | Member still owes the remainder and may top up. |
| Preference never set | Treated as `false`; member receives payouts normally. |

## Events

| Event | Emitted when | Fields |
| --- | --- | --- |
| `PayoutReinvested` | A recipient's payout is rolled into the next round | `member`, `round`, `amount` |

## Error codes

| Code | Name | Raised when |
| --- | --- | --- |
| 33 | `ContributionWindowClosed` | `set_reinvest_preference` is called after the round's contribution deadline. |
| — | `NotAMember` | `set_reinvest_preference` is called by a non-member. |

## Function reference

| Function | Caller | Purpose |
| --- | --- | --- |
| `set_reinvest_preference(member, reinvest)` | Member (self) | Enable/disable auto-reinvest of this member's payouts. Must be before the round deadline. |
| `get_reinvest_preference(member)` | Anyone | Read the member's current reinvest flag (`false` if unset). |
