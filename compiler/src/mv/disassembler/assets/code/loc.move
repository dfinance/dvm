module M {
    resource struct R { f: u64, g: u64}

    struct R1<T> {
        t: T,
        r: u8
    }

    public fun t0(r: &R): &u64 {
        &r.f
    }

    public fun t1(r: &mut R): &u64 {
        &mut r.f
    }

    public fun t2(r: &mut R): &mut u64 {
        &mut r.f
    }

    public fun t00<T>(r: &R1<T>): &T {
        &r.t
    }

    public fun t01<T>(r: &mut R1<T>): &T {
        &mut r.t
    }

    public fun t02<T>(r: &mut R1<T>): &mut T {
        &mut r.t
    }
}