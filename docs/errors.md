# Ahjoor Contract Error Codes

Consolidated reference of every numeric error code exposed across all Ahjoor
smart contracts. Off-chain parsers and relay nodes use these codes to
unambiguously decode `InvokeHostFunctionTrapped` errors without per-contract
decode tables.

Each contract owns a non-overlapping numeric range defined in
`contracts/ahjoor-errors/src/lib.rs`.

| Contract              | Range       |
|-----------------------|-------------|
| ahjoor-rosca          | 1000 – 1299 |
| ahjoor-payments       | 2000 – 2299 |
| ahjoor-escrow         | 3000 – 3299 |
| ahjoor-refund         | 4000 – 4099 |
| ahjoor-token-whitelist | 5000 – 5099 |

---

## ahjoor-rosca (1000 – 1299)

### Core Errors

| Code | Name | Description |
|------|------|-------------|
| 1001 | `AlreadyInitialized` | Contract has already been initialized. |
| 1002 | `TokenNotApproved` | Soroban token has not been authorized for spending. |
| 1003 | `CustomOrderLengthMismatch` | Custom round order length does not match the member list. |
| 1004 | `CustomOrderNonMember` | Address in the custom order is not a group member. |
| 1005 | `AmountMustBePositive` | Contribution amount must be greater than zero. |
| 1006 | `RoundDeadlinePassed` | The current round deadline has already elapsed. |
| 1007 | `MemberHasExited` | The member has previously exited the group. |
| 1008 | `NotAMember` | Caller is not a member of this group. |
| 1009 | `AlreadyContributed` | Member has already contributed for this round. |
| 1010 | `InvalidExchangeRate` | Oracle exchange rate is invalid or zero. |
| 1011 | `ExceedsTokenLimit` | Contribution amount exceeds the per-token limit. |
| 1012 | `ExceedsRemainingContribution` | Amount is more than the remaining contribution needed. |
| 1013 | `DeadlineNotPassed` | The round deadline has not yet passed. |
| 1014 | `PenaltyDisabled` | Penalty enforcement is disabled for this group. |
| 1015 | `NotADefaulter` | Member is not on the defaulter list for this round. |
| 1016 | `CannotChangeMidRound` | Group settings cannot be changed while a round is active. |
| 1017 | `AlreadyAMember` | Address is already a member of this group. |
| 1018 | `NoRewardsToClaim` | Member has no pending rewards to claim. |
| 1019 | `OnlyMembersAllowed` | Only group members can perform this action. |
| 1020 | `ProposalNotFound` | Governance proposal does not exist. |
| 1021 | `VotingDeadlinePassed` | The voting window for this proposal has closed. |
| 1022 | `ProposalNotPending` | Proposal is not in the pending state. |
| 1023 | `AlreadyVoted` | Member has already voted on this proposal. |
| 1024 | `VotingNotEnded` | Voting period has not yet ended. |
| 1025 | `ContractPaused` | Group contract is paused. |
| 1026 | `AllMembersSuspended` | All members are currently suspended. |
| 1027 | `AlreadyPaused` | Group is already in a paused state. |
| 1028 | `NotPaused` | Group is not currently paused. |
| 1029 | `MemberAlreadyExited` | Member has already exited the group. |
| 1030 | `ExitRequestPending` | An exit request is already pending for this member. |
| 1031 | `NoExitRequestFound` | No pending exit request found for this member. |
| 1032 | `ExitNotAllowedMidRound` | Members cannot exit during an active round. |
| 1033 | `ContributionWindowClosed` | Contribution rejected because the round deadline has passed. |
| 1034 | `FeeExceedsMaximum` | Fee basis points exceeds maximum allowed (500 bps = 5%). |
| 1035 | `InvalidMaxDefaults` | Max defaults must be at least 1. |
| 1036 | `GroupFull` | Maximum members reached. |
| 1037 | `InvalidMaxMembers` | Invalid maximum member count (must be between 1 and 100). |
| 1038 | `DelegationAlreadyExists` | Delegation already exists for this delegator. |
| 1039 | `NoDelegationFound` | No delegation found for this delegator. |
| 1040 | `CannotVoteWithActiveDelegation` | Delegator cannot vote while delegation is active. |
| 1041 | `CannotSubDelegate` | Delegate cannot further sub-delegate. |
| 1042 | `InviteNotFound` | Invite not found or expired. |
| 1043 | `InviteAlreadyRedeemed` | Invite has already been redeemed. |
| 1044 | `InviteWrongRecipient` | Invite is for a different address. |
| 1045 | `AdminActionNotFound` | Admin action not found. |
| 1046 | `AdminActionAlreadyExecuted` | Admin action has already been executed. |
| 1047 | `AdminActionExpired` | Admin action has expired. |
| 1048 | `AdminAlreadyApproved` | Admin has already approved this action. |
| 1049 | `InsufficientApprovals` | Insufficient approvals for admin action. |
| 1050 | `NotACoAdmin` | Not a co-admin. |

### Extended Errors

