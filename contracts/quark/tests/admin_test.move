#[test_only]
module quark_test::admin_test {
    use quark_test::admin_v2;
    use std::signer;

    const EIS_NOT_ADMIN: u64 = 1;
    const EIS_NOT_PENDING_ADMIN: u64 = 2;
    const EIS_PENDING_ADMIN: u64 = 3;

    fun init_module(sender: &signer) {
        admin_v2::test_init_admin(sender);
    }

    #[test(quark_test = @quark_test, admin = @0x2)]
    fun test_set_pending_admin(quark_test: &signer, admin: &signer) {
        init_module(quark_test);
        assert!(admin_v2::is_admin(signer::address_of(quark_test)), EIS_NOT_ADMIN);
        admin_v2::set_pending_admin(quark_test, signer::address_of(admin));
        assert!(admin_v2::is_pending_admin(signer::address_of(admin)), EIS_NOT_PENDING_ADMIN);
    }

    #[test(quark_test = @quark_test, admin = @0x2)]
    fun test_accept_admin(quark_test: &signer, admin: &signer) {
        init_module(quark_test);
        admin_v2::set_pending_admin(quark_test, signer::address_of(admin));
        assert!(admin_v2::is_pending_admin(signer::address_of(admin)), EIS_NOT_PENDING_ADMIN);
        admin_v2::accept_admin(admin);
    }

    #[test(quark_test = @quark_test, admin = @0x2, user = @0x3)]
    #[expected_failure(abort_code = 2, location = admin_v2)]
    fun test_should_not_pending_admin_accept_admin(quark_test: &signer, admin: &signer, user: &signer) {
        init_module(quark_test);
        admin_v2::set_pending_admin(quark_test, signer::address_of(admin));
        admin_v2::accept_admin(user);
    }

    #[test(quark_test = @quark_test, admin = @0x2, user = @0x3)]
    #[expected_failure(abort_code = 1, location = admin_v2)]
    fun test_should_not_admin_set_pending_admin(quark_test: &signer, admin: &signer, user: &signer) {
        init_module(quark_test);
        admin_v2::set_pending_admin(admin, signer::address_of(user));
    }
}