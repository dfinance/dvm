address 0x0:

module Timestamp {
    // A singleton resource holding the current Unix time in microseconds
    resource struct CurrentTimeMicroseconds {
        microseconds: u64,
    }

    // Get the timestamp representing `now` in microseconds.
    public fun now_microseconds(): u64 acquires CurrentTimeMicroseconds {
        borrow_global<CurrentTimeMicroseconds>(0x0).microseconds
    }

    // Helper function to determine if the blockchain is at genesis state.
    public fun is_genesis(): bool {
        !::exists<Self::CurrentTimeMicroseconds>(0x0)
    }
}