| Code | Name | Description |
|------|------|-------------|
| 1051 | `InvalidTier` | Tier must be at least 1 bps. |
| 1052 | `InsurancePoolNegative` | Insurance pool balance would go negative. |
| 1053 | `InvalidInsuranceContribution` | Invalid insurance contribution amount. |
| 1054 | `SkipLimitReached` | Member has reached the maximum allowed skips for the current cycle. |
| 1055 | `AlreadySkipped` | Member has already requested a skip for this round. |
| 1056 | `InsufficientWeight` | Member has zero contribution weight in weighted voting mode. |
| 1057 | `EmergencyPayoutRequested` | Emergency payout already requested for this member in this cycle. |
| 1058 | `EmergencyPayoutQuorumNotMet` | Emergency payout quorum not met. |
| 1059 | `EmergencyPayoutVoteExpired` | Emergency payout vote window expired. |
| 1060 | `EmergencyPayoutAlreadyExecuted` | Emergency payout already executed for this member in this cycle. |
| 1061 | `EmergencyPayoutLimitReached` | Maximum emergency payouts per cycle reached. |
| 1062 | `GroupAlreadyDissolved` | Group is already dissolved. |
| 1063 | `DissolutionVoteInProgress` | Dissolution vote already in progress. |
| 1064 | `DissolutionQuorumNotMet` | Dissolution quorum not met. |
| 1065 | `DissolutionVoteExpired` | Dissolution vote window expired. |
| 1066 | `NoFundsToDistribute` | No funds to distribute during dissolution. |
| 1067 | `InvalidEmergencyConfig` | Invalid emergency payout configuration. |
| 1068 | `InvalidDissolutionConfig` | Invalid dissolution configuration. |
| 1069 | `GroupNotYetActive` | Group start time is in the future. |
| 1070 | `OnlyAdminAllowed` | Action requires admin privileges. |
| 1071 | `InvalidAmount` | Invalid amount or index range. |
| 1072 | `CoSignerAlreadySet` | Co-signer already set for this member. |
| 1073 | `NoCoSignerFound` | No co-signer found for this member. |
| 1074 | `CoSignerNotAccepted` | Co-signer has not accepted the designation. |
| 1075 | `NotTheCoSigner` | Not the designated co-signer for this member. |
| 1076 | `CoSignerWindowNotOpen` | Co-signer window has not opened (member has not defaulted). |
| 1077 | `CoSignerWindowExpired` | Co-signer window has expired. |
| 1078 | `GroupFrozen` | Group is frozen by contract-level admin pending investigation. |
| 1079 | `GroupNotFrozen` | Group is not currently frozen. |
| 1080 | `SnapshotTooSoon` | Snapshot taken too soon; min snapshot interval not elapsed. |
| 1081 | `TierNotFound` | Tier ID does not exist in this group's tier definitions. |
| 1082 | `InvalidTierDefinition` | Tier definition is invalid (e.g. zero contribution_amount or payout_weight). |
| 1083 | `InsufficientCreditScore` | Member's credit score is below the group's minimum threshold. |
| 1084 | `RoundDurationOutOfBounds` | Round duration is out of the configured bounds. |
| 1085 | `DelegationExpired` | Contribution delegation has passed its expiry ledger. |
| 1086 | `NotContribDelegate` | Caller is not the registered proxy for this member. |
| 1087 | `SplitProposalNotFound` | Split proposal not found. |
| 1088 | `SplitMembersInvalid` | Member list for split is invalid (overlap or missing members). |
| 1089 | `SplitConfirmationWindowClosed` | Split confirmation window has closed. |
| 1090 | `SourceGroupAlreadySplit` | Group has already been split. |
| 1091 | `SplitAlreadyConfirmed` | Member already confirmed split participation. |
| 1092 | `SplitNotFullyConfirmed` | Not all members have confirmed; cannot execute split yet. |

### Extended Errors 2

| Code | Name | Description |
|------|------|-------------|
| 1101 | `AuctionNotEnabled` | Slot auction feature is not enabled. |
| 1102 | `AuctionNotOpen` | No auction is currently open. |
| 1103 | `AuctionWindowClosed` | Auction bidding window has closed. |
| 1104 | `IncorrectContributionAmount` | Contribution amount does not match the required amount. |
| 1105 | `InvalidSlotIndex` | Slot index is out of range. |
| 1106 | `MigrationAlreadyExecuted` | Migration has already been executed. |
| 1107 | `MigrationAlreadyPending` | A migration request is already pending for this member. |
| 1108 | `MigrationNotApproved` | Migration has not been approved by the target group. |
| 1109 | `MigrationNotFound` | No migration request found for this member. |
| 1110 | `NoBidFound` | No bid found for the given criteria. |
| 1111 | `SlotOccupied` | Target slot is already occupied by another member. |
| 1112 | `TokenMismatch` | Token mismatch between source and target groups. |
| 1113 | `OutstandingLoanExists` | Member already has an outstanding emergency loan. |
| 1114 | `NoCopayersRegistered` | No co-payer splits registered for this member. |
| 1115 | `CopayerAmountsMismatch` | Co-payer split amounts do not sum to the required contribution amount. |
| 1116 | `ReceiptNotFound` | Contribution receipt not found for the given ID. |
| 1117 | `CopayerSplitsAlreadySet` | Member has already registered co-payer splits; revoke first. |
| 1118 | `ProxyRoundsExhausted` | Proxy has consumed all authorized rounds. |

---

## ahjoor-payments (2000 – 2299)

### Core Errors

