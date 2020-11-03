script {
    use 0x1::Math::{Self, num, scale_to_decimals};

    fun main() {
        // 1
        let a = num(3, 0);
        let b = num(1, 0);
        let c = Math::sub(a, b);
        assert(scale_to_decimals(c, 0) == 2u128, 1);

        // 2
        let a1 = num(3, 18);
        let b1 = num(1, 18);
        let c1 = Math::sub(a1, b1);
        assert(Math::equals(c1, num(2, 18)), 2);

        // 3
        let a2 = num(1, 17);  // 10
        let b2 = num(2, 18);  // 2
        let c2 = Math::sub(a2, b2);
        assert(Math::equals(c2, num(8, 18)), 3);
    }
}