module RMod {
    struct R { f: u64, g: u64 }

    public fun foo_r(f: u64, g: u64): R {
        R {f : f, g: g}
    }

     public fun foo_r_1(val: Self::R): (u64, u64) {
        let R { f, g } = val;
        (f, g)
     }

     public fun foo_r_ref(val: &Self::R): (u64, u64) {
        (val.f, val.g)
     }

     public fun foo_r_mut_ref(val: &mut Self::R): (u64, u64) {
         (val.f, val.g)
     }
}