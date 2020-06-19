address 0x1 {

module Event {
    native public fun emit<T: copyable>(msg: T);
}
}