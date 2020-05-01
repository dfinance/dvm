address 0x000000000000000000000000000000000000000000000000:

module Foo {
    use 0x000000000000000000000000000000000000000000000000::Base as OtherBase1;
    use 0x010101010101010101010101010101010101010101010101::Base;
    struct Bar<__G_1: copyable, __G_2> {
        key: __G_1,
        value: __G_2,
    }

    native struct BarNative<__G_1, __G_2>;

    struct G {
        t: T,
    }

    resource struct GBase {
        t: Base::Test,
        t2: OtherBase1::Test1,
    }

    resource struct GBase2 {
        t: OtherBase1::Test1,
        t2: Base::Test,
    }

    resource struct Pool<__G_1: copyable> {
        t: __G_1,
    }

    resource struct Pool1<__G_1: resource> {
        t: __G_1,
    }

    resource struct R {
        f: u64,
        g: u64,
    }

    struct T {
        g: u64,
    }

    struct Vec<__G_1> {
        g: vector<__G_1>,
        t: vector<T>,
    }

}
