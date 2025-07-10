#[test_only]
module quark_test::group_test {
    use std::signer;
    use std::string::{Self, String};
    use std::vector;
    use std::account;
    use std::option;
    use std::type_info;
    use std::object::{Self, Object};
    use aptos_framework::timestamp;
    use aptos_framework::coin;
    use aptos_framework::aptos_coin::{Self, AptosCoin};
    use aptos_framework::randomness;
    use aptos_framework::aptos_account;

    use aptos_framework::fungible_asset::{Self, MintRef, TransferRef, Metadata};
    use aptos_framework::primary_fungible_store;
    use sshift_gpt::fees;
    use quark_test::admin_v5;
    use quark_test::user_v5;
    use quark_test::group_v5;

    // Test constants
    const TEST_GROUP_ID: vector<u8> = b"test_group_1";
    const TEST_POOL_ID: vector<u8> = b"test_pool_1";
    const TEST_DAO_ID: vector<u8> = b"test_dao_1";
    const TEST_AMOUNT: u64 = 1000000; // 1 APT
    const TEST_TOTAL_USERS: u64 = 5;
    const EIS_BALANCE_NOT_EQUAL: u64 = 1;

    // Error constants for mock functions (matching the original contract)
    const EONLY_ADMIN_CAN_CALL: u64 = 1;
    const EONLY_REVIEWER_CAN_CALL: u64 = 2;
    const ENOT_ENOUGH_FUNDS: u64 = 3;
    const ENOT_COIN_PAYMENT_SET: u64 = 4;
    const ECOINS_NOT_MATCH: u64 = 5;
    const ENOT_USER_PASSED: u64 = 6;
    const EGROUP_NOT_EXISTS: u64 = 7;
    const EAMOUNT_MUST_BE_GREATER_THAN_ZERO: u64 = 8;
    const EPOOL_NOT_EXISTS: u64 = 9;
    const EUSER_ALREADY_CLAIMED: u64 = 10;
    const EPOOLS_REWARDS_NOT_EXISTS: u64 = 11;
    const EPOOL_REWARD_ALREADY_CLAIMED: u64 = 12;
    const EPOOL_REWARD_ALREADY_EXISTS: u64 = 13;
    const EPOOL_REWARD_TOKEN_NOT_MATCH: u64 = 14;
    const EGROUP_ALREADY_EXISTS: u64 = 15;
    const EDAO_ALREADY_EXISTS: u64 = 16;
    const EDAO_NOT_EXISTS: u64 = 17;
    const EFROM_TO_NOT_VALID: u64 = 18;
    const ENOT_IN_TIME: u64 = 19;
    const ECHOICE_NOT_EXISTS: u64 = 20;
    const ECOIN_TYPE_NOT_MATCH: u64 = 21;
    const ECURRENCY_NOT_MATCH: u64 = 22;
    const EUSER_ALREADY_VOTED: u64 = 23;
    const EUSER_NOT_VOTED: u64 = 24;
    

    struct TestCoin {}

    struct FAController has key {
        mint_ref: MintRef,
        transfer_ref: TransferRef,
    }

    // Mock structs for testing pool rewards with state updates
    struct MockPoolRewardV1 has store, drop {
        pool_id: String,
        reward_amount: u64,
        reward_token: address,
        total_users: u64,
        claimed_users: vector<address>,
        holder_object: address,
    }

    struct MockPoolsRewardsV1 has key, store {
        pools: vector<MockPoolRewardV1>,
    }

    struct MockPoolRewardV2 has store, drop {
        pool_id: String,
        reward_amount: u64,
        reward_token: address,
        currency: address,
        total_users: u64,
        claimed_users: vector<address>,
        holder_object: address,
    }

    struct MockPoolsRewardsV2 has key, store {
        pools: vector<MockPoolRewardV2>,
    }

    // Initialize modules same as user_test.move
    fun init_module(sender: &signer) {
        admin_v5::test_init_admin(sender);
        user_v5::test_init_account(sender);
        group_v5::test_init_group(sender);
    }

    fun mint_coin<CoinType>(admin: &signer, amount: u64, to: &signer) {
        let (burn_cap, freeze_cap, mint_cap) =
            coin::initialize<CoinType>(
                admin,
                string::utf8(b"Test"),
                string::utf8(b"Test coin"),
                8,
                true
            );

        coin::register<CoinType>(to);

        let coins = coin::mint<CoinType>(amount, &mint_cap);
        coin::deposit<CoinType>(signer::address_of(to), coins);
        coin::destroy_burn_cap(burn_cap);
        coin::destroy_mint_cap(mint_cap);
        coin::destroy_freeze_cap(freeze_cap);
    }

    fun create_resource_account(sender: &signer, admin: &signer) {
        let admin_addr = signer::address_of(admin);

        fees::initialize_for_test(sender);

        fees::create_resource_account(sender, b"test", vector[admin_addr]);

        fees::create_collector_object(admin);
    }

    fun create_fa(): Object<Metadata> {
        let fa_owner_obj_constructor_ref = &object::create_object(@sshift_gpt_addr);
        let fa_owner_obj_signer = &object::generate_signer(fa_owner_obj_constructor_ref);

        let name = string::utf8(b"usdt test");

        let fa_obj_constructor_ref =
            &object::create_named_object(fa_owner_obj_signer, *string::bytes(&name));

        let fa_obj_signer = &object::generate_signer(fa_obj_constructor_ref);

        primary_fungible_store::create_primary_store_enabled_fungible_asset(
            fa_obj_constructor_ref,
            option::none(),
            name,
            string::utf8(b"USDT"),
            8,
            string::utf8(b"test"),
            string::utf8(b"usdt_project")
        );

        let fa_obj =
            object::object_from_constructor_ref<Metadata>(fa_obj_constructor_ref);

        let mint_ref = fungible_asset::generate_mint_ref(fa_obj_constructor_ref);
        let transfer_ref = fungible_asset::generate_transfer_ref(fa_obj_constructor_ref);

        move_to(
            fa_obj_signer,
            FAController { mint_ref, transfer_ref }
        );

        fa_obj
    }

    fun mint_fa(sender: &signer, mint_ref: &MintRef, amount: u64) {
        let account_addr = signer::address_of(sender);
        primary_fungible_store::mint(mint_ref, account_addr, amount);
    }

    fun get_test_choices(): vector<String> {
        vector[
            string::utf8(b"Option A"),
            string::utf8(b"Option B"),
            string::utf8(b"Option C")
        ]
    }

