fn main() -> i64 {
    let content = file_read("C:/Vitalis-V60/compiler/_test_args.sl")
    let clen = str_len(content)
    print_str("file_read: ")
    print(clen)
    println_str(" chars")

    let buf = array_new(5)
    array_set(buf, 0, 77)
    array_set(buf, 1, 90)
    array_set(buf, 2, 0)
    array_set(buf, 3, 255)
    array_set(buf, 4, 42)
    file_write_bytes("C:/Vitalis-V60/compiler/_test_out.bin", buf)
    println_str("wrote 5 bytes")
    0
}