| Code | Name | Description |
|------|------|-------------|
| 2001 | `RateLimitExceeded` | Merchant API rate limit exceeded. |
| 2002 | `SubscriptionPaused` | Subscription is currently paused. |
| 2003 | `OracleConditionNotMet` | Oracle price condition has not been satisfied. |
| 2004 | `SubscriptionInTrial` | Subscription's trial period has not elapsed; charging is deferred. |
| 2005 | `TokenNotAllowed` | Payment token is not on the allowed list. |
| 2006 | `DuplicateExternalId` | External ID has already been used for a payment. |
| 2007 | `MultisigNotRequired` | Multisig approval is not required for this payment. |
| 2008 | `AlreadyApproved` | Payment has already been approved by this signer. |
| 2009 | `NotASigner` | Caller is not a registered signer for this payment. |
| 2010 | `VoucherExpired` | Voucher has passed its expiration. |
| 2011 | `VoucherExhausted` | Voucher usage limit has been reached. |
| 2012 | `VoucherRevoked` | Voucher has been revoked by the issuer. |
| 2013 | `VoucherNotFound` | Voucher does not exist. |
| 2014 | `WithdrawalRateLimitExceeded` | Withdrawal rate limit exceeded. |
| 2015 | `ReferralAlreadyExists` | Referred merchant already has a merchant record. |
| 2016 | `NoCommissionToClaim` | No pending commission to claim. |
| 2017 | `DynamicPaymentExpired` | Dynamic payment has expired. |
| 2018 | `TippingNotEnabled` | Tip supplied on a payment that does not have tipping enabled. |
| 2019 | `TipExceedsMaxBps` | Tip amount exceeds the admin-configured maximum tip bps of the base amount. |
| 2020 | `MerchantVolumeCapped` | Merchant transaction volume has reached the configured cap. |
| 2021 | `SlippageExceeded` | Slippage tolerance exceeded on dynamic payment settlement. |
| 2022 | `OracleNotWhitelisted` | Oracle address is not on the admin whitelist. |
| 2023 | `CustomerSpendLimitExceeded` | Customer cumulative spend would exceed the merchant-configured cap. |
| 2024 | `CapturePastDeadline` | Capture attempted after the authorized capture deadline ledger. |
| 2025 | `EvidenceWindowClosed` | Evidence submission window has closed. |
| 2026 | `EvidenceLimitReached` | Evidence submission limit reached for this party. |
| 2027 | `CoolingOffExpired` | Cooling-off period has expired. |
| 2028 | `NotInCoolingOff` | Payment not in cooling-off status. |
| 2029 | `CoolingOffExceedsMax` | Cooling-off period exceeds maximum allowed. |
| 2030 | `PauseCountExceeded` | Subscription pause count exceeded. |
| 2031 | `UnauthorizedPause` | Unauthorized to pause subscription. |
| 2032 | `InsufficientMerchantReserve` | Merchant refund reserve is below the configured minimum. |
| 2033 | `KYBVerificationRequired` | KYB verification required but merchant not verified. |
| 2034 | `RetryNotDue` | `retry_failed_debit` called before back-off interval has elapsed. |
| 2035 | `DebitRecordNotFound` | Failed debit record not found. |
| 2036 | `DebitAlreadyAbandoned` | Debit record is already abandoned; no further retries. |
| 2037 | `DebitAlreadySucceeded` | Debit record already succeeded; no retry needed. |
| 2038 | `InvalidPaymentStatus` | Payment is not in a pending state and cannot be extended. |
| 2039 | `MaxExtensionsReached` | Maximum number of extensions reached for this payment. |
| 2040 | `MaxExtensionLedgersExceeded` | Additional ledgers exceed the maximum allowed per extension. |

### Extended Errors

| Code | Name | Description |
|------|------|-------------|
| 2050 | `CustomerBlocked` | Customer is blocked by merchant. |
| 2051 | `DaoNotConfigured` | DAO mediation has not been configured by admin. |
| 2052 | `NotADaoMember` | Caller is not a registered DAO mediator member. |
| 2053 | `DaoAlreadyEscalated` | Payment dispute has already been escalated to the DAO. |
| 2054 | `DaoVoteWindowOpen` | DAO vote window is still open; verdict cannot be executed yet. |
| 2055 | `DaoVoteWindowClosed` | DAO vote window has closed; no further votes accepted. |
| 2056 | `DaoAlreadyVoted` | This DAO member has already cast a vote on this case. |

---

## ahjoor-escrow (3000 – 3299)

### Core Errors

