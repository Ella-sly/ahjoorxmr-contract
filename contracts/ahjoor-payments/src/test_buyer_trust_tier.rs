use soroban_sdk::testutils::Ledger;
use super::*;
use soroban_sdk::token::StellarAssetClient as TokenAdminClient;
use soroban_sdk::{testutils::Address as _, Address, Env};

fn setup<'a>() -> (
    Env,
    AhjoorPaymentsContractClient<'a>,
    Address,
    Address,
    Address,
    Address,
    TokenAdminClient<'a>,
) {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(AhjoorPaymentsContract, ());
    let client = AhjoorPaymentsContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let merchant = Address::generate(&env);
    let buyer = Address::generate(&env);
    let token_addr = env.register_stellar_asset_contract_v2(admin.clone()).address();
    let tac = TokenAdminClient::new(&env, &token_addr);

    client.initialize(&admin, &admin, &0u32);
    client.set_min_collateral(&0i128);
    client.approve_merchant(&merchant);
    tac.mint(&buyer, &100_000);

    (env, client, admin, merchant, buyer, token_addr, tac)
}

#[test]
fn test_set_and_get_buyer_tier() {
    let (_env, client, _admin, merchant, buyer, _token, _tac) = setup();

    // Default tier is New
    assert_eq!(client.get_buyer_tier(&merchant, &buyer), BuyerTrustTierLevel::New);

    // Set to Trusted
    client.set_buyer_tier(&merchant, &buyer, &BuyerTrustTierLevel::Trusted);
    assert_eq!(client.get_buyer_tier(&merchant, &buyer), BuyerTrustTierLevel::Trusted);

    // Upgrade to VIP
    client.set_buyer_tier(&merchant, &buyer, &BuyerTrustTierLevel::VIP);
    assert_eq!(client.get_buyer_tier(&merchant, &buyer), BuyerTrustTierLevel::VIP);
}

#[test]
fn test_tier_downgrade_takes_effect_immediately() {
    let (_env, client, _admin, merchant, buyer, token, _tac) = setup();

    // Set VIP tier with high limit
    client.set_buyer_tier(&merchant, &buyer, &BuyerTrustTierLevel::VIP);
    client.set_tier_spending_limit(&merchant, &BuyerTrustTierLevel::VIP, &50_000i128, &3600u64);

    // Payment succeeds under VIP limit
    let pid = client.create_payment(&buyer, &merchant, &10_000, &token, &None, &None, &None);
    client.complete_payment(&pid);

    // Downgrade to New (strict limit)
    client.set_buyer_tier(&merchant, &buyer, &BuyerTrustTierLevel::New);
    client.set_tier_spending_limit(&merchant, &BuyerTrustTierLevel::New, &100i128, &3600u64);

    // New payment with downgraded tier is rejected
    let pid2 = client.create_payment(&buyer, &merchant, &500, &token, &None, &None, &None);
    let result = client.try_complete_payment(&pid2);
    assert!(result.is_err());
}

#[test]
fn test_tier_limit_falls_back_to_global_when_unset() {
    let (_env, client, _admin, merchant, buyer, token, _tac) = setup();

    // Set buyer tier but no tier-specific limit
    client.set_buyer_tier(&merchant, &buyer, &BuyerTrustTierLevel::Trusted);

    // Set global default limit
    client.set_default_spend_limit(&merchant, &200i128, &3600u64);

    // Payment exceeding global default should fail
    let pid = client.create_payment(&buyer, &merchant, &300, &token, &None, &None, &None);
    let result = client.try_complete_payment(&pid);
    assert!(result.is_err());
}

#[test]
fn test_per_customer_override_takes_priority_over_tier() {
    let (_env, client, _admin, merchant, buyer, token, _tac) = setup();

    // Set Trusted tier with low limit
    client.set_buyer_tier(&merchant, &buyer, &BuyerTrustTierLevel::Trusted);
    client.set_tier_spending_limit(&merchant, &BuyerTrustTierLevel::Trusted, &100i128, &3600u64);

    // Set per-customer override with high limit
    client.set_customer_spend_limit(&merchant, &buyer, &50_000i128, &3600u64);

    // Payment within per-customer override should succeed
    let pid = client.create_payment(&buyer, &merchant, &10_000, &token, &None, &None, &None);
    client.complete_payment(&pid);
    assert_eq!(client.get_payment(&pid).status, PaymentStatus::Completed);
}