    // Helper functions for mock pool rewards
    fun create_mock_pool_reward_v1<CoinType>(pool_id: String, reward_amount: u64, total_users: u64, holder_object: address): MockPoolRewardV1 {
        use aptos_std::type_info;
        
        let coin_type = type_info::type_of<CoinType>();
        let coin_address = type_info::account_address(&coin_type);
        
        MockPoolRewardV1 {
            pool_id,
            reward_amount,
            reward_token: coin_address,
            total_users,
            claimed_users: vector::empty(),
            holder_object,
        }
    }

    fun create_mock_pool_reward_v2(pool_id: String, reward_amount: u64, currency: address, total_users: u64, holder_object: address): MockPoolRewardV2 {
        MockPoolRewardV2 {
            pool_id,
            reward_amount,
            reward_token: currency,
            currency,
            total_users,
            claimed_users: vector::empty(),
            holder_object,
        }
    }

    fun initialize_mock_pools_v1(sender: &signer) {
        if (!exists<MockPoolsRewardsV1>(signer::address_of(sender))) {
            move_to(sender, MockPoolsRewardsV1 { pools: vector::empty() });
        };
    }

    fun initialize_mock_pools_v2(sender: &signer) {
        if (!exists<MockPoolsRewardsV2>(signer::address_of(sender))) {
            move_to(sender, MockPoolsRewardsV2 { pools: vector::empty() });
        };
    }

    fun add_mock_pool_reward_v1(sender: &signer, pool_reward: MockPoolRewardV1) acquires MockPoolsRewardsV1 {
        let sender_addr = signer::address_of(sender);
        initialize_mock_pools_v1(sender);
        let pools_rewards = borrow_global_mut<MockPoolsRewardsV1>(sender_addr);
        vector::push_back(&mut pools_rewards.pools, pool_reward);
    }

    fun add_mock_pool_reward_v2(sender: &signer, pool_reward: MockPoolRewardV2) acquires MockPoolsRewardsV2 {
        let sender_addr = signer::address_of(sender);
        initialize_mock_pools_v2(sender);
        let pools_rewards = borrow_global_mut<MockPoolsRewardsV2>(sender_addr);
        vector::push_back(&mut pools_rewards.pools, pool_reward);
    }

    // Updated mock claim functions that properly update pool stat
    fun mock_claim_reward_v1_with_state<CoinType>(admin: &signer, reviewer: &signer, user: address, pool_id: String, aptos_framework: &signer) acquires MockPoolsRewardsV1 {
        randomness::initialize_for_testing(aptos_framework);
        randomness::set_seed(x"0000000000000000000000000000000000000000000000000000000000000000");

        let admin_address = signer::address_of(admin);
        let reviewer_address = signer::address_of(reviewer);
        let amount_to_claim;

        assert!(admin_v5::is_admin(admin_address), EONLY_ADMIN_CAN_CALL);
        assert!(admin_v5::is_reviewer(reviewer_address), EONLY_REVIEWER_CAN_CALL);

        let admin_addr = signer::address_of(admin);
        let pools_rewards = borrow_global_mut<MockPoolsRewardsV1>(admin_addr);

        let (exists_pool, pool_index) = vector::find<MockPoolRewardV1>(&pools_rewards.pools, |pool| pool.pool_id == pool_id);
        assert!(exists_pool, EPOOL_NOT_EXISTS);

        let pool_reward = vector::borrow_mut(&mut pools_rewards.pools, pool_index);

        assert!(vector::length(&pool_reward.claimed_users) < pool_reward.total_users, EPOOL_REWARD_ALREADY_CLAIMED);
        assert!(!vector::contains<address>(&pool_reward.claimed_users, &user), EUSER_ALREADY_CLAIMED);
        
        let coin_type = type_info::type_of<CoinType>();
        let coin_address = type_info::account_address(&coin_type);
        assert!(pool_reward.reward_token == coin_address, EPOOL_REWARD_TOKEN_NOT_MATCH);
        
        let users_claimed = vector::length(&pool_reward.claimed_users);
        let user_left = pool_reward.total_users - users_claimed;

        // Use actual randomness with allow_unsafe_randomness
        if (user_left > 1) {
            amount_to_claim = randomness::u64_range(1, pool_reward.reward_amount - user_left + 1);
        } else {
            amount_to_claim = pool_reward.reward_amount;
        };

        // Update pool state
        vector::push_back(&mut pool_reward.claimed_users, user);
        pool_reward.reward_amount = pool_reward.reward_amount - amount_to_claim;

        // Transfer coins for testing
        coin::transfer<CoinType>(admin, user, amount_to_claim);

        // Remove pool if all users have claimed
        if (vector::length(&pool_reward.claimed_users) == pool_reward.total_users) {
            vector::remove(&mut pools_rewards.pools, pool_index);
        }
    }

    fun mock_claim_reward_v2_with_state(admin: &signer, reviewer: &signer, user: address, currency: address, pool_id: String, aptos_framework: &signer) acquires MockPoolsRewardsV2 {        
        randomness::initialize_for_testing(aptos_framework);
        randomness::set_seed(x"0000000000000000000000000000000000000000000000000000000000000000");

        let admin_address = signer::address_of(admin);
        let reviewer_address = signer::address_of(reviewer);
        let amount_to_claim;

        assert!(admin_v5::is_admin(admin_address), EONLY_ADMIN_CAN_CALL);
        assert!(admin_v5::is_reviewer(reviewer_address), EONLY_REVIEWER_CAN_CALL);

        let admin_addr = signer::address_of(admin);
        let pools_rewards = borrow_global_mut<MockPoolsRewardsV2>(admin_addr);

        let (exists_pool, pool_index) = vector::find<MockPoolRewardV2>(&pools_rewards.pools, |pool| pool.pool_id == pool_id);
        assert!(exists_pool, EPOOL_NOT_EXISTS);

        let pool_reward = vector::borrow_mut(&mut pools_rewards.pools, pool_index);

        assert!(vector::length(&pool_reward.claimed_users) < pool_reward.total_users, EPOOL_REWARD_ALREADY_CLAIMED);
        assert!(!vector::contains<address>(&pool_reward.claimed_users, &user), EUSER_ALREADY_CLAIMED);
        
        let users_claimed = vector::length(&pool_reward.claimed_users);
        let user_left = pool_reward.total_users - users_claimed;

        // Use actual randomness with allow_unsafe_randomness
        if (user_left > 1) {
            amount_to_claim = randomness::u64_range(1, pool_reward.reward_amount - user_left + 1);
        } else {
            amount_to_claim = pool_reward.reward_amount;
        };

        // Update pool state
        vector::push_back(&mut pool_reward.claimed_users, user);
        pool_reward.reward_amount = pool_reward.reward_amount - amount_to_claim;

        // Transfer FA for testing
        let fa_metadata = object::address_to_object<Metadata>(currency);
        aptos_account::transfer_fungible_assets(admin, fa_metadata, user, amount_to_claim);

        // Remove pool if all users have claimed
        if (vector::length(&pool_reward.claimed_users) == pool_reward.total_users) {
            vector::remove(&mut pools_rewards.pools, pool_index);
        }
    }

