address 0x4 {
module Bench {
    public fun arith() {
        let i = 0;
        while (i < 10000) {
            1;
            10 + 3;
            10;
            7 + 5;
            let x = 1;
            let y = x + 3;
            assert(x + y == 5, 10);
            i = i + 1;
        };
    }

    public fun call() {
        let i = 0;
        while (i < 3000) {
            let b = call_1(0x0, 128);
            call_2(b);
            i = i + 1;
        };
    }

    fun call_1(addr: address, val: u64): bool {
        let b = call_1_1(&addr);
        call_1_2(val, val);
        b
    }

    fun call_1_1(_addr: &address): bool {
        true
    }

    fun call_1_2(val1: u64, val2: u64): bool {
        val1 == val2
    }

    fun call_2(b: bool) {
        call_2_1(b);
        assert(call_2_2() == 400, 200);
    }

    fun call_2_1(b: bool) {
        assert(b == b, 100)
    }

    fun call_2_2(): u64 {
        100 + 300
    }
}
}

script {
    use 0x04::Bench;

    fun main() {
        Bench::arith();
        Bench::call();
    }
}