#[test_only]
module quark::admin_test {
    use quark::admin;
    use std::signer;
    use std::type_info;
    use std::object::{Self, Object};
    use std::string;
    use std::option;
    use aptos_framework::fungible_asset::{Self, MintRef, TransferRef, Metadata};
    use aptos_framework::primary_fungible_store;

    const EIS_NOT_ADMIN: u64 = 1;
    const EIS_NOT_PENDING_ADMIN: u64 = 2;
    const EIS_PENDING_ADMIN: u64 = 3;
    const EIS_NOT_REVIEWER_PENDING_ADMIN: u64 = 4;
    const EIS_NOT_REVIEWER: u64 = 5;
    const ECOIN_NOT_FOUND: u64 = 6;
    const ECOIN_SHOULD_NOT_BE_IN_LIST: u64 = 7;

    struct TestCoin {}

    struct TestCoin2 {}

    struct FAController has key {
        mint_ref: MintRef,
        transfer_ref: TransferRef,
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

    fun init_module(sender: &signer) {
        admin::test_init_admin(sender);
    }

    #[test(quark = @quark, admin = @0x2)]
    fun test_set_pending_admin(quark: &signer, admin: &signer) {
        init_module(quark);
        assert!(admin::is_admin(signer::address_of(quark)), EIS_NOT_ADMIN);
        admin::set_pending_admin(quark, signer::address_of(admin));
        assert!(admin::is_pending_admin(signer::address_of(admin)), EIS_NOT_PENDING_ADMIN);
    }

    #[test(quark = @quark, admin = @0x2)]
    fun test_accept_admin(quark: &signer, admin: &signer) {
        init_module(quark);
        admin::set_pending_admin(quark, signer::address_of(admin));
        assert!(admin::is_pending_admin(signer::address_of(admin)), EIS_NOT_PENDING_ADMIN);
        admin::accept_admin(admin);
    }

    #[test(quark = @quark, reviewer = @0x2)]
    fun test_set_reviewer_pending_admin(quark: &signer, reviewer: &signer) {
        init_module(quark);
        admin::set_reviewer_pending_admin(quark, signer::address_of(reviewer));
        assert!(admin::is_reviewer_pending_admin(signer::address_of(reviewer)), EIS_NOT_REVIEWER_PENDING_ADMIN);
    }

    #[test(quark = @quark, reviewer = @0x2)]
    fun test_accept_reviewer_pending_admin(quark: &signer, reviewer: &signer) {
        init_module(quark);
        admin::set_reviewer_pending_admin(quark, signer::address_of(reviewer));
        assert!(admin::is_reviewer_pending_admin(signer::address_of(reviewer)), EIS_NOT_REVIEWER_PENDING_ADMIN);
        admin::accept_reviewer_pending_admin(reviewer);
    }

    #[test(quark = @quark)]
    fun test_add_fees_currency_v1_payment_list(quark: &signer) {
        init_module(quark);
        admin::init_fees_currency_payment_list(quark);
        admin::add_fees_currency_v1_payment_list<TestCoin>(quark);

        let coin_type = type_info::type_of<TestCoin>();
        let coin_address = type_info::account_address(&coin_type);

        assert!(admin::exist_fees_currency_payment_list(coin_address), ECOIN_NOT_FOUND);
    }

    #[test(quark = @quark)]
    fun test_remove_fees_currency_v1_payment_list(quark: &signer) {
        init_module(quark);
        admin::init_fees_currency_payment_list(quark);
        admin::add_fees_currency_v1_payment_list<TestCoin>(quark);

        let coin_type = type_info::type_of<TestCoin>();
        let coin_address = type_info::account_address(&coin_type);

        admin::remove_fees_currency_v1_payment_list<TestCoin>(quark);

        assert!(!admin::exist_fees_currency_payment_list(coin_address), ECOIN_SHOULD_NOT_BE_IN_LIST);
    }