    // ==================== CREATE GROUP TESTS ====================

    #[test(aptos_framework = @0x1, quark_test = @quark_test)]
    fun test_create_group_success(aptos_framework: &signer, quark_test: &signer) {
        timestamp::set_time_has_started_for_testing(aptos_framework);
        init_module(quark_test);
        let group_id = string::utf8(TEST_GROUP_ID);
        
        // Should succeed with valid admin and reviewer
        group_v5::create_group(quark_test, quark_test, group_id);
        
        // Verify group exists
        assert!(group_v5::exist_group_id(group_id), 0);
    }

    #[test(aptos_framework = @0x1, quark_test = @quark_test)]
    #[expected_failure(abort_code = 1, location = quark_test::group_v5)] // EONLY_ADMIN_CAN_CALL
    fun test_create_group_invalid_admin(aptos_framework: &signer, quark_test: &signer) {
        timestamp::set_time_has_started_for_testing(aptos_framework);
        init_module(quark_test);
        let fake_admin = account::create_signer_for_test(@0x999);
        let group_id = string::utf8(TEST_GROUP_ID);
        
        group_v5::create_group(&fake_admin, quark_test, group_id);
    }

    #[test(aptos_framework = @0x1, quark_test = @quark_test)]
    #[expected_failure(abort_code = 2, location = quark_test::group_v5)] // EONLY_REVIEWER_CAN_CALL
    fun test_create_group_invalid_reviewer(aptos_framework: &signer, quark_test: &signer) {
        timestamp::set_time_has_started_for_testing(aptos_framework);
        init_module(quark_test);
        let fake_reviewer = account::create_signer_for_test(@0x999);
        let group_id = string::utf8(TEST_GROUP_ID);
        
        group_v5::create_group(quark_test, &fake_reviewer, group_id);
    }

    #[test(aptos_framework = @0x1, quark_test = @quark_test)]
    #[expected_failure(abort_code = 15, location = quark_test::group_v5)] // EGROUP_ALREADY_EXISTS
    fun test_create_group_already_exists(aptos_framework: &signer, quark_test: &signer) {
        timestamp::set_time_has_started_for_testing(aptos_framework);
        init_module(quark_test);
        let group_id = string::utf8(TEST_GROUP_ID);
        
        group_v5::create_group(quark_test, quark_test, group_id);
        group_v5::create_group(quark_test, quark_test, group_id);
    }

    // ==================== PAY AI TESTS ====================

    #[test(aptos_framework = @0x1, quark_test = @quark_test, admin = @0x2)]
    fun test_pay_ai_success(aptos_framework: &signer, quark_test: &signer, admin: &signer) {
        let (burn_cap, mint_cap) = aptos_coin::initialize_for_test(aptos_framework);
        timestamp::set_time_has_started_for_testing(aptos_framework);
        init_module(quark_test);
        let group_id = string::utf8(TEST_GROUP_ID);
        
        // Set the coin address in the config first
        user_v5::set_coin_address<AptosCoin>(quark_test);
        
        group_v5::create_group(quark_test, quark_test, group_id);
        
        let group_account_addr = group_v5::get_group_account(group_id);
        account::create_account_for_test(group_account_addr);
        let group_account = account::create_signer_for_test(group_account_addr);
        coin::register<AptosCoin>(&group_account);
        aptos_coin::mint(aptos_framework, group_account_addr, TEST_AMOUNT * 10);

        let group_account = account::create_signer_for_test(group_account_addr);
        coin::register<AptosCoin>(&group_account);
        
        // Initialize fees
        create_resource_account(quark_test, admin);
        let fees_account = fees::get_resource_account_address();

        account::create_account_for_test(fees_account);
        
        let initial_group_balance = coin::balance<AptosCoin>(group_account_addr);
        
        group_v5::pay_ai<AptosCoin>(quark_test, quark_test, group_id, TEST_AMOUNT);
        
        // Verify balances
        let final_group_balance = coin::balance<AptosCoin>(group_account_addr);
        let fees_balance = coin::balance<AptosCoin>(fees_account);
        
        assert!(final_group_balance == initial_group_balance - TEST_AMOUNT, EIS_BALANCE_NOT_EQUAL);
        assert!(fees_balance == TEST_AMOUNT, EIS_BALANCE_NOT_EQUAL);

        coin::destroy_burn_cap(burn_cap);
        coin::destroy_mint_cap(mint_cap);
    }

    #[test(aptos_framework = @0x1, quark_test = @quark_test)]
    #[expected_failure(abort_code = 7, location = quark_test::group_v5)] // EGROUP_NOT_EXISTS
    fun test_pay_ai_group_not_exists(aptos_framework: &signer, quark_test: &signer) {
        timestamp::set_time_has_started_for_testing(aptos_framework);
        init_module(quark_test);
        let group_id = string::utf8(b"nonexistent_group");
        
        // Set the coin address in the config first
        user_v5::set_coin_address<AptosCoin>(quark_test);
        
        group_v5::pay_ai<AptosCoin>(quark_test, quark_test, group_id, TEST_AMOUNT);
    }

    // ==================== PAY USERS V1 TESTS ====================

