script {
    use 0x1::Module_ID_;

    fun main(value: u64, account: address) {
        assert(Module_ID_::load(account) == value, 1);
    }
}