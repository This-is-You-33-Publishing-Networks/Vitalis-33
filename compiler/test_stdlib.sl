// Test the v60 self-hosting bootstrap stdlib primitives

fn main() -> i64 {
    // Test char_to_int
    let code_a = char_to_int("A")
    let code_0 = char_to_int("0")
    let code_space = char_to_int(" ")

    // A=65, 0=48, space=32
    assert_eq(code_a, 65)
    assert_eq(code_0, 48)
    assert_eq(code_space, 32)

    // Test int_to_char
    let ch = int_to_char(65)
    assert_true(str_eq(ch, "A"))

    let ch2 = int_to_char(48)
    assert_true(str_eq(ch2, "0"))

    // Test str_cat (concatenation)
    let hello = str_cat("hello", " world")
    assert_true(str_eq(hello, "hello world"))

    // Test array_new + array_len + array_get + array_set
    let arr = array_new(5)
    assert_eq(array_len(arr), 5)
    assert_eq(array_get(arr, 0), 0)  // zero-filled

    array_set(arr, 0, 42)
    array_set(arr, 1, 99)
    assert_eq(array_get(arr, 0), 42)
    assert_eq(array_get(arr, 1), 99)

    // Test args_count (should be at least 1 for vtc itself)
    let argc = args_count()
    assert_true(argc > 0)

    println_str("All v60 bootstrap stdlib tests passed!")
    42
}