| Code | Name | Description |
|------|------|-------------|
| 3001 | `InvalidDeadline` | Deadline is invalid or in the past. |
| 3002 | `InvalidTrancheIndex` | Tranche index is out of range. |
| 3003 | `TrancheAlreadyClaimed` | This tranche has already been claimed. |
| 3004 | `AlreadyInitialized` | Escrow contract has already been initialized. |
| 3005 | `AtLeastOneBuyerIsRequired` | Escrow must have at least one buyer. |
| 3006 | `DeadlineMustBeFuture` | Deadline must be a future timestamp. |
| 3007 | `BuyerContributionMustBePositive` | Buyer contribution amount must be positive. |
| 3008 | `DuplicateBuyerInList` | Duplicate buyer address in the buyer list. |
| 3009 | `BatchMustContainAtLeastOneEscrowConfig` | Batch must contain at least one escrow configuration. |
| 3010 | `BatchSizeExceedsMaximum10Escrows` | Batch size exceeds the maximum of 10 escrows. |
| 3011 | `OnlySellerCanMarkComplete` | Only the seller can mark the escrow as complete. |
| 3012 | `EscrowIsNotActive` | Escrow is not in an active state. |
| 3013 | `NoInspectorSetUseReleaseEscrowDirectly` | No inspector set; use release escrow directly. |
| 3014 | `EscrowIsNotAwaitingInspection` | Escrow is not in awaiting inspection state. |
| 3015 | `OnlyAssignedInspectorCanSubmitReport` | Only the assigned inspector can submit a report. |
| 3016 | `OnlyBuyerOrSellerCanProposeInspectorReplacement` | Only buyer or seller can propose inspector replacement. |
| 3017 | `NoInspectorSetEscrow` | No inspector has been set for this escrow. |
| 3018 | `OnlyAdminCanSetInspectorScoreThreshold` | Only admin can set the inspector score threshold. |
| 3019 | `MinScoreBpsExceedsMaximum` | Minimum score BPS exceeds the maximum allowed. |
| 3020 | `OnlyAdminCanAppealInspectorRuling` | Only admin can appeal an inspector ruling. |
| 3021 | `InspectorRulingAlreadyAppealedEscrow` | Inspector ruling has already been appealed. |
| 3022 | `InspectorScoreBelowMinimumThresholdHighValueEscrow` | Inspector score is below the minimum threshold for high-value escrows. |
| 3023 | `EscrowAmountMustBePositive` | Escrow amount must be positive. |
| 3024 | `DeadlineMustBeAfterMinLockUntil` | Deadline must be after the minimum lock-until period. |
| 3025 | `ArbiterFeeExceedsMaximum1000Bps` | Arbiter fee exceeds maximum of 1000 bps (10%). |
| 3026 | `IncompleteReleaseCondition` | Release condition is incomplete or malformed. |
| 3027 | `ReleaseConditionThresholdMustBePositive` | Release condition threshold must be positive. |
| 3028 | `InvalidReleaseComparison` | Invalid comparison operator in release condition. |
| 3029 | `Maximum5SellersAllowed` | Maximum of 5 sellers allowed per escrow. |
| 3030 | `SellerAllocationsMustSumTo10000Bps` | Seller allocations must sum to exactly 10000 bps. |
| 3031 | `DisputeTimeoutSecondsMustBePositive` | Dispute timeout seconds must be positive. |
| 3032 | `OnlyCurrentHolderCanTransferReceipt` | Only the current receipt holder can transfer it. |
| 3033 | `ActiveMilestoneInProgress` | An active milestone is currently in progress. |
| 3034 | `InspectionPending` | Inspection is still pending. |
| 3035 | `OnlyListedBuyerCanApproveMultiBuyerRelease` | Only a listed buyer can approve multi-buyer release. |
| 3036 | `BuyerHasAlreadyApprovedRelease` | Buyer has already approved the release. |
| 3037 | `OnlyBuyerOrArbiterCanReleaseEscrow` | Only buyer or arbiter can release the escrow. |
| 3038 | `SellerVetoActive` | Seller veto is currently active. |
| 3039 | `SellerVetoActive2` | Seller veto is currently active (alternate check). |
| 3040 | `ConditionNotMet` | Release condition has not been met. |
| 3041 | `OnlyBuyerOrSellerCanWaiveCondition` | Only buyer or seller can waive a release condition. |
| 3042 | `NoConditionalReleaseSetEscrow` | No conditional release has been set for this escrow. |
| 3043 | `OnlyBuyerOrSellerCanSubmitEvidence` | Only buyer or seller can submit evidence. |
| 3044 | `MaximumEvidenceEntriesReachedParty` | Maximum evidence entries reached for this party. |
| 3045 | `OnlyBuyerCanSetRenewalAllowance` | Only the buyer can set the renewal allowance. |
| 3046 | `AutoRenewIsNotEnabledEscrow` | Auto-renew is not enabled for this escrow. |
| 3047 | `OnlyBuyerCanCancelAutoRenew` | Only the buyer can cancel auto-renew. |
| 3048 | `OnlyBuyerCanCancelAutoRenewal` | Only the buyer can cancel auto-renewal. |
| 3049 | `NoAutoRenewConfigSetEscrow` | No auto-renew configuration set for this escrow. |
| 3050 | `ReleaseAmountMustBePositive` | Release amount must be positive. |

### Extended Errors