#[test]
fn test_each_tier_has_independent_limit() {
    let (env, client, _admin, merchant, buyer, token, _tac) = setup();
    let buyer2 = Address::generate(&env);
    let tac = soroban_sdk::token::StellarAssetClient::new(&env, &token);
    tac.mint(&buyer2, &100_000);

    // Set different limits per tier
    client.set_tier_spending_limit(&merchant, &BuyerTrustTierLevel::New, &100i128, &3600u64);
    client.set_tier_spending_limit(&merchant, &BuyerTrustTierLevel::Trusted, &10_000i128, &3600u64);

    // buyer → New tier, buyer2 → Trusted tier
    client.set_buyer_tier(&merchant, &buyer, &BuyerTrustTierLevel::New);
    client.set_buyer_tier(&merchant, &buyer2, &BuyerTrustTierLevel::Trusted);

    // New-tier buyer fails on large payment
    let p1 = client.create_payment(&buyer, &merchant, &500, &token, &None, &None, &None);
    assert!(client.try_complete_payment(&p1).is_err());

    // Trusted-tier buyer succeeds on same payment
    let p2 = client.create_payment(&buyer2, &merchant, &500, &token, &None, &None, &None);
    client.complete_payment(&p2);
    assert_eq!(client.get_payment(&p2).status, PaymentStatus::Completed);
}

#[test]
fn test_tier_upgrade_allows_higher_limit() {
    let (_env, client, _admin, merchant, buyer, token, _tac) = setup();

    // Start with New tier (low limit)
    client.set_tier_spending_limit(&merchant, &BuyerTrustTierLevel::New, &100i128, &3600u64);
    client.set_buyer_tier(&merchant, &buyer, &BuyerTrustTierLevel::New);

    // Payment exceeding New tier limit fails
    let p1 = client.create_payment(&buyer, &merchant, &500, &token, &None, &None, &None);
    assert!(client.try_complete_payment(&p1).is_err());

    // Upgrade to VIP with higher limit
    client.set_tier_spending_limit(&merchant, &BuyerTrustTierLevel::VIP, &10_000i128, &3600u64);
    client.set_buyer_tier(&merchant, &buyer, &BuyerTrustTierLevel::VIP);

    // Same payment amount now succeeds
    let p2 = client.create_payment(&buyer, &merchant, &500, &token, &None, &None, &None);
    client.complete_payment(&p2);
    assert_eq!(client.get_payment(&p2).status, PaymentStatus::Completed);
}

#[test]
fn test_multiple_buyers_different_tiers() {
    let (env, client, _admin, merchant, buyer1, token, tac) = setup();
    let buyer2 = Address::generate(&env);
    let buyer3 = Address::generate(&env);
    tac.mint(&buyer2, &100_000);
    tac.mint(&buyer3, &100_000);

    // Set different tiers for different buyers
    client.set_tier_spending_limit(&merchant, &BuyerTrustTierLevel::New, &50i128, &3600u64);
    client.set_tier_spending_limit(&merchant, &BuyerTrustTierLevel::Standard, &500i128, &3600u64);
    client.set_tier_spending_limit(&merchant, &BuyerTrustTierLevel::Trusted, &5_000i128, &3600u64);

    client.set_buyer_tier(&merchant, &buyer1, &BuyerTrustTierLevel::New);
    client.set_buyer_tier(&merchant, &buyer2, &BuyerTrustTierLevel::Standard);
    client.set_buyer_tier(&merchant, &buyer3, &BuyerTrustTierLevel::Trusted);

    // Each buyer respects their tier limit
    let p1 = client.create_payment(&buyer1, &merchant, &100, &token, &None, &None, &None);
    assert!(client.try_complete_payment(&p1).is_err());

    let p2 = client.create_payment(&buyer2, &merchant, &400, &token, &None, &None, &None);
    client.complete_payment(&p2);
    assert_eq!(client.get_payment(&p2).status, PaymentStatus::Completed);

    let p3 = client.create_payment(&buyer3, &merchant, &4_000, &token, &None, &None, &None);
    client.complete_payment(&p3);
    assert_eq!(client.get_payment(&p3).status, PaymentStatus::Completed);
}

