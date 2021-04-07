script {
    use 0x1::Account;
    use 0x1::Coins;
    use 0x1::XFI;
    use 0x1::Dfinance;

    fun test_balance(addr_1: &signer, addr_2: &signer, init_usdt: u128, init_pont: u128, init_btc: u128) {
        assert(Account::get_native_balance<Coins::USDT>(addr_1) == init_usdt, 1);
        assert(Account::get_native_balance<XFI::T>(addr_1) == init_pont, 2);
        assert(Account::get_native_balance<Coins::BTC>(addr_1) == init_btc, 3);

        let move_usdt = init_usdt / 2;
        let usdt = Account::deposit_native<Coins::USDT>(addr_1, move_usdt);
        Account::withdraw_native<Coins::USDT>(addr_2, usdt);

        assert(Account::get_native_balance<Coins::USDT>(addr_1) == init_usdt - move_usdt, 4);
        assert(Account::get_native_balance<Coins::USDT>(addr_2) == move_usdt, 5);

        let pont_1 = Account::deposit_native<XFI::T>(addr_1, 1);
        let pont_2 = Account::deposit_native<XFI::T>(addr_1, 1);
        let pont_3 = Account::deposit_native<XFI::T>(addr_1, 1);

        let pont_1 = Dfinance::join(pont_1, pont_3);

        Account::withdraw_native<XFI::T>(addr_2, pont_1);
        Account::withdraw_native<XFI::T>(addr_2, pont_2);
        assert(Account::get_native_balance<XFI::T>(addr_1) == init_pont - 3, 6);
        assert(Account::get_native_balance<XFI::T>(addr_2) == 3, 6);
    }
}