| Code | Name | Description |
|------|------|-------------|
| 3051 | `ReleaseAmountExceedsEscrowBalance` | Release amount exceeds the escrow balance. |
| 3052 | `AtLeastOneMilestoneRequired` | Escrow requires at least one milestone. |
| 3053 | `TooManyMilestones` | Too many milestones defined. |
| 3054 | `MilestoneAmountMustBePositive` | Milestone amount must be positive. |
| 3055 | `NewMilestonesMustStartAsPending` | New milestones must start in pending state. |
| 3056 | `EscrowAlreadyTerminal` | Escrow is already in a terminal state. |
| 3057 | `OnlyBuyerOrArbiterCanApproveMilestones` | Only buyer or arbiter can approve milestones. |
| 3058 | `MilestoneIndexOutRange` | Milestone index is out of range. |
| 3059 | `MilestoneNotPending` | Milestone is not in pending state. |
| 3060 | `OnlyBuyerOrSellerCanDisputeEscrow` | Only buyer or seller can dispute the escrow. |
| 3061 | `DisputeAmountOutOfRange` | Dispute amount is out of valid range. |
| 3062 | `BuyerPercentMustBeBetween0And100` | Buyer percent must be between 0 and 100. |
| 3063 | `EscrowIsNotDisputed` | Escrow is not in disputed state. |
| 3064 | `OnlyArbiterCanResolveDispute` | Only the arbiter can resolve a dispute. |
| 3065 | `EscrowIsNotCoolingOffState` | Escrow is not in the cooling-off state. |
| 3066 | `OnlyBuyerOrSellerCanFlagResolutionError` | Only buyer or seller can flag a resolution error. |
| 3067 | `CoolingOffWindowHasExpired` | Cooling-off window has expired. |
| 3068 | `ResolutionAlreadyFlagged` | Resolution has already been flagged. |
| 3069 | `CoolingOffWindowHasNotElapsed` | Cooling-off window has not yet elapsed. |
| 3070 | `ResolutionIsFlaggedAdminMustReviewBeforeFinalization` | Resolution is flagged; admin must review before finalization. |
| 3071 | `OnlyAdminCanClearResolutionFlags` | Only admin can clear resolution flags. |
| 3072 | `NoFlagToClear` | No resolution flag to clear. |
| 3073 | `OnlyAdminCanConfigureCoolingOffPeriod` | Only admin can configure the cooling-off period. |
| 3074 | `FeeConfigurationExceedsEscrowAmount` | Fee configuration exceeds the escrow amount. |
| 3075 | `TimeoutMustBePositive` | Timeout must be a positive value. |
| 3076 | `MultiplierMustBePositive` | Multiplier must be a positive value. |
| 3077 | `DeadlineMustBePositive` | Deadline must be a positive value. |
| 3078 | `DisputeAlreadyResolved` | Dispute has already been resolved. |
| 3079 | `DisputeTimeoutDeadlineHasNotPassedYet` | Dispute timeout deadline has not yet passed. |
| 3080 | `MaxOracleAgeMustBePositive` | Max oracle age must be positive. |
| 3081 | `InsuranceTriggerDaysMustBePositive` | Insurance trigger days must be positive. |
| 3082 | `InsuranceContributionMustBePositive` | Insurance contribution must be positive. |
| 3083 | `OnlyBuyerOrSellerCanClaimInsurance` | Only buyer or seller can claim insurance. |
| 3084 | `InsuranceAlreadyClaimed` | Insurance has already been claimed for this escrow. |
| 3085 | `AdminConfirmationRequired` | Admin confirmation is required to proceed. |
| 3086 | `InsuranceTriggerPeriodNotReached` | Insurance trigger period has not yet been reached. |
| 3087 | `EscrowTokenNotCoveredByInsurancePool` | Escrow token is not covered by the insurance pool. |
| 3088 | `InsurancePoolHasInsufficientBalance` | Insurance pool has insufficient balance. |
| 3089 | `FeeExceedsMaximum200Bps` | Fee exceeds maximum of 200 bps (2%). |
| 3090 | `WithdrawalAmountMustBePositive` | Withdrawal amount must be positive. |
| 3091 | `InsufficientAccruedFees` | Insufficient accrued fees for withdrawal. |
| 3092 | `ReleaseConditionNotMet` | Release condition has not been met. |
| 3093 | `EscrowHasNotExpiredYet` | Escrow has not yet expired. |
| 3094 | `OnlyBuyerOrSellerCanProposeDeadlineExtension` | Only buyer or seller can propose a deadline extension. |
| 3095 | `CannotExtendDeadlineWhileEscrowIsDisputed` | Cannot extend deadline while escrow is disputed. |
| 3096 | `NewDeadlineMustBeGreaterThanCurrentDeadline` | New deadline must be greater than the current deadline. |
| 3097 | `OnlyBuyerOrSellerCanAcceptDeadlineExtension` | Only buyer or seller can accept a deadline extension. |
| 3098 | `ProposerCannotAcceptTheirOwnDeadlineExtension` | Proposer cannot accept their own deadline extension. |
| 3099 | `DeadlineExtensionProposalHasExpired` | Deadline extension proposal has expired. |
| 3100 | `OnlyBuyerOrSellerCanProposeAmendment` | Only buyer or seller can propose an amendment. |

### Extended Errors 2

