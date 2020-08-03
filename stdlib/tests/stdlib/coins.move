//#eth_btc:4078
script {
    use 0x01::Coins;

    fun test_has_price() {
        assert(Coins::has_price<Coins::ETH, Coins::BTC>(), 1);
        assert(!Coins::has_price<Coins::ETH, Coins::USDT>(), 2);
    }
}

//#eth_btc:4078
//#btc_dfi:20
script {
    use 0x01::Coins;
    use 0x01::DFI;

    fun test_get_price() {
        assert(Coins::get_price<Coins::ETH, Coins::BTC>() == 4078, 1);
        assert(Coins::get_price<Coins::BTC, DFI::T>() == 20, 2);
    }
}

//#eth_btc:4078
//#status:4008 MISSING_DATA
script {
    use 0x01::Coins;
    use 0x01::DFI;

    fun test_price_not_found() {
        assert(Coins::get_price<Coins::ETH, Coins::BTC>() == 4078, 1);
        assert(Coins::get_price<Coins::BTC, DFI::T>() == 20, 2);
    }
}