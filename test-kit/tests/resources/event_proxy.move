address 0x1 {
module EventProxy {
    use 0x1::Event;
    public fun store<T: copyable>(account: &signer, msg: T) {
        Event::emit<T>(account, msg);
    }
}
}