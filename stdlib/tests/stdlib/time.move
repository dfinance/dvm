//#time:25.06.2020T10:38:10
script {
    use 0x01::Time;

    fun success() {
        assert(Time::now() == 1593081490, 1);
    }
}