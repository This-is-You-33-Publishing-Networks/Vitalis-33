fn main() -> i64 {
    let ac = args_count()
    print_str("args_count = ")
    println(ac)
    let mut i = 0
    while i < ac {
        print_str("  arg[")
        print(i)
        print_str("] = ")
        println_str(args_get(i))
        i = i + 1
    }
    0
}