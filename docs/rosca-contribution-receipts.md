# Contribution Receipts in ROSCA

This document provides a comprehensive specification of the NFT-style contribution receipt issuance, storage, and retrieval mechanism in the `ahjoor-rosca` smart contract.

---

## 1. Overview & Receipt Format

When a ROSCA round completes and is finalized, the contract automatically mints on-chain **NFT-style contribution receipts** for every member who successfully contributed to that round. Each receipt acts as a tamper-proof cryptographic record of participation, enabling members to verify historical contributions on-chain or off-chain.

### Receipt Data Structure

The receipt format is defined by the `ContributionReceipt` struct in `types.rs`:

```rust
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContributionReceipt {
    pub receipt_id: u32,
    pub member: Address,
    pub round: u32,
    pub amount_contributed: i128,
    pub token: Address,
    pub minted_at: u64,
    pub receipt_hash: BytesN<32>,
}
```

### Field Definitions

| Field | Type | Description |
| :--- | :--- | :--- |
| `receipt_id` | `u32` | A unique, auto-incrementing sequential identifier assigned to each minted receipt (starting at `0`). |
| `member` | `Address` | The Stellar account address of the contributing ROSCA member. |
| `round` | `u32` | The ROSCA round sequence number for which the contribution was recorded. |
| `amount_contributed` | `i128` | The total token amount contributed by the member during the round. |
| `token` | `Address` | The address of the token (Stellar Asset Contract) used for the contribution. |
| `minted_at` | `u64` | The Unix timestamp (in seconds) recorded when the round was finalized and the receipt was minted. |
| `receipt_hash` | `BytesN<32>` | A deterministic 256-bit SHA-256 cryptographic hash generated from the receipt parameters to prove authenticity. |

---

## 2. Issuance Mechanism & Execution Flow

Contribution receipts are **issued automatically** by the contract during round finalization. Members do not need to call a separate minting function.

### Issuing Function

Receipts are minted inside the `finalize_round` (and `finalize_round_chunk`) execution path:

```rust
pub fn finalize_round(env: Env)
```

> [!NOTE]
> If `close_round` is executed to advance round state, the contract enforces `RoundPendingFinalization` if rounds accumulate without calling `finalize_round`. This guarantees that payout finalization, reward distribution, and receipt issuance occur for every completed round.

### Step-by-Step Minting Process

1. **Member Iteration:** When `finalize_round` runs, the contract retrieves the list of members who paid their contribution for the current round (`paid_members`).
2. **Counter Lookup:** The contract fetches the global sequential counter from storage discriminant `DataKey3::ContributionReceiptCounter` (defaults to `0`).
3. **Deterministic Hash Generation:** For each paid member, a 32-byte SHA-256 preimage is constructed by concatenating:
   - Big-endian bytes of the receipt counter (`counter.to_be_bytes()`)
   - Big-endian bytes of the current round number (`current_round.to_be_bytes()`)
   - XDR-encoded byte array of the member's address (`member.to_xdr(&env)`)

   The preimage is hashed using Soroban cryptographic host functions (`env.crypto().sha256(&preimage)`).
4. **Receipt Instantiation & Persistent Storage:**
   - The `ContributionReceipt` struct is created with the current ledger timestamp (`env.ledger().timestamp()`).
   - The receipt record is written to persistent storage under `DataKey3::ContributionReceipt(receipt_id)` with extended TTL (`PERSISTENT_LIFETIME_THRESHOLD`, `PERSISTENT_BUMP_AMOUNT`).
5. **Member Indexing:**
   - The `receipt_id` is appended to the member's list of receipt IDs stored under `DataKey3::MemberReceiptIds(member)`.
   - The member's receipt index TTL is extended in persistent storage.
6. **Event Emission:**
   - The contract emits the `ContributionReceiptMinted` event for off-chain indexing.
7. **Counter Increment:**
   - The global `ContributionReceiptCounter` is incremented by 1 and updated in instance storage.

### Emitted Event Specification

```rust
#[contractevent]
#[derive(Clone, Debug)]
pub struct ContributionReceiptMinted {
    pub receipt_id: u32,
    pub member: Address,
    pub round: u32,
    pub amount_contributed: i128,
    pub receipt_hash: BytesN<32>,
}
```

---

## 3. Retrieval & Verification

Members and external applications can retrieve and verify receipts using read-only view functions.

### Contract Query Functions

#### 1. Retrieve a Single Receipt by ID
```rust
pub fn get_contribution_receipt(env: Env, receipt_id: u32) -> ContributionReceipt
```
- Returns the complete `ContributionReceipt` struct for the given `receipt_id`.
- **Error:** If `receipt_id` does not exist or has not been minted, the contract panics with `ExtError2::ReceiptNotFound` (Error Code `116`).

#### 2. Retrieve All Receipt IDs for a Member
```rust
pub fn get_member_receipt_ids(env: Env, member: Address) -> Vec<u32>
```
- Returns a list (`Vec<u32>`) of all receipt IDs issued to `member` across all ROSCA rounds.
- Returns an empty list if the member has no recorded contribution receipts.

#### 3. Query Total Receipt Counter
```rust
pub fn get_contribution_receipt_count(env: Env) -> u32
```
- Returns the total number of contribution receipts minted across all members and rounds.

---

## 4. Off-Chain Cryptographic Verification

To verify that a contribution receipt is authentic and un-tampered:

1. Query the receipt using `get_contribution_receipt(receipt_id)`.
2. Reconstruct the byte preimage off-chain:
   - `preimage = counter_bytes_u32_be + round_bytes_u32_be + member_address_xdr`
3. Compute the SHA-256 hash of the reconstructed preimage.
4. Compare the calculated hash with `receipt.receipt_hash`. A match confirms the receipt was generated directly by contract state during `finalize_round`.

---

## 5. Storage Keys & Error Codes Reference

### Storage Keys (`DataKey3`)

| Storage Key | Storage Type | Description |
| :--- | :--- | :--- |
| `DataKey3::ContributionReceiptCounter` | Instance | Global counter for total receipts minted (`u32`). |
| `DataKey3::ContributionReceipt(u32)` | Persistent | Maps receipt ID to `ContributionReceipt` struct. |
| `DataKey3::MemberReceiptIds(Address)` | Persistent | Maps member address to a vector of receipt IDs (`Vec<u32>`). |

### Related Error Code

| Error Name | Error Enum | Numeric Code | Description |
| :--- | :--- | :--- | :--- |
| `ReceiptNotFound` | `ExtError2` | `116` | Returned when `get_contribution_receipt` is called with an invalid or non-existent `receipt_id`. |
