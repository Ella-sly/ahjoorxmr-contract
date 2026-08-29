use soroban_sdk::{contractclient, Address, Env};
use crate::VotingMode;

/// Minimal cross-contract read interface used by `clone_group` to pull
/// configuration values from an existing ROSCA group contract.
///
/// Only view (read-only) functions are exposed here — no state is mutated
/// on the source contract during cloning.
#[contractclient(name = "RoscaCloneClient")]
pub trait RoscaCloneInterface {
    // ── Token & Contribution ────────────────────────────────────────────────
    fn get_token(env: Env) -> Address;
    fn get_contribution_amount(env: Env) -> i128;

    // ── Round Schedule ───────────────────────────────────────────────────────
    fn get_round_duration(env: Env) -> u64;
    fn get_use_timestamp_schedule(env: Env) -> bool;
    fn get_round_duration_seconds(env: Env) -> u64;

    // ── Fees & Penalties ────────────────────────────────────────────────────
    fn get_fee_bps(env: Env) -> u32;
    fn get_penalty_amount(env: Env) -> i128;
    fn get_exit_penalty_bps(env: Env) -> u32;

    // ── Defaults & Grace ────────────────────────────────────────────────────
    fn get_max_defaults(env: Env) -> u32;
    fn get_grace_period_ledgers(env: Env) -> u32;
    fn get_grace_period_seconds(env: Env) -> u64;

    // ── Skip Config ─────────────────────────────────────────────────────────
    fn get_skip_fee(env: Env) -> i128;
    fn get_max_skips_per_cycle(env: Env) -> u32;

    // ── Voting ───────────────────────────────────────────────────────────────
    fn get_voting_mode(env: Env) -> VotingMode;

    // ── Slot Auction ─────────────────────────────────────────────────────────
    fn get_auction_enabled(env: Env) -> bool;
    fn get_auction_window_ledgers(env: Env) -> u64;

    // ── Membership ───────────────────────────────────────────────────────────
    fn get_max_members_opt(env: Env) -> Option<u32>;

    // ── Reserve ──────────────────────────────────────────────────────────────
    fn get_reserve_enabled(env: Env) -> bool;
    fn get_reserve_contribution_bps(env: Env) -> u32;
}
