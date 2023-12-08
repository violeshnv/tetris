use std::ops::{AddAssign, Index, IndexMut};
use terminal::Color;

pub const POINT_OF_BLOCK_COUNT: usize = 4;
pub const ORIENTATION_COUNT: usize = 4;

type PointArray = [Point; POINT_OF_BLOCK_COUNT];
type PointMatrix = [PointArray; ORIENTATION_COUNT];

#[derive(Default, Clone, Copy)]
pub struct Point {
    pub x: isize,
    pub y: isize,
}

impl Point {
    pub const fn new(x: isize, y: isize) -> Self {
        Point { x, y }
    }
    const fn new_n(n: isize) -> Self {
        Point::new(n / 10, n % 10)
    }
}

impl AddAssign<&Self> for Point {
    fn add_assign(&mut self, rhs: &Self) {
        self.x += rhs.x;
        self.y += rhs.y;
    }
}

impl AddAssign<&Point> for PointArray {
    fn add_assign(&mut self, rhs: &Point) {
        for p in self {
            p.add_assign(rhs);
        }
    }
}

impl AddAssign<&Point> for PointMatrix {
    fn add_assign(&mut self, rhs: &Point) {
        for pa in self {
            pa.add_assign(rhs);
        }
    }
}

#[derive(Clone, Copy)]
pub struct Block {
    color: Color,
    point_matrix: PointMatrix,
}

impl Block {
    const fn new(color: Color, point_matrix: PointMatrix) -> Self {
        Block {
            color,
            point_matrix,
        }
    }

    const fn new_n(color: Color, ns: [isize; POINT_OF_BLOCK_COUNT * ORIENTATION_COUNT]) -> Self {
        // why default() is not const fn?
        let mut point_matrix = [[Point::new_n(0); POINT_OF_BLOCK_COUNT]; ORIENTATION_COUNT];
        let mut i: usize = 0;
        while {
            let mut j: usize = 0;
            while {
                point_matrix[i][j] = Point::new_n(ns[i * POINT_OF_BLOCK_COUNT + j]);
                j += 1;
                j != POINT_OF_BLOCK_COUNT
            } {}
            i += 1;
            i != ORIENTATION_COUNT
        } {}

        Block::new(color, point_matrix)
    }
}

pub const BLOCKS: [Block; 7] = [
    // OrangeRicky J
    Block::new_n(
        Color::Rgb(30, 144, 255),
        [
            01, 11, 21, 20, //
            12, 22, 11, 10, //
            02, 01, 11, 21, //
            12, 11, 00, 10, //
        ],
    ),
    // BlueRicky L
    Block::new_n(
        Color::Rgb(255, 140, 0),
        [
            01, 11, 21, 00, //
            12, 11, 10, 20, //
            22, 01, 11, 21, //
            02, 12, 11, 10, //
        ],
    ),
    // ClevelandZ Z
    Block::new_n(
        Color::Rgb(255, 99, 71),
        [
            11, 21, 00, 10, //
            12, 11, 21, 20, //
            12, 22, 01, 11, //
            02, 01, 11, 10, //
        ],
    ),
    // RhodeIslandZ S
    Block::new_n(
        Color::Rgb(0, 250, 154),
        [
            01, 11, 10, 20, //
            22, 11, 21, 10, //
            02, 12, 11, 21, //
            12, 01, 11, 00, //
        ],
    ),
    // Hero I
    Block::new_n(
        Color::Rgb(176, 224, 230),
        [
            01, 11, 21, 31, //
            23, 21, 22, 20, //
            02, 12, 22, 32, //
            13, 11, 12, 10, //
        ],
    ),
    // Teewee T
    Block::new_n(
        Color::Rgb(138, 43, 226),
        [
            01, 11, 21, 10, //
            12, 11, 21, 10, //
            12, 01, 11, 21, //
            12, 01, 11, 10, //
        ],
    ),
    // Smashboy O
    Block::new_n(
        Color::Rgb(250, 250, 210),
        [
            11, 21, 10, 20, //
            11, 21, 10, 20, //
            11, 21, 10, 20, //
            11, 21, 10, 20, //
        ],
    ),
];

pub struct StackedBlock {
    pub colors: Vec<Vec<Color>>,
}

impl StackedBlock {
    pub fn new(column: usize, row: usize) -> Self {
        debug_assert!(row > 0 && column > 0);
        let default_color = Color::Reset;
        StackedBlock {
            colors: vec![vec![default_color; column as usize]; row as usize],
        }
    }

