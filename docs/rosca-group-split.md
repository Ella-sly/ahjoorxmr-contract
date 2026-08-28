# ROSCA Group Split Flow

A group split divides the membership of an active ROSCA group into two sub-groups. The split is an administrator-proposed, member-confirmed workflow with a bounded confirmation window. After execution, the source group is marked `Split` and cannot be split again.

## Actors and responsibilities

- **Group administrator** — configures the confirmation window, creates a split proposal, and executes it.
- **Group members** — confirm that they will participate in the proposed sub-group. Each member can confirm only once.
- **Any caller** — may expire a proposal after its confirmation window has closed.

Administrator mutations and member confirmations require authorization from the address supplied to the contract call. Proposal expiry is permissionless, and proposal lookup is read-only.

## Lifecycle

```text
Active group
    │
    ├─ administrator proposes member assignment
    │
    ├─ members confirm during the confirmation window
    │       ├─ administrator executes → source becomes Split
    │       └─ window closes → anyone expires the proposal
    │
    └─ Expired proposal (cannot be executed)
```

A split proposal has one of three statuses:

| Status | Meaning |
| --- | --- |
| `Pending` | The proposal may receive confirmations and may be executed before expiry. |
| `Executed` | The proposal was executed and the source group was marked `Split`. |
| `Expired` | The confirmation window closed before the proposal was executed. |

## 1. Configure the confirmation window (optional)

The administrator can set the window used by future proposals with:

```text
set_split_confirmation_window(admin, window_ledgers)
```

`window_ledgers` is measured in ledger sequences. If it is not configured, the default is **200 ledgers**. The value is stored on the group contract and applies when a new proposal is created; changing it does not change an existing proposal's expiry ledger.

## 2. Create a split proposal

The administrator creates a proposal with:

```text
propose_group_split(
    admin,
    group_id,
    group_a_members,
    group_b_members,
    split_reason_hash,
) -> proposal_id
```

### Member assignment rules

The two member lists must form a complete partition of the group's current membership:

1. Every current member must occur in **exactly one** list.
2. No address outside the current membership may occur in either list.
3. A member appearing in both lists, or in neither list, is rejected with `SplitMembersInvalid`.

`split_reason_hash` is a 32-byte hash supplied by the administrator for an off-chain explanation or supporting record. The contract stores the hash, not the explanation itself.

The newly created `SplitProposal` stores:

- the proposal ID;
- both member assignments;
- the reason hash;
- an initially empty confirmation list;
- the creation ledger;
- the expiry ledger; and
- status `Pending`.

The proposal ID is incremented by the contract's split-proposal counter. A `GroupSplitProposed` event is emitted with the source `group_id` and `proposal_id`.

## 3. Member confirmation window

Each assigned member confirms with:

```text
confirm_split_participation(member, group_id, proposal_id)
```

The member must be a current group member and must belong to one of the proposal's two assignments. A successful call adds the member to `confirmations`. Calling it twice for the same member returns `SplitAlreadyConfirmed`.

A confirmation is accepted while:

```text
current_ledger <= expiry_ledger
```

After the expiry ledger, confirmation returns `SplitConfirmationWindowClosed`. The member's confirmation is not automatically recorded by a read-only lookup; clients should query the proposal with:

```text
get_split_proposal(proposal_id)
```

### Confirmation and execution behavior

The contract permits the administrator to execute a still-pending proposal before its expiry, including when some members have not confirmed. Integrations should therefore show which members are confirmed and should not present execution as proof that every member accepted the assignment.

## 4. Execute the split

The administrator executes a pending proposal with:

```text
execute_group_split(admin, group_id, proposal_id)
```

Execution is rejected when:

- the proposal does not exist;
- the proposal is already executed or expired; or
- the confirmation window has closed.

On successful execution:

1. Members who confirmed are assigned to group A or group B according to the proposal.
2. Members who did not confirm are treated as unconfirmed and receive an equal per-member share of the source contract's current token balance, when that share is positive.
3. The source group's status is set to `Split`, preventing another split proposal for that source group.
4. The proposal status becomes `Executed`.
5. A `GroupSplitExecuted` event is emitted with the source group ID and the two resulting group IDs.

The reserve/refund calculation is:

```text
per_member_share = current_token_balance / total_source_members
```

It uses integer division. Any remainder stays in the contract. If the balance is zero, or the calculated share is zero, no refund transfer is made.

### Resulting group IDs

The execution event contains deterministic identifiers for the two resulting groups:

```text
group_a_id = group_id * 1000 + proposal_id * 2 - 1
group_b_id = group_id * 1000 + proposal_id * 2
```

These identifiers are included in the `GroupSplitExecuted` event for downstream group tracking. The source contract instance records the source status and proposal result; consumers should use the event and their group-management workflow to associate the two sub-groups with their confirmed memberships.

## 5. Expire an abandoned proposal

Anyone may expire a pending proposal after its expiry ledger:

```text
expire_split_proposal(proposal_id)
```

The call succeeds only when:

```text
current_ledger > expiry_ledger
```

It changes the proposal status to `Expired`. An expired proposal cannot receive confirmations or be executed. Expiry is not automatic, so an account or service must submit this transaction if it wants the stored status to change promptly.

## Errors and client handling

| Error | Recommended handling |
| --- | --- |
| `OnlyAdminAllowed` | Check that the caller is the configured group administrator. |
| `NotAMember` | The confirming address is not a current group member. |
| `SplitMembersInvalid` | Rebuild the two assignments so every current member appears exactly once and no extra address is included. |
| `SplitProposalNotFound` | Refresh the proposal ID or handle an already expired/executed proposal. |
| `SplitAlreadyConfirmed` | Treat the member as already confirmed; do not submit the confirmation again. |
| `SplitConfirmationWindowClosed` | Stop confirmations/execution and expire the proposal if appropriate. |
| `SourceGroupAlreadySplit` | The source group has already completed a split. |

## Events

Clients can monitor these events:

- `GroupSplitProposed(source_group_id, proposal_id)` — a new split proposal was created.
- `GroupSplitExecuted(source_group_id, group_a_id, group_b_id)` — the source group was split and the resulting identifiers were published.

For auditability, retain the proposal returned by `get_split_proposal` together with both events and the ledger at which each transaction was submitted.
