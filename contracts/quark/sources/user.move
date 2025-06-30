module quark_test::user_v4 {
    use std::signer;
    use std::option::{Self, Option};
    use std::account::{Self, SignerCapability};
    use std::string::{Self, String};
    use std::vector;
    use std::object;
    use aptos_std::type_info;
    use aptos_framework::coin;
    use aptos_framework::aptos_account;
    use aptos_framework::fungible_asset::Metadata;
    use aptos_framework::primary_fungible_store;
    use sshift_gpt::fees;
    use quark_test::admin_v4;

    const EONLY_ADMIN_CAN_CALL: u64 = 1;
    const EONLY_REVIEWER_CAN_CALL: u64 = 2;
    const ENOT_ENOUGH_FUNDS: u64 = 3;
    const ENOT_COIN_PAYMENT_SET: u64 = 4;
    const ECOINS_NOT_MATCH: u64 = 5;
    const ERESOURCE_ACCOUNT_NOT_EXISTS: u64 = 6;
    const EAMOUNT_MUST_BE_GREATER_THAN_ZERO: u64 = 7;

    struct Account has key {
        telegram_id: String,
        signer_cap: SignerCapability,
    }

    struct Config has key {
        coin_addr: Option<address>,
    }

    fun init_module(sender: &signer) {
        move_to(sender, Config { coin_addr: option::none() });
    }

    public entry fun set_coin_address<CoinType>(sender: &signer) acquires Config {
        let admin_address = signer::address_of(sender);
        assert!(admin_v4::is_admin(admin_address), EONLY_ADMIN_CAN_CALL);

        let config = borrow_global_mut<Config>(@quark_test);

        let coin_type = type_info::type_of<CoinType>();
        let coin_type_addr = type_info::account_address(&coin_type);
        
        config.coin_addr = option::some(coin_type_addr);
    }
    

    public entry fun create_account(sender: &signer, telegram_id: String) {
        let (_, signer_cap) = account::create_resource_account(sender, *string::bytes(&telegram_id));
        move_to(sender, Account { telegram_id, signer_cap });
    }

    public entry fun withdraw_funds_v1<CoinType>(sender: &signer, amount: u64) acquires Account {
        let user = signer::address_of(sender);

        let account = borrow_global<Account>(user);
        let resource_account = account::create_signer_with_capability(&account.signer_cap);
        let resource_account_address = signer::address_of(&resource_account);
        assert!(coin::balance<CoinType>(resource_account_address) >= amount, ENOT_ENOUGH_FUNDS);
        aptos_account::transfer_coins<CoinType>(&resource_account, user, amount);
    }

    public entry fun withdraw_funds_v2(sender: &signer, amount: u64, currency: address) acquires Account {
        let user = signer::address_of(sender);

        let user_account = borrow_global<Account>(user);

        let resource_account = account::create_signer_with_capability(&user_account.signer_cap);

        let resource_account_address = signer::address_of(&resource_account);

        let metadata = object::address_to_object<Metadata>(currency);

        assert!(primary_fungible_store::balance(resource_account_address, metadata) >= amount, ENOT_ENOUGH_FUNDS);

        aptos_account::transfer_fungible_assets(&resource_account, metadata, user, amount);
    }

    public entry fun pay_ai<CoinType>(admin: &signer, reviewer: &signer, user: address, amount: u64) acquires Account, Config {
        let admin_address = signer::address_of(admin);
        assert!(admin_v4::is_admin(admin_address), EONLY_ADMIN_CAN_CALL);

        let reviewer_address = signer::address_of(reviewer);
        assert!(admin_v4::is_reviewer(reviewer_address), EONLY_REVIEWER_CAN_CALL);

        pay_ai_fee<CoinType>(user, amount);
    }

    public entry fun pay_to_users_v1<CoinType>(admin: &signer, reviewer: &signer, user: address, amount: u64, recipients: vector<address>) acquires Account {
        let admin_address = signer::address_of(admin);
        assert!(admin_v4::is_admin(admin_address), EONLY_ADMIN_CAN_CALL);

        let reviewer_address = signer::address_of(reviewer);
        assert!(admin_v4::is_reviewer(reviewer_address), EONLY_REVIEWER_CAN_CALL);

        let user_account = borrow_global<Account>(user);
        let resource_account = account::create_signer_with_capability(&user_account.signer_cap);
        
        vector::for_each(recipients, |recipient| {
            aptos_account::transfer_coins<CoinType>(&resource_account, recipient, amount);
        });   
    }

    public entry fun pay_to_users_v2(admin: &signer, reviewer: &signer, user: address, amount: u64, currency: address, recipients: vector<address>) acquires Account {
        let admin_address = signer::address_of(admin);
        assert!(admin_v4::is_admin(admin_address), EONLY_ADMIN_CAN_CALL);

        let reviewer_address = signer::address_of(reviewer);
        assert!(admin_v4::is_reviewer(reviewer_address), EONLY_REVIEWER_CAN_CALL);

        let user_account = borrow_global<Account>(user);
        let resource_account = account::create_signer_with_capability(&user_account.signer_cap);

        let fa_metadata = object::address_to_object<Metadata>(currency);

        vector::for_each(recipients, |recipient| {
            aptos_account::transfer_fungible_assets(&resource_account, fa_metadata, recipient, amount);
        });
    }
    

    fun pay_ai_fee<CoinType>(user: address, amount: u64) acquires Account, Config {
        let config = borrow_global<Config>(@quark_test);
        assert!(option::is_some(&config.coin_addr), ENOT_COIN_PAYMENT_SET);
        assert!(fees::resource_account_exists(), ERESOURCE_ACCOUNT_NOT_EXISTS);
        assert!(amount > 0, EAMOUNT_MUST_BE_GREATER_THAN_ZERO);
        
        let coin_type = type_info::type_of<CoinType>();
        let coin_type_addr = type_info::account_address(&coin_type);     
        let coin_addr = option::borrow(&config.coin_addr);
        assert!(&coin_type_addr == coin_addr, ECOINS_NOT_MATCH);

        let user_account = borrow_global<Account>(user);
        let resource_account = account::create_signer_with_capability(&user_account.signer_cap);

        let resource_account_address = signer::address_of(&resource_account);

        assert!(coin::balance<CoinType>(resource_account_address) >= amount, ENOT_ENOUGH_FUNDS);

        let resource_account_fees = fees::get_resource_account_address();

        aptos_account::transfer_coins<CoinType>(&resource_account, resource_account_fees, amount);        
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

    #[test_only]
    public fun test_init_account(admin: &signer) {
        init_module(admin);
    }
}