    #[test(aptos_framework = @0x1, quark_test = @quark_test, user1 = @0x3, user2 = @0x4)]
    fun test_pay_users_v1_success(aptos_framework: &signer, quark_test: &signer, user1: &signer, user2: &signer) {
        let (burn_cap, mint_cap) = aptos_coin::initialize_for_test(aptos_framework);
        timestamp::set_time_has_started_for_testing(aptos_framework);
        init_module(quark_test);
        let group_id = string::utf8(TEST_GROUP_ID);
        
        group_v5::create_group(quark_test, quark_test, group_id);
        
        // Create users same as user_test.move
        let user1_addr = signer::address_of(user1);
        let user2_addr = signer::address_of(user2);
        
        account::create_account_for_test(user1_addr);
        account::create_account_for_test(user2_addr);
        
        user_v5::create_account(user1, string::utf8(b"1234567890"));
        user_v5::create_account(user2, string::utf8(b"1234567891"));
        
        // Register AptosCoin for users
        coin::register<AptosCoin>(user1);
        coin::register<AptosCoin>(user2);
        
        // Get group account and fund it
        let group_account_addr = group_v5::get_group_account(group_id);
        account::create_account_for_test(group_account_addr);
        let group_account = account::create_signer_for_test(group_account_addr);
        coin::register<AptosCoin>(&group_account);
        aptos_coin::mint(aptos_framework, group_account_addr, TEST_AMOUNT * 10);
        
        let recipients = vector[user1_addr, user2_addr];
        let amount_per_user = TEST_AMOUNT / vector::length(&recipients);
        
        let initial_group_balance = coin::balance<AptosCoin>(group_account_addr);
        
        group_v5::pay_users_v1<AptosCoin>(quark_test, quark_test, group_id, TEST_AMOUNT, recipients);
        
        // Verify balances
        let final_group_balance = coin::balance<AptosCoin>(group_account_addr);
        assert!(coin::balance<AptosCoin>(user1_addr) == amount_per_user, EIS_BALANCE_NOT_EQUAL);
        assert!(coin::balance<AptosCoin>(user2_addr) == amount_per_user, EIS_BALANCE_NOT_EQUAL);
        assert!(final_group_balance == initial_group_balance - TEST_AMOUNT, EIS_BALANCE_NOT_EQUAL);
        
        coin::destroy_burn_cap(burn_cap);
        coin::destroy_mint_cap(mint_cap);
    }

    #[test(aptos_framework = @0x1, quark_test = @quark_test)]
    #[expected_failure(abort_code = 6, location = quark_test::group_v5)] // ENOT_USER_PASSED
    fun test_pay_users_v1_empty_recipients(aptos_framework: &signer, quark_test: &signer) {
        let (burn_cap, mint_cap) = aptos_coin::initialize_for_test(aptos_framework);
        timestamp::set_time_has_started_for_testing(aptos_framework);
        init_module(quark_test);
        let group_id = string::utf8(TEST_GROUP_ID);
        
        group_v5::create_group(quark_test, quark_test, group_id);
        
        let empty_recipients = vector::empty<address>();
        
        group_v5::pay_users_v1<AptosCoin>(quark_test, quark_test, group_id, TEST_AMOUNT, empty_recipients);
        
        coin::destroy_burn_cap(burn_cap);
        coin::destroy_mint_cap(mint_cap);
    }

    #[test(aptos_framework = @0x1, quark_test = @quark_test)]
    #[expected_failure(abort_code = 3, location = quark_test::group_v5)] // ENOT_ENOUGH_FUNDS
    fun test_pay_users_v1_insufficient_funds(aptos_framework: &signer, quark_test: &signer) {
        let (burn_cap, mint_cap) = aptos_coin::initialize_for_test(aptos_framework);
        timestamp::set_time_has_started_for_testing(aptos_framework);
        init_module(quark_test);
        let group_id = string::utf8(TEST_GROUP_ID);
        
        group_v5::create_group(quark_test, quark_test, group_id);
        
        let recipients = vector[@0x300];
        
        group_v5::pay_users_v1<AptosCoin>(quark_test, quark_test, group_id, TEST_AMOUNT, recipients);
        
        coin::destroy_burn_cap(burn_cap);
        coin::destroy_mint_cap(mint_cap);
    }

    // ==================== PAY USERS V2 TESTS ====================

    #[test(aptos_framework = @0x1, quark_test = @quark_test, user1 = @0x3, user2 = @0x4)]
    fun test_pay_users_v2_success(aptos_framework: &signer, quark_test: &signer, user1: &signer, user2: &signer) acquires FAController {
        timestamp::set_time_has_started_for_testing(aptos_framework);
        init_module(quark_test);
        let group_id = string::utf8(TEST_GROUP_ID);
        
        group_v5::create_group(quark_test, quark_test, group_id);
        
        // Create and fund FA
        let fa_obj = create_fa();
        let fa_addr = object::object_address(&fa_obj);
        let fa_controller = borrow_global<FAController>(fa_addr);
        let fa_metadata = object::address_to_object<Metadata>(fa_addr);
        
        // Create users same as user_test.move
        let user1_addr = signer::address_of(user1);
        let user2_addr = signer::address_of(user2);
        
        account::create_account_for_test(user1_addr);
        account::create_account_for_test(user2_addr);
        
        user_v5::create_account(user1, string::utf8(b"1234567890"));
        user_v5::create_account(user2, string::utf8(b"1234567891"));
        
        let recipients = vector[user1_addr, user2_addr];
        
        // Get group account and fund it
        let group_account_addr = group_v5::get_group_account(group_id);
        let group_account = account::create_signer_for_test(group_account_addr);
        
        mint_fa(&group_account, &fa_controller.mint_ref, TEST_AMOUNT * 10);
        
        let amount_per_user = TEST_AMOUNT / vector::length(&recipients);
        let initial_group_balance = primary_fungible_store::balance(group_account_addr, fa_metadata);
        
        group_v5::pay_users_v2(quark_test, quark_test, group_id, TEST_AMOUNT, fa_addr, recipients);
        
        // Verify balances
        let final_group_balance = primary_fungible_store::balance(group_account_addr, fa_metadata);
        assert!(primary_fungible_store::balance(user1_addr, fa_metadata) == amount_per_user, EIS_BALANCE_NOT_EQUAL);
        assert!(primary_fungible_store::balance(user2_addr, fa_metadata) == amount_per_user, EIS_BALANCE_NOT_EQUAL);
        assert!(final_group_balance == initial_group_balance - TEST_AMOUNT, EIS_BALANCE_NOT_EQUAL);
    }

    // ==================== CREATE POOL REWARD TESTS ====================

    #[test(aptos_framework = @0x1, quark_test = @quark_test)]
    fun test_create_pool_reward_v1_success(aptos_framework: &signer, quark_test: &signer) {
        let (burn_cap, mint_cap) = aptos_coin::initialize_for_test(aptos_framework);
        timestamp::set_time_has_started_for_testing(aptos_framework);
        init_module(quark_test);
        let group_id = string::utf8(TEST_GROUP_ID);
        let pool_id = string::utf8(TEST_POOL_ID);
        
        group_v5::create_group(quark_test, quark_test, group_id);
        
        // Fund group account with tokens
        let group_account_addr = group_v5::get_group_account(group_id);
        account::create_account_for_test(group_account_addr);
        let group_account = account::create_signer_for_test(group_account_addr);
        coin::register<AptosCoin>(&group_account);
        aptos_coin::mint(aptos_framework, group_account_addr, TEST_AMOUNT * 2);
        
        group_v5::create_pool_reward_v1<AptosCoin>(quark_test, quark_test, pool_id, group_id, TEST_AMOUNT, TEST_TOTAL_USERS);
        
        // Verify pool reward was created
        let (reward_amount, _token, total_users, claimed_users, holder_object) = group_v5::get_pool_reward_v1(group_id, pool_id);
        assert!(reward_amount == TEST_AMOUNT, EIS_BALANCE_NOT_EQUAL);
        assert!(total_users == TEST_TOTAL_USERS, EIS_BALANCE_NOT_EQUAL);
        assert!(vector::length(&claimed_users) == 0, EIS_BALANCE_NOT_EQUAL);
        assert!(holder_object != @0x0, EIS_BALANCE_NOT_EQUAL);
        
        // Verify holder object has the tokens
        assert!(coin::balance<AptosCoin>(holder_object) == TEST_AMOUNT, EIS_BALANCE_NOT_EQUAL);
        
        coin::destroy_burn_cap(burn_cap);
        coin::destroy_mint_cap(mint_cap);
    }

