module CWrapper {
    use 0x0::CMod;
    resource struct C { val: CMod::C }

    public fun foo_c(val: bool): CMod::C {
      CMod::foo_c(val)
    }

    public fun foo_c_1(val: CMod::C): bool {
        CMod::foo_c_1(val)
    }

    public fun foo_c_ref(val: &CMod::C): bool {
       CMod::foo_c_ref(val)
    }

    public fun foo_c_mut_ref(val: &mut CMod::C): bool {
       CMod::foo_c_mut_ref(val)
    }

    public fun pick_c(): bool {
       CMod::pick_c()
    }
}