#[test]
fn test_tier_limit_window_resets() {
    let (env, client, _admin, merchant, buyer, token, _tac) = setup();

    // Set Trusted tier with 1-second window for testing
    client.set_buyer_tier(&merchant, &buyer, &BuyerTrustTierLevel::Trusted);
    client.set_tier_spending_limit(&merchant, &BuyerTrustTierLevel::Trusted, &1_000i128, &1u64);

    // First payment succeeds
    let p1 = client.create_payment(&buyer, &merchant, &800, &token, &None, &None, &None);
    client.complete_payment(&p1);

    // Second payment exceeds limit
    let p2 = client.create_payment(&buyer, &merchant, &300, &token, &None, &None, &None);
    assert!(client.try_complete_payment(&p2).is_err());

    // Advance time beyond window
    env.ledger().with_mut(|li| li.timestamp += 2);

    // New payment succeeds after window reset
    let p3 = client.create_payment(&buyer, &merchant, &800, &token, &None, &None, &None);
    client.complete_payment(&p3);
    assert_eq!(client.get_payment(&p3).status, PaymentStatus::Completed);
}

#[test]
fn test_standard_tier_distinct_from_new_and_trusted() {
    let (_env, client, _admin, merchant, buyer, token, _tac) = setup();

    // Set distinct limits for all tiers
    client.set_tier_spending_limit(&merchant, &BuyerTrustTierLevel::New, &50i128, &3600u64);
    client.set_tier_spending_limit(&merchant, &BuyerTrustTierLevel::Standard, &500i128, &3600u64);
    client.set_tier_spending_limit(&merchant, &BuyerTrustTierLevel::Trusted, &5_000i128, &3600u64);

    // Set buyer to Standard tier
    client.set_buyer_tier(&merchant, &buyer, &BuyerTrustTierLevel::Standard);

    // Payment above New limit but within Standard limit succeeds
    let p1 = client.create_payment(&buyer, &merchant, &300, &token, &None, &None, &None);
    client.complete_payment(&p1);
    assert_eq!(client.get_payment(&p1).status, PaymentStatus::Completed);

    // Payment above Standard limit fails
    let p2 = client.create_payment(&buyer, &merchant, &600, &token, &None, &None, &None);
    assert!(client.try_complete_payment(&p2).is_err());
}

#[test]
fn test_vip_tier_has_highest_privileges() {
    let (_env, client, _admin, merchant, buyer, token, _tac) = setup();

    // Set VIP limit much higher than other tiers
    client.set_tier_spending_limit(&merchant, &BuyerTrustTierLevel::New, &100i128, &3600u64);
    client.set_tier_spending_limit(&merchant, &BuyerTrustTierLevel::Trusted, &1_000i128, &3600u64);
    client.set_tier_spending_limit(&merchant, &BuyerTrustTierLevel::VIP, &50_000i128, &3600u64);

    client.set_buyer_tier(&merchant, &buyer, &BuyerTrustTierLevel::VIP);

    // VIP can make large payment that other tiers cannot
    let p1 = client.create_payment(&buyer, &merchant, &25_000, &token, &None, &None, &None);
    client.complete_payment(&p1);
    assert_eq!(client.get_payment(&p1).status, PaymentStatus::Completed);
}

#[test]
fn test_unset_tier_uses_new_tier_by_default() {
    let (_env, client, _admin, merchant, buyer, token, _tac) = setup();

    // Set New tier limit but don't explicitly assign buyer to any tier
    client.set_tier_spending_limit(&merchant, &BuyerTrustTierLevel::New, &200i128, &3600u64);

    // Buyer should default to New tier
    assert_eq!(client.get_buyer_tier(&merchant, &buyer), BuyerTrustTierLevel::New);

    // Payment within New tier limit succeeds
    let p1 = client.create_payment(&buyer, &merchant, &150, &token, &None, &None, &None);
    client.complete_payment(&p1);
    assert_eq!(client.get_payment(&p1).status, PaymentStatus::Completed);

    // Payment exceeding New tier limit fails
    let p2 = client.create_payment(&buyer, &merchant, &250, &token, &None, &None, &None);
    assert!(client.try_complete_payment(&p2).is_err());
}

