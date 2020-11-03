script {
    use 0x1::Math::{Self, num, scale_to_decimals};

    fun main() {
        // 1
        let a = num(1, 0);
        let b = num(2, 0);
        let c = Math::add(a, b);
        assert(scale_to_decimals(c, 0) == 3, 1);

        // 2
        let a1 = num(1, 18);
        let b1 = num(2, 18);
        let c1 = Math::add(a1, b1);
        assert(Math::equals(c1, num(3, 18)), 2);

        // 3
        let a2 = num(1, 18);  // 1
        let b2 = num(2, 17);  // 20
        let c2 = Math::add(a2, b2);
        assert(Math::equals(c2, num(21, 18)), 3);
    }
}
