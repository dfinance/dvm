script {
    use 0x1::Math::{Self, num, scale_to_decimals};

    fun main() {
        // 1
        let a = num(2, 0);
        let b = num(3, 0);
        let c = Math::mul(a, b);
        assert(scale_to_decimals(c, 0) == 6u128, 1);

        // 2
        let a1 = num(2, 18);
        let b1 = num(3, 18);
        let c1 = Math::mul(a1, b1);
        assert(Math::equals(c1, num(0, 0)), 2);

        // 3
        let a2 = num(1, 18);
        let b2 = num(2, 0);  // 2
        let c2 = Math::mul(a2, b2);
        assert(Math::equals(c2, num(2, 18)), 3);

        // 4
        let a2 = num(1, 15);  // 1 * 10^-15
        let b2 = num(2, 3);  // 2 * 10^-3
        let c2 = Math::mul(a2, b2);
        assert(Math::equals(c2, num(2, 18)), 4);
    }
}
