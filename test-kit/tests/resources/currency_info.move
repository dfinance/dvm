script {
    use 0x1::Dfinance;
    use 0x1::XFI;
    use 0x1::Coins;

    fun main() {
        assert(Dfinance::denom<Coins::BTC>() == b"BTC", 1);
        assert(Dfinance::denom<XFI::T>() == b"XFI", 2);

        assert(Dfinance::decimals<Coins::BTC>() == 10, 3);
        assert(Dfinance::decimals<XFI::T>() == 2, 4);

        assert(Dfinance::is_token<Coins::BTC>() == true, 5);
        assert(Dfinance::is_token<XFI::T>() == true, 6);

        assert(Dfinance::total_supply<Coins::BTC>() == 1024, 7);
        assert(Dfinance::total_supply<XFI::T>() == 42, 8);
    }
}