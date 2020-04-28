module Store {
    resource struct U64 {val: u64}
    resource struct Address {val: address}

    public fun store_u64(val: u64) {
        let foo = U64 {val: val};
        move_to_sender<U64>(foo);
    }

    public fun store_address(val: address) {
            let addr = Address {val: val};
            move_to_sender<Address>(addr);
    }
}