| Code | Name | Description |
|------|------|-------------|
| 3101 | `CannotAmendTerminalEscrow` | Cannot amend a terminal escrow. |
| 3102 | `NewAmountMustBePositive` | New amendment amount must be positive. |
| 3103 | `OnlyBuyerOrSellerCanSignAmendment` | Only buyer or seller can sign an amendment. |
| 3104 | `AmendmentNonceMismatch` | Amendment nonce does not match the expected value. |
| 3105 | `AmendmentProposalHasExpired` | Amendment proposal has expired. |
| 3106 | `AmendmentRequiresBuyerAndSellerSignatures` | Amendment requires both buyer and seller signatures. |
| 3107 | `OnlyBuyerCanTopUpEscrow` | Only the buyer can top up the escrow. |
| 3108 | `EscrowIsNotActiveOrAwaitingInspection` | Escrow is not active or awaiting inspection. |
| 3109 | `AdditionalAmountMustBePositive` | Additional top-up amount must be positive. |
| 3110 | `TopUpLimitExceeded` | Top-up limit has been exceeded. |
| 3111 | `OnlySellerCanAcknowledgeTopUp` | Only the seller can acknowledge a top-up. |
| 3112 | `OnlySellerCanRequestPartialRelease` | Only the seller can request a partial release. |
| 3113 | `PartialReleaseOnlyAllowedActiveEscrow` | Partial release is only allowed on active escrows. |
| 3114 | `PartialReleaseAmountMustBePositive` | Partial release amount must be positive. |
| 3115 | `PartialReleaseAmountCannotExceedEscrowAmount` | Partial release amount cannot exceed the escrow amount. |
| 3116 | `RequestAlreadyPending` | A request is already pending. |
| 3117 | `OnlyBuyerCanApprovePartialRelease` | Only the buyer can approve a partial release. |
| 3118 | `InvalidRequestID` | Invalid request ID. |
| 3119 | `DelegateMustBeDifferentFromSeller` | Delegate must be a different address from the seller. |
| 3120 | `SellerNotPartOfEscrow` | Seller is not part of this escrow. |
| 3121 | `CanOnlyDelegateBeforeEscrowIsReleased` | Can only delegate before the escrow is released. |
| 3122 | `OnlyBuyerCanRejectPartialRelease` | Only the buyer can reject a partial release. |
| 3123 | `OnlySellerCanEscalatePartialRelease` | Only the seller can escalate a partial release. |
| 3124 | `ResponseDeadlineNotYetPassed` | Response deadline has not yet passed. |
| 3125 | `NewBuyerMustBeDifferentFromCurrentBuyer` | New buyer must be a different address from the current buyer. |
| 3126 | `BuyerTransferOnlyAllowedActiveEscrows` | Buyer transfer is only allowed on active escrows. |
| 3127 | `OnlyCurrentBuyerCanTransferBuyerRole` | Only the current buyer can transfer the buyer role. |
| 3128 | `OnlyBuyerOrSellerCanUpdateMetadata` | Only buyer or seller can update escrow metadata. |
| 3129 | `OnlyAdminCanUpgradeContract` | Only admin can upgrade the contract. |
| 3130 | `OnlyAdminCanMigrateContract` | Only admin can migrate the contract. |
| 3131 | `MigrationAlreadyCompletedVersion` | Migration to this version has already been completed. |
| 3132 | `UnlockAtMustBeFuture` | Timelock unlock time must be in the future. |
| 3133 | `AlreadyClaimed` | Timelocked funds have already been claimed. |
| 3134 | `OnlyBeneficiaryCanClaim` | Only the beneficiary can claim timelocked funds. |
| 3135 | `UnlockTimeHasNotPassed` | Timelock unlock time has not yet passed. |
| 3136 | `EscrowNotActive` | Escrow is not in an active state. |
| 3137 | `PastUnlockTimeUseClaimTimelocked` | Unlock time has passed; use claim_timelocked instead. |
| 3138 | `OnlyBuyerCanCancel` | Only the buyer can cancel the escrow. |
| 3139 | `DisputeActive` | A dispute is currently active. |
| 3140 | `OnlyAdminCanSetTokenWhitelistContract` | Only admin can set the token whitelist contract. |
| 3141 | `ContractAlreadyPaused` | Contract is already paused. |
| 3142 | `ContractIsNotPaused` | Contract is not currently paused. |
| 3143 | `TokenNotAllowed` | Token is not on the allowed list. |
| 3144 | `DeadlineDurationMustBePositive` | Deadline duration must be positive. |
| 3145 | `TemplateIsDeactivated` | Escrow template has been deactivated. |
| 3146 | `ArbiterAlreadyPool` | Arbiter is already configured as a pool. |
| 3147 | `ArbiterNotPool` | Arbiter is not configured as a pool. |
| 3148 | `ArbiterPoolIsEmpty` | Arbiter pool has no available arbiters. |
| 3149 | `OnlyTemplateCreatorCanUpdate` | Only the template creator can update the template. |
| 3150 | `OnlyTemplateCreatorCanDeactivate` | Only the template creator can deactivate the template. |

### Extended Errors 3

