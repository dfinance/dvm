//#status:1081
script {
    use 0x02::Foo;
    fun invalid_link() {
        assert(Foo::get_val() == 1, 1);
    }
}

address 0x02 {
    //#address:0x02
    module Foo {
        public fun get_val(): u64 {
            1
        }
    }
}

script {
    use 0x02::Foo;
    fun success() {
        assert(Foo::get_val() == 1, 1);
    }
}