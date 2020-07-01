module M {
    use 0x1::Transaction;

    resource struct R<T: resource> {}
    resource struct R1 {}

    fun build_in_functions<T: resource>(sender: &signer, r: R<T>, r1: R1) {
        exists<R<T>>(Transaction::sender());
        exists<R1>(Transaction::sender());
        move_to<R1>(sender, r1);
        move_to<R<T>>(sender, r);
    }

    fun mf(): R1 acquires R1 {
        move_from(0x0)
    }

    fun mfg<T: resource>(): R<T> acquires R {
        move_from(0x0)
    }

    fun mts<T: resource>(r: R<T>, r1: R1) {
        move_to_sender<R1>(r1);
        move_to_sender<R<T>>(r);
    }

    fun bg<T: resource>() acquires R, R1 {
        borrow_global<R1>(0x0);
        borrow_global<R<T>>(0x0);
    }

    fun bgm<T: resource>() acquires R, R1 {
        borrow_global_mut<R1>(0x0);
        borrow_global_mut<R<T>>(0x0);
    }
}