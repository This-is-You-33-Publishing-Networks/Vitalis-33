fn main() -> i64 {
    let ac = args_count()
    print_str("total args: ")
    println(ac)
    let mut i = 0
    while i < ac {
        print_str("  [")
        print(i)
        print_str("] = '")
        print_str(args_get(i))
        println_str("'")
        i = i + 1
    }
    0
}