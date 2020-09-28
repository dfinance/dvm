script {
    use 0x01::U256;

    fun test_u8_cast_success() {
        assert(U256::as_u8(U256::from_u8(0)) == 0, 0);
        assert(U256::as_u8(U256::from_u8(13)) == 13, 1);
        assert(U256::as_u8(U256::from_u8(255)) == 255, 2);
    }
}

//#status:4017 ARITHMETIC_ERROR
script {
    use 0x01::U256;

    fun test_u8_cast_error() {
        U256::as_u8(U256::from_u64(256));
    }
}

script {
    use 0x01::U256;

    fun test_u64_cast_success() {
        assert(U256::as_u64(U256::from_u64(0)) == 0, 0);
        assert(U256::as_u64(U256::from_u64(18446744073709551615u64)) == 18446744073709551615u64, 1);
        assert(U256::as_u64(U256::from_u64(255)) == 255, 2);
    }
}

//#status:4017 ARITHMETIC_ERROR
script {
    use 0x01::U256;

    fun test_u64_cast_error() {
        U256::as_u64(U256::from_u128(18446744073709551616u128));
    }
}

script {
    use 0x01::U256;

    fun test_u128_cast_success() {
        assert(U256::as_u128(U256::from_u128(0)) == 0, 0);
        assert(U256::as_u128(U256::from_u128(18446744073709551615u128)) == 18446744073709551615u128, 1);
        assert(U256::as_u128(U256::from_u128(340282366920938463463374607431768211455u128)) == 340282366920938463463374607431768211455u128, 2);
    }
}

//#status:4017 ARITHMETIC_ERROR
script {
    use 0x01::U256;

    fun test_u128_cast_error() {
        let u128_max = U256::from_u128(340282366920938463463374607431768211455u128);
        let u128_overflowed = U256::add(u128_max, U256::from_u128(1));
        U256::as_u128(u128_overflowed);
    }
}


script {
    use 0x01::U256;

    fun test_math_success() {
        // add
        let l = U256::from_u128(63374607431768211455u128);
        let r = U256::from_u128(63374607431768211455u128);
        assert(U256::as_u128(U256::add(l, r)) == 126749214863536422910u128, 0);

        //sub
        let l = U256::from_u128(63374607431768211456u128);
        let r = U256::from_u128(63374607431768211455u128);
        assert(U256::as_u128(U256::sub(l, r)) == 1u128, 1);

        // mul
        let l = U256::from_u64(18446744073709551615u64);
        let r = U256::from_u64(18446744073709551615u64);
        assert(U256::as_u128(U256::mul(l, r)) == 340282366920938463426481119284349108225u128, 2);

        // div
        let l = U256::from_u128(10u128);
        let r = U256::from_u128(2u128);
        assert(U256::as_u128(U256::div(l, r)) == 5u128, 3);
    }
}

//#status:4017 ARITHMETIC_ERROR
script {
    use 0x01::U256;

    fun test_sub_with_overflow() {
        let l = U256::from_u128(1u128);
        let r = U256::from_u128(2u128);
        U256::sub(l, r);
    }
}

//#status:4017 ARITHMETIC_ERROR
script {
    use 0x01::U256;

    fun test_div_by_zero() {
        let l = U256::from_u128(1u128);
        let r = U256::from_u128(0u128);
        U256::div(l, r);
    }
}

//#status:4017 ARITHMETIC_ERROR
script {
    use 0x01::U256;

    fun test_sum_with_overflow() {
        let l = U256::from_u128(340282366920938463463374607431768211455u128);
        let r = U256::from_u128(340282366920938463463374607431768211455u128);
        let val = U256::mul(l, r);

        loop {
            val = U256::add(val, U256::from_u128(340282366920938463463374607431768211455u128));
        }
    }
}

//#status:4017 ARITHMETIC_ERROR
script {
    use 0x01::U256;

    fun test_mul_with_overflow() {
        let l = U256::from_u128(340282366920938463463374607431768211455u128);
        let r = U256::from_u128(340282366920938463463374607431768211455u128);
        let val = U256::mul(l, r);
        U256::mul(val, U256::from_u128(340282366920938463463374607431768211455u128));
    }
}