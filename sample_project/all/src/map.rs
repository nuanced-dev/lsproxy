pub struct Map {
    pub m: [[i8; 8]; 8],
    pub w: i32,
    pub h: i32,
}

impl Map {
    pub fn new() -> Self {
        let m = [
            [0, 0, 0, 0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0, 0, 0, 0],
            [0, 0, 0, 0, 1, 1, 1, 0],
            [0, 0, 1, 0, 0, 0, 1, 0],
            [0, 0, 1, 0, 0, 0, 1, 0],
            [0, 0, 1, 1, 1, 1, 1, 0],
            [0, 0, 0, 0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0, 0, 0, 0],
        ];
        Map { m, w: 8, h: 8 }
    }

    pub fn get(&self, x: i32, y: i32) -> i8 {
        self.m[y as usize][x as usize]
    }
}
