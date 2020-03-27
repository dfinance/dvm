module RWrapper {
    struct R { r: 0x0::RMod::R }

    public fun foo_r(f: u64, g: u64): 0x0::RMod::R {
        0x0::RMod::foo_r(f, g)
    }

     public fun foo_r_1(val: 0x0::RMod::R): (u64, u64) {
         0x0::RMod::foo_r_1(val)
     }

     public fun foo_r_ref(val: &0x0::RMod::R): (u64, u64) {
        0x0::RMod::foo_r_ref(val)
     }

     public fun foo_r_mut_ref(val: &mut 0x0::RMod::R): (u64, u64) {
        0x0::RMod::foo_r_mut_ref(val)
     }
}