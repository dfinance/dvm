/// Dfinance is a governance module which handles balances merging. It's basically
/// a mediator or wrapper around money-related operations. It holds knowledge about
/// registered coins and rules of their usage. Also it lessens load from 0x0::Account
module Dfinance {

    use 0x0::Transaction;

    resource struct T<Coin> {
        value: u64
    }

    resource struct Info<Coin> {
        denom: vector<u8>,
        decimals: u8
    }

    public fun value<Coin>(coin: &T<Coin>): u64 {
        coin.value
    }

    public fun zero<Coin>(): T<Coin> {
        T<Coin> { value: 0 }
    }

    public fun split<Coin>(coin: T<Coin>, amount: u64): (T<Coin>, T<Coin>) {
        let other = withdraw(&mut coin, amount);
        (coin, other)
    }

    public fun join<Coin>(coin1: T<Coin>, coin2: T<Coin>): T<Coin> {
        deposit(&mut coin1, coin2);
        coin1
    }

    public fun deposit<Coin>(coin: &mut T<Coin>, check: T<Coin>) {
        let T { value } = check; // destroy check
        coin.value = coin.value + value;
    }

    public fun withdraw<Coin>(coin: &mut T<Coin>, amount: u64): T<Coin> {
        Transaction::assert(coin.value >= amount, 10);
        coin.value = coin.value - amount;
        T { value: amount }
    }

    /// Working with CoinInfo - coin registration procedure, 0x0 account used

    /// What can be done here:
    ///   - proposals API: user creates resource Info, pushes it into queue
    ///     0x0 government reads and registers proposed resources by taking them
    ///   - try to find the way to share Info using custom module instead of
    ///     writing into main register (see above)

    /// getter for denom. reads denom information from 0x0 resource
    public fun denom<Coin>(): vector<u8> acquires Info {
        *&borrow_global<Info<Coin>>(0x0).denom
    }

    /// getter for currency decimals
    public fun decimals<Coin>(): u8 acquires Info {
        borrow_global<Info<Coin>>(0x0).decimals
    }

    /// only 0x0 address and add denom descriptions, 0x0 holds information resource
    public fun register_coin<Coin>(denom: vector<u8>, decimals: u8) {
        assert_can_register_coin();

        move_to_sender<Info<Coin>>(Info { denom, decimals });
    }

    /// check whether sender is 0x0, helper method
    fun assert_can_register_coin() {
        Transaction::assert(Transaction::sender() == 0x0, 1);
    }
}
