mod point;
mod node;
mod map;
mod astar;

use std::io::{self, Write};
use point::Point;
use map::Map;
use astar::AStar;

fn main() {
    let mut astar = AStar::new();
    let map = Map::new();
    let start = Point::new(0, 0);
    let end = Point::new(7, 7);

    if astar.search(start, end, map) {
        let (path, cost) = astar.path();
        
        for y in -1..9 {
            for x in -1..9 {
                if x < 0 || y < 0 || x > 7 || y > 7 || astar.m.get(x, y) == 1 {
                    print!("█");
                } else {
                    if path.contains(&Point::new(x, y)) {
                        print!("x");
                    } else {
                        print!(".");
                    }
                }
            }
            println!();
        }

        print!("\nPath cost {}: ", cost);
        io::stdout().flush().unwrap();
        
        for p in path {
            print!("({}, {}) ", p.x, p.y);
        }
        println!("\n");
    }
}
