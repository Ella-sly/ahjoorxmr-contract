# View Functions Implementation Summary

This document summarizes the three new view functions that have been implemented across the contracts.

## 1. `get_accrued_fees` - Escrow Contract

**Location**: `/contracts/ahjoor-escrow/src/lib.rs`

**Function Signature**:

```rust
pub fn get_accrued_fees(env: Env, token: Address) -> i128
```

**Description**: Returns the accumulated protocol fees for a given token that are awaiting withdrawal.

**Implementation Details**:

- Reads from `DataKey2::AccruedFees(token)` in instance storage
- Returns `0` if no fees have been accrued for the token
- Placed near `get_protocol_fee` function for logical grouping

**Usage Example**:

```rust
let usdc_token = Address::from(...);
let accrued = contract.get_accrued_fees(&env, usdc_token);
```

---

## 2. `get_payment_refunded_amount` - Payments Contract

**Location**: `/contracts/ahjoor-payments/src/lib.rs`

**Function Signature**:

```rust
pub fn get_payment_refunded_amount(env: Env, payment_id: u32) -> i128
```

**Description**: Returns the cumulative amount that has been refunded for a given payment.

**Implementation Details**:

- Fetches the `Payment` struct from persistent storage using `payment_id`
- Returns the `refunded_amount` field from the payment record
- Panics with "Payment not found" if the payment_id doesn't exist
- Placed near `get_max_batch_size` function for logical grouping

**Usage Example**:

```rust
let payment_id = 123;
let refunded = contract.get_payment_refunded_amount(&env, payment_id);
```

---

## 3. `get_max_batch_size` - Refund Contract

**Location**: `/contracts/ahjoor-refund/src/lib.rs`

**Function Signature**:

```rust
pub fn get_max_batch_size(env: Env) -> u32
```

**Description**: Returns the maximum batch size for batch refund operations.

**Implementation Details**:

- Reads from `DataKey::MaxBatchSize` in instance storage
- Returns `DEFAULT_MAX_BATCH_SIZE` (20) if not configured
- Placed near `get_payment_contract` function for logical grouping

**Usage Example**:

```rust
let max_batch = contract.get_max_batch_size(&env);
```

---

## Testing Recommendations

To verify these implementations:

1. **Escrow Contract - `get_accrued_fees`**:
    - Test with a token that has accrued fees
    - Test with a token that has no accrued fees (should return 0)
    - Verify the value matches what's stored after protocol fee collection

2. **Payments Contract - `get_payment_refunded_amount`**:
    - Test with a payment that has been partially refunded
    - Test with a payment that has not been refunded (should return 0)
    - Test with an invalid payment_id (should panic)

3. **Refund Contract - `get_max_batch_size`**:
    - Test when admin has set a custom max batch size
    - Test when no custom size is set (should return default of 20)

---

## Storage Keys Used

- **Escrow**: `DataKey2::AccruedFees(Address)` - Instance storage
- **Payments**: `DataKey::Payment(u32)` - Persistent storage (reads `refunded_amount` field)
- **Refund**: `DataKey::MaxBatchSize` - Instance storage

---

## Related Functions

### Escrow Contract

- `withdraw_fees()` - Uses the same storage key to withdraw accrued fees
- `get_protocol_fee()` - Returns the fee configuration

### Payments Contract

- `partial_refund()` - Updates the `refunded_amount` field
- `get_max_batch_size()` - Already existed in payments contract

### Refund Contract

- `set_max_batch_size()` - Admin function to configure the max batch size
- Uses the same constant `DEFAULT_MAX_BATCH_SIZE` for consistency

---

## Implementation Notes

All three functions follow the existing patterns in their respective contracts:

1. **Consistent naming**: All use `get_` prefix for view functions
2. **Documentation**: Each includes a doc comment explaining its purpose
3. **Error handling**: Appropriate use of `unwrap_or()` for defaults or `expect()` for required values
4. **Storage access**: Correctly uses instance/persistent storage as appropriate
5. **No state changes**: All are pure read operations (view functions)

The implementations are minimal, efficient, and follow Soroban best practices for view functions.