#[test]
fn test_tier_limit_accumulates_within_window() {
    let (_env, client, _admin, merchant, buyer, token, _tac) = setup();

    client.set_buyer_tier(&merchant, &buyer, &BuyerTrustTierLevel::Trusted);
    client.set_tier_spending_limit(&merchant, &BuyerTrustTierLevel::Trusted, &1_000i128, &3600u64);

    // First payment of 400 succeeds
    let p1 = client.create_payment(&buyer, &merchant, &400, &token, &None, &None, &None);
    client.complete_payment(&p1);

    // Second payment of 400 succeeds (total 800)
    let p2 = client.create_payment(&buyer, &merchant, &400, &token, &None, &None, &None);
    client.complete_payment(&p2);

    // Third payment of 300 would exceed limit (total 1100)
    let p3 = client.create_payment(&buyer, &merchant, &300, &token, &None, &None, &None);
    assert!(client.try_complete_payment(&p3).is_err());

    // Smaller payment within remaining limit succeeds
    let p4 = client.create_payment(&buyer, &merchant, &150, &token, &None, &None, &None);
    client.complete_payment(&p4);
    assert_eq!(client.get_payment(&p4).status, PaymentStatus::Completed);
}

#[test]
fn test_tier_change_during_window_applies_new_limit() {
    let (_env, client, _admin, merchant, buyer, token, _tac) = setup();

    // Start with Trusted tier
    client.set_tier_spending_limit(&merchant, &BuyerTrustTierLevel::Trusted, &2_000i128, &3600u64);
    client.set_tier_spending_limit(&merchant, &BuyerTrustTierLevel::Standard, &500i128, &3600u64);
    client.set_buyer_tier(&merchant, &buyer, &BuyerTrustTierLevel::Trusted);

    // Make payment of 1000
    let p1 = client.create_payment(&buyer, &merchant, &1_000, &token, &None, &None, &None);
    client.complete_payment(&p1);

    // Downgrade to Standard tier mid-window
    client.set_buyer_tier(&merchant, &buyer, &BuyerTrustTierLevel::Standard);

    // Previous spend (1000) now exceeds Standard limit (500), so new payment fails
    let p2 = client.create_payment(&buyer, &merchant, &100, &token, &None, &None, &None);
    assert!(client.try_complete_payment(&p2).is_err());
}

#[test]
fn test_removing_per_customer_override_falls_back_to_tier() {
    let (_env, client, _admin, merchant, buyer, token, _tac) = setup();

    // Set Trusted tier with moderate limit
    client.set_buyer_tier(&merchant, &buyer, &BuyerTrustTierLevel::Trusted);
    client.set_tier_spending_limit(&merchant, &BuyerTrustTierLevel::Trusted, &1_000i128, &3600u64);

    // Set high per-customer override
    client.set_customer_spend_limit(&merchant, &buyer, &10_000i128, &3600u64);

    // Payment within override succeeds
    let p1 = client.create_payment(&buyer, &merchant, &5_000, &token, &None, &None, &None);
    client.complete_payment(&p1);

    // Remove per-customer override
    client.remove_customer_spend_limit(&merchant, &buyer);

    // Now should fall back to tier limit; previous spend (5000) exceeds tier limit (1000)
    let p2 = client.create_payment(&buyer, &merchant, &100, &token, &None, &None, &None);
    assert!(client.try_complete_payment(&p2).is_err());
}

#[test]
fn test_zero_tier_limit_rejected() {
    let (_env, client, _admin, merchant, _buyer, _token, _tac) = setup();

    // Spend limits must be positive; a zero tier limit is rejected
    let result = client.try_set_tier_spending_limit(
        &merchant,
        &BuyerTrustTierLevel::New,
        &0i128,
        &3600u64,
    );
    assert!(result.is_err(), "Zero tier limit should be rejected");
}
