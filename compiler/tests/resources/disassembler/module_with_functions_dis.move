address 0x0000000000000000000000000000000000000000 {

module Foo {
    use 0x0000000000000000000000000000000000000000::Base as OtherBase1;
    use 0x0101010101010101010101010101010101010101::Base;
    resource struct Res<__G_1, __G_2: copyable> {
        g: __G_1,
        q: __G_1,
        e: T<__G_2>,
    }

    resource struct T<__G_1: copyable> {
        g: __G_1,
    }

    resource struct T1 {
        g: u64,
    }

    resource struct TRes<__G_1: resource> {
        g: __G_1,
    }

    resource struct X_phantom_resource_X_ {
        dummy_field: bool,
    }

    fun fun_acquires(): (u64, u64, address) acquires T1 {
        borrow_global<T1>(0x0);
        abort 1
    }

    fun fun_multiple_acquires<__G_1: copyable>(): (u64, u64, address, T<__G_1>) acquires T, T1, TRes {
        borrow_global<T<u64>>(0x0);
        borrow_global<T1>(0x0);
        borrow_global<TRes<X_phantom_resource_X_>>(0x0);
        abort 1
    }

    fun fun_with_multiple_ret<__G_1>(): (&u64, &mut u64, u128, Base::Test, OtherBase1::Test1, vector<Res<__G_1, u64>>) {
        abort 1
    }

    fun fun_with_ret(): u64 {
        abort 1
    }

    fun get_generic<__G_1, __G_2, __G_3>(_arg_1: __G_2) {
        abort 1
    }

    native public fun print<__G_1>(_arg_1: &__G_1);

    native public fun print_copyable<__G_1: copyable>(_arg_1: &__G_1);

    native public fun print_double(_arg_1: &mut u64, _arg_2: &u64);

    native fun print_empty();

    native public fun print_multiple_resource<__G_1: resource, __G_2>(_arg_1: &__G_1, _arg_2: __G_2);

    fun print_private() {
        abort 1
    }

    native public fun print_resource<__G_1: resource>(_arg_1: &__G_1);

    fun return_generic<__G_1, __G_2, __G_3>(): __G_2 {
        abort 1
    }

    native public fun return_vector<__G_1: copyable>(_arg_1: &__G_1): vector<__G_1>;

    native public fun vector_params<__G_1: copyable>(_arg_1: vector<__G_1>);

}
}
