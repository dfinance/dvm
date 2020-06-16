address 0x1 {

module Event {
    resource struct EventHandleGenerator {
        counter: u64,
        addr: address,
    }

    public fun new_event_generator(addr: address,): EventHandleGenerator {
        EventHandleGenerator{ counter: 0, addr }
    }
}
}