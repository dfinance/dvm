address 0x1 {
module Currency {
    struct USD {val: u64}
    struct ETH {val: u64}
    struct BTC {val: u64}
    struct Value<Curr> {val: u64}

    public fun make_currency<Curr: copyable>(val: u64): Value<Curr> {
        Value { val }
    }

    public fun make_btc(val: u64): BTC {
        BTC { val }
    }
}
}