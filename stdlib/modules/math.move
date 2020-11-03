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
    const POW_10_TO_MAX_DECIMALS: u128 = 1000000000000000000;

    const ERR_MORE_THAN_18_DECIMALS: u64 = 401;

    struct Num { value: u128, dec: u8 }

    public fun num(value: u128, dec: u8): Num {
        assert(
            dec <= MAX_DECIMALS,
            ERR_MORE_THAN_18_DECIMALS
        );
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
        let multiplied = U256::mul(
            U256::from_u128(num1),
            U256::from_u128(num2)
        );

        let new_decimals = dec1 + dec2;
        let multiplied_scaled = if (new_decimals < MAX_DECIMALS) {
            let decimals_underflow = MAX_DECIMALS - new_decimals;
            // scale multiplied to 18th decimals
            U256::mul(multiplied, U256::from_u128(pow_10(decimals_underflow)))
        } else if (new_decimals > MAX_DECIMALS) {
            let decimals_overflow = new_decimals - MAX_DECIMALS;
            // scale multiplied down
            U256::div(multiplied, U256::from_u128(pow_10(decimals_overflow)))
        } else {
            multiplied
        };
        let multiplied_scaled_u128 = U256::as_u128(multiplied_scaled);
        num(multiplied_scaled_u128, MAX_DECIMALS)
    }

    public fun div(val1: Num, val2: Num): Num {
        let (num1, dec1) = num_unpack(val1);
        let (num2, dec2) = num_unpack(val2);

        let num1_scaling_factor = pow_10(MAX_DECIMALS - dec1);
        // 18th
        let num1_scaled = U256::mul(U256::from_u128(num1), U256::from_u128(num1_scaling_factor));
        // 36th
        let num1_scaled_with_overflow = U256::mul(num1_scaled, U256::from_u128(POW_10_TO_MAX_DECIMALS));

        let num2_scaling_factor = pow_10(MAX_DECIMALS - dec2);
        let num2_scaled = U256::mul(U256::from_u128(num2), U256::from_u128(num2_scaling_factor));
        // 18th
        let division = U256::div(num1_scaled_with_overflow, num2_scaled);
        num(U256::as_u128(division), MAX_DECIMALS)
    }

    public fun equals(val1: Num, val2: Num): bool {
        let num1 = scale_to_decimals(val1, 18);
        let num2 = scale_to_decimals(val2, 18);
        num1 == num2
    }
}
}
