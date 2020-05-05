address 0x0:

module Account {
    use 0x0::Transaction;

    resource struct T<CoinType> { value: u64 }

    // A resource that holds the coins stored in this account
    resource struct Balance<Token> {
        coin: T<Token>,
    }

    resource struct Coin1 {
        value: u64,
    }

    resource struct Coin2 {
            value: u64,
    }

    native fun save_balance<Token>(balance: Balance<Token>, addr: address);

    public fun save_coin<Coin>(balance: u64, addr: address) {
        let balance = Balance<Coin> {
            coin: T<Coin> { value: balance }
        };
        save_balance<Coin>(balance, addr);
    }

    public fun balance<Coin>(): u64 acquires Balance {
        let balance = borrow_global<Balance<Coin>>(Transaction::sender());
        balance.coin.value
    }
}