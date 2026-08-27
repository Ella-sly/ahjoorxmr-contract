#![cfg(test)]
use super::*;
use soroban_sdk::token::Client as TokenClient;
use soroban_sdk::token::StellarAssetClient as TokenAdminClient;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    Address, Env, Vec,
};

fn setup_group<'a>() -> (
    Env,
    AhjoorContractClient<'a>,
    Address,
    Vec<Address>,
    Address,
    TokenClient<'a>,
    TokenAdminClient<'a>,
) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(AhjoorContract, ());
    let client = AhjoorContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let mut members = Vec::new(&env);
    for _ in 0..3 {
        members.push_back(Address::generate(&env));
    }
    let token_addr = env
        .register_stellar_asset_contract_v2(admin.clone())
        .address();
    let token_client = TokenClient::new(&env, &token_addr);
    let tac = TokenAdminClient::new(&env, &token_addr);
    for i in 0..members.len() {
        tac.mint(&members.get(i).unwrap(), &50_000);
    }
    client.init(
        &admin,
        &members,
        &100i128,
        &token_addr,
        &3600u64,
        &RoscaConfig {
            strategy: PayoutStrategy::RoundRobin,
            custom_order: None,
            penalty_amount: 50,
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
        },
        &None,
    );
    (env, client, admin, members, token_addr, token_client, tac)
}

#[test]
fn test_prepay_rounds_locks_balance() {
    let (_env, client, _admin, members, _token, _tc, _tac) = setup_group();
    let member = members.get(0).unwrap();

    client.prepay_rounds(&member, &3u32);
    // One round applied immediately to current round → 2 remaining
    assert_eq!(client.get_prepaid_balance(&member), 200);
    let (paid, remaining) = client.get_member_contribution_status(&member);
    assert_eq!(paid, 100);
    assert_eq!(remaining, 0);
}

#[test]
fn test_prepay_auto_settles_across_rounds() {
    let (_env, client, _admin, members, token, _tc, _tac) = setup_group();
    let prepaid_member = members.get(0).unwrap();
    let m1 = members.get(1).unwrap();
    let m2 = members.get(2).unwrap();

    // Prepay 3 rounds (current + 2 future)
    client.prepay_rounds(&prepaid_member, &3u32);
    assert_eq!(client.get_prepaid_balance(&prepaid_member), 200);

    // Other members contribute — when the last one pays, the round auto-advances
    // and prepaid is drawn for the next round inside reset_round_state.
    client.contribute(&m1, &token, &100i128);
    client.contribute(&m2, &token, &100i128);

    assert_eq!(client.get_prepaid_balance(&prepaid_member), 100);
    let (paid, _) = client.get_member_contribution_status(&prepaid_member);
    assert_eq!(paid, 100);

    // Next round: others pay again → auto-advance consumes the last prepaid unit
    client.contribute(&m1, &token, &100i128);
    client.contribute(&m2, &token, &100i128);

    assert_eq!(client.get_prepaid_balance(&prepaid_member), 0);
    let (paid2, _) = client.get_member_contribution_status(&prepaid_member);
    assert_eq!(paid2, 100);
}

#[test]
fn test_withdraw_unused_prepaid() {
    let (_env, client, _admin, members, _token, token_client, _tac) = setup_group();
    let member = members.get(0).unwrap();
    let before = token_client.balance(&member);

    client.prepay_rounds(&member, &2u32);
    // 100 consumed for current round, 100 remaining
    assert_eq!(client.get_prepaid_balance(&member), 100);

    client.withdraw_prepaid_balance(&member, &100i128);
    assert_eq!(client.get_prepaid_balance(&member), 0);
    assert_eq!(token_client.balance(&member), before - 100);
}

#[test]
fn test_cannot_withdraw_consumed_prepaid() {
    let (_env, client, _admin, members, _token, _tc, _tac) = setup_group();
    let member = members.get(0).unwrap();

    client.prepay_rounds(&member, &1u32);
    // Fully consumed by current round
    assert_eq!(client.get_prepaid_balance(&member), 0);

    let result = client.try_withdraw_prepaid_balance(&member, &100i128);
    assert_eq!(
        result.unwrap_err().unwrap(),
        ExtError2::InsufficientPrepaidBalance.into()
    );
}

#[test]
fn test_partial_prepay_falls_back_to_default() {
    let (env, client, admin, members, token, _tc, _tac) = setup_group();
    let underfunded = members.get(0).unwrap();
    let m1 = members.get(1).unwrap();
    let m2 = members.get(2).unwrap();

    // Prepay 2 rounds → 100 applied now, 100 remaining. Withdraw 50 → only 50 left
    // for the next round (insufficient).
    client.prepay_rounds(&underfunded, &2u32);
    client.withdraw_prepaid_balance(&underfunded, &50i128);
    assert_eq!(client.get_prepaid_balance(&underfunded), 50);

    client.contribute(&m1, &token, &100i128);
    client.contribute(&m2, &token, &100i128);
    env.ledger().set_timestamp(env.ledger().timestamp() + 3601);
    client.finalize_round();

    // New round opened with only 50 prepaid — insufficient, so underfunded is unpaid
    assert_eq!(client.get_prepaid_balance(&underfunded), 50);
    let (paid, _) = client.get_member_contribution_status(&underfunded);
    assert_eq!(paid, 0);

    // Others pay; underfunded does not → default on finalize
    client.contribute(&m1, &token, &100i128);
    client.contribute(&m2, &token, &100i128);
    env.ledger().set_timestamp(env.ledger().timestamp() + 3601);
    client.finalize_round();

    let status = client.get_member_status(&underfunded);
    assert!(status.default_count >= 1);
}
