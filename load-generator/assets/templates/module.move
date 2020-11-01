address 0x1 {
module Module_ID_ {
    resource struct Struct_ID_ {
        value: u64,
    }

    public fun store(account: &signer, value: u64) {
        move_to<Struct_ID_>(account, Struct_ID_ { value: value });
    }

    public fun load(account: address): u64 acquires Struct_ID_ {
        borrow_global<Struct_ID_>(account).value
    }
}
}