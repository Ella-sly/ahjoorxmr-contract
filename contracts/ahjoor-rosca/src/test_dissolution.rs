#![cfg(test)]
//! Coverage for the group dissolution lifecycle (`set_dissolution_config`,
//! `start_dissolution_vote`, `vote_dissolve_group`, `execute_dissolution`,
//! `dissolve_group`).
use super::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token::{Client as TokenClient, StellarAssetClient as TokenAdminClient},
    Address, BytesN, Env, Vec,
};

const CONTRIBUTION: i128 = 100;
const START_BALANCE: i128 = 1_000;
const START_TS: u64 = 1_000;
const VOTE_WINDOW: u64 = 500;

struct Setup<'a> {
    env: Env,
    client: AhjoorContractClient<'a>,
    admin: Address,
    members: Vec<Address>,
    token: TokenClient<'a>,
    token_admin: TokenAdminClient<'a>,
}

fn setup<'a>(member_count: u32) -> Setup<'a> {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(START_TS);

    let contract_id = env.register(AhjoorContract, ());
    let client = AhjoorContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token_addr = env
        .register_stellar_asset_contract_v2(admin.clone())
        .address();
    let token = TokenClient::new(&env, &token_addr);
    let token_admin = TokenAdminClient::new(&env, &token_addr);

    let mut members = Vec::new(&env);
    for _ in 0..member_count {
        let m = Address::generate(&env);
        token_admin.mint(&m, &START_BALANCE);
        members.push_back(m);
    }

    client.init(
        &admin,
        &members,
        &CONTRIBUTION,
        &token_addr,
        &3600,
        &RoscaConfig {
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
            grace_period_seconds: 0,
            use_timestamp_schedule: false,
            round_duration_seconds: 0,
            max_members: None,
            skip_fee: 0,
            max_skips_per_cycle: 0,
            voting_mode: VotingMode::Equal,
            late_fee_bps: 0,
            auction_enabled: false,
            auction_window_ledgers: 0,
            randomize_payout_order: false,
            reserve_enabled: false,
            reserve_contribution_bps: 0,
        },
        &None,
    );

    Setup {
        env,
        client,
        admin,
        members,
        token,
        token_admin,
    }
}

fn reason(env: &Env) -> BytesN<32> {
    BytesN::from_array(env, &[9u8; 32])
}

/// Funds the contract by having the first `n` members contribute for the
/// current round (fewer than all members, so no payout is triggered).
fn contribute_from_first(s: &Setup, n: u32) {
    for i in 0..n {
        s.client
            .contribute(&s.members.get(i).unwrap(), &s.token.address, &CONTRIBUTION);
    }
}

fn member(s: &Setup, i: u32) -> Address {
    s.members.get(i).unwrap()
}

// ===========================================================================
//  set_dissolution_config
// ===========================================================================

#[test]
fn test_set_dissolution_config_rejects_invalid_values() {
    let s = setup(4);

    let low_quorum = s.client.try_set_dissolution_config(&s.admin, &999, &VOTE_WINDOW);
    assert_eq!(
        low_quorum.unwrap_err().unwrap(),
        ExtError::InvalidDissolutionConfig.into()
    );
    let high_quorum = s
        .client
        .try_set_dissolution_config(&s.admin, &10_001, &VOTE_WINDOW);
    assert_eq!(
        high_quorum.unwrap_err().unwrap(),
        ExtError::InvalidDissolutionConfig.into()
    );
    let zero_window = s.client.try_set_dissolution_config(&s.admin, &7500, &0);
    assert_eq!(
        zero_window.unwrap_err().unwrap(),
        ExtError::InvalidDissolutionConfig.into()
    );
}

#[test]
#[should_panic(expected = "Only admin can set dissolution config")]
fn test_set_dissolution_config_rejects_non_admin() {
    let s = setup(4);
    s.client
        .set_dissolution_config(&member(&s, 0), &7500, &VOTE_WINDOW);
}

