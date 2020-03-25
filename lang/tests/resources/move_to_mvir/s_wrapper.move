module SWrapper {
    use 0x0::SMod;
    use 0x0::RMod;

    struct S<T> { s: 0x0::SMod::S<T> }

    public fun create_r(): RMod::R {
       SMod::create_r()
    }

    public fun foo_s<T>(t: T): SMod::S<T> {
       SMod::foo_s(t)
    }

    public fun foo_s_1<T>(val: SMod::S<T>): T {
       SMod::foo_s_1(val)
    }
}