script {
    fun success() {}
}

//#error:10
script {
    fun test_assert() {
        assert(false, 10);
    }
}

//#error
script {
    fun error_with_main_status() {
        0 - 1;
    }
}

//#status:4017  ARITHMETIC_ERROR
script {
    fun error_with_main_status_1() {
        0 - 1;
    }
}

//#status:4002  ARITHMETIC_ERROR
//#gas:1
script {
    fun error_out_of_gas() {
        loop {}
    }
}