#[test]
fn test_set_dissolution_config_sets_vote_window() {
    let s = setup(4);
    s.client.set_dissolution_config(&s.admin, &5000, &VOTE_WINDOW);
    s.client.start_dissolution_vote(&member(&s, 0));

    // Still inside the configured window: a second vote cannot be started.
    s.env.ledger().set_timestamp(START_TS + VOTE_WINDOW - 1);
    let in_progress = s.client.try_start_dissolution_vote(&member(&s, 1));
    assert_eq!(
        in_progress.unwrap_err().unwrap(),
        ExtError::DissolutionVoteInProgress.into()
    );

    // Past the configured window: voting is closed.
    s.env.ledger().set_timestamp(START_TS + VOTE_WINDOW + 1);
    let expired = s.client.try_vote_dissolve_group(&member(&s, 0), &true);
    assert_eq!(
        expired.unwrap_err().unwrap(),
        ExtError::DissolutionVoteExpired.into()
    );
}

// ===========================================================================
//  start_dissolution_vote / vote_dissolve_group
// ===========================================================================

#[test]
fn test_start_dissolution_vote_rejects_non_member() {
    let s = setup(4);
    let outsider = Address::generate(&s.env);
    let result = s.client.try_start_dissolution_vote(&outsider);
    assert_eq!(result.unwrap_err().unwrap(), Error::NotAMember.into());
}

#[test]
#[should_panic(expected = "No dissolution vote in progress")]
fn test_vote_without_started_vote_panics() {
    let s = setup(4);
    s.client.vote_dissolve_group(&member(&s, 0), &true);
}

#[test]
fn test_vote_rejects_non_member() {
    let s = setup(4);
    s.client.set_dissolution_config(&s.admin, &7500, &VOTE_WINDOW);
    s.client.start_dissolution_vote(&member(&s, 0));

    let outsider = Address::generate(&s.env);
    let result = s.client.try_vote_dissolve_group(&outsider, &true);
    assert_eq!(
        result.unwrap_err().unwrap(),
        Error::OnlyMembersAllowed.into()
    );
}

#[test]
#[should_panic(expected = "Already voted on dissolution")]
fn test_vote_twice_panics() {
    let s = setup(4);
    s.client.set_dissolution_config(&s.admin, &7500, &VOTE_WINDOW);
    s.client.start_dissolution_vote(&member(&s, 0));

    s.client.vote_dissolve_group(&member(&s, 0), &true);
    s.client.vote_dissolve_group(&member(&s, 0), &true);
}

// ===========================================================================
//  execute_dissolution
// ===========================================================================

#[test]
fn test_vote_passes_and_distributes_pro_rata() {
    let s = setup(4);
    // Members 0..3 contribute; member 3 does not.
    contribute_from_first(&s, 3);
    // Extra funds in the pool are shared pro-rata by contribution.
    s.token_admin.mint(&s.client.address, &30);
    let pool = 3 * CONTRIBUTION + 30;
    assert_eq!(s.token.balance(&s.client.address), pool);

    // 75% of 4 equal-weight members => 3 approvals required.
    s.client.set_dissolution_config(&s.admin, &7500, &VOTE_WINDOW);
    s.client.start_dissolution_vote(&member(&s, 0));
    s.client.vote_dissolve_group(&member(&s, 0), &true);
    s.client.vote_dissolve_group(&member(&s, 1), &true);
    s.client.vote_dissolve_group(&member(&s, 2), &true);
    s.client.vote_dissolve_group(&member(&s, 3), &false);

    s.env.ledger().set_timestamp(START_TS + VOTE_WINDOW + 1);
    s.client.execute_dissolution();

    assert_eq!(s.client.get_group_status(), GroupStatus::Dissolved);

    // Each contributor's share = contribution * pool / total contributions.
    let share = CONTRIBUTION * pool / (3 * CONTRIBUTION);
    assert_eq!(share, 110);
    for i in 0..3 {
        assert_eq!(
            s.token.balance(&member(&s, i)),
            START_BALANCE - CONTRIBUTION + share
        );
    }
    // The non-contributor receives nothing.
    assert_eq!(s.token.balance(&member(&s, 3)), START_BALANCE);
    assert_eq!(s.token.balance(&s.client.address), 0);
}