| Code | Name | Description |
|------|------|-------------|
| 3151 | `TemplateAlreadyDeactivated` | Template has already been deactivated. |
| 3152 | `InactivityReleaseIsNotEnabledEscrow` | Inactivity release is not enabled for this escrow. |
| 3153 | `OnlyEscrowSellerCanClaimInactivityRelease` | Only the escrow seller can claim inactivity release. |
| 3154 | `BuyerInactivityWindowHasNotElapsed` | Buyer inactivity window has not yet elapsed. |
| 3155 | `PenaltyCannotExceed10000Bps` | Penalty cannot exceed 10000 bps (100%). |
| 3156 | `ResponseWindowMustBePositive` | Response window must be positive. |
| 3157 | `OnlyBuyerOrSellerCanRequestCancellation` | Only buyer or seller can request cancellation. |
| 3158 | `NoPendingCancellationEscrow` | No pending cancellation request for this escrow. |
| 3159 | `InitiatorCannotAcceptTheirOwnCancellationRequest` | Initiator cannot accept their own cancellation request. |
| 3160 | `OnlyBuyerOrSellerCanAcceptCancellation` | Only buyer or seller can accept a cancellation request. |
| 3161 | `InitiatorCannotRejectTheirOwnCancellationRequest` | Initiator cannot reject their own cancellation request. |
| 3162 | `OnlyBuyerOrSellerCanRejectCancellation` | Only buyer or seller can reject a cancellation request. |
| 3163 | `ResponseWindowHasNotElapsed` | Response window has not yet elapsed. |
| 3164 | `BountyAmountMustBePositive` | Bounty amount must be positive. |
| 3165 | `ClaimDeadlineMustBeFuture` | Claim deadline must be in the future. |
| 3166 | `SubmissionDeadlineMustBeAfterClaimDeadline` | Submission deadline must be after the claim deadline. |
| 3167 | `TokenNotWhitelisted` | Token is not whitelisted for this bounty. |
| 3168 | `BountyIsNotAvailableClaiming` | Bounty is not available for claiming. |
| 3169 | `ClaimDeadlineHasPassed` | Claim deadline has passed. |
| 3170 | `BountyIsNotClaimedStatus` | Bounty is not in claimed status. |
| 3171 | `OnlyAssignedSolverCanSubmitWork` | Only the assigned solver can submit work. |
| 3172 | `SubmissionDeadlineHasPassed` | Submission deadline has passed. |
| 3173 | `OnlyBuyerCanApproveSubmission` | Only the buyer can approve a submission. |
| 3174 | `NoSubmissionHasBeenMade` | No submission has been made for this bounty. |
| 3175 | `OnlyBuyerCanRejectSubmission` | Only the buyer can reject a submission. |
| 3176 | `MaximumRejectionRoundsReached` | Maximum rejection rounds reached. |
| 3177 | `OnlyBuyerCanCancelBounty` | Only the buyer can cancel the bounty. |
| 3178 | `CannotCancelBountyCurrentState` | Cannot cancel bounty in its current state. |
| 3179 | `OnlyAdminCanSetMaxBountyRejectionRounds` | Only admin can set max bounty rejection rounds. |
| 3180 | `BountyMustHaveAtLeastOneMilestone` | Bounty must have at least one milestone. |
| 3181 | `BountyMustBeClaimedBeforeSubmittingMilestones` | Bounty must be claimed before submitting milestones. |
| 3182 | `OnlySolverCanSubmitMilestones` | Only the solver can submit milestones. |
| 3183 | `MilestoneIndexOutBounds` | Milestone index is out of bounds. |
| 3184 | `PreviousMilestoneNotYetVerified` | Previous milestone has not yet been verified. |
| 3185 | `MilestoneIsNotAwaitingSubmission` | Milestone is not awaiting submission. |
| 3186 | `MilestoneIsNotAwaitingVerification` | Milestone is not awaiting verification. |
| 3187 | `OnlyBountyCreatorCanReplaceVerifier` | Only the bounty creator can replace the verifier. |
| 3188 | `VerifierCanOnlyBeReplacedBeforeMilestoneIsSubmitted` | Verifier can only be replaced before milestone submission. |
| 3189 | `OnlyBuyerCanConfigureCollateralHealth` | Only the buyer can configure collateral health settings. |
| 3190 | `MinCollateralRatioBpsOutOfRange` | Minimum collateral ratio BPS is out of range. |
| 3191 | `TopUpAmountMustBePositive` | Top-up amount must be positive. |
| 3192 | `EscrowIsNotActiveOrUnderCollateralized` | Escrow is not active or under-collateralized. |
| 3193 | `OnlyBuyerCanConfigureMultiPartyApproval` | Only the buyer can configure multi-party approval. |
| 3194 | `ApproversCountMustBeBetween2And10` | Approvers count must be between 2 and 10. |
| 3195 | `ThresholdMustBeBetween1AndApproversCount` | Threshold must be between 1 and the approvers count. |
| 3196 | `CannotReconfigureApprovalsAlreadyProgress` | Cannot reconfigure approvals; escrow already in progress. |
| 3197 | `CallerIsNotAuthorizedApproverEscrow` | Caller is not an authorized approver for this escrow. |
| 3198 | `ApproverHasAlreadyApprovedEscrow` | Approver has already approved this escrow. |
| 3199 | `ReleaseScheduleMustContainAtLeastOneTranche` | Release schedule must contain at least one tranche. |
| 3200 | `EachTrancheAmountMustBePositive` | Each tranche amount must be positive. |

### Extended Errors 4

