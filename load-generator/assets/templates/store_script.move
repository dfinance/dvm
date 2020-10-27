script {
    use 0x1::Module_ID_;

    fun main(account: &signer, value: u64) {
        Module_ID_::store(account, value);
    }
}