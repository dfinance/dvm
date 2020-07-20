//#block:1024
//#success
script {
    use 0x01::Block;

    fun success() {
        assert(Block::get_current_block_height() == 1024, 1);
    }
}