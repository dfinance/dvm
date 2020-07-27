address 0x2 {
    //#address:0x02
    module Currency {
        struct USD {}
        struct BTC {}
    }
}

//#status:4016
script {
    use 0x01::Oracle;
    use 0x02::Currency;

    fun price_not_found() {
        assert(Oracle::get_price<Currency::USD, Currency::BTC>() == 4078, 1);
    }
}

//#usd_btc:4078
script {
    use 0x01::Oracle;
    use 0x02::Currency;

    fun success() {
        assert(Oracle::get_price<Currency::USD, Currency::BTC>() == 4078, 1);
    }
}