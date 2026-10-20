# Add escrow-backed payment option to payments contract

## Summary
Extend the `ahjoor-payments` contract to support escrow-backed payments, enabling funds to be held in the escrow contract before invoice settlement.

## Motivation
- Bridge the gap between direct payments and full escrow workflows
- Provide additional payment security without the complexity of full escrow agreements
- Enable hybrid payment models where initial funds are escrowed before invoice approval
- Support use cases requiring proof of funds before service delivery
- Leverage existing escrow infrastructure for payment guarantees

## Problem Statement
Currently, the payments contract operates independently from the escrow system. Users who want payment security must choose between:
1. Direct invoice payments (fast but no guarantee)
2. Full escrow agreements (secure but heavyweight)

There's no middle ground for scenarios where:
- Buyers want to demonstrate fund availability
- Sellers want payment assurance before delivery
- Parties need simpler security than full escrow contracts provide

## Proposed Solution

### High-Level Design
Add an optional escrow-backed payment mode where:
1. Invoice is created with an `escrow_id` reference
2. Funds are verified to exist in the referenced escrow contract
3. Payment settlement pulls funds from escrow instead of direct transfer
4. Escrow state is updated to reflect the payment withdrawal

### Implementation Scope

#### 1. Data Model Changes
```rust
pub struct Invoice {
    // ... existing fields
    pub escrow_backed: bool,
    pub escrow_contract_id: Option<Address>,
    pub escrow_id: Option<u64>,
}
```

#### 2. New Functions
- `create_escrow_backed_invoice()` - Create invoice linked to escrow
- `verify_escrow_funds()` - Internal: Check escrow has sufficient funds
- `settle_from_escrow()` - Process payment from escrow contract

#### 3. Modified Functions
- Update `pay_invoice()` to handle escrow-backed payments
- Extend validation logic to verify escrow state

#### 4. Integration Points
- Cross-contract calls to `ahjoor-escrow` contract
- Verify escrow status and available balance
- Coordinate state updates between contracts

### Security Considerations
- [ ] Verify caller authorization for escrow operations
- [ ] Prevent double-spend (escrow + direct payment)
- [ ] Handle escrow state transitions correctly
- [ ] Validate escrow contract address authenticity
- [ ] Implement proper reentrancy guards

## Acceptance Criteria
- [ ] Invoice can be created with escrow backing
- [ ] System verifies sufficient funds in escrow before approval
- [ ] Payment correctly withdraws from escrow contract
- [ ] Escrow state updates reflect payment withdrawal
- [ ] Proper error handling for insufficient escrow funds
- [ ] Proper error handling for invalid escrow references
- [ ] Events emitted for escrow-backed payment lifecycle
- [ ] Comprehensive test coverage including:
  - Happy path escrow-backed payment
  - Insufficient escrow funds scenario
  - Invalid escrow reference handling
  - State consistency between contracts
  - Authorization edge cases

## Open Questions
1. Should escrow-backed invoices support partial payments from escrow?
2. What happens if escrow is disputed after invoice creation?
3. Should there be a timeout for settling escrow-backed invoices?
4. Do we need escrow administrator approval for invoice payments?
5. Should multi-token invoices support escrow backing?

## Technical Considerations
- Cross-contract call patterns and gas implications
- State synchronization between payments and escrow contracts
- Handling escrow contract upgrades or migrations
- Version compatibility between contract versions

## Alternative Approaches Considered
1. **Separate escrow-invoice contract**: More isolation but duplicate logic
2. **Escrow-only payments**: Forces all payments through escrow (too restrictive)
3. **Off-chain coordination**: Less secure, requires trusted third party

## Dependencies
- Requires stable `ahjoor-escrow` contract interface
- May need escrow contract modifications for payment withdrawal support
- Client SDKs will need updates to support new payment flow

## Priority
Medium - Expands payment flexibility and security options

## Estimated Effort
Medium (2-3 weeks)
- Contract modifications: 5 days
- Testing and security review: 5 days
- Integration and documentation: 3 days

## Labels
- enhancement
- payments
- escrow
- cross-contract
- security

## Related Issues
- Related to escrow contract architecture
- May impact invoice dispute resolution flows
- Consider alignment with multi-token invoice features
