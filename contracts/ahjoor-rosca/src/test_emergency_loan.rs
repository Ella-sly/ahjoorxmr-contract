#![cfg(test)]
use super::*;
use soroban_sdk::token::StellarAssetClient as TokenAdminClient;
use soroban_sdk::{testutils::Address as _, Address, Env};

fn base_config() -> RoscaConfig {
    RoscaConfig {
        strategy: PayoutStrategy::RoundRobin,
        custom_order: None,
        penalty_amount: 0,
        exit_penalty_bps: 0,
        collective_goal: None,
        member_goals: None,
        fee_bps: 0,
        fee_recipient: None,
        max_defaults: 3,
        grace_period_ledgers: 0,
        use_timestamp_schedule: false,
        round_duration_seconds: 0,
        max_members: None,
        skip_fee: 0,
        max_skips_per_cycle: 0,
        voting_mode: VotingMode::Equal,
        late_fee_bps: 0,
        grace_period_seconds: 0,
        auction_enabled: false,
        auction_window_ledgers: 0,
        randomize_payout_order: false,
        reserve_enabled: false,
        reserve_contribution_bps: 0,
    }
}

/// Sets up a group with the emergency reserve enabled and funded, contract-
/// internally, since there is no public setter for `ReserveEnabled`.
fn setup_with_reserve<'a>(
    reserve_balance: i128,
) -> (
    Env,
    AhjoorContractClient<'a>,
    Address,
    Address,
    soroban_sdk::Vec<Address>,
) {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(AhjoorContract, ());
    let client = AhjoorContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token_addr = env
        .register_stellar_asset_contract_v2(admin.clone())
        .address();
    let token_admin_client = TokenAdminClient::new(&env, &token_addr);

    let mut members = soroban_sdk::Vec::new(&env);
    for _ in 0..3 {
        let addr = Address::generate(&env);
        token_admin_client.mint(&addr, &10_000);
        members.push_back(addr);
    }

    client.init(
        &admin,
        &members,
        &100,
        &token_addr,
        &3600,
        &base_config(),
        &None,
    );

    // Fund the contract itself so it can transfer out the loan.
    token_admin_client.mint(&contract_id, &reserve_balance);

    env.as_contract(&contract_id, || {
        env.storage()
            .instance()
            .set(&DataKey3::ReserveEnabled, &true);
        env.storage()
            .persistent()
            .set(&DataKey3::EmergencyReserveBalance, &reserve_balance);
    });

    (env, client, admin, token_addr, members)
}

// ─── #754: request_emergency_loan requires group membership ───────────────

#[test]
fn test_emergency_loan_granted_to_member() {
    let (_env, client, _admin, _token_addr, members) = setup_with_reserve(1_000);
    let borrower = members.get(0).unwrap();

    let loan_id = client.request_emergency_loan(&borrower, &100, &1000);
    let loan = client.get_emergency_loan(&loan_id);
    assert_eq!(loan.borrower, borrower);
    assert_eq!(loan.amount, 100);
}

#[test]
#[should_panic(expected = "Error(Contract, #8)")]
fn test_emergency_loan_rejects_non_member() {
    let (env, client, _admin, _token_addr, _members) = setup_with_reserve(1_000);
    let outsider = Address::generate(&env);

    client.request_emergency_loan(&outsider, &100, &1000);
}
