module quark_test::admin_v3 {
    use std::signer;
    use std::option::{Self, Option};

    struct Admin has key {
        account: address,
        pending_admin: Option<address>,
    }

    const ONLY_ADMIN_CAN_CALL: u64 = 1;
    const ONLY_PENDING_ADMIN_CAN_CALL: u64 = 2;

    fun init_module(sender: &signer) {
        let account = signer::address_of(sender);
        move_to(sender, Admin { account, pending_admin: option::none() });
    }

    public entry fun set_pending_admin(sender: &signer, new_admin: address) acquires Admin {
        let account = signer::address_of(sender);
        assert!(is_admin(account), ONLY_ADMIN_CAN_CALL);
        let admin = borrow_global_mut<Admin>(@quark_test);
        admin.pending_admin = option::some(new_admin);
    }

    public entry fun accept_admin(sender: &signer) acquires Admin {
        let account = signer::address_of(sender);
        assert!(is_pending_admin(account), ONLY_PENDING_ADMIN_CAN_CALL);
        let admin = borrow_global_mut<Admin>(@quark_test);
        admin.account = account;
        admin.pending_admin = option::none();
    }

    #[view]
    public fun is_admin(account: address): bool acquires Admin {
        let admin = borrow_global<Admin>(@quark_test);
        if (account == admin.account) {
            return true;
        };
        false
    }

    #[view]
    public fun is_pending_admin(account: address): bool acquires Admin {
        let admin = borrow_global<Admin>(@quark_test);
        if (option::is_some(&admin.pending_admin)) {
            let pending_admin = option::borrow(&admin.pending_admin);
            if (pending_admin == &account) {
                return true;
            }
        };
        false
    }

    #[view]
    public fun get_pending_admin(): address acquires Admin {
        let admin = borrow_global<Admin>(@quark_test);
        *option::borrow(&admin.pending_admin)
    }

    #[view]
    public fun get_admin(): address acquires Admin {
        let admin = borrow_global<Admin>(@quark_test);
        admin.account
    }

    #[test_only]
    public fun test_init_admin(sender: &signer) {
        init_module(sender);
    }
}