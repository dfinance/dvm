// TODO:
//
// - add support for extra token emission (say, I want to create 100 more tokens in a week)
// - make it impossible to init Token with system resource or another generic

/// Token module which allows users to register their own currencies
/// just like they would do it on Ethereum network (ERC20). It is a
/// standard library module and only allows registering custom tokens
/// within Dfinance ecosystem to make regular currency-related operations
/// possible on custom tokens.
module Token {

    use 0x1::Dfinance;
    use 0x1::Account;
    use 0x1::Signer;
    use 0x1::Event;

    const DECIMALS_MIN : u8 = 8;
    const DECIMALS_MAX : u8 = 18;

    /// Token resource. Must be used with custom token type. Which means
    /// that first token creator must deploy a token module which will have
    /// empty resource type in it which should be then passed as type argument
    /// into Token::initialize() method.
    resource struct T<Token: resource> {}

    /// This is the event data for TokenCreated event which can only be fired
    /// from this module, from Token::initialize() method.
    struct TokenCreatedEvent {
        creator: address,
        total_supply: u128,
        denom: vector<u8>,
        decimals: u8
    }

    /// Public getter for total supply of any token
    public fun total_supply<Token: resource>(): u128 {
        Dfinance::total_supply<T<Token>>()
    }

    /// Pubic getter for decimals of any token
    public fun decimals<Token: resource>(): u8  {
        Dfinance::decimals<T<Token>>()
    }

    /// Public getter for denom property of any token
    public fun denom<Token: resource>(): vector<u8> {
        Dfinance::denom<T<Token>>()
    }

    /// Initialize token. For this method to work user must provide custom
    /// resource type which he had previously created within his own module.
    public fun initialize<Token: resource>(
        account: &signer,
        total_supply: u128,
        decimals: u8,
        denom: vector<u8>
    ) {

        // decimals are a must currently. they are required in CDP module
        // TODO: discuss
        assert(decimals >= DECIMALS_MIN && decimals <= DECIMALS_MAX, 20);

        // token resource can only be issued by global Dfinance currency-tracking module
        let token_resource = Dfinance::tokenize<T<Token>>(account, total_supply, decimals, copy denom);
        let creator = Signer::address_of(account);

        // create balance resource and fill it with newly created token, voila!
        Account::accept<T<Token>>(account);
        Account::deposit(account, creator, token_resource);

        // finally fire the TokenEmitted event
        Event::emit<TokenCreatedEvent>(TokenCreatedEvent {
            total_supply,
            decimals,
            creator,
            denom
        });
    }
}