    pub fn is_valid_index(&self, point: &Point) -> bool {
        let Point { x, y } = *point;
        let (ux, uy) = (x as usize, y as usize);
        let (row, col) = (self.get_row(), self.get_column());
        0 <= x && 0 <= y && ux < col && uy < row
    }

    pub fn get_row(&self) -> usize {
        self.colors.len()
    }

    pub fn get_column(&self) -> usize {
        self.colors.last().unwrap().len()
    }

    pub fn cover(&mut self, color: Color, points: &[Point]) {
        for p in points {
            if self.is_valid_index(p) {
                self[p] = color;
            }
        }
    }

    pub fn is_overlapped(&self, points: &[Point]) -> bool {
        points
            .iter()
            .any(|p| self.is_valid_index(p) && self[p] != Color::Reset)
    }

    // pub fn stack(&mut self, color: Color, points: &[Point]) -> bool {
    //     if self.is_overlapped(points) {
    //         false
    //     } else {
    //         self.(color, points);
    //         true
    //     }
    // }

    pub fn full_lines(&self) -> Vec<usize> {
        self.colors
            .iter()
            .enumerate()
            .filter_map(|(i, line)| {
                if line.iter().all(|x| *x != Color::Reset) {
                    Some(i)
                } else {
                    None
                }
            })
            .collect()
    }

    // pub fn lines_full(&self) -> Vec<bool> {
    //     self.colors
    //         .iter()
    //         .map(|line| line.iter().all(|x| *x != Color::Reset))
    //         .collect()
    // }

    pub fn eliminate(&mut self, lines: &Vec<usize>) {
        debug_assert!(*lines == self.full_lines(), "eliminate non-full lines");
        let mut full = vec![true; self.get_row()];
        for line in lines.iter() {
            full[*line] = false;
        }

        let mut iter = full.iter();
        self.colors.retain(|_| *iter.next().unwrap());

        self.colors.resize(
            self.colors.len() + lines.len(),
            vec![Color::Reset; self.get_column()],
        );
    }
}

impl Index<&Point> for StackedBlock {
    type Output = Color;

    fn index(&self, point: &Point) -> &Self::Output {
        let Point { x, y } = *point;
        let (ux, uy) = (x as usize, y as usize);
        debug_assert!(self.is_valid_index(&point));
        &self.colors[uy][ux]
    }
}

impl IndexMut<&Point> for StackedBlock {
    fn index_mut(&mut self, point: &Point) -> &mut Self::Output {
        let Point { x, y } = *point;
        let (ux, uy) = (x as usize, y as usize);
        debug_assert!(self.is_valid_index(&point));
        &mut self.colors[uy][ux]
    }
}

pub struct FallingBlock {
    block: Block,
    orientation: usize,
}

impl Default for FallingBlock {
    fn default() -> Self {
        let x = rand::random::<usize>() % (BLOCKS.len() * ORIENTATION_COUNT);
        Self::new(x / ORIENTATION_COUNT, x % ORIENTATION_COUNT)
    }
}

impl FallingBlock {
    pub fn new(block_idx: usize, orientation: usize) -> Self {
        debug_assert!(block_idx < BLOCKS.len() && orientation < ORIENTATION_COUNT);
        FallingBlock {
            block: BLOCKS[block_idx],
            orientation,
        }
    }

    pub fn shift(&mut self, p: &Point) {
        for points in self.block.point_matrix.iter_mut() {
            for point in points.iter_mut() {
                *point += p;
            }
        }
    }

    pub fn color(&self) -> &Color {
        &self.block.color
    }

    pub fn points(&self) -> &PointArray {
        debug_assert!(self.orientation < 4);
        &self.block.point_matrix[self.orientation]
    }

    // pub fn points_mut(&mut self) -> &mut PointArray {
    //     debug_assert!(self.orientation < 4);
    //     &mut self.block.point_matrix[self.orientation]
    // }

    // pub fn rotate(&mut self) {
    //     self.rotate_counterclockwise();
    // }

    // pub fn rotate_clockwise(&mut self) {
    //     self.orientation = (self.orientation + ORIENTATION_COUNT - 1) & (ORIENTATION_COUNT - 1);
    // }

    // pub fn rotate_counterclockwise(&mut self) {
    //     self.orientation = (self.orientation + 1) & (ORIENTATION_COUNT - 1);
    // }
}

impl AddAssign<isize> for FallingBlock {
    fn add_assign(&mut self, rhs: isize) {
        self.orientation =
            ((self.orientation as isize - rhs) & (ORIENTATION_COUNT - 1) as isize) as usize;
    }
}