    #[test(quark = @quark)]
    fun test_add_fees_currency_v2_payment_list(quark: &signer) {
        init_module(quark);
        admin::init_fees_currency_payment_list(quark);
        admin::add_fees_currency_v2_payment_list(quark, @0x1);

        let fa = create_fa();

        admin::add_fees_currency_v2_payment_list(quark, object::object_address(&fa));

        assert!(admin::exist_fees_currency_payment_list(object::object_address(&fa)), ECOIN_NOT_FOUND);
    }

    #[test(quark = @quark)]
    fun test_remove_fees_currency_v2_payment_list(quark: &signer) {
        init_module(quark);
        let fa = create_fa();
        admin::init_fees_currency_payment_list(quark);

        admin::add_fees_currency_v2_payment_list(quark, object::object_address(&fa));

        admin::remove_fees_currency_v2_payment_list(quark, object::object_address(&fa));

        assert!(!admin::exist_fees_currency_payment_list(object::object_address(&fa)), ECOIN_SHOULD_NOT_BE_IN_LIST);
    }

    #[test(quark = @quark, fake_admin = @0x2)]
    #[expected_failure(abort_code = 1, location = admin)]
    fun test_should_not_admin_add_fees_currency_v1_payment_list(quark: &signer, fake_admin: &signer) {
        init_module(quark);
        
        admin::add_fees_currency_v1_payment_list<TestCoin>(fake_admin);
    }

    #[test(quark = @quark, fake_admin = @0x2)]
    #[expected_failure(abort_code = 1, location = admin)]
    fun test_should_not_admin_add_fees_currency_v2_payment_list(quark: &signer, fake_admin: &signer) {
        init_module(quark);
        admin::add_fees_currency_v2_payment_list(fake_admin, @0x1);
    }

    #[test(quark = @quark, fake_admin = @0x2)]
    #[expected_failure(abort_code = 1, location = admin)]
    fun test_should_not_admin_remove_fees_currency_v1_payment_list(quark: &signer, fake_admin: &signer) {
        init_module(quark);

        admin::init_fees_currency_payment_list(quark);
        admin::add_fees_currency_v1_payment_list<TestCoin>(quark);

        admin::remove_fees_currency_v1_payment_list<TestCoin2>(fake_admin);
    }

    #[test(quark = @quark, fake_admin = @0x2)]
    #[expected_failure(abort_code = 1, location = admin)]
    fun test_should_not_admin_remove_fees_currency_v2_payment_list(quark: &signer, fake_admin: &signer) {
        init_module(quark);

        let fa = create_fa();

        admin::init_fees_currency_payment_list(quark);

        admin::add_fees_currency_v2_payment_list(quark, object::object_address(&fa));

        admin::remove_fees_currency_v2_payment_list(fake_admin, object::object_address(&fa));
    }

    #[test(quark = @quark, admin = @0x2)]
    #[expected_failure(abort_code = 6, location = admin)]
    fun test_should_not_owner_init_fees_currency_payment_list(quark: &signer, admin: &signer) {
        init_module(quark);

        admin::set_pending_admin(quark, signer::address_of(admin));
        admin::accept_admin(admin);

        admin::init_fees_currency_payment_list(admin);
    }

    #[test(quark = @quark, admin = @0x2, user = @0x3)]
    #[expected_failure(abort_code = 2, location = admin)]
    fun test_should_not_pending_admin_accept_admin(quark: &signer, admin: &signer, user: &signer) {
        init_module(quark);
        admin::set_pending_admin(quark, signer::address_of(admin));
        admin::accept_admin(user);
    }

    #[test(quark = @quark, admin = @0x2, user = @0x3)]
    #[expected_failure(abort_code = 1, location = admin)]
    fun test_should_not_admin_set_pending_admin(quark: &signer, admin: &signer, user: &signer) {
        init_module(quark);
        admin::set_pending_admin(admin, signer::address_of(user));
    }
}