| Code | Name | Description |
|------|------|-------------|
| 3201 | `EachTrancheUnlockAtMustBeFuture` | Each tranche unlock time must be in the future. |
| 3202 | `OnlyBeneficiarySellerCanClaimScheduledReleases` | Only the beneficiary seller can claim scheduled releases. |
| 3203 | `EscrowIsNotClaimableState` | Escrow is not in a claimable state. |
| 3204 | `NoTranchesAreCurrentlyClaimable` | No tranches are currently claimable. |
| 3205 | `ContractIsPaused` | Contract is paused. |
| 3206 | `OnlyAdminCanPauseContract` | Only admin can pause the contract. |
| 3207 | `OnlyAdminCanResumeContract` | Only admin can resume the contract. |
| 3208 | `EscrowStillLocked` | Escrow is still locked. |
| 3209 | `OraclePriceIsStale` | Oracle price data is stale. |
| 3210 | `InvalidOraclePrice` | Oracle returned an invalid price. |
| 3211 | `InsufficientRenewalAllowance` | Insufficient renewal allowance for auto-renew. |
| 3212 | `EscrowRenewalDurationMustBePositive` | Escrow renewal duration must be positive. |
| 3213 | `OnlyAdminCanSetMaxTopUpBps` | Only admin can set max top-up BPS. |
| 3214 | `CollateralForfeitBpsCannotExceed10000` | Collateral forfeit BPS cannot exceed 10000. |
| 3215 | `AtLeastOneSellerRequired` | At least one seller is required. |
| 3216 | `OnlySellerCanDepositCollateral` | Only the seller can deposit collateral. |
| 3217 | `EscrowIsNotAwaitingCollateral` | Escrow is not awaiting collateral. |
| 3218 | `CollateralDepositWindowHasExpired` | Collateral deposit window has expired. |
| 3219 | `RatingMustBeBetween1And5` | Rating must be between 1 and 5. |
| 3220 | `RatingOnlyAllowedAfterEscrowIsReleasedOrResolved` | Rating is only allowed after escrow is released or resolved. |
| 3221 | `OnlyBuyerOrSellerCanSubmitRating` | Only buyer or seller can submit a rating. |
| 3222 | `RatingAlreadySubmittedEscrow` | Rating has already been submitted for this escrow. |
| 3223 | `OnlySellerCanSubmitDeliveryProof` | Only the seller can submit delivery proof. |
| 3224 | `ProofSubmissionLockedEscrowIsUnderDispute` | Proof submission is locked; escrow is under dispute. |
| 3225 | `InvalidDeliveryProof` | Delivery proof is invalid. |
| 3226 | `OnlyEscrowSellerCanRaiseVeto` | Only the escrow seller can raise a veto. |
| 3227 | `VetoCooldownActive` | Veto cooldown is currently active. |
| 3228 | `OnlyBuyerCanApprove` | Only the buyer can approve. |
| 3229 | `NoPendingSellerTransfer` | No pending seller transfer. |
| 3230 | `OnlyAdminCanSetVetoOverrideWindow` | Only admin can set the veto override window. |
| 3231 | `WindowSecondsMustBePositive` | Window seconds must be positive. |
| 3232 | `OnlyEscrowSellerCanCancelVeto` | Only the escrow seller can cancel a veto. |
| 3233 | `VetoWindowElapsed` | Veto window has elapsed. |
| 3234 | `OnlyAdminCanOverrideSellerVeto` | Only admin can override a seller veto. |
| 3235 | `ActiveDisputeExists` | An active dispute exists. |
| 3236 | `VetoWindowNotElapsed` | Veto window has not yet elapsed. |
| 3237 | `VetoWindowHasNotExpiredYet` | Veto window has not expired yet. |
| 3238 | `OnlyCurrentSellerCanInitiateTransfer` | Only the current seller can initiate a transfer. |
| 3239 | `EscrowMustBeActiveToTransferSellerRole` | Escrow must be active to transfer seller role. |
| 3240 | `OnlyBuyerCanVeto` | Only the buyer can veto. |
| 3241 | `OnlyAdminCanSetVetoWindow` | Only admin can set the veto window. |
| 3242 | `ReleaseBpsMustBePositiveEachMilestone` | Release BPS must be positive for each milestone. |
| 3243 | `MilestoneReleaseBpsMustSumTo10000` | Milestone release BPS must sum to 10000. |
| 3244 | `OnlyEscrowSellerMaySubmitMilestones` | Only the escrow seller may submit milestones. |
| 3245 | `MilestoneMustBePendingOrRejectedToSubmit` | Milestone must be pending or rejected to submit. |
| 3246 | `MilestoneMustBeSubmittedBeforeApproval` | Milestone must be submitted before approval. |
| 3247 | `OnlyEscrowBuyerMayRejectMilestones` | Only the escrow buyer may reject milestones. |
| 3248 | `OnlySubmittedMilestoneCanBeRejected` | Only submitted milestones can be rejected. |
| 3249 | `OnlyLosingPartyCanFlagResolutionError` | Caller is the winning party and may not flag a resolution error. |

---

## ahjoor-refund (4000 – 4099)

> **Note:** The refund contract uses `panic!()` string messages on-chain rather
> than a `#[contracterror]` enum. The codes below are the off-chain namespace
> assignments defined in `ahjoor-errors` for future migration.

| Code | Name | Description |
|------|------|-------------|
| 4001 | `AlreadyInitialized` | Contract has already been initialized. |
| 4002 | `FeeExceedsMaximum` | Fee exceeds the maximum allowed. |
| 4003 | `AmountMustBePositive` | Refund amount must be positive. |
| 4004 | `InvalidReasonCode` | Invalid refund reason code. |
| 4005 | `RefundCooldownActive` | Refund cooldown period is still active. |
| 4006 | `PaymentNotFound` | Payment not found for this refund request. |
| 4007 | `PaymentNotCompleted` | Payment has not been completed yet. |
| 4008 | `ExceedsRefundableAmount` | Refund amount exceeds the refundable balance. |

---

## ahjoor-token-whitelist (5000 – 5099)

| Code | Name | Description |
|------|------|-------------|
| 5001 | `NotInitialized` | Contract has not been initialized yet. |
| 5002 | `AlreadyInitialized` | Contract has already been initialized. |
| 5003 | `Unauthorized` | Caller is not authorized to perform this action. |
| 5004 | `TokenAlreadyWhitelisted` | Token is already on the whitelist. |
| 5005 | `TokenNotWhitelisted` | Token is not on the whitelist. |
| 5006 | `QuotaExceeded` | Token quota has been exceeded. |
| 5007 | `TokenAlreadyHasQuota` | Token already has a quota configured. |
| 5008 | `TokenHasNoQuota` | Token does not have a quota configured. |
| 5009 | `RiskTierNotDefined` | Risk tier is not defined for this token. |
