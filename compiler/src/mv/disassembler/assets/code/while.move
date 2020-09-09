module M {
    fun w(v: u64) {
        foo();
        while (v < 3) {
            v = v + 1
        };
        foo();
    }

    fun w_ret(v: u64): u64 {
        foo();
        while (v < 3) {
            v = v + 1;
            return v
        };
        foo();
        return v
    }

    fun w_innner_loop(v: u64) {
        foo();
        while (cond()) {
            while (v < 3) {
                v = v + 1;
            };
            return
        };
        foo();
    }

    fun w_abort(v: u64) {
        foo();
        while (v < 3) {
            v = v + 1;
            abort v
        };
        foo();
    }

    fun w1() {
        while (cond()) {
            while (!cond()) {
                if (cond()) {
                    break
                }
            }
        }
    }


    fun f(v: u64) {
        while (v < 10) { v = v + 1 };
        while (v < 10) (v = v + 1);
        while (v < 10) v = v + 1
    }

    fun w2(cond: bool) {
        while (cond) {
            if (cond) {
                w4();
                break
            }
        }
    }

    fun w4() {}

    fun w3(v: u64): (bool, u64) {
        while (v < 3) {
            v = v + 1
        };
        (true, 1)
    }

    fun w5() {
        while (cond()) {
            while (!cond()) {
                if (cond()) {
                    w4();
                } else {
                    break
                }
            }
        }
    }

    fun w6() {
        while (cond()) {
            while (!cond()) {
                if (cond()) {} else {
                    break
                }
            }
        }
    }


    public fun cond(): bool {
        true
    }

    public fun foo() {}
}