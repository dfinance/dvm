module Store {
    resource struct U64 {val: u64}

    public fun store_u64(val: u64) {
        let foo = U64 {val: val};
        move_to_sender<U64>(foo);
    }
}