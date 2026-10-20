# Add get_version view function to token-whitelist contract

## Summary
Add a `get_version()` view function to the `ahjoo-token-whitelist` contract to expose the contract version for client applications and integrations.

## Motivation
- Enable clients to programmatically verify the deployed contract version
- Support version-dependent feature detection in frontend applications
- Improve contract introspection and debugging capabilities
- Align with versioning best practices across the ahjoorxmr contract suite

## Proposed Implementation
Add a public view function to `contracts/ahjoo-token-whitelist/src/lib.rs`:

```rust
#[contractimpl]
impl TokenWhitelist {
    /// Returns the contract version
    pub fn get_version(env: Env) -> String {
        String::from_str(&env, env!("CARGO_PKG_VERSION"))
    }
}
```

## Acceptance Criteria
- [ ] `get_version()` function added to TokenWhitelist contract
- [ ] Function returns version string from `Cargo.toml`
- [ ] Function is marked as view/read-only
- [ ] Client binding updated to include the new function
- [ ] Basic test added to verify version string format

## Additional Context
This follows the pattern established in other contracts in the repository and supports better version management across the ahjoorxmr ecosystem.

## Priority
Low - Quality of life improvement

## Labels
- enhancement
- token-whitelist
- good first issue