#[test]
fn test_vote_fails_when_quorum_not_met() {
    let s = setup(4);
    contribute_from_first(&s, 3);

    s.client.set_dissolution_config(&s.admin, &7500, &VOTE_WINDOW);
    s.client.start_dissolution_vote(&member(&s, 0));
    // Only 2 of the required 3 approvals.
    s.client.vote_dissolve_group(&member(&s, 0), &true);
    s.client.vote_dissolve_group(&member(&s, 1), &true);
    s.client.vote_dissolve_group(&member(&s, 2), &false);

    s.env.ledger().set_timestamp(START_TS + VOTE_WINDOW + 1);
    let result = s.client.try_execute_dissolution();
    assert_eq!(
        result.unwrap_err().unwrap(),
        ExtError::DissolutionQuorumNotMet.into()
    );

    // Nothing was distributed and the group is still active.
    assert_ne!(s.client.get_group_status(), GroupStatus::Dissolved);
    assert_eq!(s.token.balance(&s.client.address), 3 * CONTRIBUTION);
    for i in 0..3 {
        assert_eq!(s.token.balance(&member(&s, i)), START_BALANCE - CONTRIBUTION);
    }
}

#[test]
#[should_panic(expected = "Voting period not ended")]
fn test_execute_before_vote_ends_panics() {
    let s = setup(4);
    contribute_from_first(&s, 3);

    s.client.set_dissolution_config(&s.admin, &7500, &VOTE_WINDOW);
    s.client.start_dissolution_vote(&member(&s, 0));
    for i in 0..4 {
        s.client.vote_dissolve_group(&member(&s, i), &true);
    }

    // Quorum is met, but the window has not closed yet.
    s.env.ledger().set_timestamp(START_TS + VOTE_WINDOW);
    s.client.execute_dissolution();
}

#[test]
#[should_panic(expected = "No dissolution vote in progress")]
fn test_execute_without_vote_panics() {
    let s = setup(4);
    contribute_from_first(&s, 3);
    s.client.execute_dissolution();
}

// ===========================================================================
//  dissolve_group
// ===========================================================================

#[test]
fn test_admin_dissolve_group_distributes_pro_rata() {
    let s = setup(3);
    contribute_from_first(&s, 2);

    s.client.dissolve_group(&s.admin, &reason(&s.env));

    assert_eq!(s.client.get_group_status(), GroupStatus::Dissolved);
    assert_eq!(s.token.balance(&member(&s, 0)), START_BALANCE);
    assert_eq!(s.token.balance(&member(&s, 1)), START_BALANCE);
    assert_eq!(s.token.balance(&member(&s, 2)), START_BALANCE);
    assert_eq!(s.token.balance(&s.client.address), 0);
}

#[test]
#[should_panic(expected = "Only admin can dissolve group")]
fn test_dissolve_group_rejects_non_admin() {
    let s = setup(3);
    contribute_from_first(&s, 2);
    s.client.dissolve_group(&member(&s, 0), &reason(&s.env));
}

#[test]
fn test_dissolve_group_rejects_empty_pool() {
    let s = setup(3);
    let result = s.client.try_dissolve_group(&s.admin, &reason(&s.env));
    assert_eq!(
        result.unwrap_err().unwrap(),
        ExtError::NoFundsToDistribute.into()
    );
}

#[test]
fn test_dissolved_group_rejects_further_actions() {
    let s = setup(3);
    contribute_from_first(&s, 2);
    s.client.dissolve_group(&s.admin, &reason(&s.env));

    // New contributions are rejected.
    let contribution = s
        .client
        .try_contribute(&member(&s, 2), &s.token.address, &CONTRIBUTION);
    assert_eq!(
        contribution.unwrap_err().unwrap(),
        ExtError::GroupAlreadyDissolved.into()
    );

    // The group cannot be dissolved again or put to a new dissolution vote.
    s.token_admin.mint(&s.client.address, &CONTRIBUTION);
    let again = s.client.try_dissolve_group(&s.admin, &reason(&s.env));
    assert_eq!(
        again.unwrap_err().unwrap(),
        ExtError::GroupAlreadyDissolved.into()
    );
    let vote = s.client.try_start_dissolution_vote(&member(&s, 0));
    assert_eq!(
        vote.unwrap_err().unwrap(),
        ExtError::GroupAlreadyDissolved.into()
    );
}
