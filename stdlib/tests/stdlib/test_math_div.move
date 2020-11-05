script {
    use 0x1::Math::{Self, num, scale_to_decimals};

    fun main() {
        // 1
        let a = num(6, 0);
        let b = num(2, 0);
        let c = Math::div(a, b);
        assert(scale_to_decimals(c, 0) == 3, 1);

        // 2
        let a1 = num(4, 18);
        let b1 = num(2, 0);
        let c1 = Math::div(a1, b1);
        assert(Math::equals(c1, num(2, 18)), 2);

        // 3
        let a2 = num(3, 0);
        let b2 = num(2, 0);
        let c2 = Math::div(a2, b2);
        assert(Math::equals(c2, num(15, 1)), 3);

        // 4
        let a2 = num(15, 1);  // 1.5
        let b2 = num(2, 1);  // 0.2
        let c2 = Math::div(a2, b2);  // should be 7.5
        assert(Math::equals(c2, num(75, 1)), 4);

        // 5
        let a2 = num(620000000000, 10);  // 62
        let b2 = num(1000000000000000000, 16);  // 100
        let c2 = Math::div(a2, b2);  // should be 0.62
        assert(Math::equals(c2, num(62, 2)), 5);

        // 6
        let a2 = num(620000000000, 10);  // 62
        let b2 = num(100000000000000000000, 18);  // 100
        let c2 = Math::div(a2, b2);  // should be 0.62
        assert(Math::equals(c2, num(62, 2)), 6);
    }
}
