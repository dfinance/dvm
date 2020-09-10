address 0x0000000000000000000000000000000000000001 {
module M {

    fun bar() {
    }

    fun cond(): bool {
        true
    }

    fun foo() {
    }

    fun i1() {
        if (cond()) {
            foo();
            bar();
            return
        } else {
            return
        }
    }

    fun i2() {
        if (cond()) {
            foo();
            return
        } else {
            bar();
            return
        }
    }

}
}
