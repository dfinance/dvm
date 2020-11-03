address 0x1 {

/// Math module implements basic mathematical operations in Move
/// using native U256 type. Most of the methods require Math::Num type
/// to operate. So before any action make sure to wrap your numbers
/// using Math::num() like this:
/// ```
/// let amount = Math::num(10000000, 2);
/// let result = Math::div(amount, num(100, 0));
/// let (val, dec) = Math::num_unpack(result); // unwrap result into u128
/// ```
module Math {

    use 0x1::U256;

    // max signs in u128
    const MAX_DECIMALS: u8 = 18;

    const ERR_MORE_THAN_18_DECIMALS: u64 = 401;

    struct Num { value: u128, dec: u8 }

    public fun num(value: u128, dec: u8): Num {
        Num { value, dec }
    }

    public fun scale_to_decimals(num: Num, scale_dec: u8): u128 {
        let (value, dec) = num_unpack(num);
        if (dec < scale_dec) {
            (value * pow_10(scale_dec - dec))
        } else {
            (value / pow_10(dec - scale_dec))
        }
    }

    public fun add(val1: Num, val2: Num): Num {
        // if val1 <= u128 and val2 <= u128, combination could overflow
        let (num1, dec1) = num_unpack(val1);
        let (num2, dec2) = num_unpack(val2);
        let max_dec = max(dec1, dec2);

        let num1_scaled = num1 * pow_10(max_dec - dec1);
        let num2_scaled = num2 * pow_10(max_dec - dec2);

        num(num1_scaled + num2_scaled, max_dec)
    }

    public fun pow(base: u64, exp: u8): u128 {
        let result_val = 1u128;
        let i = 0;
        while (i < exp) {
            result_val = result_val * (base as u128);
            i = i + 1;
        };
        result_val
    }

    public fun pow_10(exp: u8): u128 {
        pow(10, exp)
    }

    public fun num_unpack(num: Num): (u128, u8) {
        let Num { value, dec } = num;
        (value, dec)
    }

    fun max(a: u8, b: u8): u8 {
        if (a > b) a else b
    }

    public fun sub(val1: Num, val2: Num): Num {
        let (num1, dec1) = num_unpack(val1);
        let (num2, dec2) = num_unpack(val2);
        let max_dec = max(dec1, dec2);

        let num1_scaled = num1 * pow_10(max_dec - dec1);
        let num2_scaled = num2 * pow_10(max_dec - dec2);

        num(num1_scaled - num2_scaled, max_dec)
    }

    public fun mul(val1: Num, val2: Num): Num {
        let (num1, dec1) = num_unpack(val1);
        let (num2, dec2) = num_unpack(val2);

        assert(dec1 <= MAX_DECIMALS, ERR_MORE_THAN_18_DECIMALS);
        assert(dec2 <= MAX_DECIMALS, ERR_MORE_THAN_18_DECIMALS);

        let scaling_factor = pow_10(dec2);
        let res = U256::div(
            U256::mul(
                U256::from_u128(num1),
                U256::from_u128(num2)
            ),
            U256::from_u128(scaling_factor)
        );
        num(U256::as_u128(res), dec1)
    }

    public fun div(val1: Num, val2: Num): Num {
        let (num1, dec1) = num_unpack(val1);
        let (num2, dec2) = num_unpack(val2);

        assert(dec1 <= MAX_DECIMALS, ERR_MORE_THAN_18_DECIMALS);
        assert(dec2 <= MAX_DECIMALS, ERR_MORE_THAN_18_DECIMALS);

        let scaling_factor = pow_10(dec2);
        let res = U256::div(
            U256::mul(
                U256::from_u128(num1),
                U256::from_u128(scaling_factor)
            ),
            U256::from_u128(num2)
        );
        num(U256::as_u128(res), dec1)
    }
}
}
