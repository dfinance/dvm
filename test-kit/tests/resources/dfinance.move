address 0x1 {

module Dfinance {
    resource struct SimpleCoin {}

    resource struct Info<Coin> {
        value: u64
    }

    native fun register_token_info<Coin: resource>(info: Info<Coin>);

    public fun store_info<Coin: resource>(value: u64) {
        register_token_info<Coin>(Info {value});
    }
}
}