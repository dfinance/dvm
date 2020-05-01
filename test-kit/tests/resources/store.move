module Store {
    resource struct U64 {val: u64}
    resource struct Address {val: address}
    resource struct VectorU8 {val: vector<u8>}

    public fun store_u64(val: u64) {
        let foo = U64 {val: val};
        move_to_sender<U64>(foo);
    }

    public fun store_address(val: address) {
        let addr = Address {val: val};
        move_to_sender<Address>(addr);
    }

    public fun store_vector_u8(val: vector<u8>) {
        let vec = VectorU8 {val: val};
        move_to_sender<VectorU8>(vec);
    }
}