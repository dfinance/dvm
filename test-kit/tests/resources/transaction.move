module Transaction {
    native public fun sender(): address;

    public fun assert(check: bool, code: u64) {
            if (check) () else abort code
    }
}