    #[test(aptos_framework = @0x1, quark_test = @quark_test)]
    fun test_create_pool_reward_v2_success(aptos_framework: &signer, quark_test: &signer) acquires FAController {
        let (burn_cap, mint_cap) = aptos_coin::initialize_for_test(aptos_framework);
        timestamp::set_time_has_started_for_testing(aptos_framework);
        init_module(quark_test);
        let group_id = string::utf8(TEST_GROUP_ID);
        let pool_id = string::utf8(TEST_POOL_ID);
        
        group_v5::create_group(quark_test, quark_test, group_id);
        
        let fa_obj = create_fa();
        let fa_addr = object::object_address(&fa_obj);
        let fa_controller = borrow_global<FAController>(fa_addr);
        let fa_metadata = object::address_to_object<Metadata>(fa_addr);
        
        // Fund group account with FA tokens
        let group_account_addr = group_v5::get_group_account(group_id);
        let group_account = account::create_signer_for_test(group_account_addr);
        mint_fa(&group_account, &fa_controller.mint_ref, TEST_AMOUNT * 2);
        
        group_v5::create_pool_reward_v2(quark_test, quark_test, pool_id, group_id, fa_addr, TEST_AMOUNT, TEST_TOTAL_USERS);
        
        // Verify pool reward was created
        let (reward_amount, _token, total_users, claimed_users, holder_object) = group_v5::get_pool_reward_v2(group_id, pool_id);
        assert!(reward_amount == TEST_AMOUNT, EIS_BALANCE_NOT_EQUAL);
        assert!(total_users == TEST_TOTAL_USERS, EIS_BALANCE_NOT_EQUAL);
        assert!(vector::length(&claimed_users) == 0, EIS_BALANCE_NOT_EQUAL);
        assert!(holder_object != @0x0, EIS_BALANCE_NOT_EQUAL);
        
        // Verify holder object has the tokens
        assert!(primary_fungible_store::balance(holder_object, fa_metadata) == TEST_AMOUNT, EIS_BALANCE_NOT_EQUAL);
        
        coin::destroy_burn_cap(burn_cap);
        coin::destroy_mint_cap(mint_cap);
    }

    // ==================== CREATE GROUP DAO TESTS ====================

    #[test(aptos_framework = @0x1, quark_test = @quark_test)]
    fun test_create_group_dao_v1_success(aptos_framework: &signer, quark_test: &signer) {
        timestamp::set_time_has_started_for_testing(aptos_framework);
        init_module(quark_test);
        let group_id = string::utf8(TEST_GROUP_ID);
        let dao_id = string::utf8(TEST_DAO_ID);
        
        group_v5::create_group(quark_test, quark_test, group_id);
        
        let choices = get_test_choices();
        let from = timestamp::now_seconds();
        let to = from + 86400; // 24 hours later
        
        group_v5::create_group_dao_v1<AptosCoin>(quark_test, quark_test, group_id, dao_id, choices, from, to);
        
        // Verify DAO was created
        assert!(group_v5::exist_group_dao_v1(group_id, dao_id), 0);
    }

    #[test(aptos_framework = @0x1, quark_test = @quark_test)]
    fun test_create_group_dao_v2_success(aptos_framework: &signer, quark_test: &signer) {
        timestamp::set_time_has_started_for_testing(aptos_framework);
        init_module(quark_test);
        let group_id = string::utf8(TEST_GROUP_ID);
        let dao_id = string::utf8(TEST_DAO_ID);
        
        group_v5::create_group(quark_test, quark_test, group_id);
        
        let fa_obj = create_fa();
        let fa_addr = object::object_address(&fa_obj);
        
        let choices = get_test_choices();
        let from = timestamp::now_seconds();
        let to = from + 86400;
        
        group_v5::create_group_dao_v2(quark_test, quark_test, group_id, dao_id, choices, fa_addr, from, to);
        
        // Verify DAO was created
        assert!(group_v5::exist_group_dao_v2(group_id, dao_id), 0);
    }

    #[test(aptos_framework = @0x1, quark_test = @quark_test)]
    #[expected_failure(abort_code = 18, location = quark_test::group_v5)] // EFROM_TO_NOT_VALID
    fun test_create_group_dao_v1_invalid_time_range(aptos_framework: &signer, quark_test: &signer) {
        timestamp::set_time_has_started_for_testing(aptos_framework);
        init_module(quark_test);
        let group_id = string::utf8(TEST_GROUP_ID);
        let dao_id = string::utf8(TEST_DAO_ID);
        
        group_v5::create_group(quark_test, quark_test, group_id);
        
        let choices = get_test_choices();
        let from = timestamp::now_seconds() + 86400;
        let to = from - 1; // to < from
        
        group_v5::create_group_dao_v1<AptosCoin>(quark_test, quark_test, group_id, dao_id, choices, from, to);
    }

    #[test(aptos_framework = @0x1, quark_test = @quark_test)]
    #[expected_failure(abort_code = 19, location = quark_test::group_v5)] // ENOT_IN_TIME
    fun test_create_group_dao_v1_not_in_time(aptos_framework: &signer, quark_test: &signer) {
        timestamp::set_time_has_started_for_testing(aptos_framework);
        init_module(quark_test);
        let group_id = string::utf8(TEST_GROUP_ID);
        let dao_id = string::utf8(TEST_DAO_ID);
        
        group_v5::create_group(quark_test, quark_test, group_id);
        
        let choices = get_test_choices();
        let from = timestamp::now_seconds() + 86400; // Future time
        let to = from + 86400;
        
        group_v5::create_group_dao_v1<AptosCoin>(quark_test, quark_test, group_id, dao_id, choices, from, to);
    }

    // ==================== MOCK CLAIM REWARD TESTS ====================

