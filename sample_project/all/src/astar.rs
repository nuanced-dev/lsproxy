use crate::point::Point;
use crate::node::Node;
use crate::map::Map;

pub struct AStar {
    neighbours: [Point; 8],
    open: Vec<Node>,
    closed: Vec<Node>,
    pub m: Map,
    start: Point,
    end: Point,
}

impl AStar {
    pub fn new() -> Self {
        let neighbours = [
            Point::new(-1, -1), Point::new(1, -1),
            Point::new(-1, 1),  Point::new(1, 1),
            Point::new(0, -1),  Point::new(-1, 0),
            Point::new(0, 1),   Point::new(1, 0),
        ];
        AStar {
            neighbours,
            open: Vec::new(),
            closed: Vec::new(),
            m: Map::new(),
            start: Point::new(0, 0),
            end: Point::new(0, 0),
        }
    }

    fn calc_dist(&self, p: &Point) -> i32 {
        let x = self.end.x - p.x;
        let y = self.end.y - p.y;
        x * x + y * y
    }

    fn is_valid(&self, p: &Point) -> bool {
        p.x >= 0 && p.y >= 0 && p.x < self.m.w && p.y < self.m.h
    }

    fn exist_point(&mut self, p: &Point, cost: i32) -> bool {
        if let Some(pos) = self.closed.iter().position(|n| n.pos == *p) {
            if self.closed[pos].cost + self.closed[pos].dist < cost {
                return true;
            }
            self.closed.remove(pos);
            return false;
        }
        if let Some(pos) = self.open.iter().position(|n| n.pos == *p) {
            if self.open[pos].cost + self.open[pos].dist < cost {
                return true;
            }
            self.open.remove(pos);
            return false;
        }
        false
    }

    fn fill_open(&mut self, n: &Node) -> bool {
        // Create a local copy of neighbours to avoid borrowing self
        let neighbours = self.neighbours;
        
        for (i, neighbour_offset) in neighbours.iter().enumerate() {
            let step_cost = if i < 4 { 1 } else { 1 };
            let neighbour = n.pos + *neighbour_offset;
            
            if neighbour == self.end {
                return true;
            }

            if self.is_valid(&neighbour) && self.m.get(neighbour.x, neighbour.y) != 1 {
                let nc = step_cost + n.cost;
                let dist = self.calc_dist(&neighbour);
                
                if !self.exist_point(&neighbour, nc + dist) {
                    self.open.push(Node {
                        cost: nc,
                        dist,
                        pos: neighbour,
                        parent: n.pos,
                    });
                }
            }
        }
        false
    }

    pub fn search(&mut self, s: Point, e: Point, mp: Map) -> bool {
        self.end = e;
        self.start = s;
        self.m = mp;
        
        self.open.push(Node {
            cost: 0,
            pos: s,
            parent: Point::new(0, 0),
            dist: self.calc_dist(&s),
        });

        while !self.open.is_empty() {
            self.open.sort_by(|a, b| (a.cost + a.dist).cmp(&(b.cost + b.dist)));
            let n = self.open.remove(0);
            self.closed.push(n.clone());
            if self.fill_open(&n) {
                return true;
            }
        }
        false
    }

    pub fn path(&self) -> (Vec<Point>, i32) {
        let mut path = vec![self.end];
        let cost = 1 + self.closed.last().unwrap().cost;
        path.push(self.closed.last().unwrap().pos);
        let mut parent = self.closed.last().unwrap().parent;

        for node in self.closed.iter().rev() {
            if node.pos == parent && !(node.pos == self.start) {
                path.push(node.pos);
                parent = node.parent;
            }
        }
        path.push(self.start);
        path.reverse();
        (path, cost)
    }
}
