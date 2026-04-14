// Vitalis — Traits and Impl Blocks
//
// Demonstrates defining traits and implementing methods on structs.

struct Circle {
    radius: f64,
}

struct Square {
    side: f64,
}

impl Circle {
    fn area(self) -> f64 {
        3.14159 * self.radius * self.radius
    }
}

impl Square {
    fn area(self) -> f64 {
        self.side * self.side
    }
}

fn main() -> i64 {
    let c = Circle { radius: 5.0 };
    let s = Square { side: 4.0 };
    let total = c.area() + s.area();
    // Circle: ~78.54, Square: 16.0, Total: ~94.54
    to_i64(total)
}
