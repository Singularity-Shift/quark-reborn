module quark::user {
    use std::signer;
    use std::option::{Self, Option};
    use std::account::{Self, SignerCapability};
    use std::string::{Self, String};
    use std::vector;
    use std::object;
    use std::event;
    use aptos_framework::timestamp;
    use aptos_std::type_info;
    use aptos_framework::coin;
    use aptos_framework::aptos_account;
    use aptos_framework::fungible_asset::Metadata;
    use aptos_framework::primary_fungible_store;
    use sshift_gpt::fees;
    use quark::admin;

    const EONLY_ADMIN_CAN_CALL: u64 = 1;
    const EONLY_REVIEWER_CAN_CALL: u64 = 2;
    const ENOT_ENOUGH_FUNDS: u64 = 3;
    const ENOT_COIN_PAYMENT_SET: u64 = 4;
    const ECOINS_NOT_MATCH: u64 = 5;
    const ENOT_USER_PASSED: u64 = 6;
    const ERESOURCE_ACCOUNT_NOT_EXISTS: u64 = 7;
    const EAMOUNT_MUST_BE_GREATER_THAN_ZERO: u64 = 8;

    struct Account has key {
        telegram_id: String,
        signer_cap: SignerCapability,
    }

    struct Config has key {
        coin_addr: Option<address>,
    }
    
    #[event]
    struct CreateAccountEvent has drop, store {
        owner: address,
        user: address,
        created_at: u64,
    }

    #[event]
    struct PayAiEvent has drop, store {
        user: address,
        amount: u64,
        currency: address,
        recipient: address,
        created_at: u64,
    }

    #[event]
    struct PayToUsersV1Event has drop, store {
        user: address,
        amount: u64,
        recipients: vector<address>,
        currency: address,
        created_at: u64,
    }

    #[event]
    struct PayToUsersV2Event has drop, store {
        user: address,
        amount: u64,
        recipients: vector<address>,
        currency: address,
        created_at: u64,
    }

    #[event]
    struct WithdrawFundsV1Event has drop, store {
        user: address,
        amount: u64,
        currency: address,
        created_at: u64,
    }

    #[event]
    struct WithdrawFundsV2Event has drop, store {
        user: address,
        amount: u64,
        currency: address,
        created_at: u64,
    }
    

    fun init_module(sender: &signer) {
        move_to(sender, Config { coin_addr: option::none() });
    }

    public entry fun set_coin_address<CoinType>(sender: &signer) acquires Config {
        let admin_address = signer::address_of(sender);
        assert!(admin::is_admin(admin_address), EONLY_ADMIN_CAN_CALL);

        let config = borrow_global_mut<Config>(@quark);

        let coin_type = type_info::type_of<CoinType>();
        let coin_type_addr = type_info::account_address(&coin_type);
        
        config.coin_addr = option::some(coin_type_addr);
    }
    

    public entry fun create_account(sender: &signer, telegram_id: String) {
        let (user, signer_cap) = account::create_resource_account(sender, *string::bytes(&telegram_id));
        move_to(sender, Account { telegram_id, signer_cap });

        let user_address = signer::address_of(&user);
        let owner_address = signer::address_of(sender);

        event::emit(CreateAccountEvent {
            user: user_address,
            owner: owner_address,
            created_at: timestamp::now_seconds(),
        });
    }

    public entry fun withdraw_funds_v1<CoinType>(sender: &signer, amount: u64) acquires Account {
        let user = signer::address_of(sender);

        let account = borrow_global<Account>(user);
        let resource_account = account::create_signer_with_capability(&account.signer_cap);
        let resource_account_address = signer::address_of(&resource_account);
        assert!(coin::balance<CoinType>(resource_account_address) >= amount, ENOT_ENOUGH_FUNDS);
        aptos_account::transfer_coins<CoinType>(&resource_account, user, amount);

        let coin_type = type_info::type_of<CoinType>();
        let coin_type_addr = type_info::account_address(&coin_type);

        event::emit(WithdrawFundsV1Event {
            user,
            amount,
            currency: coin_type_addr,
            created_at: timestamp::now_seconds(),
        });
    }

    public entry fun withdraw_funds_v2(sender: &signer, amount: u64, currency: address) acquires Account {
        let user = signer::address_of(sender);

        let user_account = borrow_global<Account>(user);

        let resource_account = account::create_signer_with_capability(&user_account.signer_cap);

        let resource_account_address = signer::address_of(&resource_account);

        let metadata = object::address_to_object<Metadata>(currency);

        assert!(primary_fungible_store::balance(resource_account_address, metadata) >= amount, ENOT_ENOUGH_FUNDS);

        aptos_account::transfer_fungible_assets(&resource_account, metadata, user, amount);

        event::emit(WithdrawFundsV2Event {
            user,
            amount,
            currency,
            created_at: timestamp::now_seconds(),
        });
    }

    public entry fun pay_ai<CoinType>(admin: &signer, reviewer: &signer, user: address, amount: u64) acquires Account, Config {
        let admin_address = signer::address_of(admin);
        assert!(admin::is_admin(admin_address), EONLY_ADMIN_CAN_CALL);

        let reviewer_address = signer::address_of(reviewer);
        assert!(admin::is_reviewer(reviewer_address), EONLY_REVIEWER_CAN_CALL);

        pay_ai_fee<CoinType>(user, amount);
    }

    public entry fun pay_ai_v2(admin: &signer, reviewer: &signer, user: address, amount: u64, currency: address) acquires Account {
        let admin_address = signer::address_of(admin);
        assert!(admin::is_admin(admin_address), EONLY_ADMIN_CAN_CALL);

        let reviewer_address = signer::address_of(reviewer);
        assert!(admin::is_reviewer(reviewer_address), EONLY_REVIEWER_CAN_CALL);

        pay_ai_fee_v2(user, amount, currency);
    }

    public entry fun pay_to_users_v1<CoinType>(admin: &signer, reviewer: &signer, user: address, amount: u64, recipients: vector<address>) acquires Account {
        assert!(vector::length(&recipients) > 0, ENOT_USER_PASSED);
        let admin_address = signer::address_of(admin);
        assert!(admin::is_admin(admin_address), EONLY_ADMIN_CAN_CALL);

        let reviewer_address = signer::address_of(reviewer);
        assert!(admin::is_reviewer(reviewer_address), EONLY_REVIEWER_CAN_CALL);

        let user_account = borrow_global<Account>(user);
        let resource_account = account::create_signer_with_capability(&user_account.signer_cap);

        let amount_split = amount / vector::length(&recipients);
        
        vector::for_each(recipients, |recipient| {
            aptos_account::transfer_coins<CoinType>(&resource_account, recipient, amount_split);
        });

        let coin_type = type_info::type_of<CoinType>();
        let coin_type_addr = type_info::account_address(&coin_type);

        event::emit(PayToUsersV1Event {
            user,
            amount,
            recipients,
            currency: coin_type_addr,
            created_at: timestamp::now_seconds(),
        });
    }

    public entry fun pay_to_users_v2(admin: &signer, reviewer: &signer, user: address, amount: u64, currency: address, recipients: vector<address>) acquires Account {
        assert!(vector::length(&recipients) > 0, ENOT_USER_PASSED);
        
        let admin_address = signer::address_of(admin);
        assert!(admin::is_admin(admin_address), EONLY_ADMIN_CAN_CALL);

        let reviewer_address = signer::address_of(reviewer);
        assert!(admin::is_reviewer(reviewer_address), EONLY_REVIEWER_CAN_CALL);

        let user_account = borrow_global<Account>(user);
        let resource_account = account::create_signer_with_capability(&user_account.signer_cap);

        let fa_metadata = object::address_to_object<Metadata>(currency);

        let amount_split = amount / vector::length(&recipients);

        vector::for_each(recipients, |recipient| {
            aptos_account::transfer_fungible_assets(&resource_account, fa_metadata, recipient, amount_split);
        });

        event::emit(PayToUsersV2Event {
            user,
            amount,
            recipients,
            currency,
            created_at: timestamp::now_seconds(),
        });
    }
    

    fun pay_ai_fee<CoinType>(user: address, amount: u64) acquires Account, Config {
        let config = borrow_global<Config>(@quark);
        assert!(option::is_some(&config.coin_addr), ENOT_COIN_PAYMENT_SET);
        assert!(fees::resource_account_exists(), ERESOURCE_ACCOUNT_NOT_EXISTS);
        assert!(amount > 0, EAMOUNT_MUST_BE_GREATER_THAN_ZERO);
        
        let coin_type = type_info::type_of<CoinType>();
        let coin_type_addr = type_info::account_address(&coin_type);     
        let coin_addr = option::borrow(&config.coin_addr);
        assert!(&coin_type_addr == coin_addr || admin::exist_fees_currency_payment_list(coin_type_addr), ECOINS_NOT_MATCH);

        let user_account = borrow_global<Account>(user);
        let resource_account = account::create_signer_with_capability(&user_account.signer_cap);

        let resource_account_address = signer::address_of(&resource_account);

        assert!(coin::balance<CoinType>(resource_account_address) >= amount, ENOT_ENOUGH_FUNDS);

        let resource_account_fees = fees::get_resource_account_address();

        aptos_account::transfer_coins<CoinType>(&resource_account, resource_account_fees, amount);        

        event::emit(PayAiEvent {
            user,
            amount,
            currency: coin_type_addr,
            recipient: resource_account_fees,
            created_at: timestamp::now_seconds(),
        });
    }

    fun pay_ai_fee_v2(user: address, amount: u64, currency: address) acquires Account {
        assert!(admin::exist_fees_currency_payment_list(currency), ECOINS_NOT_MATCH);
        assert!(fees::resource_account_exists(), ERESOURCE_ACCOUNT_NOT_EXISTS);
        assert!(amount > 0, EAMOUNT_MUST_BE_GREATER_THAN_ZERO);

        let fa_metadata = object::address_to_object<Metadata>(currency);

        let user_account = borrow_global<Account>(user);
        let resource_account = account::create_signer_with_capability(&user_account.signer_cap);

        let resource_account_address = signer::address_of(&resource_account);

        assert!(primary_fungible_store::balance(resource_account_address, fa_metadata) >= amount, ENOT_ENOUGH_FUNDS);

        let resource_account_fees = fees::get_resource_account_address();

        aptos_account::transfer_fungible_assets(&resource_account, fa_metadata, resource_account_fees, amount);

        event::emit(PayAiEvent {
            user,
            amount,
            currency,
            recipient: resource_account_fees,
            created_at: timestamp::now_seconds(),
        });
    }

    #[view]
    public fun exists_resource_account(user: address): bool {
        exists<Account>(user)
    }

    #[view]
    public fun get_resource_account(user: address): address acquires Account {
        let user_account = borrow_global<Account>(user);
        let resource_account = account::create_signer_with_capability(&user_account.signer_cap);
        signer::address_of(&resource_account)
    }

    #[view]
    public fun get_token_address(): Option<address> acquires Config {
        let config = borrow_global<Config>(@quark);
        config.coin_addr
    }

    #[test_only]
    public fun test_init_account(admin: &signer) {
        init_module(admin);
    }
}