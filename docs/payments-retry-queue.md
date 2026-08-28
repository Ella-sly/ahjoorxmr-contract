# Payments Failed-Debit Retry Queue

The `ahjoor-payments` contract implements a failed-debit retry queue designed to gracefully handle scenarios where a payment attempt fails due to insufficient customer funds. Instead of permanently failing the payment immediately, the contract records the failure and schedules it for future retry attempts using an exponential backoff mechanism.

## Purpose

The retry queue ensures that subscription payments or pre-approved debits do not immediately cancel if a customer's wallet is temporarily empty. It provides a structured window for the customer to top up their balance and for the contract to automatically re-attempt the capture.

## Creation of a Failed-Debit Record

When a debit is initiated (e.g., via `initiate_allowed_payment`), the contract attempts to transfer tokens from the customer. If the customer has insufficient balance, rather than reverting the transaction and discarding the payment, the contract creates a `FailedDebitRecord` and places it in the retry queue with a status of `Pending`.

## Failed Debit Lifecycle

A failed debit transitions through the following statuses (defined by `FailedDebitStatus`):

- **`Pending`**: The debit has failed but is scheduled for retry. It remains in this state as long as it has not exceeded the maximum allowed retry attempts and has not yet succeeded.
- **`Succeeded`**: The debit was successfully captured during a retry attempt.
- **`Abandoned`**: The debit failed consecutively until it exhausted the maximum number of retry attempts. It will no longer be retried.

### Record Tracking

A `FailedDebitRecord` tracks crucial context about the failed payment:
- `id`: The unique identifier of the record.
- `plan_id`: The associated payment plan context.
- `invoice_id`: The associated recurring invoice (if applicable).
- `merchant`: The merchant receiving the payment.
- `customer`: The customer being debited.
- `token`: The token asset used for payment.
- `amount`: The debit amount.
- `attempt_number`: The current retry attempt count.
- `next_retry_ledger`: The ledger sequence number at or after which the next retry is permitted.

## Retry Mechanism and Backoff Scheduling

### `retry_failed_debit`

Anyone can call `retry_failed_debit(record_id)` to re-attempt a pending failed debit.
- **Backoff Enforcement**: The contract checks if the current ledger sequence is greater than or equal to the `next_retry_ledger`. If a retry is attempted before this window has elapsed, the transaction will panic (e.g., with `RetryNotDue`).
- **Success Path**: If the customer has sufficient balance, the transfer succeeds. The record's status is updated to `Succeeded`, and its `attempt_number` reflects the final successful attempt.
- **Failure Path**: If the transfer fails again due to insufficient funds, the `attempt_number` is incremented.

### Exponential Backoff

The retry schedule uses an exponential backoff strategy:
- The delay doubles after each failed attempt.
- The next retry ledger is calculated based on this increasing delay.
- This prevents the network from being spammed with continuous failing retry transactions.

### Exhaustion of Attempts

If a retry fails and the new `attempt_number` exceeds the configured maximum retry attempts (`max_retry_attempts`), the record's status is updated to `Abandoned`. No further retries are permitted.

### Early Retry: `trigger_early_retry`

The `trigger_early_retry(customer, record_id)` function provides a mechanism for a customer to manually trigger a retry *before* the scheduled `next_retry_ledger`. Unlike `retry_failed_debit`, this function bypasses the backoff check. It is typically invoked by a frontend immediately after a customer tops up their balance, ensuring the payment clears instantly without waiting for the scheduled backoff delay.

## Interaction with Recurring Invoices

Failed debits are deeply integrated with the recurring invoice system:
- When an invoice cycle is triggered and the customer lacks funds, a `FailedDebitRecord` is created linking to the `invoice_id`.
- If the debit is later successfully captured (becomes `Succeeded`), the contract automatically advances the recurring invoice's cycle counter (`cycles_triggered`), and updates the `next_due_ledger` and `next_due_at` timestamps.
- If the debit becomes `Abandoned`, the invoice cycle is not advanced, requiring manual intervention or cancellation of the subscription.

## Configuration

An admin configures the retry mechanism globally using `set_retry_config(admin, base_retry_interval, max_retry_interval, max_retry_attempts)`.

- **`base_retry_interval`**: The initial backoff delay (in ledgers) before the first retry.
- **`max_retry_interval`**: The absolute maximum backoff delay permitted, capping the exponential growth.
- **`max_retry_attempts`**: The maximum number of retry attempts before a pending record is marked as `Abandoned`.

## Edge Cases and Integration Notes

Developers integrating with the ahjoor-payments contract should be aware of the following:

- **Customer Top-ups**: The standard recovery path relies on the customer depositing sufficient funds into their wallet. Once topped up, either a scheduled `retry_failed_debit` or a manual `trigger_early_retry` will successfully capture the payment.
- **Status Checks**: Integrations should periodically check `FailedDebitRecord` statuses. A payment should not be considered "failed permanently" until its status is `Abandoned`.
- **Early Retry Requirement**: `trigger_early_retry` requires authentication/authorization from the `customer` to prevent arbitrary third parties from bypassing the backoff mechanism and spamming attempts.
- **Not Due Rejection**: Automated keepers executing `retry_failed_debit` must ensure they respect `next_retry_ledger`, otherwise the transaction will waste gas and fail.
