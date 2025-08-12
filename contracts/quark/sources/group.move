module quark::group {
    use std::signer;
    use std::string::{Self, String};
    use std::vector;
    use std::object;
    use std::account::{Self, SignerCapability};
    use std::event;
    use std::option;
    use aptos_framework::timestamp;
    use aptos_framework::randomness;
    use aptos_std::type_info;
    use aptos_framework::coin;
    use aptos_framework::aptos_account;
    use aptos_framework::fungible_asset::Metadata;
    use aptos_framework::primary_fungible_store;
    use sshift_gpt::fees;
    use quark::admin;
    use quark::user;

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
    const ERESOURCE_ACCOUNT_NOT_EXISTS: u64 = 25;
    const EGROUP_ALREADY_MIGRATED: u64 = 26;

    struct Group has store, drop {
        group_id: String,
        account: address,
    }

    struct GroupSigner has key {
        signer_cap: SignerCapability,
    }

    struct Groups has key, store, drop {
        groups: vector<Group>,
    }

    struct PoolRewardHolder has key {
        extend_ref: object::ExtendRef,
    }

    struct PoolRewardV1 has key, store, copy, drop {
        pool_id: String,
        reward_amount: u64,
        reward_token: address,
        total_users: u64,
        claimed_users: vector<address>,
        holder_object: address,
    }

    struct PoolsRewardsV1 has key, store, copy, drop {
        pools: vector<PoolRewardV1>,
    }

    struct PoolRewardV2 has key, store, drop {
        pool_id: String,
        reward_amount: u64,
        reward_token: address,
        currency: address,
        total_users: u64,
        claimed_users: vector<address>,
        holder_object: address,
    }

    struct PoolsRewardsV2 has key, store, drop {
        pools: vector<PoolRewardV2>,
    }

    struct UserChoice has store, drop {
        dao_id: String,
        choice_id: u64,
        vote_weight: u64,
        user: address,
    }

    struct GroupDaoV1 has store {
        dao_id: String,
        group_id: String,
        choices: vector<String>,
        choices_weights: vector<u64>,
        user_choices: vector<UserChoice>,
        coin_type: address,
        from: u64,
        to: u64,
    }

    struct GroupDaosV1 has key, store {
        daos: vector<GroupDaoV1>,
    }

    struct GroupDaoV2 has store {
        dao_id: String,
        group_id: String,
        choices: vector<String>,
        choices_weights: vector<u64>,
        user_choices: vector<UserChoice>,
        currency: address,
        from: u64,
        to: u64,
    }

    struct GroupDaosV2 has key, store {
        daos: vector<GroupDaoV2>,
    }

    // View-only structs with copy ability
    struct UserChoiceView has copy, drop {
        dao_id: String,
        choice_id: u64,
        vote_weight: u64,
        user: address,
    }

    struct GroupDaoV1View has copy, drop {
        dao_id: String,
        group_id: String,
        choices: vector<String>,
        choices_weights: vector<u64>,
        user_choices: vector<UserChoiceView>,
        coin_type: address,
        from: u64,
        to: u64,
    }

    struct GroupDaosV1View has copy, drop {
        daos: vector<GroupDaoV1View>,
    }

    struct GroupDaoV2View has copy, drop {
        dao_id: String,
        group_id: String,
        choices: vector<String>,
        choices_weights: vector<u64>,
        user_choices: vector<UserChoiceView>,
        currency: address,
        from: u64,
        to: u64,
    }

    struct GroupDaosV2View has copy, drop {
        daos: vector<GroupDaoV2View>,
    }

    // Pool reward view structs
    struct PoolRewardV1View has copy, drop {
        pool_id: String,
        reward_amount: u64,
        reward_token: address,
        total_users: u64,
        claimed_users: vector<address>,
        holder_object: address,
    }

    struct PoolsRewardsV1View has copy, drop {
        pools: vector<PoolRewardV1View>,
    }

    struct PoolRewardV2View has copy, drop {
        pool_id: String,
        reward_amount: u64,
        reward_token: address,
        currency: address,
        total_users: u64,
        claimed_users: vector<address>,
        holder_object: address,
    }

    struct PoolsRewardsV2View has copy, drop {
        pools: vector<PoolRewardV2View>,
    }

    #[event]
    struct CreateGroupDaoEvent has drop, store {
        group: address,
        dao_id: String,
        choices: vector<String>,
        created_at: u64,
        from: u64,
        to: u64,
    }

    #[event]
    struct CreateGroupEvent has drop, store {
        group: address,
        created_at: u64,
    }

    #[event]
    struct VoteGroupDaoV1Event has drop, store {
        group: address,
        dao_id: String,
        choice_id: u64,
        vote_weight: u64,
        user: address,
        coin_type: address,
        created_at: u64,
    }

    #[event]
    struct VoteGroupDaoV2Event has drop, store {
        group: address,
        dao_id: String,
        choice_id: u64,
        vote_weight: u64,
        user: address,
        currency: address,
        created_at: u64,
    }

    #[event]
    struct PayAiEvent has drop, store {
        group: address,
        amount: u64,
        currency: address,
        recipient: address,
        created_at: u64,
    }

    #[event]
    struct GroupPayToUsersV1Event has drop, store {
        group: address,
        amount: u64,
        recipients: vector<address>,
        currency: address,
        created_at: u64,
    }

    #[event]
    struct GroupPayToUsersV2Event has drop, store {
        group: address,
        amount: u64,
        recipients: vector<address>,
        currency: address,
        created_at: u64,
    }

    #[event]
    struct MigrateGroupIdEvent has drop, store {
        group_id: String,
        new_group_id: String,
    }

    fun init_module(sender: &signer) {
        move_to(sender, Groups { groups: vector::empty() });
    }

    public entry fun create_group(admin: &signer, reviewer: &signer, group_id: String) acquires Groups {
        let admin_address = signer::address_of(admin);

        assert!(admin::is_admin(admin_address), EONLY_ADMIN_CAN_CALL);
        let reviewer_address = signer::address_of(reviewer);
        assert!(admin::is_reviewer(reviewer_address), EONLY_REVIEWER_CAN_CALL);

        assert!(!exist_group_id(group_id), EGROUP_ALREADY_EXISTS);

        let (group_signer, signer_cap) = account::create_resource_account(admin, *string::bytes(&group_id));

        let group_address = signer::address_of(&group_signer);

        

        let group = Group { group_id, account: group_address };

        let groups = borrow_global_mut<Groups>(@quark);

        vector::push_back(&mut groups.groups, group);

        move_to(&group_signer, GroupSigner { signer_cap });
    }

    public entry fun pay_ai<CoinType>(admin: &signer, reviewer: &signer, group_id: String, amount: u64) acquires Groups, GroupSigner {
        let admin_address = signer::address_of(admin);
        let reviewer_address = signer::address_of(reviewer);

        assert!(admin::is_admin(admin_address), EONLY_ADMIN_CAN_CALL);
        assert!(admin::is_reviewer(reviewer_address), EONLY_REVIEWER_CAN_CALL);

        let group_account = get_group_account(group_id);

        pay_ai_fees<CoinType>(group_account, amount);
    }

    public entry fun pay_ai_v2(admin: &signer, reviewer: &signer, group_id: String, amount: u64, currency: address) acquires Groups, GroupSigner {
        let admin_address = signer::address_of(admin);
        let reviewer_address = signer::address_of(reviewer);

        assert!(admin::is_admin(admin_address), EONLY_ADMIN_CAN_CALL);
        assert!(admin::is_reviewer(reviewer_address), EONLY_REVIEWER_CAN_CALL);

        let group_account = get_group_account(group_id);

        pay_ai_fees_v2(group_account, amount, currency);
    }

    public entry fun pay_users_v1<CoinType>(admin: &signer, reviewer: &signer, group_id: String, amount: u64, recipients: vector<address>) acquires Groups, GroupSigner {
        assert!(vector::length(&recipients) > 0, ENOT_USER_PASSED);
        let admin_address = signer::address_of(admin);
        let reviewer_address = signer::address_of(reviewer);

        assert!(admin::is_admin(admin_address), EONLY_ADMIN_CAN_CALL);
        assert!(admin::is_reviewer(reviewer_address), EONLY_REVIEWER_CAN_CALL);

        let groups = borrow_global<Groups>(@quark);

        let (exists_group, _group_index) = vector::find<Group>(&groups.groups, |group| group.group_id == group_id);
        assert!(exists_group, EGROUP_NOT_EXISTS);

        let group_account = get_group_account(group_id);

        let group_signer_cap = borrow_global<GroupSigner>(group_account);

        let resource_account = account::create_signer_with_capability(&group_signer_cap.signer_cap);
        let resource_account_address = signer::address_of(&resource_account);

        assert!(coin::balance<CoinType>(resource_account_address) >= amount, ENOT_ENOUGH_FUNDS);

        let amount_split = amount / vector::length(&recipients);

        vector::for_each(recipients, |recipient| {
            aptos_account::transfer_coins<CoinType>(&resource_account, recipient, amount_split);
        });

        let coin_type = type_info::type_of<CoinType>();
        let coin_type_addr = type_info::account_address(&coin_type);

        event::emit(GroupPayToUsersV1Event {
            group: resource_account_address,
            amount,
            recipients,
            currency: coin_type_addr,
            created_at: timestamp::now_seconds(),
        });
    }

    public entry fun pay_users_v2(admin: &signer, reviewer: &signer, group_id: String, amount: u64, currency: address, recipients: vector<address>) acquires Groups, GroupSigner {
        assert!(vector::length(&recipients) > 0, ENOT_USER_PASSED);

        let admin_address = signer::address_of(admin);
        let reviewer_address = signer::address_of(reviewer);

        assert!(admin::is_admin(admin_address), EONLY_ADMIN_CAN_CALL);
        assert!(admin::is_reviewer(reviewer_address), EONLY_REVIEWER_CAN_CALL);

        let group_account = get_group_account(group_id);

        let group_signer_cap = borrow_global<GroupSigner>(group_account);

        let resource_account = account::create_signer_with_capability(&group_signer_cap.signer_cap);
        let resource_account_address = signer::address_of(&resource_account);

        let fa_metadata = object::address_to_object<Metadata>(currency);

        let amount_split = amount / vector::length(&recipients);

        vector::for_each(recipients, |recipient| {
            aptos_account::transfer_fungible_assets(
                &resource_account,
                fa_metadata,
                recipient,
                amount_split,
            );
        });

        event::emit(GroupPayToUsersV2Event {
            group: resource_account_address,
            amount,
            recipients,
            currency,
            created_at: timestamp::now_seconds(),
        });
    }

    #[randomness]
    entry fun claim_reward_v1<CoinType>(admin: &signer, reviewer: &signer, user: address, pool_id: String, group_id: String) acquires PoolsRewardsV1, Groups, PoolRewardHolder {
        let admin_address = signer::address_of(admin);
        let reviewer_address = signer::address_of(reviewer);
        let amount_to_claim;

        assert!(admin::is_admin(admin_address), EONLY_ADMIN_CAN_CALL);
        assert!(admin::is_reviewer(reviewer_address), EONLY_REVIEWER_CAN_CALL);

        let group_account = get_group_account(group_id);

        let pools_rewards = borrow_global_mut<PoolsRewardsV1>(group_account);

        assert!(exists<PoolsRewardsV1>(group_account), EPOOLS_REWARDS_NOT_EXISTS);

        let (exists_pool, pool_index) = vector::find<PoolRewardV1>(&pools_rewards.pools, |pool| pool.pool_id == pool_id);

        assert!(exists_pool, EPOOL_NOT_EXISTS);

        let pool_reward = vector::borrow_mut(&mut pools_rewards.pools, pool_index);

        assert!(vector::length(&pool_reward.claimed_users) < pool_reward.total_users, EPOOL_REWARD_ALREADY_CLAIMED);

        assert!(!vector::contains<address>(&pool_reward.claimed_users, &user), EUSER_ALREADY_CLAIMED);
        
        let coin_type = type_info::type_of<CoinType>();
        let coin_address = type_info::account_address(&coin_type);

        assert!(pool_reward.reward_token == coin_address, EPOOL_REWARD_TOKEN_NOT_MATCH);
        
        let users_claimed = vector::length(&pool_reward.claimed_users);

        let user_left = pool_reward.total_users - users_claimed;

        if (user_left > 1) {
            amount_to_claim = randomness::u64_range(1, pool_reward.reward_amount - user_left);
        } else {
            amount_to_claim = pool_reward.reward_amount;
        };

        // Transfer from holder object instead of group account
        let holder_account = borrow_global<PoolRewardHolder>(pool_reward.holder_object);
        let holder_signer = object::generate_signer_for_extending(&holder_account.extend_ref);

        aptos_account::transfer_coins<CoinType>(&holder_signer, user, amount_to_claim);

        vector::push_back(&mut pool_reward.claimed_users, user);

        pool_reward.reward_amount -= amount_to_claim;

        if (vector::length(&pool_reward.claimed_users) == pool_reward.total_users) {
            vector::remove(&mut pools_rewards.pools, pool_index);
        }
    }

    #[randomness]
    entry fun claim_reward_v2(admin: &signer, reviewer: &signer, user: address, currency: address, pool_id: String, group_id: String) acquires PoolsRewardsV2, Groups, PoolRewardHolder {
        let amount_to_claim;
        let admin_address = signer::address_of(admin);
        let reviewer_address = signer::address_of(reviewer);

        assert!(admin::is_admin(admin_address), EONLY_ADMIN_CAN_CALL);
        assert!(admin::is_reviewer(reviewer_address), EONLY_REVIEWER_CAN_CALL);

        let group_account = get_group_account(group_id);

        let pools_rewards = borrow_global_mut<PoolsRewardsV2>(group_account);

        assert!(exists<PoolsRewardsV2>(group_account), EPOOLS_REWARDS_NOT_EXISTS);

        let (exists_pool, pool_index) = vector::find<PoolRewardV2>(&pools_rewards.pools, |pool| pool.pool_id == pool_id);

        assert!(exists_pool, EPOOL_NOT_EXISTS);

        let pool_reward = vector::borrow_mut(&mut pools_rewards.pools, pool_index);

        assert!(vector::length(&pool_reward.claimed_users) < pool_reward.total_users, EPOOL_REWARD_ALREADY_CLAIMED);

        assert!(!vector::contains<address>(&pool_reward.claimed_users, &user), EUSER_ALREADY_CLAIMED);
        
        let users_claimed = vector::length(&pool_reward.claimed_users);

        let user_left = pool_reward.total_users - users_claimed;

        if (user_left > 1) {
            amount_to_claim = randomness::u64_range(1, pool_reward.reward_amount - user_left);
        } else {
            amount_to_claim = pool_reward.reward_amount;
        };

        // Transfer from holder object instead of group account
        let holder_account = borrow_global<PoolRewardHolder>(pool_reward.holder_object);
        let holder_signer = object::generate_signer_for_extending(&holder_account.extend_ref);

        let fa_metadata = object::address_to_object<Metadata>(currency);

        aptos_account::transfer_fungible_assets(&holder_signer, fa_metadata, user, amount_to_claim);

        vector::push_back(&mut pool_reward.claimed_users, user);

        pool_reward.reward_amount -= amount_to_claim;

        if (vector::length(&pool_reward.claimed_users) == pool_reward.total_users) {
            vector::remove(&mut pools_rewards.pools, pool_index);
        }
    }

    public entry fun create_pool_reward_v1<CoinType>(admin: &signer, reviewer: &signer, pool_id: String, group_id: String, amount: u64, total_users: u64) acquires PoolsRewardsV1, GroupSigner, Groups {
        let admin_address = signer::address_of(admin);
        let reviewer_address = signer::address_of(reviewer);

        assert!(admin::is_admin(admin_address), EONLY_ADMIN_CAN_CALL);
        assert!(admin::is_reviewer(reviewer_address), EONLY_REVIEWER_CAN_CALL);

        let group_account = get_group_account(group_id);
        let group_signer_cap = borrow_global<GroupSigner>(group_account);
        let group_signer = account::create_signer_with_capability(&group_signer_cap.signer_cap);

        // Create holder object for this pool reward
        let holder_constructor_ref = object::create_object(signer::address_of(&group_signer));
        let holder_object_signer = object::generate_signer(&holder_constructor_ref);
        let holder_object_addr = signer::address_of(&holder_object_signer);

        // Store extend reference in the object
        let holder_extend_ref = object::generate_extend_ref(&holder_constructor_ref);
        move_to(&holder_object_signer, PoolRewardHolder { extend_ref: holder_extend_ref });

        // Create account for the object and register coin
        aptos_account::create_account(holder_object_addr);
        coin::register<CoinType>(&holder_object_signer);
        aptos_account::transfer_coins<CoinType>(&group_signer, holder_object_addr, amount);

        let coin_type = type_info::type_of<CoinType>();
        let coin_address = type_info::account_address(&coin_type);

        let pool_reward = PoolRewardV1 {
            pool_id,
            reward_amount: amount,
            reward_token: coin_address, 
            total_users,
            claimed_users: vector::empty(),
            holder_object: holder_object_addr,
        };

        if (exists<PoolsRewardsV1>(group_account)) {            
            let pools_rewards = borrow_global<PoolsRewardsV1>(group_account);

            let (exists_pool, _pool_index) = vector::find<PoolRewardV1>(&pools_rewards.pools, |pool| pool.pool_id == pool_id);

            assert!(!exists_pool, EPOOL_REWARD_ALREADY_EXISTS);

            let pools_rewards = borrow_global_mut<PoolsRewardsV1>(group_account);

            vector::push_back(&mut pools_rewards.pools, pool_reward);
        } else {
            let pools_rewards = PoolsRewardsV1 {
                pools: vector::empty(),
            };

            vector::push_back(&mut pools_rewards.pools, pool_reward);

            let resource_account = account::create_signer_with_capability(&group_signer_cap.signer_cap);

            move_to(&resource_account, pools_rewards);
        }
    }

    public entry fun create_pool_reward_v2(admin: &signer, reviewer: &signer, pool_id: String, group_id: String, currency: address, amount: u64, total_users: u64) acquires PoolsRewardsV2, Groups, GroupSigner {
        let admin_address = signer::address_of(admin);
        let reviewer_address = signer::address_of(reviewer);

        assert!(admin::is_admin(admin_address), EONLY_ADMIN_CAN_CALL);
        assert!(admin::is_reviewer(reviewer_address), EONLY_REVIEWER_CAN_CALL);

        let group_account = get_group_account(group_id);
        let group_signer_cap = borrow_global<GroupSigner>(group_account);
        let group_signer = account::create_signer_with_capability(&group_signer_cap.signer_cap);

        // Create holder object for this pool reward
        let holder_constructor_ref = object::create_object(signer::address_of(&group_signer));
        let holder_object_signer = object::generate_signer(&holder_constructor_ref);
        let holder_object_addr = signer::address_of(&holder_object_signer);

        // Store extend reference in the object
        let holder_extend_ref = object::generate_extend_ref(&holder_constructor_ref);
        move_to(&holder_object_signer, PoolRewardHolder { extend_ref: holder_extend_ref });

        // Create account for the object and transfer fungible assets
        aptos_account::create_account(holder_object_addr);
        let fa_metadata = object::address_to_object<Metadata>(currency);
        aptos_account::transfer_fungible_assets(&group_signer, fa_metadata, holder_object_addr, amount);

        let pool_reward = PoolRewardV2 {
            pool_id,
            reward_amount: amount,
            reward_token: currency,
            currency,
            total_users,
            claimed_users: vector::empty(),
            holder_object: holder_object_addr,
        };
        
        if (exists<PoolsRewardsV2>(group_account)) {
            let pools_rewards = borrow_global_mut<PoolsRewardsV2>(group_account);

            vector::push_back(&mut pools_rewards.pools, pool_reward);
        } else {
            let pools_rewards = PoolsRewardsV2 {
                pools: vector::empty(),
            };

            vector::push_back(&mut pools_rewards.pools, pool_reward);

            let resource_account = account::create_signer_with_capability(&group_signer_cap.signer_cap);

            move_to(&resource_account, pools_rewards);
        }
    }

    public entry fun create_group_dao_v1<CoinType>(admin: &signer, reviewer: &signer, group_id: String, dao_id: String, choices: vector<String>, from: u64, to: u64) acquires GroupDaosV1, GroupSigner, Groups {
        let admin_address = signer::address_of(admin);
        let reviewer_address = signer::address_of(reviewer);
        
        assert!(admin::is_admin(admin_address), EONLY_ADMIN_CAN_CALL);
        assert!(admin::is_reviewer(reviewer_address), EONLY_REVIEWER_CAN_CALL);

        assert!(from <= to, EFROM_TO_NOT_VALID);

        let now = timestamp::now_seconds();

        assert!(now <= from && from <= to, ENOT_IN_TIME);

        let choices_weights = vector::map<String, u64>(choices, |_| 0);

        let group_account = get_group_account(group_id);

        let coin_type = type_info::type_of<CoinType>();
        let coin_address = type_info::account_address(&coin_type);

        let group_dao = GroupDaoV1 {
            dao_id,
            group_id,
            choices,
            choices_weights,
            user_choices: vector::empty(),
            coin_type: coin_address,
            from,
            to,
        };
        
        if (exists<GroupDaosV1>(group_account)) {
            let group_daos = borrow_global_mut<GroupDaosV1>(group_account);

            let (exists_dao, _dao_index) = vector::find<GroupDaoV1>(&group_daos.daos, |dao| dao.dao_id == dao_id);
            assert!(!exists_dao, EDAO_ALREADY_EXISTS);

            vector::push_back(&mut group_daos.daos, group_dao);
        } else {
            let group_daos = GroupDaosV1 {
                daos: vector::empty(),
            };

            vector::push_back(&mut group_daos.daos, group_dao);

            let group_signer_cap = borrow_global<GroupSigner>(group_account);
            let resource_account = account::create_signer_with_capability(&group_signer_cap.signer_cap);

            move_to(&resource_account, group_daos);
        };

        event::emit(CreateGroupDaoEvent {
            group: group_account,
            dao_id,
            choices,
            created_at: now,
            from,
            to,
        });
    }

    public entry fun create_group_dao_v2(admin: &signer, reviewer: &signer, group_id: String, dao_id: String, choices: vector<String>, currency: address, from: u64, to: u64) acquires GroupDaosV2, GroupSigner, Groups {
        let admin_address = signer::address_of(admin);
        let reviewer_address = signer::address_of(reviewer);

        assert!(admin::is_admin(admin_address), EONLY_ADMIN_CAN_CALL);
        assert!(admin::is_reviewer(reviewer_address), EONLY_REVIEWER_CAN_CALL);

        assert!(from <= to, EFROM_TO_NOT_VALID);

        let now = timestamp::now_seconds();

        assert!(now <= from && from <= to, ENOT_IN_TIME);

        let choices_weights = vector::map<String, u64>(choices, |_| 0);

        let group_account = get_group_account(group_id);

        let group_dao = GroupDaoV2 {
            dao_id,
            group_id,
            choices,
            choices_weights,
            user_choices: vector::empty(),
            currency,
            from,
            to,
        };

        if (exists<GroupDaosV2>(group_account)) {
            let group_daos = borrow_global_mut<GroupDaosV2>(group_account);

            let (exists_dao, _dao_index) = vector::find<GroupDaoV2>(&group_daos.daos, |dao| dao.dao_id == dao_id);
            assert!(!exists_dao, EDAO_ALREADY_EXISTS);
            
            vector::push_back(&mut group_daos.daos, group_dao);
        } else {
            let group_daos = GroupDaosV2 {
                daos: vector::empty(),
            };

            vector::push_back(&mut group_daos.daos, group_dao);

            let group_signer_cap = borrow_global<GroupSigner>(group_account);
            let resource_account = account::create_signer_with_capability(&group_signer_cap.signer_cap);

            move_to(&resource_account, group_daos);
        };

        event::emit(CreateGroupDaoEvent {
            group: group_account,
            dao_id,
            choices,
            created_at: now,
            from,
            to,
        });
    }

    public entry fun vote_group_dao_v1<CoinType>(user: &signer, group_id: String, dao_id: String, choice_id: u64) acquires GroupDaosV1, Groups {
        let user_address = signer::address_of(user);

        let group_account = get_group_account(group_id);

        assert!(exists<GroupDaosV1>(group_account), EDAO_NOT_EXISTS);
        let group_daos = borrow_global_mut<GroupDaosV1>(group_account);

        let (exists_dao, dao_index) = vector::find<GroupDaoV1>(&group_daos.daos, |dao| dao.dao_id == dao_id);
        assert!(exists_dao, EDAO_NOT_EXISTS);

        let group_dao = vector::borrow_mut(&mut group_daos.daos, dao_index);

        let (exists_choice, _choice_index) = vector::find<UserChoice>(&group_dao.user_choices, |choice| choice.user == user_address);
        assert!(!exists_choice, EUSER_ALREADY_VOTED);

        let now = timestamp::now_seconds();

        assert!(now >= group_dao.from && now <= group_dao.to, ENOT_IN_TIME);
        
        let coin_type = type_info::type_of<CoinType>();
        let coin_address = type_info::account_address(&coin_type);

        assert!(group_dao.coin_type == coin_address, ECOIN_TYPE_NOT_MATCH);

        let balance = coin::balance<CoinType>(user_address);

        let now = timestamp::now_seconds();

        let user_choice = UserChoice {
            dao_id,
            choice_id,
            vote_weight: balance,
            user: user_address,
        };

        vector::push_back(&mut group_dao.user_choices, user_choice);

        group_dao.choices_weights[choice_id] += balance;

        event::emit(VoteGroupDaoV1Event {
            group: group_account,
            dao_id,
            choice_id,
            vote_weight: balance,
            user: user_address,
            coin_type: coin_address,
            created_at: now,
        });   
    }

    public entry fun vote_group_dao_v2(user: &signer, group_id: String, dao_id: String, choice_id: u64, currency: address) acquires GroupDaosV2, Groups {
        let user_address = signer::address_of(user);

        let group_account = get_group_account(group_id);

        assert!(exists<GroupDaosV2>(group_account), EDAO_NOT_EXISTS);
        let group_daos = borrow_global_mut<GroupDaosV2>(group_account);

        let (exists_dao, dao_index) = vector::find<GroupDaoV2>(&group_daos.daos, |dao| dao.dao_id == dao_id);
        assert!(exists_dao, EDAO_NOT_EXISTS);

        let group_dao = vector::borrow_mut(&mut group_daos.daos, dao_index);
        assert!(group_dao.currency == currency, ECURRENCY_NOT_MATCH);

        let (exists_choice, _choice_index) = vector::find<UserChoice>(&group_dao.user_choices, |choice| choice.user == user_address);
        assert!(!exists_choice, EUSER_ALREADY_VOTED);

        let now = timestamp::now_seconds();

        assert!(now >= group_dao.from && now <= group_dao.to, ENOT_IN_TIME);

        let metadata = object::address_to_object<Metadata>(currency);

        let balance = primary_fungible_store::balance<Metadata>(user_address, metadata);

        let user_choice = UserChoice {
            dao_id,
            choice_id,
            vote_weight: balance,
            user: user_address,
        };

        vector::push_back(&mut group_dao.user_choices, user_choice);

        group_dao.choices_weights[choice_id] += balance;

        event::emit(VoteGroupDaoV2Event {
            group: group_account,
            dao_id,
            choice_id,
            vote_weight: balance,
            user: user_address,
            currency,
            created_at: now,
        });
    }

    public entry fun migrate_group_id(admin: &signer, reviewer: &signer, group_id: String, new_group_id: String) acquires Groups {
        let admin_address = signer::address_of(admin);
        let reviewer_address = signer::address_of(reviewer);

        assert!(admin::is_admin(admin_address), EONLY_ADMIN_CAN_CALL);
        assert!(admin::is_reviewer(reviewer_address), EONLY_REVIEWER_CAN_CALL);

        assert!(group_id != new_group_id, EGROUP_ALREADY_MIGRATED);

        let groups = borrow_global_mut<Groups>(@quark);

        let (exists_group, group_index) = vector::find<Group>(&groups.groups, |group| group.group_id == group_id);
        assert!(exists_group, EGROUP_NOT_EXISTS);

        let (exists_new_group, new_group_index) = vector::find<Group>(&groups.groups, |group| group.group_id == new_group_id);

        if (exists_new_group) {
            vector::remove(&mut groups.groups, new_group_index);
        };

        let group = vector::borrow_mut(&mut groups.groups, group_index);

        group.group_id = new_group_id;

        event::emit(MigrateGroupIdEvent {
            group_id,
            new_group_id,
        });
    }

    #[view]
    public fun get_group_account(group_id: String): address acquires Groups {
        let groups = borrow_global<Groups>(@quark);

        let (exists_group, group_index) = vector::find<Group>(&groups.groups, |group| group.group_id == group_id);
        assert!(exists_group, EGROUP_NOT_EXISTS);

        let group = vector::borrow(&groups.groups, group_index);

        group.account
    }

    #[view]
    public fun exist_group_id(group_id: String): bool acquires Groups {
        let groups = borrow_global<Groups>(@quark);

        let (exists_group, _group_index) = vector::find<Group>(&groups.groups, |group| group.group_id == group_id);

        exists_group
    }

    #[view]
    public fun get_pool_reward_v1(group_id: String, pool_id: String): (u64, address, u64, vector<address>, address) acquires PoolsRewardsV1, Groups {
        let group_account = get_group_account(group_id);

        let pools_rewards = borrow_global<PoolsRewardsV1>(group_account);

        let (exists_pool, pool_index) = vector::find<PoolRewardV1>(&pools_rewards.pools, |pool| pool.pool_id == pool_id);

        assert!(exists_pool, EPOOL_NOT_EXISTS);

        let pool = vector::borrow(&pools_rewards.pools, pool_index);

        (pool.reward_amount, pool.reward_token, pool.total_users, pool.claimed_users, pool.holder_object)
    }

    #[view]
    public fun get_pool_reward_v2(group_id: String, pool_id: String): (u64, address, u64, vector<address>, address) acquires PoolsRewardsV2, Groups {
        let group_account = get_group_account(group_id);

        let pools_rewards = borrow_global<PoolsRewardsV2>(group_account);

        let (exists_pool, pool_index) = vector::find<PoolRewardV2>(&pools_rewards.pools, |pool| pool.pool_id == pool_id);

        assert!(exists_pool, EPOOL_NOT_EXISTS);

        let pool = vector::borrow(&pools_rewards.pools, pool_index);

        (pool.reward_amount, pool.reward_token, pool.total_users, pool.claimed_users, pool.holder_object)
    }

    #[view]
    public fun get_pools_rewards_v1(group_id: String): PoolsRewardsV1View acquires PoolsRewardsV1, Groups {
        let group_account = get_group_account(group_id);

        let pools_rewards = borrow_global<PoolsRewardsV1>(group_account);

        convert_pool_rewards_v1_to_view(pools_rewards)
    }

    #[view]
    public fun get_pools_rewards_v2(group_id: String): PoolsRewardsV2View acquires PoolsRewardsV2, Groups {
        let group_account = get_group_account(group_id);

        let pools_rewards = borrow_global<PoolsRewardsV2>(group_account);

        convert_pool_rewards_v2_to_view(pools_rewards)
    }

    #[view]
    public fun get_group_daos_v1(group_id: String): GroupDaosV1View acquires GroupDaosV1, Groups {
        let group_account = get_group_account(group_id);

        let group_daos = borrow_global<GroupDaosV1>(group_account);

        convert_group_daos_v1_to_view(group_daos)
    }

    #[view]
    public fun get_group_daos_v2(group_id: String): GroupDaosV2View acquires GroupDaosV2, Groups {
        let group_account = get_group_account(group_id);

        let group_daos = borrow_global<GroupDaosV2>(group_account);

        convert_group_daos_v2_to_view(group_daos)
    }

    #[view]
    public fun get_group_dao_v1(group_id: String, dao_id: String): GroupDaoV1View acquires GroupDaosV1, Groups {
        let group_account = get_group_account(group_id);

        let group_daos = borrow_global<GroupDaosV1>(group_account);

        let (exists_dao, dao_index) = vector::find<GroupDaoV1>(&group_daos.daos, |dao| dao.dao_id == dao_id);
        assert!(exists_dao, EDAO_NOT_EXISTS);

        let group_dao = vector::borrow(&group_daos.daos, dao_index);
        convert_group_dao_v1_to_view(group_dao)
    }

    #[view]
    public fun get_group_dao_v2(group_id: String, dao_id: String): GroupDaoV2View acquires GroupDaosV2, Groups {
        let group_account = get_group_account(group_id);

        let group_daos = borrow_global<GroupDaosV2>(group_account);

        let (exists_dao, dao_index) = vector::find<GroupDaoV2>(&group_daos.daos, |dao| dao.dao_id == dao_id);
        assert!(exists_dao, EDAO_NOT_EXISTS);

        let group_dao = vector::borrow(&group_daos.daos, dao_index);
        convert_group_dao_v2_to_view(group_dao)
    }

    #[view]
    public fun exist_group_dao_v1(group_id: String, dao_id: String): bool acquires GroupDaosV1, Groups {
        let group_account = get_group_account(group_id);

        let group_daos = borrow_global<GroupDaosV1>(group_account);

        let (exists_dao, _dao_index) = vector::find<GroupDaoV1>(&group_daos.daos, |dao| dao.dao_id == dao_id);

        exists_dao
    }

    #[view]
    public fun exist_group_dao_v2(group_id: String, dao_id: String): bool acquires GroupDaosV2, Groups {
        let group_account = get_group_account(group_id);

        let group_daos = borrow_global<GroupDaosV2>(group_account);

        let (exists_dao, _dao_index) = vector::find<GroupDaoV2>(&group_daos.daos, |dao| dao.dao_id == dao_id);

        exists_dao
    }

    #[view]
    public fun exist_group_user_choice_v1(group_id: String, dao_id: String, user_address: address): bool acquires GroupDaosV1, Groups {
        let group_account = get_group_account(group_id);

        let group_daos = borrow_global<GroupDaosV1>(group_account);

        let (exists_dao, dao_index) = vector::find<GroupDaoV1>(&group_daos.daos, |dao| dao.dao_id == dao_id);
        assert!(exists_dao, EDAO_NOT_EXISTS);

        let group_dao = vector::borrow(&group_daos.daos, dao_index);

        let (exists_choice, _choice_index) = vector::find<UserChoice>(&group_dao.user_choices, |choice| choice.user == user_address);

        exists_choice
    }

    #[view]
    public fun exist_group_user_choice_v2(group_id: String, dao_id: String, user_address: address): bool acquires GroupDaosV2, Groups {
        let group_account = get_group_account(group_id);

        let group_daos = borrow_global<GroupDaosV2>(group_account);

        let (exists_dao, dao_index) = vector::find<GroupDaoV2>(&group_daos.daos, |dao| dao.dao_id == dao_id);
        assert!(exists_dao, EDAO_NOT_EXISTS);

        let group_dao = vector::borrow(&group_daos.daos, dao_index);

        let (exists_choice, _choice_index) = vector::find<UserChoice>(&group_dao.user_choices, |choice| choice.user == user_address);

        exists_choice
    }

    #[view]
    public fun get_group_user_choices_v1(group_id: String, dao_id: String): vector<UserChoiceView> acquires GroupDaosV1, Groups {
        let group_account = get_group_account(group_id);

        let group_daos = borrow_global<GroupDaosV1>(group_account);

        let (exists_dao, dao_index) = vector::find<GroupDaoV1>(&group_daos.daos, |dao| dao.dao_id == dao_id);
        assert!(exists_dao, EDAO_NOT_EXISTS);

        let group_dao = vector::borrow(&group_daos.daos, dao_index);

        convert_user_choices_to_view(&group_dao.user_choices)
    }

    #[view]
    public fun get_group_user_choices_v2(group_id: String, dao_id: String): vector<UserChoiceView> acquires GroupDaosV2, Groups {
        let group_account = get_group_account(group_id);

        let group_daos = borrow_global<GroupDaosV2>(group_account);

        let (exists_dao, dao_index) = vector::find<GroupDaoV2>(&group_daos.daos, |dao| dao.dao_id == dao_id);
        assert!(exists_dao, EDAO_NOT_EXISTS);

        let group_dao = vector::borrow(&group_daos.daos, dao_index);

        convert_user_choices_to_view(&group_dao.user_choices)
    }

    #[view]
    public fun get_group_user_choice_v1(group_id: String, dao_id: String, user_address: address): UserChoiceView acquires GroupDaosV1, Groups {
        let group_account = get_group_account(group_id);

        let group_daos = borrow_global<GroupDaosV1>(group_account);

        let (exists_dao, dao_index) = vector::find<GroupDaoV1>(&group_daos.daos, |dao| dao.dao_id == dao_id);
        assert!(exists_dao, EDAO_NOT_EXISTS);

        let group_dao = vector::borrow(&group_daos.daos, dao_index);

        let (exists_choice, _choice_index) = vector::find<UserChoice>(&group_dao.user_choices, |choice| choice.user == user_address);
        assert!(exists_choice, EUSER_NOT_VOTED);

        let user_choice = vector::borrow(&group_dao.user_choices, _choice_index);
        convert_user_choice_to_view(user_choice)
    }

    #[view]
    public fun get_group_user_choice_v2(group_id: String, dao_id: String, user_address: address): UserChoiceView acquires GroupDaosV2, Groups {
        let group_account = get_group_account(group_id);

        let group_daos = borrow_global<GroupDaosV2>(group_account);

        let (exists_dao, dao_index) = vector::find<GroupDaoV2>(&group_daos.daos, |dao| dao.dao_id == dao_id);
        assert!(exists_dao, EDAO_NOT_EXISTS);

        let group_dao = vector::borrow(&group_daos.daos, dao_index);

        let (exists_choice, _choice_index) = vector::find<UserChoice>(&group_dao.user_choices, |choice| choice.user == user_address);
        assert!(exists_choice, EUSER_NOT_VOTED);

        let user_choice = vector::borrow(&group_dao.user_choices, _choice_index);
        convert_user_choice_to_view(user_choice)
    }

    // Helper functions to convert original structs to view structs
    fun convert_user_choice_to_view(user_choice: &UserChoice): UserChoiceView {
        UserChoiceView {
            dao_id: user_choice.dao_id,
            choice_id: user_choice.choice_id,
            vote_weight: user_choice.vote_weight,
            user: user_choice.user,
        }
    }

    fun convert_user_choices_to_view(user_choices: &vector<UserChoice>): vector<UserChoiceView> {
        vector::map_ref(user_choices, |user_choice| convert_user_choice_to_view(user_choice))
    }

    fun convert_group_dao_v1_to_view(group_dao: &GroupDaoV1): GroupDaoV1View {
        GroupDaoV1View {
            dao_id: group_dao.dao_id,
            group_id: group_dao.group_id,
            choices: group_dao.choices,
            choices_weights: group_dao.choices_weights,
            user_choices: convert_user_choices_to_view(&group_dao.user_choices),
            coin_type: group_dao.coin_type,
            from: group_dao.from,
            to: group_dao.to,
        }
    }

    fun convert_group_dao_v2_to_view(group_dao: &GroupDaoV2): GroupDaoV2View {
        GroupDaoV2View {
            dao_id: group_dao.dao_id,
            group_id: group_dao.group_id,
            choices: group_dao.choices,
            choices_weights: group_dao.choices_weights,
            user_choices: convert_user_choices_to_view(&group_dao.user_choices),
            currency: group_dao.currency,
            from: group_dao.from,
            to: group_dao.to,
        }
    }

    fun convert_group_daos_v1_to_view(group_daos: &GroupDaosV1): GroupDaosV1View {
        GroupDaosV1View {
            daos: vector::map_ref(&group_daos.daos, |dao| convert_group_dao_v1_to_view(dao)),
        }
    }

    fun convert_group_daos_v2_to_view(group_daos: &GroupDaosV2): GroupDaosV2View {
        GroupDaosV2View {
            daos: vector::map_ref(&group_daos.daos, |dao| convert_group_dao_v2_to_view(dao)),
        }
    }

    // Helper functions to convert pool reward structs to view structs
    fun convert_pool_reward_v1_to_view(pool_reward: &PoolRewardV1): PoolRewardV1View {
        PoolRewardV1View {
            pool_id: pool_reward.pool_id,
            reward_amount: pool_reward.reward_amount,
            reward_token: pool_reward.reward_token,
            total_users: pool_reward.total_users,
            claimed_users: pool_reward.claimed_users,
            holder_object: pool_reward.holder_object,
        }
    }

    fun convert_pool_rewards_v1_to_view(pool_rewards: &PoolsRewardsV1): PoolsRewardsV1View {
        PoolsRewardsV1View {
            pools: vector::map_ref(&pool_rewards.pools, |pool| convert_pool_reward_v1_to_view(pool)),
        }
    }

    fun convert_pool_reward_v2_to_view(pool_reward: &PoolRewardV2): PoolRewardV2View {
        PoolRewardV2View {
            pool_id: pool_reward.pool_id,
            reward_amount: pool_reward.reward_amount,
            reward_token: pool_reward.reward_token,
            currency: pool_reward.currency,
            total_users: pool_reward.total_users,
            claimed_users: pool_reward.claimed_users,
            holder_object: pool_reward.holder_object,
        }
    }

    fun convert_pool_rewards_v2_to_view(pool_rewards: &PoolsRewardsV2): PoolsRewardsV2View {
        PoolsRewardsV2View {
            pools: vector::map_ref(&pool_rewards.pools, |pool| convert_pool_reward_v2_to_view(pool)),
        }
    }

    fun pay_ai_fees<CoinType>(group_account: address, amount: u64) acquires GroupSigner {
        let coin_type = user::get_token_address();

        assert!(option::is_some(&coin_type), ENOT_COIN_PAYMENT_SET);
        assert!(fees::resource_account_exists(), ERESOURCE_ACCOUNT_NOT_EXISTS);
        assert!(amount > 0, EAMOUNT_MUST_BE_GREATER_THAN_ZERO);

        let group_signer_cap = borrow_global<GroupSigner>(group_account);
        let resource_account = account::create_signer_with_capability(&group_signer_cap.signer_cap);
        let resource_account_address = signer::address_of(&resource_account);

        let coin_info = type_info::type_of<CoinType>();
        let coin_type_addr = type_info::account_address(&coin_info);
        assert!(&coin_type_addr == option::borrow(&coin_type) || admin::exist_fees_currency_payment_list(coin_type_addr), ECOINS_NOT_MATCH);
        assert!(coin::balance<CoinType>(resource_account_address) >= amount, ENOT_ENOUGH_FUNDS);

        let resource_account_fees = fees::get_resource_account_address();

        aptos_account::transfer_coins<CoinType>(&resource_account, resource_account_fees, amount);

        event::emit(PayAiEvent {
            group: resource_account_address,
            amount,
            currency: coin_type_addr,
            recipient: resource_account_fees,
            created_at: timestamp::now_seconds(),
        });

    }

    fun pay_ai_fees_v2(group_account: address, amount: u64, currency: address) acquires GroupSigner {
        assert!(fees::resource_account_exists(), ERESOURCE_ACCOUNT_NOT_EXISTS);
        assert!(amount > 0, EAMOUNT_MUST_BE_GREATER_THAN_ZERO);
        assert!(admin::exist_fees_currency_payment_list(currency), ECOINS_NOT_MATCH);

        let group_signer_cap = borrow_global<GroupSigner>(group_account);
        let resource_account = account::create_signer_with_capability(&group_signer_cap.signer_cap);
        let resource_account_address = signer::address_of(&resource_account);

        let fa_metadata = object::address_to_object<Metadata>(currency);

        assert!(primary_fungible_store::balance(resource_account_address, fa_metadata) >= amount, ENOT_ENOUGH_FUNDS);

        let resource_account_fees = fees::get_resource_account_address();

        aptos_account::transfer_fungible_assets(&resource_account, fa_metadata, resource_account_fees, amount);

        event::emit(PayAiEvent {
            group: resource_account_address,
            amount,
            currency,
            recipient: resource_account_fees,
            created_at: timestamp::now_seconds(),
        });
    }

    #[test_only]
    public fun test_init_group(admin: &signer) {
        init_module(admin);
    }

    #[test_only]
    public fun count_group(group_id: String): u64 acquires Groups {
        let groups = borrow_global<Groups>(@quark);

        let count = 0;

        vector::for_each_ref(&groups.groups, |group| {
            if (group.group_id == group_id) {
                count = count + 1;
            };
        });

        count
    }
}