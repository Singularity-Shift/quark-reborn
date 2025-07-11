module quark_test::admin_v5 {
    use std::signer;
    use std::option::{Self, Option};

    struct Admin has key {
        account: address,
        pending_admin: Option<address>,
        reviewer_account: address,
        reviewer_pending_admin: Option<address>,
    }

    struct Config has key {
        coin_addr: Option<address>,
    }

    const ONLY_ADMIN_CAN_CALL: u64 = 1;
    const ONLY_PENDING_ADMIN_CAN_CALL: u64 = 2;
    const ONLY_REVIEWER_CAN_CALL: u64 = 3;
    const ONLY_REVIEWER_PENDING_ADMIN_CAN_CALL: u64 = 4;

    fun init_module(sender: &signer) {
        let account = signer::address_of(sender);
        move_to(sender, Admin { account, pending_admin: option::none(), reviewer_account: account, reviewer_pending_admin: option::none() });
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

    public entry fun set_reviewer_pending_admin(sender: &signer, new_admin: address) acquires Admin {
        let account = signer::address_of(sender);
        assert!(is_reviewer(account), ONLY_REVIEWER_CAN_CALL);
        let admin = borrow_global_mut<Admin>(@quark_test);
        admin.reviewer_pending_admin = option::some(new_admin);
    }

    public entry fun accept_reviewer_pending_admin(sender: &signer) acquires Admin {
        let account = signer::address_of(sender);
        assert!(is_reviewer_pending_admin(account), ONLY_REVIEWER_PENDING_ADMIN_CAN_CALL);
        let admin = borrow_global_mut<Admin>(@quark_test);
        admin.reviewer_account = account;
        admin.reviewer_pending_admin = option::none();
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
    public fun is_reviewer(reviewer: address): bool acquires Admin {
        let reviewer_account = borrow_global<Admin>(@quark_test);
        if (reviewer == reviewer_account.reviewer_account) {
            return true;
        };
        false
    }

    #[view]
    public fun is_reviewer_pending_admin(account: address): bool acquires Admin {
        let admin = borrow_global<Admin>(@quark_test);
        if (option::is_some(&admin.reviewer_pending_admin)) {
            let pending_reviewer = option::borrow(&admin.reviewer_pending_admin);
            if (pending_reviewer == &account) {
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

    #[view]
    public fun get_reviewer_pending_admin(): address acquires Admin {
        let admin = borrow_global<Admin>(@quark_test);
        *option::borrow(&admin.reviewer_pending_admin)
    }

    #[view]
    public fun get_reviewer(): address acquires Admin {
        let admin = borrow_global<Admin>(@quark_test);
        admin.reviewer_account
    }

    #[test_only]
    public fun test_init_admin(sender: &signer) {
        init_module(sender);
    }
}