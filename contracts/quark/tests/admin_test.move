#[test_only]
module quark::admin_test {
    use quark::admin;
    use std::signer;

    const EIS_NOT_ADMIN: u64 = 1;
    const EIS_NOT_PENDING_ADMIN: u64 = 2;
    const EIS_PENDING_ADMIN: u64 = 3;
    const EIS_NOT_REVIEWER_PENDING_ADMIN: u64 = 4;
    const EIS_NOT_REVIEWER: u64 = 5;

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