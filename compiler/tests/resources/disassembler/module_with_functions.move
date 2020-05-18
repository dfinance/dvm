address 0x0000000000000000000000000000000000000000 {

module Foo {
     use 0x0::Base as Base1;
     use 0x0101010101010101010101010101010101010101::Base;

     resource struct T<V: copyable> {g: V}
     resource struct TRes<V: resource> {g: V}
     resource struct T1 {g: u64}
     resource struct Res<Q, V: copyable> {
        g: Q,
        q: Q,
        e: T<V>,
     }

     native public fun print<T>(x: &T);
     native public fun print_resource<T: resource>(x: &T);
     native public fun print_multiple_resource<T: resource, T1>(x: &T, x2: T1);
     native public fun print_copyable<T: copyable>(x: &T);
     native public fun print_double(x: &mut u64, y: &u64);
     native fun print_empty();

     native public fun return_vector<T: copyable>(x: &T): vector<T>;
     native public fun vector_params<T: copyable>(x: vector<T>);

     fun return_generic<G, G1, G2>(): G1 {
         abort 1
     }

     fun get_generic<G, _G_1, G2>(_g: _G_1) {
              abort 1
     }

     fun print_private() {
         abort 1
     }

     fun fun_with_ret(): u64 {
         abort 1
     }

     fun fun_with_multiple_ret<G>(): (&u64, &mut u64, u128, Base::Test, Base1::Test1, vector<Res<G, u64>>) {
         abort 1
     }

     fun fun_acquires(): (u64, u64, address) acquires T1 {
        borrow_global<T1>(0x0);
        abort 1
     }

     fun fun_multiple_acquires<G:copyable>(): (u64, u64, address, T<G>) acquires T, T1, TRes {
        borrow_global<T<u64>>(0x0);
        borrow_global<TRes<T1>>(0x0);
        borrow_global<T1>(0x0);
        abort 1
     }
}
}
