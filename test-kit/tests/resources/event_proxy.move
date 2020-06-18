address 0x1 {
module EventProxy {
    use 0x1::Event;
    public fun store<T: copyable>(msg: T) {
        Event::emit<T>(msg);
    }
}
}