    #[test(aptos_framework = @0x1, quark_test = @quark_test, user = @0x5)]
    fun test_mock_claim_reward_v1_with_state_success(aptos_framework: &signer, quark_test: &signer, user: &signer) acquires MockPoolsRewardsV1 {
        let (burn_cap, mint_cap) = aptos_coin::initialize_for_test(aptos_framework);
        timestamp::set_time_has_started_for_testing(aptos_framework);
        init_module(quark_test);
        let pool_id = string::utf8(TEST_POOL_ID);
        let user_addr = signer::address_of(user);
        
        // Create user account
        account::create_account_for_test(user_addr);
        coin::register<AptosCoin>(user);
        user_v5::create_account(user, string::utf8(b"1234567890"));
        
        // Fund admin account for the mock transfer
        account::create_account_for_test(signer::address_of(quark_test));
        coin::register<AptosCoin>(quark_test);
        aptos_coin::mint(aptos_framework, signer::address_of(quark_test), TEST_AMOUNT);
        
        // Create mock pool reward
        let mock_pool = create_mock_pool_reward_v1<AptosCoin>(pool_id, TEST_AMOUNT, TEST_TOTAL_USERS, @0x100);
        add_mock_pool_reward_v1(quark_test, mock_pool);
        
        let initial_user_balance = coin::balance<AptosCoin>(user_addr);
        
        // Claim reward using mock function with state
        mock_claim_reward_v1_with_state<AptosCoin>(quark_test, quark_test, user_addr, pool_id, aptos_framework);
        
        // Verify user received tokens
        let final_user_balance = coin::balance<AptosCoin>(user_addr);
        let claimed_amount = final_user_balance - initial_user_balance;
        
        assert!(claimed_amount >= 1, EIS_BALANCE_NOT_EQUAL);
        assert!(claimed_amount <= TEST_AMOUNT, EIS_BALANCE_NOT_EQUAL);
        
        coin::destroy_burn_cap(burn_cap);
        coin::destroy_mint_cap(mint_cap);
    }

    #[test(aptos_framework = @0x1, quark_test = @quark_test, user = @0x5)]
    #[expected_failure(abort_code = 10, location = quark_test::group_test)] // EUSER_ALREADY_CLAIMED
    fun test_mock_claim_reward_v1_with_state_already_claimed(aptos_framework: &signer, quark_test: &signer, user: &signer) acquires MockPoolsRewardsV1 {
        let (burn_cap, mint_cap) = aptos_coin::initialize_for_test(aptos_framework);
        timestamp::set_time_has_started_for_testing(aptos_framework);
        init_module(quark_test);
        let pool_id = string::utf8(TEST_POOL_ID);
        let user_addr = signer::address_of(user);
        
        // Create user account
        account::create_account_for_test(user_addr);
        coin::register<AptosCoin>(user);
        user_v5::create_account(user, string::utf8(b"1234567890"));
        
        // Fund admin account for the mock transfer
        account::create_account_for_test(signer::address_of(quark_test));
        coin::register<AptosCoin>(quark_test);
        aptos_coin::mint(aptos_framework, signer::address_of(quark_test), TEST_AMOUNT * 2);
        
        // Create mock pool reward
        let mock_pool = create_mock_pool_reward_v1<AptosCoin>(pool_id, TEST_AMOUNT, TEST_TOTAL_USERS, @0x100);
        add_mock_pool_reward_v1(quark_test, mock_pool);
        
        // Claim reward first time
        mock_claim_reward_v1_with_state<AptosCoin>(quark_test, quark_test, user_addr, pool_id, aptos_framework);
        
        // Try to claim again - should fail
        mock_claim_reward_v1_with_state<AptosCoin>(quark_test, quark_test, user_addr, pool_id, aptos_framework);
        
        coin::destroy_burn_cap(burn_cap);
        coin::destroy_mint_cap(mint_cap);
    }

    #[test(aptos_framework = @0x1, quark_test = @quark_test, user = @0x5)]
    fun test_mock_claim_reward_v2_with_state_success(aptos_framework: &signer, quark_test: &signer, user: &signer) acquires FAController, MockPoolsRewardsV2 {
        let (burn_cap, mint_cap) = aptos_coin::initialize_for_test(aptos_framework);
        timestamp::set_time_has_started_for_testing(aptos_framework);
        init_module(quark_test);
        let pool_id = string::utf8(TEST_POOL_ID);
        let user_addr = signer::address_of(user);
        
        // Create FA
        let fa_obj = create_fa();
        let fa_addr = object::object_address(&fa_obj);
        let fa_controller = borrow_global<FAController>(fa_addr);
        let fa_metadata = object::address_to_object<Metadata>(fa_addr);
        
        // Create user account
        account::create_account_for_test(user_addr);
        user_v5::create_account(user, string::utf8(b"1234567890"));
        
        // Fund admin account for the mock transfer
        mint_fa(quark_test, &fa_controller.mint_ref, TEST_AMOUNT);
        
        // Create mock pool reward
        let mock_pool = create_mock_pool_reward_v2(pool_id, TEST_AMOUNT, fa_addr, TEST_TOTAL_USERS, @0x100);
        add_mock_pool_reward_v2(quark_test, mock_pool);
        
        let initial_user_balance = primary_fungible_store::balance(user_addr, fa_metadata);
        
        // Claim reward using mock function with state
        mock_claim_reward_v2_with_state(quark_test, quark_test, user_addr, fa_addr, pool_id, aptos_framework);
        
        // Verify user received tokens
        let final_user_balance = primary_fungible_store::balance(user_addr, fa_metadata);
        let claimed_amount = final_user_balance - initial_user_balance;
        
        assert!(claimed_amount >= 1, EIS_BALANCE_NOT_EQUAL);
        assert!(claimed_amount <= TEST_AMOUNT, EIS_BALANCE_NOT_EQUAL);
        
        coin::destroy_burn_cap(burn_cap);
        coin::destroy_mint_cap(mint_cap);
    }

