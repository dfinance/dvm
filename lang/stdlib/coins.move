address 0x0:

module Coins {
    use 0x0::Vector;
    use 0x0::Transaction;
    use 0x0::Compare;

    // coin struct
    struct Coin {
        denom: vector<u8>,
        value: u128,
    }

    resource struct T {
        // all balances.
        balances: vector<Coin>,
    }

    // Create a new Coin.T with a value of 0
    public fun zero_balances(): T {
        T { balances: Vector::empty() }
    }

    // get zero coin.
    public fun zero_coin(): Coin {
        Coin { denom: Vector::empty(), value: 0 }
    }

    // get coin value.
    public fun coin_value(coin_ref: &Coin): u128 {
        coin_ref.value
    }

    // get coin denom.
    public fun coin_denom(coin_ref: &Coin): vector<u8> {
        *&coin_ref.denom
    }

    // "Divides" the given coin into two, where original coin is modified in place
    // The original coin will have value = original value - `amount`
    // The new coin will have a value = `amount`
    // Fails if the coins value is less than `amount`
    public fun withdraw(coins_ref: &mut T, amount: u128, denom: vector<u8>): Coin {
        let i = 0;
        let length = Vector::length(&coins_ref.balances);

        while (i < length) {
            let coin = Vector::borrow_mut(&mut coins_ref.balances, i);
            if (Compare::cmp_lcs_bytes(&coin.denom, &denom) == 0) {
                Transaction::assert(coin.value > amount, 10);
                coin.value = coin.value - amount;
                return Coin { denom: denom, value: amount }
            };
            i = i + 1;
        };

        Transaction::assert(i == length, 11);
        Coin {denom: denom, value: amount }
    }

    // "Merges" the two coins
    // The coin passed in by reference will have a value equal to the sum of the two coins
    // The `check` coin is consumed in the process
    public fun deposit(coins_ref: &mut T, check: Coin) {
        let length = Vector::length(&coins_ref.balances);
        let i = 0;

        let check_denom = coin_denom(&check);
        let check_value = coin_value(&check);

        Transaction::assert(check_value > 0, 10);

        while (i < length) {
            let coin = Vector::borrow_mut(&mut coins_ref.balances, i);

            if (Compare::cmp_lcs_bytes(&coin.denom, &check_denom) == 0) {
                coin.value = coin.value + check_value;
                return
            };

            i = i + 1;
        };

        Vector::push_back(&mut coins_ref.balances, Coin { denom: check_denom, value: check_value });
    }
}