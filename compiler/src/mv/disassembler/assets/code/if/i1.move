module M {
    fun cond(): bool {
        true
    }

    fun i1() {
        if (cond()) {
            foo();
        } else {
            return
        };
        bar();
    }

    fun i2() {
        if (cond()) {
            foo();
        } else {
            bar();
            return
        }
    }

    fun foo() {}

    fun bar() {}
}