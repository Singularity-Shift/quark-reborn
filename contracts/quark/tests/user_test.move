#[test_only]
module quark::account_test {
    use quark::user;
    use quark::admin;
    use std::signer;
    use std::string;
    use std::object::{Self, Object};
    use std::option;
    use aptos_framework::timestamp;
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

    struct TestCoin2 {}

    struct FakeCoin {}

    struct FAController has key {
        mint_ref: MintRef,
        transfer_ref: TransferRef,
    }

    fun init_module(sender: &signer) {
        admin::test_init_admin(sender);
        user::test_init_account(sender);
        admin::init_fees_currency_payment_list(sender);
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

    #[test(aptos_framework = @0x1,quark = @quark, user = @0x2)]
    fun test_create_account(aptos_framework: &signer, quark: &signer, user: &signer) {
        timestamp::set_time_has_started_for_testing(aptos_framework);
        init_module(quark);
        user::create_account(user, string::utf8(b"1234567890"));
    }

    #[test(aptos_framework = @0x1, quark = @quark, user = @0x2)]
    fun test_withdraw_funds_v1(aptos_framework: &signer, quark: &signer, user: &signer) {
        let (burn_cap, mint_cap) = aptos_coin::initialize_for_test(aptos_framework);
        timestamp::set_time_has_started_for_testing(aptos_framework);

        let user_address = signer::address_of(user);

        account::create_account_for_test(user_address);
        coin::register<AptosCoin>(user);

        aptos_coin::mint(aptos_framework, user_address, 20000000);

        init_module(quark);
        mint_coin<TestCoin>(quark, 5000, user);
        user::create_account(user, string::utf8(b"1234567890"));
        let resource_account_address = user::get_resource_account(user_address);

        aptos_account::transfer_coins<TestCoin>(user, resource_account_address, 5000);

        user::withdraw_funds_v1<TestCoin>(user, 2500);
        assert!(coin::balance<TestCoin>(resource_account_address) == 2500, EIS_BALANCE_NOT_EQUAL);
        coin::destroy_burn_cap(burn_cap);
        coin::destroy_mint_cap(mint_cap);
    }

    #[test(aptos_framework = @0x1, quark = @quark, user = @0x2)]
    fun test_withdraw_funds_v2(aptos_framework: &signer, quark: &signer, user: &signer) acquires FAController {
        let (burn_cap, mint_cap) = aptos_coin::initialize_for_test(aptos_framework);
        timestamp::set_time_has_started_for_testing(aptos_framework);

        let user_address = signer::address_of(user);

        account::create_account_for_test(user_address);
        coin::register<AptosCoin>(user);

        aptos_coin::mint(aptos_framework, user_address, 20000000);

        init_module(quark);
        user::create_account(user, string::utf8(b"1234567890"));
        
        let resource_account_address = user::get_resource_account(user_address);

        let fa_obj = create_fa();
        let fa_addr = object::object_address(&fa_obj);
        let fa_controller = borrow_global<FAController>(fa_addr);
        let fa_metadata = object::address_to_object<Metadata>(fa_addr);

        mint_fa(user, &fa_controller.mint_ref, 5000);

        aptos_account::transfer_fungible_assets(user, fa_metadata, resource_account_address, 5000);

        user::withdraw_funds_v2(user, 2500, fa_addr);
        assert!(primary_fungible_store::balance(resource_account_address, fa_metadata) == 2500, EIS_BALANCE_NOT_EQUAL);
        coin::destroy_burn_cap(burn_cap);
        coin::destroy_mint_cap(mint_cap);
    }

    #[test(aptos_framework = @0x1, sshift_gpt = @sshift_gpt, quark = @quark, admin = @0x2, reviewer = @0x3, user = @0x4)]
    fun test_pay_ai(aptos_framework: &signer, sshift_gpt: &signer, quark: &signer, admin: &signer, reviewer: &signer, user: &signer) {
        let (burn_cap, mint_cap) = aptos_coin::initialize_for_test(aptos_framework);
        timestamp::set_time_has_started_for_testing(aptos_framework);

        let user_address = signer::address_of(user);

        account::create_account_for_test(user_address);
        coin::register<AptosCoin>(user);

        mint_coin<TestCoin>(quark, 5000, user);

        init_module(quark);
        user::create_account(user, string::utf8(b"1234567890"));
        let resource_account_address = user::get_resource_account(user_address);

        aptos_account::transfer_coins<TestCoin>(user, resource_account_address, 5000);

        user::set_coin_address<TestCoin>(quark);

        create_resource_account(sshift_gpt, admin);

        let resource_account_fees = fees::get_resource_account_address();

        admin::set_reviewer_pending_admin(quark, signer::address_of(reviewer));
        admin::accept_reviewer_pending_admin(reviewer);

        user::pay_ai<TestCoin>(quark, reviewer, user_address, 1000);
        assert!(coin::balance<TestCoin>(resource_account_address) == 4000, EIS_BALANCE_NOT_EQUAL);
        assert!(coin::balance<TestCoin>(resource_account_fees) == 1000, EIS_BALANCE_NOT_EQUAL);
        coin::destroy_burn_cap(burn_cap);
        coin::destroy_mint_cap(mint_cap);
    }

    #[test(aptos_framework = @0x1, sshift_gpt = @sshift_gpt, quark = @quark, admin = @0x2, reviewer = @0x3, user = @0x4)]
    fun test_pay_ai_v2(aptos_framework: &signer, sshift_gpt: &signer, quark: &signer, admin: &signer, reviewer: &signer, user: &signer) acquires FAController {
        let (burn_cap, mint_cap) = aptos_coin::initialize_for_test(aptos_framework);
        timestamp::set_time_has_started_for_testing(aptos_framework);

        let user_address = signer::address_of(user);

        account::create_account_for_test(user_address);
        coin::register<AptosCoin>(user);

        let fa_obj = create_fa();
        let fa_addr = object::object_address(&fa_obj);
        let fa_controller = borrow_global<FAController>(fa_addr);
        let fa_metadata = object::address_to_object<Metadata>(fa_addr);

        mint_fa(user, &fa_controller.mint_ref, 5000);

        init_module(quark);
        create_resource_account(sshift_gpt, admin);

        user::create_account(user, string::utf8(b"1234567890"));

        admin::set_pending_admin(quark, signer::address_of(admin));
        admin::accept_admin(admin);

        admin::set_reviewer_pending_admin(quark, signer::address_of(reviewer));
        admin::accept_reviewer_pending_admin(reviewer);

        admin::add_fees_currency_v2_payment_list(admin, fa_addr);

        let resource_account_address = user::get_resource_account(user_address);

        aptos_account::transfer_fungible_assets(user, fa_metadata, resource_account_address, 2000);

        user::pay_ai_v2(admin, reviewer, user_address, 1000, fa_addr);

        assert!(primary_fungible_store::balance(resource_account_address, fa_metadata) == 1000, EIS_BALANCE_NOT_EQUAL);
        coin::destroy_burn_cap(burn_cap);
        coin::destroy_mint_cap(mint_cap);
    }

    #[test(aptos_framework = @0x1, sshift_gpt = @sshift_gpt, quark = @quark, admin = @0x2, reviewer = @0x3, user = @0x4)]
    fun test_pay_ai_v1_with_token_from_list(aptos_framework: &signer, sshift_gpt: &signer, quark: &signer, admin: &signer, reviewer: &signer, user: &signer) {
        let (burn_cap, mint_cap) = aptos_coin::initialize_for_test(aptos_framework);
        timestamp::set_time_has_started_for_testing(aptos_framework);
        
        let user_address = signer::address_of(user);

        account::create_account_for_test(user_address);
        coin::register<AptosCoin>(user);

        mint_coin<TestCoin>(quark, 5000, user);

        init_module(quark);
        user::create_account(user, string::utf8(b"1234567890"));

        admin::set_pending_admin(quark, signer::address_of(admin));
        admin::accept_admin(admin);

        admin::set_reviewer_pending_admin(quark, signer::address_of(reviewer));
        admin::accept_reviewer_pending_admin(reviewer);

        admin::add_fees_currency_v1_payment_list<TestCoin>(admin);

        user::set_coin_address<AptosCoin>(admin);

        create_resource_account(sshift_gpt, admin);

        let resource_account_fees = fees::get_resource_account_address();

        let resource_account_address = user::get_resource_account(user_address);

        aptos_account::transfer_coins<TestCoin>(user, resource_account_address, 5000);

        user::pay_ai<TestCoin>(admin, reviewer, user_address, 1000);

        assert!(coin::balance<TestCoin>(resource_account_address) == 4000, EIS_BALANCE_NOT_EQUAL);
        assert!(coin::balance<TestCoin>(resource_account_fees) == 1000, EIS_BALANCE_NOT_EQUAL);
        coin::destroy_burn_cap(burn_cap);
        coin::destroy_mint_cap(mint_cap);
    }

    #[test(aptos_framework = @0x1, quark = @quark, reviewer = @0x2, user = @0x3, user2 = @0x4, user3 = @0x5)]
    fun test_pay_to_users_v1(aptos_framework: &signer, quark: &signer, reviewer: &signer, user: &signer, user2: &signer, user3: &signer) {
        let (burn_cap, mint_cap) = aptos_coin::initialize_for_test(aptos_framework);
        timestamp::set_time_has_started_for_testing(aptos_framework);

        let user_address = signer::address_of(user);
        let user2_address = signer::address_of(user2);
        let user3_address = signer::address_of(user3);

        account::create_account_for_test(user_address);
        account::create_account_for_test(user2_address);
        account::create_account_for_test(user3_address);

        coin::register<AptosCoin>(user);

        mint_coin<TestCoin>(quark, 5000, user);

        init_module(quark);
        user::create_account(user, string::utf8(b"1234567890"));
        user::create_account(user2, string::utf8(b"1234567891"));
        user::create_account(user3, string::utf8(b"1234567892"));

        let resource_account_address = user::get_resource_account(user_address);

        admin::set_reviewer_pending_admin(quark, signer::address_of(reviewer));
        admin::accept_reviewer_pending_admin(reviewer);

        aptos_account::transfer_coins<TestCoin>(user, resource_account_address, 5000);

        user::set_coin_address<TestCoin>(quark);

        user::pay_to_users_v1<TestCoin>(quark, reviewer, user_address, 1000, vector[user2_address, user3_address]);

        assert!(coin::balance<TestCoin>(resource_account_address) == 4000, EIS_BALANCE_NOT_EQUAL);
        assert!(coin::balance<TestCoin>(user2_address) == 500, EIS_BALANCE_NOT_EQUAL);
        assert!(coin::balance<TestCoin>(user3_address) == 500, EIS_BALANCE_NOT_EQUAL);
        coin::destroy_burn_cap(burn_cap);
        coin::destroy_mint_cap(mint_cap);
    }

    #[test(aptos_framework = @0x1, quark = @quark, reviewer=@0x2, user = @0x3, user2 = @0x4, user3 = @0x5)]
    fun test_pay_to_users_v2(aptos_framework: &signer, quark: &signer, reviewer: &signer, user: &signer, user2: &signer, user3: &signer) acquires FAController {
        let (burn_cap, mint_cap) = aptos_coin::initialize_for_test(aptos_framework);
        timestamp::set_time_has_started_for_testing(aptos_framework);

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

        init_module(quark);
        user::create_account(user, string::utf8(b"1234567890"));
        user::create_account(user2, string::utf8(b"1234567891"));
        user::create_account(user3, string::utf8(b"1234567892"));

        let resource_account_address = user::get_resource_account(user_address);

        admin::set_reviewer_pending_admin(quark, signer::address_of(reviewer));
        admin::accept_reviewer_pending_admin(reviewer);

        aptos_account::transfer_fungible_assets(user, fa_metadata, resource_account_address, 5000);

        user::pay_to_users_v2(quark, reviewer, user_address, 1000, fa_addr, vector[user2_address, user3_address]);

        assert!(primary_fungible_store::balance(resource_account_address, fa_metadata) == 4000, EIS_BALANCE_NOT_EQUAL);
        assert!(primary_fungible_store::balance(user2_address, fa_metadata) == 500, EIS_BALANCE_NOT_EQUAL);
        assert!(primary_fungible_store::balance(user3_address, fa_metadata) == 500, EIS_BALANCE_NOT_EQUAL);
        coin::destroy_burn_cap(burn_cap);
        coin::destroy_mint_cap(mint_cap);
    }
    
    #[test(quark = @quark, user = @0x2, user2 = @0x3)]
    #[expected_failure(abort_code = 1, location = user)]
    fun test_not_admin_should_not_set_coin_address(quark: &signer, user: &signer) {
        init_module(quark);
        user::set_coin_address<TestCoin>(user);
    }

    #[test(aptos_framework = @0x1, sshift_gpt = @sshift_gpt, quark = @quark, admin = @0x2, reviewer = @0x3, user = @0x4, user2 = @0x5)]
    #[expected_failure(abort_code = 1, location = user)]
    fun test_not_admin_should_not_pay_ai(aptos_framework: &signer, sshift_gpt: &signer, quark: &signer, admin: &signer, reviewer: &signer, user: &signer, user2: &signer) {
        let (burn_cap, mint_cap) = aptos_coin::initialize_for_test(aptos_framework);
        timestamp::set_time_has_started_for_testing(aptos_framework);

        let user_address = signer::address_of(user);

        account::create_account_for_test(user_address);
        coin::register<AptosCoin>(user);

        mint_coin<TestCoin>(quark, 5000, user);

        init_module(quark);
        user::create_account(user, string::utf8(b"1234567890"));

        let resource_account_address = user::get_resource_account(user_address);

        admin::set_reviewer_pending_admin(quark, signer::address_of(reviewer));
        admin::accept_reviewer_pending_admin(reviewer);

        aptos_account::transfer_coins<TestCoin>(user, resource_account_address, 5000);

        user::set_coin_address<TestCoin>(quark);

        create_resource_account(sshift_gpt, admin);

        user::pay_ai<TestCoin>(user2, reviewer, user_address, 1000);

        coin::destroy_burn_cap(burn_cap);
        coin::destroy_mint_cap(mint_cap);
    }

    #[test(aptos_framework = @0x1, sshift_gpt = @sshift_gpt, quark = @quark, admin = @0x2, reviewer = @0x3, user = @0x4)]
    #[expected_failure(abort_code = 5, location = user)]
    fun test_should_not_user_pay_ai_v1_with_fees_currency_not_in_list(aptos_framework: &signer, sshift_gpt: &signer, quark: &signer, admin: &signer, reviewer: &signer, user: &signer) {
        let (burn_cap, mint_cap) = aptos_coin::initialize_for_test(aptos_framework);
        timestamp::set_time_has_started_for_testing(aptos_framework);

        let user_address = signer::address_of(user);

        account::create_account_for_test(user_address);
        coin::register<AptosCoin>(user);

        mint_coin<FakeCoin>(quark, 5000, user);

        init_module(quark);
        user::create_account(user, string::utf8(b"1234567890"));

        admin::set_reviewer_pending_admin(quark, signer::address_of(reviewer));
        admin::accept_reviewer_pending_admin(reviewer);

        admin::set_pending_admin(quark, signer::address_of(admin));
        admin::accept_admin(admin);

        user::set_coin_address<AptosCoin>(admin);
        admin::add_fees_currency_v1_payment_list<AptosCoin>(admin);

        create_resource_account(sshift_gpt, admin);

        let user_resource_account_address = user::get_resource_account(user_address);

        aptos_account::transfer_coins<FakeCoin>(user, user_resource_account_address, 1000);

        user::pay_ai<FakeCoin>(admin, reviewer, user_address, 1000);

        coin::destroy_burn_cap(burn_cap);
        coin::destroy_mint_cap(mint_cap);
    }

    #[test(aptos_framework = @0x1, quark = @quark, admin = @0x2, reviewer = @0x3, user = @0x4)]
    #[expected_failure(abort_code = 5, location = user)]
    fun test_should_not_user_pay_ai_v2_with_fees_currency_not_in_list(aptos_framework: &signer, quark: &signer, admin: &signer, reviewer: &signer, user: &signer) acquires FAController {
        let (burn_cap, mint_cap) = aptos_coin::initialize_for_test(aptos_framework);
        timestamp::set_time_has_started_for_testing(aptos_framework);

        let user_address = signer::address_of(user);

        account::create_account_for_test(user_address);
        coin::register<AptosCoin>(user);

        let fa_obj = create_fa();
        let fa_addr = object::object_address(&fa_obj);
        let fa_controller = borrow_global<FAController>(fa_addr);

        let fa_obj2 = create_fa();
        let fa_addr2 = object::object_address(&fa_obj2);
        let fa_controller2 = borrow_global<FAController>(fa_addr2);
        let fa_metadata2 = object::address_to_object<Metadata>(fa_addr2);

        mint_fa(user, &fa_controller.mint_ref, 5000);
        mint_fa(user, &fa_controller2.mint_ref, 5000);

        init_module(quark);
        user::create_account(user, string::utf8(b"1234567890"));

        admin::set_pending_admin(quark, signer::address_of(admin));
        admin::accept_admin(admin);

        admin::set_reviewer_pending_admin(quark, signer::address_of(reviewer));
        admin::accept_reviewer_pending_admin(reviewer);

        admin::add_fees_currency_v2_payment_list(admin, fa_addr);

        let resource_account_address = user::get_resource_account(user_address);

        aptos_account::transfer_fungible_assets(user, fa_metadata2, resource_account_address, 2000);

        user::pay_ai_v2(admin, reviewer, user_address, 1000, fa_addr2);

        coin::destroy_burn_cap(burn_cap);
        coin::destroy_mint_cap(mint_cap);
    }

    #[test(aptos_framework = @0x1, quark = @quark, reviewer = @0x2, user = @0x3, user2 = @0x4, user3 = @0x5)]
    #[expected_failure(abort_code = 1, location = user)]
    fun test_not_admin_should_not_pay_to_users_v1(aptos_framework: &signer, quark: &signer, reviewer: &signer, user: &signer, user2: &signer, user3: &signer) {
        let (burn_cap, mint_cap) = aptos_coin::initialize_for_test(aptos_framework);
        timestamp::set_time_has_started_for_testing(aptos_framework);

        let user_address = signer::address_of(user);
        let user2_address = signer::address_of(user2);
        let user3_address = signer::address_of(user3);

        account::create_account_for_test(user_address);
        account::create_account_for_test(user2_address);
        account::create_account_for_test(user3_address);

        coin::register<AptosCoin>(user);

        mint_coin<TestCoin>(quark, 5000, user);

        init_module(quark);
        user::create_account(user, string::utf8(b"1234567890"));
        user::create_account(user2, string::utf8(b"1234567891"));
        user::create_account(user3, string::utf8(b"1234567892"));

        let resource_account_address = user::get_resource_account(user_address);

        admin::set_reviewer_pending_admin(quark, signer::address_of(reviewer));
        admin::accept_reviewer_pending_admin(reviewer);

        aptos_account::transfer_coins<TestCoin>(user, resource_account_address, 5000);

        user::set_coin_address<TestCoin>(quark);

        user::pay_to_users_v1<TestCoin>(user2, reviewer, user_address, 1000, vector[user_address, user3_address]);

        coin::destroy_burn_cap(burn_cap);
        coin::destroy_mint_cap(mint_cap);
    }

    #[test(aptos_framework = @0x1, quark = @quark, reviewer = @0x2, user = @0x3, user2 = @0x4, user3 = @0x5)]
    #[expected_failure(abort_code = 1, location = user)]
    fun test_not_admin_should_not_pay_to_users_v2(aptos_framework: &signer, quark: &signer, reviewer: &signer, user: &signer, user2: &signer, user3: &signer) acquires FAController {
        let (burn_cap, mint_cap) = aptos_coin::initialize_for_test(aptos_framework);
        timestamp::set_time_has_started_for_testing(aptos_framework);

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

        init_module(quark);
        user::create_account(user, string::utf8(b"1234567890"));
        user::create_account(user2, string::utf8(b"1234567891"));
        user::create_account(user3, string::utf8(b"1234567892"));

        let resource_account_address = user::get_resource_account(user_address);

        admin::set_reviewer_pending_admin(quark, signer::address_of(reviewer));
        admin::accept_reviewer_pending_admin(reviewer);

        aptos_account::transfer_fungible_assets(user, fa_metadata, resource_account_address, 5000);

        user::pay_to_users_v2(user2, reviewer, user_address, 1000, fa_addr, vector[user_address, user3_address]);

        coin::destroy_burn_cap(burn_cap);
        coin::destroy_mint_cap(mint_cap);
    }

    #[test(aptos_framework = @0x1, sshift_gpt = @sshift_gpt, quark = @quark, admin = @0x2, reviewer = @0x3, user = @0x4, user2 = @0x5)]
    #[expected_failure(abort_code = 2, location = user)]
    fun test_with_fake_reviewer_should_not_pay_ai(aptos_framework: &signer, sshift_gpt: &signer, quark: &signer, admin: &signer, reviewer: &signer, user: &signer, user2: &signer) {
        let (burn_cap, mint_cap) = aptos_coin::initialize_for_test(aptos_framework);
        timestamp::set_time_has_started_for_testing(aptos_framework);

        let user_address = signer::address_of(user);

        account::create_account_for_test(user_address);
        coin::register<AptosCoin>(user);

        mint_coin<TestCoin>(quark, 5000, user);

        init_module(quark);
        user::create_account(user, string::utf8(b"1234567890"));

        let resource_account_address = user::get_resource_account(user_address);

        admin::set_reviewer_pending_admin(quark, signer::address_of(reviewer));
        admin::accept_reviewer_pending_admin(reviewer);

        aptos_account::transfer_coins<TestCoin>(user, resource_account_address, 5000);

        user::set_coin_address<TestCoin>(quark);

        create_resource_account(sshift_gpt, admin);

        user::pay_ai<TestCoin>(quark, user2, user_address, 1000);

        coin::destroy_burn_cap(burn_cap);
        coin::destroy_mint_cap(mint_cap);
    }

    #[test(aptos_framework = @0x1, quark = @quark, reviewer = @0x2, user = @0x3, user2 = @0x4, user3 = @0x5)]
    #[expected_failure(abort_code = 2, location = user)]
    fun test_with_fake_reviewer_should_not_pay_to_users_v1(aptos_framework: &signer, quark: &signer, reviewer: &signer, user: &signer, user2: &signer, user3: &signer) {
        let (burn_cap, mint_cap) = aptos_coin::initialize_for_test(aptos_framework);
        timestamp::set_time_has_started_for_testing(aptos_framework);

        let user_address = signer::address_of(user);
        let user2_address = signer::address_of(user2);
        let user3_address = signer::address_of(user3);

        account::create_account_for_test(user_address);
        account::create_account_for_test(user2_address);
        account::create_account_for_test(user3_address);

        coin::register<AptosCoin>(user);

        mint_coin<TestCoin>(quark, 5000, user);

        init_module(quark);
        user::create_account(user, string::utf8(b"1234567890"));
        user::create_account(user2, string::utf8(b"1234567891"));
        user::create_account(user3, string::utf8(b"1234567892"));

        let resource_account_address = user::get_resource_account(user_address);

        admin::set_reviewer_pending_admin(quark, signer::address_of(reviewer));
        admin::accept_reviewer_pending_admin(reviewer);

        aptos_account::transfer_coins<TestCoin>(user, resource_account_address, 5000);

        user::set_coin_address<TestCoin>(quark);

        user::pay_to_users_v1<TestCoin>(quark, user2, user_address, 1000, vector[user_address, user3_address]);

        coin::destroy_burn_cap(burn_cap);
        coin::destroy_mint_cap(mint_cap);
    }

    #[test(aptos_framework = @0x1, quark = @quark, reviewer = @0x2, user = @0x3, user2 = @0x4, user3 = @0x5)]
    #[expected_failure(abort_code = 2, location = user)]
    fun test_with_fake_reviewer_should_not_pay_to_users_v2(aptos_framework: &signer, quark: &signer, reviewer: &signer, user: &signer, user2: &signer, user3: &signer) acquires FAController {
        let (burn_cap, mint_cap) = aptos_coin::initialize_for_test(aptos_framework);
        timestamp::set_time_has_started_for_testing(aptos_framework);

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

        init_module(quark);
        user::create_account(user, string::utf8(b"1234567890"));
        user::create_account(user2, string::utf8(b"1234567891"));
        user::create_account(user3, string::utf8(b"1234567892"));

        let resource_account_address = user::get_resource_account(user_address);

        admin::set_reviewer_pending_admin(quark, signer::address_of(reviewer));
        admin::accept_reviewer_pending_admin(reviewer);

        aptos_account::transfer_fungible_assets(user, fa_metadata, resource_account_address, 5000);

        user::pay_to_users_v2(quark, user2, user_address, 1000, fa_addr, vector[user_address, user3_address]);

        coin::destroy_burn_cap(burn_cap);
        coin::destroy_mint_cap(mint_cap);
    }
}