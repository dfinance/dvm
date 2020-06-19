address 0x0000000000000000000000000000000000000001 {

module Foo {
     use 0x1::Base;
     use 0x0101010101010101010101010101010101010101::Base as Base1;

     struct T {g: u64}

     struct Vec<G> {g: vector<G>, t: vector<T> }

     resource struct R { f: u64, g: u64, }

     native struct BarNative<K, V>;

     resource struct Pool<AssetType: copyable> {
            t: AssetType,
     }

     resource struct Pool1<AssetType: resource> {
            t: AssetType,
     }

     struct Bar<K: copyable, V> {
        key: K,
        value: V,
     }

     struct G {t: T}

     resource struct GBase {t: Base1::Test, t2: Base::Test1}

     resource struct GBase2 {
             t: 0x00000000000000000000000000000001::Base::Test1,
             t2: 0x0101010101010101010101010101010101010101::Base::Test,
     }
}
}
