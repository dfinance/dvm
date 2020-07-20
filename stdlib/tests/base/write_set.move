address 0x01 {
    //#address:0x01
    module Foo {
        use 0x01::Signer;

        resource struct Val {
            val: u64
        }

        public fun get_val(account: &signer): u64 acquires Val {
            borrow_global<Val>(Signer::address_of(account)).val
        }

        public fun store_val(account: &signer, val: u64) {
            move_to<Val>(account, Val {val: val})
        }
    }
}

//#status:4008 MISSING_DATA
script {
    use 0x01::Foo;

    fun load_empty_val(account: &signer) {
        assert(Foo::get_val(account) == 13, 1);
    }
}

script {
    use 0x01::Foo;

    fun store_val(account: &signer) {
        Foo::store_val(account, 13);
    }
}

script {
    use 0x01::Foo;

    fun load_val(account: &signer) {
        assert(Foo::get_val(account) == 13, 1);
    }
}