#![cfg(test)]
use super::*;
use soroban_sdk::token::StellarAssetClient as TokenAdminClient;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    Address, Env,
};

fn setup<'a>() -> (
    Env,
    AhjoorPaymentsContractClient<'a>,
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
    let token_addr = env
        .register_stellar_asset_contract_v2(admin.clone())
        .address();
    let tac = TokenAdminClient::new(&env, &token_addr);
    client.initialize(&admin, &admin, &0u32);
    client.set_min_collateral(&0i128);
    client.approve_merchant(&merchant);
    (env, client, admin, merchant, token_addr, tac)
}

#[test]
fn test_invoice_cap_enforced_at_boundary() {
    let (env, client, admin, merchant, token_addr, tac) = setup();
    let customer = Address::generate(&env);
    tac.mint(&customer, &10_000);

    client.set_max_invoices_per_period(&admin, &merchant, &100u32, &2u32);

    let _p1 = client.create_payment(&customer, &merchant, &10, &token_addr, &None, &None, &None);
    let _p2 = client.create_payment(&customer, &merchant, &10, &token_addr, &None, &None, &None);

    let window = client.get_invoice_count_window(&merchant);
    assert_eq!(window.count, 2);

    let rejected = client.try_create_payment(&customer, &merchant, &10, &token_addr, &None, &None, &None);
    assert_eq!(
        rejected.unwrap_err().unwrap(),
        ExtError::MerchantInvoiceCapExceeded.into()
    );
}

#[test]
fn test_invoice_cap_window_resets() {
    let (env, client, admin, merchant, token_addr, tac) = setup();
    let customer = Address::generate(&env);
    tac.mint(&customer, &10_000);

    env.ledger().set_sequence_number(1_000);
    client.set_max_invoices_per_period(&admin, &merchant, &50u32, &2u32);

    let _p1 = client.create_payment(&customer, &merchant, &10, &token_addr, &None, &None, &None);
    let _p2 = client.create_payment(&customer, &merchant, &10, &token_addr, &None, &None, &None);
    assert!(client
        .try_create_payment(&customer, &merchant, &10, &token_addr, &None, &None, &None)
        .is_err());

    // Advance past the period window
    env.ledger().set_sequence_number(1_050);
    let p3 = client.create_payment(&customer, &merchant, &10, &token_addr, &None, &None, &None);
    assert!(p3 > 0);
    let window = client.get_invoice_count_window(&merchant);
    assert_eq!(window.count, 1);
    assert_eq!(window.window_start, 1_050);
}

#[test]
fn test_merchant_without_cap_unaffected() {
    let (env, client, _admin, merchant, token_addr, tac) = setup();
    let customer = Address::generate(&env);
    tac.mint(&customer, &10_000);

    // No cap configured — many invoices succeed
    for _ in 0..5 {
        let _ = client.create_payment(&customer, &merchant, &10, &token_addr, &None, &None, &None);
    }
    let window = client.get_invoice_count_window(&merchant);
    assert_eq!(window.count, 0);
}

#[test]
fn test_invoice_cap_independent_of_volume_cap() {
    let (env, client, admin, merchant, token_addr, tac) = setup();
    let customer = Address::generate(&env);
    tac.mint(&customer, &10_000);

    // High volume cap, low invoice count
    client.set_merchant_volume_cap(&admin, &merchant, &1_000_000i128, &3600u64);
    client.set_max_invoices_per_period(&admin, &merchant, &100u32, &1u32);

    let _p1 = client.create_payment(&customer, &merchant, &1, &token_addr, &None, &None, &None);
    let rejected = client.try_create_payment(&customer, &merchant, &1, &token_addr, &None, &None, &None);
    assert_eq!(
        rejected.unwrap_err().unwrap(),
        ExtError::MerchantInvoiceCapExceeded.into()
    );
}