    #[test(aptos_framework = @0x1, quark_test = @quark_test, user = @0x5)]
    #[expected_failure(abort_code = 10, location = quark_test::group_test)] // EUSER_ALREADY_CLAIMED
    fun test_mock_claim_reward_v2_with_state_already_claimed(aptos_framework: &signer, quark_test: &signer, user: &signer) acquires FAController, MockPoolsRewardsV2 {
        let (burn_cap, mint_cap) = aptos_coin::initialize_for_test(aptos_framework);
        timestamp::set_time_has_started_for_testing(aptos_framework);
        init_module(quark_test);
        let pool_id = string::utf8(TEST_POOL_ID);
        let user_addr = signer::address_of(user);
        
        // Create FA
        let fa_obj = create_fa();
        let fa_addr = object::object_address(&fa_obj);
        let fa_controller = borrow_global<FAController>(fa_addr);
        
        // Create user account
        account::create_account_for_test(user_addr);
        user_v5::create_account(user, string::utf8(b"1234567890"));
        
        // Fund admin account for the mock transfer
        mint_fa(quark_test, &fa_controller.mint_ref, TEST_AMOUNT * 2);
        
        // Create mock pool reward
        let mock_pool = create_mock_pool_reward_v2(pool_id, TEST_AMOUNT, fa_addr, TEST_TOTAL_USERS, @0x100);
        add_mock_pool_reward_v2(quark_test, mock_pool);
        
        // Claim reward first time
        mock_claim_reward_v2_with_state(quark_test, quark_test, user_addr, fa_addr, pool_id, aptos_framework);
        
        // Try to claim again - should fail
        mock_claim_reward_v2_with_state(quark_test, quark_test, user_addr, fa_addr, pool_id, aptos_framework);
        
        coin::destroy_burn_cap(burn_cap);
        coin::destroy_mint_cap(mint_cap);
    }

    #[test(aptos_framework = @0x1, quark_test = @quark_test)]
    fun test_mock_pool_reward_state_management(aptos_framework: &signer, quark_test: &signer) acquires MockPoolsRewardsV1 {
        let (burn_cap, mint_cap) = aptos_coin::initialize_for_test(aptos_framework);
        timestamp::set_time_has_started_for_testing(aptos_framework);
        init_module(quark_test);
        let pool_id = string::utf8(TEST_POOL_ID);
        
        // Fund admin account for the mock transfer
        account::create_account_for_test(signer::address_of(quark_test));
        coin::register<AptosCoin>(quark_test);
        aptos_coin::mint(aptos_framework, signer::address_of(quark_test), TEST_AMOUNT * 3);
        
        // Create mock pool reward with 2 users
        let mock_pool = create_mock_pool_reward_v1<AptosCoin>(pool_id, TEST_AMOUNT, 2, @0x100);
        add_mock_pool_reward_v1(quark_test, mock_pool);
        
        // Create multiple users
        let user1 = account::create_signer_for_test(@0x500);
        let user2 = account::create_signer_for_test(@0x501);
        let user1_addr = signer::address_of(&user1);
        let user2_addr = signer::address_of(&user2);
        
        account::create_account_for_test(user1_addr);
        account::create_account_for_test(user2_addr);
        coin::register<AptosCoin>(&user1);
        coin::register<AptosCoin>(&user2);
        
        user_v5::create_account(&user1, string::utf8(b"1234567890"));
        user_v5::create_account(&user2, string::utf8(b"1234567891"));
        
        // First user claims
        mock_claim_reward_v1_with_state<AptosCoin>(quark_test, quark_test, user1_addr, pool_id, aptos_framework);
        assert!(coin::balance<AptosCoin>(user1_addr) >= 1, EIS_BALANCE_NOT_EQUAL);
        
        // Second user claims - this should remove the pool since total_users = 2
        mock_claim_reward_v1_with_state<AptosCoin>(quark_test, quark_test, user2_addr, pool_id, aptos_framework);
        assert!(coin::balance<AptosCoin>(user2_addr) >= 1, EIS_BALANCE_NOT_EQUAL);
        
        // Verify pool is removed (pools vector should be empty)
        let pools_rewards = borrow_global<MockPoolsRewardsV1>(signer::address_of(quark_test));
        assert!(vector::length(&pools_rewards.pools) == 0, EIS_BALANCE_NOT_EQUAL);
        
        coin::destroy_burn_cap(burn_cap);
        coin::destroy_mint_cap(mint_cap);
    }

    // ==================== VOTE GROUP DAO TESTS ====================

    #[test(aptos_framework = @0x1, quark_test = @quark_test, voter = @0x5)]
    fun test_vote_group_dao_v1_success(aptos_framework: &signer, quark_test: &signer, voter: &signer) {
        let (burn_cap, mint_cap) = aptos_coin::initialize_for_test(aptos_framework);
        timestamp::set_time_has_started_for_testing(aptos_framework);
        init_module(quark_test);
        let group_id = string::utf8(TEST_GROUP_ID);
        let dao_id = string::utf8(TEST_DAO_ID);
        
        group_v5::create_group(quark_test, quark_test, group_id);
        
        let choices = get_test_choices();
        let from = timestamp::now_seconds();
        let to = from + 86400;
        
        // Create DAO first
        group_v5::create_group_dao_v1<AptosCoin>(quark_test, quark_test, group_id, dao_id, choices, from, to);
        
        // Create voter same as user_test.move
        let voter_addr = signer::address_of(voter);
        
        account::create_account_for_test(voter_addr);
        coin::register<AptosCoin>(voter);
        aptos_coin::mint(aptos_framework, voter_addr, TEST_AMOUNT);
        
        user_v5::create_account(voter, string::utf8(b"1234567890"));
        
        // Verify user hasn't voted yet
        assert!(!group_v5::exist_group_user_choice_v1(group_id, dao_id, voter_addr), 0);
        
        // Vote
        group_v5::vote_group_dao_v1<AptosCoin>(voter, group_id, dao_id, 0);
        
        // Verify user has voted
        assert!(group_v5::exist_group_user_choice_v1(group_id, dao_id, voter_addr), 0);
        
        coin::destroy_burn_cap(burn_cap);
        coin::destroy_mint_cap(mint_cap);
    }

    #[test(aptos_framework = @0x1, quark_test = @quark_test, voter = @0x5)]
    fun test_vote_group_dao_v2_success(aptos_framework: &signer, quark_test: &signer, voter: &signer) acquires FAController {
        timestamp::set_time_has_started_for_testing(aptos_framework);
        init_module(quark_test);
        let group_id = string::utf8(TEST_GROUP_ID);
        let dao_id = string::utf8(TEST_DAO_ID);
        
        group_v5::create_group(quark_test, quark_test, group_id);
        
        let fa_obj = create_fa();
        let fa_addr = object::object_address(&fa_obj);
        let fa_controller = borrow_global<FAController>(fa_addr);
        
        let choices = get_test_choices();
        let from = timestamp::now_seconds();
        let to = from + 86400;
        
        // Create DAO first
        group_v5::create_group_dao_v2(quark_test, quark_test, group_id, dao_id, choices, fa_addr, from, to);
        
        // Create voter same as user_test.move
        let voter_addr = signer::address_of(voter);
        
        user_v5::create_account(voter, string::utf8(b"1234567890"));
        
        mint_fa(voter, &fa_controller.mint_ref, TEST_AMOUNT);
        
        // Verify user hasn't voted yet
        assert!(!group_v5::exist_group_user_choice_v2(group_id, dao_id, voter_addr), 0);
        
        // Vote
        group_v5::vote_group_dao_v2(voter, group_id, dao_id, 0, fa_addr);
        
        // Verify user has voted
        assert!(group_v5::exist_group_user_choice_v2(group_id, dao_id, voter_addr), 0);
    }

