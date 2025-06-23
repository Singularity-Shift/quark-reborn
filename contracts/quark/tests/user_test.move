#[test_only]
module quark_test::account_test {
    use quark_test::user_v3;
    use quark_test::admin_v3;
    use std::signer;
    use std::string;
    use std::object::{Self, Object};
    use std::option;
    use aptos_framework::account;
    use aptos_framework::coin;
    use aptos_framework::aptos_coin::{Self, AptosCoin};
    use aptos_framework::aptos_account;
    use aptos_framework::fungible_asset::{Self, MintRef, TransferRef, Metadata};
    use aptos_framework::primary_fungible_store;
    use sshift_gpt::fees;

    const EIS_NOT_ADMIN: u64 = 1;
    const EIS_BALANCE_NOT_EQUAL: u64 = 2;

    struct TestCoin {}

    struct FAController has key {
        mint_ref: MintRef,
        transfer_ref: TransferRef,
    }

    fun init_module(sender: &signer) {
        admin_v3::test_init_admin(sender);
        user_v3::test_init_account(sender);
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

    #[test_only]
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

    #[test_only]
    fun mint_fa(sender: &signer, mint_ref: &MintRef, amount: u64) {
        let account_addr = signer::address_of(sender);

        primary_fungible_store::mint(mint_ref, account_addr, amount);
    }

    #[test(quark_test = @quark_test, user = @0x2)]
    fun test_create_account(quark_test: &signer, user: &signer) {
        init_module(quark_test);
        user_v3::create_account(user, string::utf8(b"1234567890"));
    }

    #[test(aptos_framework = @0x1, quark_test = @quark_test, user = @0x2)]
    fun test_withdraw_funds_v1(aptos_framework: &signer, quark_test: &signer, user: &signer) {
        let (burn_cap, mint_cap) = aptos_coin::initialize_for_test(aptos_framework);

        let user_address = signer::address_of(user);

        account::create_account_for_test(user_address);
        coin::register<AptosCoin>(user);

        aptos_coin::mint(aptos_framework, user_address, 20000000);

        init_module(quark_test);
        mint_coin<TestCoin>(quark_test, 5000, user);
        user_v3::create_account(user, string::utf8(b"1234567890"));
        let resource_account_address = user_v3::get_resource_account(user_address);

        aptos_account::transfer_coins<TestCoin>(user, resource_account_address, 5000);

        user_v3::withdraw_funds_v1<TestCoin>(user, 2500);
        assert!(coin::balance<TestCoin>(resource_account_address) == 2500, EIS_BALANCE_NOT_EQUAL);
        coin::destroy_burn_cap(burn_cap);
        coin::destroy_mint_cap(mint_cap);
    }

    #[test(aptos_framework = @0x1, quark_test = @quark_test, user = @0x2)]
    fun test_withdraw_funds_v2(aptos_framework: &signer, quark_test: &signer, user: &signer) acquires FAController {
        let (burn_cap, mint_cap) = aptos_coin::initialize_for_test(aptos_framework);

        let user_address = signer::address_of(user);

        account::create_account_for_test(user_address);
        coin::register<AptosCoin>(user);

        aptos_coin::mint(aptos_framework, user_address, 20000000);

        init_module(quark_test);
        user_v3::create_account(user, string::utf8(b"1234567890"));
        
        let resource_account_address = user_v3::get_resource_account(user_address);

        let fa_obj = create_fa();
        let fa_addr = object::object_address(&fa_obj);
        let fa_controller = borrow_global<FAController>(fa_addr);
        let fa_metadata = object::address_to_object<Metadata>(fa_addr);

        mint_fa(user, &fa_controller.mint_ref, 5000);

        aptos_account::transfer_fungible_assets(user, fa_metadata, resource_account_address, 5000);

        user_v3::withdraw_funds_v2(user, 2500, fa_addr);
        assert!(primary_fungible_store::balance(resource_account_address, fa_metadata) == 2500, EIS_BALANCE_NOT_EQUAL);
        coin::destroy_burn_cap(burn_cap);
        coin::destroy_mint_cap(mint_cap);
    }

    #[test(aptos_framework = @0x1, quark_test = @quark_test, admin = @0x2, user = @0x3)]
    fun test_pay_ai(aptos_framework: &signer, quark_test: &signer, admin: &signer, user: &signer) {
        let (burn_cap, mint_cap) = aptos_coin::initialize_for_test(aptos_framework);

        let user_address = signer::address_of(user);

        account::create_account_for_test(user_address);
        coin::register<AptosCoin>(user);

        mint_coin<TestCoin>(quark_test, 5000, user);

        init_module(quark_test);
        user_v3::create_account(user, string::utf8(b"1234567890"));
        let resource_account_address = user_v3::get_resource_account(user_address);

        aptos_account::transfer_coins<TestCoin>(user, resource_account_address, 5000);

        user_v3::set_coin_address<TestCoin>(quark_test);

        create_resource_account(quark_test, admin);

        let resource_account_fees = fees::get_resource_account_address();

        user_v3::pay_ai<TestCoin>(quark_test, user_address, 1000);
        assert!(coin::balance<TestCoin>(resource_account_address) == 4000, EIS_BALANCE_NOT_EQUAL);
        assert!(coin::balance<TestCoin>(resource_account_fees) == 1000, EIS_BALANCE_NOT_EQUAL);
        coin::destroy_burn_cap(burn_cap);
        coin::destroy_mint_cap(mint_cap);
    }

    #[test(aptos_framework = @0x1, quark_test = @quark_test, user = @0x2, user2 = @0x3, user3 = @0x4)]
    fun test_pay_to_users_v1(aptos_framework: &signer, quark_test: &signer, user: &signer, user2: &signer, user3: &signer) {
        let (burn_cap, mint_cap) = aptos_coin::initialize_for_test(aptos_framework);

        let user_address = signer::address_of(user);
        let user2_address = signer::address_of(user2);
        let user3_address = signer::address_of(user3);

        account::create_account_for_test(user_address);
        account::create_account_for_test(user2_address);
        account::create_account_for_test(user3_address);

        coin::register<AptosCoin>(user);

        mint_coin<TestCoin>(quark_test, 5000, user);

        init_module(quark_test);
        user_v3::create_account(user, string::utf8(b"1234567890"));
        user_v3::create_account(user2, string::utf8(b"1234567891"));
        user_v3::create_account(user3, string::utf8(b"1234567892"));

        let resource_account_address = user_v3::get_resource_account(user_address);

        aptos_account::transfer_coins<TestCoin>(user, resource_account_address, 5000);

        user_v3::set_coin_address<TestCoin>(quark_test);

        user_v3::pay_to_users_v1<TestCoin>(quark_test, user_address, 1000, vector[user2_address, user3_address]);

        assert!(coin::balance<TestCoin>(resource_account_address) == 3000, EIS_BALANCE_NOT_EQUAL);
        assert!(coin::balance<TestCoin>(user2_address) == 1000, EIS_BALANCE_NOT_EQUAL);
        assert!(coin::balance<TestCoin>(user3_address) == 1000, EIS_BALANCE_NOT_EQUAL);
        coin::destroy_burn_cap(burn_cap);
        coin::destroy_mint_cap(mint_cap);
    }

    #[test(aptos_framework = @0x1, quark_test = @quark_test, user = @0x2, user2 = @0x3, user3 = @0x4)]
    fun test_pay_to_users_v2(aptos_framework: &signer, quark_test: &signer, user: &signer, user2: &signer, user3: &signer) acquires FAController {
        let (burn_cap, mint_cap) = aptos_coin::initialize_for_test(aptos_framework);

        let user_address = signer::address_of(user);
        let user2_address = signer::address_of(user2);
        let user3_address = signer::address_of(user3);

        account::create_account_for_test(user_address);
        account::create_account_for_test(user2_address);
        account::create_account_for_test(user3_address);

        coin::register<AptosCoin>(user);

        let fa_obj = create_fa();
        let fa_addr = object::object_address(&fa_obj);
        let fa_controller = borrow_global<FAController>(fa_addr);
        let fa_metadata = object::address_to_object<Metadata>(fa_addr);

        mint_fa(user, &fa_controller.mint_ref, 5000);

        init_module(quark_test);
        user_v3::create_account(user, string::utf8(b"1234567890"));
        user_v3::create_account(user2, string::utf8(b"1234567891"));
        user_v3::create_account(user3, string::utf8(b"1234567892"));

        let resource_account_address = user_v3::get_resource_account(user_address);

        aptos_account::transfer_fungible_assets(user, fa_metadata, resource_account_address, 5000);

        user_v3::pay_to_users_v2(quark_test, user_address, 1000, fa_addr, vector[user2_address, user3_address]);

        assert!(primary_fungible_store::balance(resource_account_address, fa_metadata) == 3000, EIS_BALANCE_NOT_EQUAL);
        assert!(primary_fungible_store::balance(user2_address, fa_metadata) == 1000, EIS_BALANCE_NOT_EQUAL);
        assert!(primary_fungible_store::balance(user3_address, fa_metadata) == 1000, EIS_BALANCE_NOT_EQUAL);
        coin::destroy_burn_cap(burn_cap);
        coin::destroy_mint_cap(mint_cap);
    }
    
    #[test(quark_test = @quark_test, user = @0x2, user2 = @0x3)]
    #[expected_failure(abort_code = 1, location = user_v3)]
    fun test_not_admin_should_not_set_coin_address(quark_test: &signer, user: &signer) {
        init_module(quark_test);
        user_v3::set_coin_address<TestCoin>(user);
    }

    #[test(aptos_framework = @0x1, quark_test = @quark_test, admin = @0x2, user = @0x3, user2 = @0x4)]
    #[expected_failure(abort_code = 1, location = user_v3)]
    fun test_not_admin_should_not_pay_ai(aptos_framework: &signer, quark_test: &signer, admin: &signer, user: &signer, user2: &signer) {
        let (burn_cap, mint_cap) = aptos_coin::initialize_for_test(aptos_framework);

        let user_address = signer::address_of(user);

        account::create_account_for_test(user_address);
        coin::register<AptosCoin>(user);

        mint_coin<TestCoin>(quark_test, 5000, user);

        init_module(quark_test);
        user_v3::create_account(user, string::utf8(b"1234567890"));

        let resource_account_address = user_v3::get_resource_account(user_address);

        aptos_account::transfer_coins<TestCoin>(user, resource_account_address, 5000);

        user_v3::set_coin_address<TestCoin>(quark_test);

        create_resource_account(quark_test, admin);

        let resource_account_fees = fees::get_resource_account_address();

        user_v3::pay_ai<TestCoin>(user2, user_address, 1000);
        assert!(coin::balance<TestCoin>(resource_account_address) == 4000, EIS_BALANCE_NOT_EQUAL);
        assert!(coin::balance<TestCoin>(resource_account_fees) == 1000, EIS_BALANCE_NOT_EQUAL);
        coin::destroy_burn_cap(burn_cap);
        coin::destroy_mint_cap(mint_cap);
    }

    #[test(aptos_framework = @0x1, quark_test = @quark_test, user = @0x2, user2 = @0x3, user3 = @0x4)]
    #[expected_failure(abort_code = 1, location = user_v3)]
    fun test_not_admin_should_not_pay_to_users_v1(aptos_framework: &signer, quark_test: &signer, user: &signer, user2: &signer, user3: &signer) {
        let (burn_cap, mint_cap) = aptos_coin::initialize_for_test(aptos_framework);

        let user_address = signer::address_of(user);
        let user2_address = signer::address_of(user2);
        let user3_address = signer::address_of(user3);

        account::create_account_for_test(user_address);
        account::create_account_for_test(user2_address);
        account::create_account_for_test(user3_address);

        coin::register<AptosCoin>(user);

        mint_coin<TestCoin>(quark_test, 5000, user);

        init_module(quark_test);
        user_v3::create_account(user, string::utf8(b"1234567890"));
        user_v3::create_account(user2, string::utf8(b"1234567891"));
        user_v3::create_account(user3, string::utf8(b"1234567892"));

        let resource_account_address = user_v3::get_resource_account(user_address);

        aptos_account::transfer_coins<TestCoin>(user, resource_account_address, 5000);

        user_v3::set_coin_address<TestCoin>(quark_test);

        user_v3::pay_to_users_v1<TestCoin>(user2, user_address, 1000, vector[user_address, user3_address]);

        coin::destroy_burn_cap(burn_cap);
        coin::destroy_mint_cap(mint_cap);
    }

    #[test(aptos_framework = @0x1, quark_test = @quark_test, user = @0x3, user2 = @0x4, user3 = @0x5)]
    #[expected_failure(abort_code = 1, location = user_v3)]
    fun test_not_admin_should_not_pay_to_users_v2(aptos_framework: &signer, quark_test: &signer, user: &signer, user2: &signer, user3: &signer) acquires FAController {
        let (burn_cap, mint_cap) = aptos_coin::initialize_for_test(aptos_framework);

        let user_address = signer::address_of(user);
        let user2_address = signer::address_of(user2);
        let user3_address = signer::address_of(user3);

        account::create_account_for_test(user_address);
        account::create_account_for_test(user2_address);
        account::create_account_for_test(user3_address);

        coin::register<AptosCoin>(user);

        let fa_obj = create_fa();
        let fa_addr = object::object_address(&fa_obj);
        let fa_controller = borrow_global<FAController>(fa_addr);
        let fa_metadata = object::address_to_object<Metadata>(fa_addr);

        mint_fa(user, &fa_controller.mint_ref, 5000);

        init_module(quark_test);
        user_v3::create_account(user, string::utf8(b"1234567890"));
        user_v3::create_account(user2, string::utf8(b"1234567891"));
        user_v3::create_account(user3, string::utf8(b"1234567892"));

        let resource_account_address = user_v3::get_resource_account(user_address);

        aptos_account::transfer_fungible_assets(user, fa_metadata, resource_account_address, 5000);

        user_v3::pay_to_users_v2(user2, user_address, 1000, fa_addr, vector[user_address, user3_address]);

        coin::destroy_burn_cap(burn_cap);
        coin::destroy_mint_cap(mint_cap);
    }
}