module M {

    public fun f(cond: bool) {
        if (cond) {
            assert(cond, 0);
        } else {
          assert(cond, 0);
        }
    }

    public fun t(): u64 {
        let x;
        if (cond()) {
            return 100
        } else {
            x = 0;
        };
        x
    }

    public fun cond(): bool {
        true
    }
}