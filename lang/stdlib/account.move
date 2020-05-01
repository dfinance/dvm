address 0x0:

/// Account is the access point for assets flow. It holds withdraw-deposit handlers
/// for generic currency <Token>. It also stores log of sent and received events
/// for every account.
module Account {

    use 0x0::Transaction;
    use 0x0::Dfinance;
    use 0x0::Event;

    /// holds account data, no matter what the data is;
    resource struct T {
        sent_events: Event::EventHandle<SentPaymentEvent>,
        received_events: Event::EventHandle<ReceivedPaymentEvent>

        // maybe we should add something like `is_frozen` in libra
    }

    resource struct Balance<Token> {
        coin: Dfinance::T<Token>
    }

    /// Message for sent events
    struct SentPaymentEvent {
        amount: u64,
        denom: vector<u8>,
        payee: address,
        metadata: vector<u8>,
    }

    /// Message for received events
    struct ReceivedPaymentEvent {
        amount: u64,
        denom: vector<u8>,
        payer: address,
        metadata: vector<u8>,
    }

    /// Init wallet for measurable currency, hence accept <Token> currency
    public fun accept<Token>() {
        move_to_sender<Balance<Token>>(Balance { coin: Dfinance::zero<Token>() })
    }

    public fun can_accept<Token>(payee: address): bool {
        ::exists<Balance<Token>>(payee)
    }

    public fun balance<Token>(): u64 acquires Balance {
        balance_for<Token>(Transaction::sender())
    }

    public fun balance_for<Token>(addr: address): u64 acquires Balance {
        Dfinance::value(&borrow_global<Balance<Token>>(addr).coin)
    }

    public fun deposit<Token>(payee: address, to_deposit: Dfinance::T<Token>)
    acquires Balance {
        let value = Dfinance::value(&to_deposit);
        Transaction::assert(value > 0, 7);

        let payee_balance = borrow_global_mut<Balance<Token>>(payee);
        Dfinance::deposit(&mut payee_balance.coin, to_deposit);
    }

    public fun deposit_to_sender<Token>(to_deposit: Dfinance::T<Token>)
    acquires Balance {
        deposit(Transaction::sender(), to_deposit)
    }

    public fun deposit_with_metadata<Token>(
        payee: address,
        to_deposit: Dfinance::T<Token>,
        metadata: vector<u8>
    ) acquires T, Balance {
        deposit_with_sender_and_metadata(
            payee,
            Transaction::sender(),
            to_deposit,
            metadata
        )
    }

    public fun pay_from_sender<Token>(payee: address, amount: u64)
    acquires T, Balance {
        pay_from_sender_with_metadata<Token>(
            payee, amount, x""
        )
    }

    public fun pay_from_sender_with_metadata<Token>(payee: address, amount: u64, metadata: vector<u8>)
    acquires T, Balance {
        deposit_with_metadata<Token>(
            payee,
            withdraw_from_sender(amount),
            metadata
        )
    }

    fun deposit_with_sender_and_metadata<Token>(
        payee: address,
        sender: address,
        to_deposit: Dfinance::T<Token>,
        metadata: vector<u8>
    ) acquires T, Balance {
        let amount = Dfinance::value(&to_deposit);
        Transaction::assert(amount > 0, 7);

        let denom = Dfinance::denom<Token>();
        let sender_acc = borrow_global_mut<T>(sender);

        // add event as sent into account
        Event::emit_event<SentPaymentEvent>(
            &mut sender_acc.sent_events,
            SentPaymentEvent {
                amount, // u64 can be copied
                payee,
                denom: copy denom,
                metadata: copy metadata
            },
        );

        // there's no way to improve this place as payee is not sender :(
        if (!has_acc_and_balance<Token>(payee)) {
            // !!! save_account_resource<Token>(addr: address)
        };

        let payee_acc     = borrow_global_mut<T>(payee);
        let payee_balance = borrow_global_mut<Balance<Token>>(payee);

        // send money to payee
        Dfinance::deposit(&mut payee_balance.coin, to_deposit);
        // update payee's account with new event
        Event::emit_event<ReceivedPaymentEvent>(
            &mut payee_acc.received_events,
            ReceivedPaymentEvent {
                amount,
                denom,
                metadata,
                payer: sender
            }
        )
    }

    public fun withdraw_from_sender<Token>(amount: u64): Dfinance::T<Token>
    acquires Balance {
        let sender  = Transaction::sender();
        let balance = borrow_global_mut<Balance<Token>>(sender);

        withdraw_from_balance<Token>(balance, amount)
    }

    fun withdraw_from_balance<Token>(balance: &mut Balance<Token>, amount: u64): Dfinance::T<Token> {
        Dfinance::withdraw(&mut balance.coin, amount)
    }

    public fun has_acc_and_balance<Token>(addr: address): bool {
        ::exists<T>(addr) && ::exists<Balance<T>>(addr)
    }

    // !!! native fun save_account_resource<Token>(addr: address);
}
