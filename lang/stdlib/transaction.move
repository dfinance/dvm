address 0x0:

module Transaction {
    native public fun sender(): address;

    // inlined
    public fun assert(check: bool, code: u64) {
        if (check) () else abort code
    }
}