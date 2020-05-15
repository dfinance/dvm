address 0x0 {

module Account {
    use 0x0::Transaction;
    use 0x0::Event;

    resource struct T1<CoinType> { value: u64 }

    // A resource that holds the coins stored in this account
    resource struct Balance<Token> {
        coin: T1<Token>,
    }

    resource struct Coin1 {
        value: u64,
    }

    resource struct Coin2 {
        value: u64,
    }

    resource struct T {
        value: u64,
    }

    native fun save_balance<Token>(balance: Balance<Token>, addr: address);

    native fun save_account(
        account: Self::T,
        event_generator: Event::EventHandleGenerator,
        addr: address,
    );

    public fun save_coin<Coin>(balance: u64, addr: address) {
        let balance = Balance<Coin> {
            coin: T1<Coin> { value: balance }
        };
        save_balance<Coin>(balance, addr);
    }

    public fun balance<Coin>(): u64 acquires Balance {
        let balance = borrow_global<Balance<Coin>>(Transaction::sender());
        balance.coin.value
    }

    public fun create_account(t_value: u64, addr: address) {
        save_account(T {value: t_value}, Event::new_event_generator(Transaction::sender()), addr);
    }

    public fun get_t_value(): u64 acquires T {
            let t = borrow_global<T>(Transaction::sender());
            t.value
    }
}
}