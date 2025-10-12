use crate::point::Point;

#[derive(Clone, PartialEq)]
pub struct Node {
    pub pos: Point,
    pub parent: Point,
    pub dist: i32,
    pub cost: i32,
}

impl PartialOrd for Node {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        (self.dist + self.cost).partial_cmp(&(other.dist + other.cost))
    }
}
