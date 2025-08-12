module quark::admin {
    use std::signer;
    use std::option::{Self, Option};
    use std::vector;
    use aptos_std::type_info;

    struct Admin has key {
        account: address,
        pending_admin: Option<address>,
        reviewer_account: address,
        reviewer_pending_admin: Option<address>,
    }

    struct Config has key {
        coin_addr: Option<address>,
    }

    struct FeesCurrencyPaymentList has key {
        list: vector<address>,
    }

    const ONLY_ADMIN_CAN_CALL: u64 = 1;
    const ONLY_PENDING_ADMIN_CAN_CALL: u64 = 2;
    const ONLY_REVIEWER_CAN_CALL: u64 = 3;
    const ONLY_REVIEWER_PENDING_ADMIN_CAN_CALL: u64 = 4;
    const ECOIN_NOT_FOUND: u64 = 5;
    const EONLY_QUARK_CAN_CALL: u64 = 6;
    const EFEES_CURRENCY_PAYMENT_LIST_ALREADY_INITIALIZED: u64 = 7;
    const EFEES_CURRENCY_PAYMENT_LIST_NOT_INITIALIZED: u64 = 8;

    fun init_module(sender: &signer) {
        let account = signer::address_of(sender);
        move_to(sender, Admin { account, pending_admin: option::none(), reviewer_account: account, reviewer_pending_admin: option::none() });
    }

    public entry fun set_pending_admin(sender: &signer, new_admin: address) acquires Admin {
        let account = signer::address_of(sender);
        assert!(is_admin(account), ONLY_ADMIN_CAN_CALL);
        let admin = borrow_global_mut<Admin>(@quark);
        admin.pending_admin = option::some(new_admin);
    }

    public entry fun accept_admin(sender: &signer) acquires Admin {
        let account = signer::address_of(sender);
        assert!(is_pending_admin(account), ONLY_PENDING_ADMIN_CAN_CALL);
        let admin = borrow_global_mut<Admin>(@quark);
        admin.account = account;
        admin.pending_admin = option::none();
    }
    
    public entry fun init_fees_currency_payment_list(sender: &signer) {
        let account = signer::address_of(sender);
        assert!(account == @quark, EONLY_QUARK_CAN_CALL);
        assert!(!exists<FeesCurrencyPaymentList>(@quark), EFEES_CURRENCY_PAYMENT_LIST_ALREADY_INITIALIZED);
        let fees_currency_payment_list = FeesCurrencyPaymentList {
            list: vector::empty(),
        };

        move_to(sender, fees_currency_payment_list);
    }

    public entry fun add_fees_currency_v1_payment_list<CoinType>(sender: &signer) acquires Admin, FeesCurrencyPaymentList {
        let account = signer::address_of(sender);
        assert!(is_admin(account), ONLY_ADMIN_CAN_CALL);
        assert!(exists<FeesCurrencyPaymentList>(@quark), EFEES_CURRENCY_PAYMENT_LIST_NOT_INITIALIZED);

        let coin_type = type_info::type_of<CoinType>();
        let coin_address = type_info::account_address(&coin_type);

        let fees_currency_payment_list = borrow_global_mut<FeesCurrencyPaymentList>(@quark);
        vector::push_back(&mut fees_currency_payment_list.list, coin_address);
    }

    public entry fun remove_fees_currency_v1_payment_list<CoinType>(sender: &signer) acquires Admin, FeesCurrencyPaymentList {
        let account = signer::address_of(sender);
        assert!(is_admin(account), ONLY_ADMIN_CAN_CALL);
        assert!(exists<FeesCurrencyPaymentList>(@quark), EFEES_CURRENCY_PAYMENT_LIST_NOT_INITIALIZED);

        let coin_type = type_info::type_of<CoinType>();
        let coin_address = type_info::account_address(&coin_type);

        let fees_currency_payment_list = borrow_global_mut<FeesCurrencyPaymentList>(@quark);

        let (currency_exists, index) = vector::index_of<address>(&fees_currency_payment_list.list, &coin_address);
        assert!(currency_exists, ECOIN_NOT_FOUND);
        vector::remove(&mut fees_currency_payment_list.list, index);
    }

    public entry fun add_fees_currency_v2_payment_list(sender: &signer, currency: address) acquires Admin, FeesCurrencyPaymentList {
        let account = signer::address_of(sender);
        assert!(is_admin(account), ONLY_ADMIN_CAN_CALL);
        assert!(exists<FeesCurrencyPaymentList>(@quark), EFEES_CURRENCY_PAYMENT_LIST_NOT_INITIALIZED);

        let fees_currency_payment_list = borrow_global_mut<FeesCurrencyPaymentList>(@quark);
        vector::push_back(&mut fees_currency_payment_list.list, currency);
    }

    public entry fun remove_fees_currency_v2_payment_list(sender: &signer, currency: address) acquires Admin, FeesCurrencyPaymentList {
        let account = signer::address_of(sender);
        assert!(is_admin(account), ONLY_ADMIN_CAN_CALL);

        let fees_currency_payment_list = borrow_global_mut<FeesCurrencyPaymentList>(@quark);

        let (currency_exists, index) = vector::index_of<address>(&fees_currency_payment_list.list, &currency);
        assert!(currency_exists, ECOIN_NOT_FOUND);
        vector::remove(&mut fees_currency_payment_list.list, index);
    }

    public entry fun set_reviewer_pending_admin(sender: &signer, new_admin: address) acquires Admin {
        let account = signer::address_of(sender);
        assert!(is_reviewer(account), ONLY_REVIEWER_CAN_CALL);
        let admin = borrow_global_mut<Admin>(@quark);
        admin.reviewer_pending_admin = option::some(new_admin);
    }

    public entry fun accept_reviewer_pending_admin(sender: &signer) acquires Admin {
        let account = signer::address_of(sender);
        assert!(is_reviewer_pending_admin(account), ONLY_REVIEWER_PENDING_ADMIN_CAN_CALL);
        let admin = borrow_global_mut<Admin>(@quark);
        admin.reviewer_account = account;
        admin.reviewer_pending_admin = option::none();
    }

    #[view]
    public fun get_fees_currency_payment_list(): vector<address> acquires FeesCurrencyPaymentList {
        let fees_currency_payment_list = borrow_global<FeesCurrencyPaymentList>(@quark);
        fees_currency_payment_list.list
    }

    #[view]
    public fun exist_fees_currency_payment_list(currency: address): bool acquires FeesCurrencyPaymentList {
        let fees_currency_payment_list = borrow_global<FeesCurrencyPaymentList>(@quark);
        vector::contains<address>(&fees_currency_payment_list.list, &currency)
    }

    #[view]
    public fun is_admin(account: address): bool acquires Admin {
        let admin = borrow_global<Admin>(@quark);
        if (account == admin.account) {
            return true;
        };
        false
    }

    #[view]
    public fun is_pending_admin(account: address): bool acquires Admin {
        let admin = borrow_global<Admin>(@quark);
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
        let reviewer_account = borrow_global<Admin>(@quark);
        if (reviewer == reviewer_account.reviewer_account) {
            return true;
        };
        false
    }

    #[view]
    public fun is_reviewer_pending_admin(account: address): bool acquires Admin {
        let admin = borrow_global<Admin>(@quark);
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
        let admin = borrow_global<Admin>(@quark);
        *option::borrow(&admin.pending_admin)
    }

    #[view]
    public fun get_admin(): address acquires Admin {
        let admin = borrow_global<Admin>(@quark);
        admin.account
    }

    #[view]
    public fun get_reviewer_pending_admin(): address acquires Admin {
        let admin = borrow_global<Admin>(@quark);
        *option::borrow(&admin.reviewer_pending_admin)
    }

    #[view]
    public fun get_reviewer(): address acquires Admin {
        let admin = borrow_global<Admin>(@quark);
        admin.reviewer_account
    }

    #[test_only]
    public fun test_init_admin(sender: &signer) {
        init_module(sender);
    }
}