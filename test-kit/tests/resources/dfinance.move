address 0x1 {

module Dfinance {
    resource struct SimpleCoin {}

    resource struct Info<Coin> {
        value: u64
    }

    native fun create_signer(addr: address): signer;
    native fun destroy_signer(sig: signer);

    public fun store_info<Coin: resource>(value: u64) {
        let sig = create_signer(0x1);
        move_to<Info<Coin>>(&sig, Info {value});
        destroy_signer(sig);
    }
}
}