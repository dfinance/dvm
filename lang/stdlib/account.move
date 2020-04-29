address 0x0:

module Account {
    use 0x0::Coins;
    use 0x0::Event;
    use 0x0::Transaction;

    // Resource storing account information.
    resource struct T {
        // Store balances.
        balances: Coins::T,
        // Event handle for received event
        withdraw_events: Event::EventHandle<PaymentEvent>,
        // Event handle for sent event
        deposit_events: Event::EventHandle<PaymentEvent>,
    }

    // Payment event.
    struct PaymentEvent {
        // The amount withdraw from account.
        amount: u128,
        // The denom of currency.
        denom:  vector<u8>,
    }

    // Checks if an account exists at `check_addr`.
    public fun exists(check_addr: address): bool {
        ::exists<T>(check_addr)
    }

    // Deposits the `to_deposit` coin into the `payee`'s account with the attached `metadata` and
    // sender address.
    fun deposit_to_payee(payee: address, to_deposit: Coins::Coin) acquires T {
        // Load the payee's account
        let payee_account = borrow_global_mut<T>(payee);

        // Emit event.
        Event::emit_event<PaymentEvent>(
            &mut payee_account.deposit_events,
            PaymentEvent {
                amount: Coins::coin_value(&to_deposit),
                denom:  Coins::coin_denom(&to_deposit),
            },
        );

        // Deposit the `to_deposit` coin
        Coins::deposit(&mut payee_account.balances, to_deposit);
     }

    // Helper to withdraw `amount` from the given `account` and return the resulting Coins::Coin
    fun withdraw_from_account(account: &mut T, amount: u128, denom: vector<u8>): Coins::Coin {
        Event::emit_event<PaymentEvent>(
            &mut account.withdraw_events,
            PaymentEvent {
                amount: amount,
                denom:  copy denom,
            },
        );

        Coins::withdraw(&mut account.balances, amount, denom)
    }

    // Withdraw `amount` Coin.T from the transaction sender's account
    public fun withdraw_from_sender(amount: u128, denom: vector<u8>): Coins::Coin acquires T {
        let sender_account = borrow_global_mut<T>(Transaction::sender());
        withdraw_from_account(sender_account, amount, denom)
    }

    // Deposits the `to_deposit` coin into the `payee`'s account
    public fun deposit(payee: address, to_deposit: Coins::Coin) acquires T {
        if (!exists(payee)) {
            // create acc
        };

        deposit_to_payee(payee, to_deposit);
    }
}