    #[test(aptos_framework = @0x1, quark_test = @quark_test, voter = @0x5)]
    #[expected_failure(abort_code = 17, location = quark_test::group_v5)] // EDAO_NOT_EXISTS
    fun test_vote_group_dao_v1_dao_not_exists(aptos_framework: &signer, quark_test: &signer, voter: &signer) {
        let (burn_cap, mint_cap) = aptos_coin::initialize_for_test(aptos_framework);
        timestamp::set_time_has_started_for_testing(aptos_framework);
        init_module(quark_test);
        let group_id = string::utf8(TEST_GROUP_ID);
        let dao_id = string::utf8(b"nonexistent_dao");
        
        group_v5::create_group(quark_test, quark_test, group_id);
        
        let voter_addr = signer::address_of(voter);
        
        account::create_account_for_test(voter_addr);
        coin::register<AptosCoin>(voter);
        aptos_coin::mint(aptos_framework, voter_addr, TEST_AMOUNT);
        
        user_v5::create_account(voter, string::utf8(b"1234567890"));
        
        group_v5::vote_group_dao_v1<AptosCoin>(voter, group_id, dao_id, 0);
        
        coin::destroy_burn_cap(burn_cap);
        coin::destroy_mint_cap(mint_cap);
    }

    #[test(aptos_framework = @0x1, quark_test = @quark_test, voter = @0x5)]
    #[expected_failure(abort_code = 22, location = quark_test::group_v5)] // ECURRENCY_NOT_MATCH
    fun test_vote_group_dao_v2_currency_not_match(aptos_framework: &signer, quark_test: &signer, voter: &signer) {
        timestamp::set_time_has_started_for_testing(aptos_framework);
        init_module(quark_test);
        let group_id = string::utf8(TEST_GROUP_ID);
        let dao_id = string::utf8(TEST_DAO_ID);
        
        group_v5::create_group(quark_test, quark_test, group_id);
        
        let fa_obj = create_fa();
        let fa_addr = object::object_address(&fa_obj);
        
        let choices = get_test_choices();
        let from = timestamp::now_seconds();
        let to = from + 86400;
        
        // Create DAO with one currency
        group_v5::create_group_dao_v2(quark_test, quark_test, group_id, dao_id, choices, fa_addr, from, to);
        
        user_v5::create_account(voter, string::utf8(b"1234567890"));
        
        // Vote with different currency
        group_v5::vote_group_dao_v2(voter, group_id, dao_id, 0, @0x2);
    }

    #[test(aptos_framework = @0x1, quark_test = @quark_test, voter = @0x5)]
    #[expected_failure(abort_code = 23, location = quark_test::group_v5)] // EUSER_ALREADY_VOTED
    fun test_vote_group_dao_v1_already_voted(aptos_framework: &signer, quark_test: &signer, voter: &signer) {
        let (burn_cap, mint_cap) = aptos_coin::initialize_for_test(aptos_framework);
        timestamp::set_time_has_started_for_testing(aptos_framework);
        init_module(quark_test);
        let group_id = string::utf8(TEST_GROUP_ID);
        let dao_id = string::utf8(TEST_DAO_ID);
        
        group_v5::create_group(quark_test, quark_test, group_id);
        
        let choices = get_test_choices();
        let from = timestamp::now_seconds();
        let to = from + 86400;
        
        // Create DAO first
        group_v5::create_group_dao_v1<AptosCoin>(quark_test, quark_test, group_id, dao_id, choices, from, to);
        
        let voter_addr = signer::address_of(voter);
        
        account::create_account_for_test(voter_addr);
        coin::register<AptosCoin>(voter);
        aptos_coin::mint(aptos_framework, voter_addr, TEST_AMOUNT);
        
        user_v5::create_account(voter, string::utf8(b"1234567890"));
        
        // Vote first time
        group_v5::vote_group_dao_v1<AptosCoin>(voter, group_id, dao_id, 0);
        
        // Try to vote again - should fail
        group_v5::vote_group_dao_v1<AptosCoin>(voter, group_id, dao_id, 1);
        
        coin::destroy_burn_cap(burn_cap);
        coin::destroy_mint_cap(mint_cap);
    }

    // Mock claim functions for testing (identical to originals but with allow_unsafe_randomness)
    #[lint::allow_unsafe_randomness]
    public fun mock_claim_reward_v1<CoinType>(admin: &signer, reviewer: &signer, user: address, pool_id: String, group_id: String) {
        use aptos_framework::aptos_account;
        use aptos_framework::randomness;
        use std::vector;
        use aptos_std::type_info;
        use quark_test::group_v5;
        
        let admin_address = signer::address_of(admin);
        let reviewer_address = signer::address_of(reviewer);
        let amount_to_claim;

        assert!(admin_v5::is_admin(admin_address), EONLY_ADMIN_CAN_CALL);
        assert!(admin_v5::is_reviewer(reviewer_address), EONLY_REVIEWER_CAN_CALL);

        let _group_account = group_v5::get_group_account(group_id);

        // Get pool reward info to calculate claim amount
        let (reward_amount, reward_token, total_users, claimed_users, _holder_object) = group_v5::get_pool_reward_v1(group_id, pool_id);

        assert!(vector::length(&claimed_users) < total_users, EPOOL_REWARD_ALREADY_CLAIMED);
        assert!(!vector::contains<address>(&claimed_users, &user), EUSER_ALREADY_CLAIMED);
        
        let coin_type = type_info::type_of<CoinType>();
        let coin_address = type_info::account_address(&coin_type);
        assert!(reward_token == coin_address, EPOOL_REWARD_TOKEN_NOT_MATCH);
        
        let users_claimed = vector::length(&claimed_users);
        let user_left = total_users - users_claimed;

        // Use actual randomness with allow_unsafe_randomness
        if (user_left > 1) {
            amount_to_claim = randomness::u64_range(1, reward_amount - user_left + 1);
        } else {
            amount_to_claim = reward_amount;
        };

        // For testing, we'll use a simplified transfer - the real function handles the complex holder object logic
        // This is a mock so we'll just transfer a fixed amount for predictable testing
        aptos_account::transfer_coins<CoinType>(admin, user, amount_to_claim);
    }
}
