#![allow(unused)]

// This file is taken from the crate pav_regression
// Added following modifications:
// - genericity over f32, f64
// - struct BlockPoint that keep tracks of index of point in block through merge operations



use anyhow::{anyhow};

use ordered_float::OrderedFloat;
use num_traits::float::Float;

use num_traits::{NumAssign,FromPrimitive,NumCast};
use std::iter::{Sum, Product};
use std::ops::{Add,Neg, AddAssign};
use std::fmt::{Debug, Display, LowerExp, UpperExp};

use indxvec::Vecops;


/// Isotonic regression can be done in either mode
enum Direction {
    Ascending,
    Descending,
}
/// A point in 2D cartesian space
#[derive(Debug, PartialEq, Copy, Clone)]
pub struct Point<T:Float> {
    x: T,
    y: T,
    weight: T,
}

impl <T> Point<T> 
    where  T : Float + std::ops::AddAssign {
    /// Create a new Point
    pub fn new(x: T, y: T) -> Point<T> {
        Point { x, y, weight: T::from(1.0).unwrap() }
    }

    /// Create a new Point with a specified weight
    pub fn new_with_weight(x: T, y: T, weight: T) -> Point<T> {
        Point { x, y, weight }
    }

    // Use getters because modifying points that are part of a regression will have unpredictable
    // results.

    /// The x position of the point
    pub fn x(&self) -> T {
        self.x
    }

    /// The y position of the point
    pub fn y(&self) -> T {
        self.y
    }

    /// The weight of the point (initially 1.0)
    pub fn weight(&self) -> T {
        self.weight
    }

    fn merge_with(&mut self, other: &Point<T>) {
        self.x = ((self.x * self.weight) + (other.x * other.weight)) / (self.weight + other.weight);

        self.y = ((self.y * self.weight) + (other.y * other.weight)) / (self.weight + other.weight);

        self.weight += other.weight;
    }
}


/// 
impl <T:Float> PartialOrd for Point<T> {
    fn partial_cmp(&self, other: &Point<T>) -> Option<std::cmp::Ordering> {
        self.x.partial_cmp(&other.x)
    }
} // end of impl PartialOrd for Point<T> 




fn interpolate_two_points<T>(a: &Point<T>, b: &Point<T>, at_x: &T) -> T  
    where T : Float {
    let prop = (*at_x - (a.x)) / (b.x - a.x);
    (b.y - a.y) * prop + a.y
}


//==========================================================================================================

/// To store a block of points in isotonic regression
/// This structure does the merging.
struct BlockPoint<'a, T:Float> {
    /// sorting direction
    direction : Direction,
    /// unsorted points,   
    points : &'a Vec<Point<T>>,
    /// so that i -> points[sorted_index[i]] is sorted according to direction
    sorted_index : &'a[usize],
    /// first index in sorted index. first is in block. So the block is [first, last[
    first : usize,
    /// last index in sorted index, last is outside block
    last : usize,
} // end of BlockPoint

impl <'a, T> BlockPoint<'a, T>  
    where T : Float {
    
    /// merge two contiguous BlockPoint
    fn merge(&mut self, other : &BlockPoint<'a, T>) ->  Result<(), anyhow::Error> {
        // check contiguity
        if self.last == other.first {
            self.last = other.last;
        }
        else if self.first == other.last {
            self.first = other.first;
        }
        else {
            log::error!("not contiguous blocks");
            return Err(anyhow!("not contiguous blocks"));                    
        }
        return Ok(());
    } // end of merge


    fn get_centroid(&self) ->  Point<T> {
        panic!("not yt implemented");
    }

    /// get first index of block in the Direction ordering
    fn get_first_index(&self) -> usize {
        self.first
    }

    /// get last index of block in the Direction ordering
    fn get_last_index(&self) -> usize {
        self.last
    }
} // end of impl BlockPoint

//==========================================================================================================


/// A vector of points forming an isotonic regression, along with the
/// centroid point of the original set.

#[derive(Debug, Clone)]
pub struct IsotonicRegression<T:Float> {
    points: Vec<Point<T>>,
    centroid_point: Point<T>,
}


impl <T> IsotonicRegression<T> 
    where T : Float + std::iter::Sum + FromPrimitive + std::ops::AddAssign {
    /// Find an ascending isotonic regression from a set of points
    pub fn new_ascending(points: &[Point<T>]) -> IsotonicRegression<T> {
        IsotonicRegression::new(points, Direction::Ascending)
    }

    /// Find a descending isotonic regression from a set of points
    pub fn new_descending(points: &[Point<T>]) -> IsotonicRegression<T> {
        IsotonicRegression::new(points, Direction::Descending)
    }

    fn new(points: &[Point<T>], direction: Direction) -> IsotonicRegression<T> {
        assert!(points.len() > 0, "points is empty, can't create regression");
        let point_count: T = points.iter().map(|p| p.weight).sum();
        let mut sum_x: T = T::from(0.0).unwrap();
        let mut sum_y: T = T::from(0.0).unwrap();
        for point in points {
            sum_x += point.x * point.weight;
            sum_y += point.y * point.weight;
        }

        IsotonicRegression {
            points: isotonic(points, direction),
            centroid_point: Point::new(sum_x / point_count, sum_y / point_count),
        }
    }

    /// Find the _y_ point at position `at_x`
    pub fn interpolate(&self, at_x: T) -> T 
        where T : Float {
        if self.points.len() == 1 {
            return self.points[0].y;
        } else {
            let pos = self
                .points
                .binary_search_by_key(&OrderedFloat(at_x), |p| OrderedFloat(p.x));
            return match pos {
                Ok(ix) => self.points[ix].y,
                Err(ix) => {
                    if ix < 1 {
                        interpolate_two_points(
                            &self.points.first().unwrap(),
                            &self.centroid_point,
                            &at_x,
                        )
                    } else if ix >= self.points.len() {
                        interpolate_two_points(
                            &self.centroid_point,
                            self.points.last().unwrap(),
                            &at_x,
                        )
                    } else {
                        interpolate_two_points(&self.points[ix - 1], &self.points[ix], &at_x)
                    }
                }
            };
        }
    }

    /// Retrieve the points that make up the isotonic regression
    pub fn get_points(&self) -> &[Point<T>] {
        &self.points
    }

    /// Retrieve the mean point of the original point set
    pub fn get_centroid_point(&self) -> &Point<T> {
        &self.centroid_point
    }
}


fn isotonic<T>(points: &[Point<T>], direction: Direction) -> Vec<Point<T>> 
    where T : Float + AddAssign {
    let mut merged_points: Vec<Point<T>> = match direction {
        Direction::Ascending => points.iter().copied().collect(),
        Direction::Descending => points.iter().map(|p| Point { y: -p.y, ..*p }).collect(),
    };

    merged_points.sort_by_key(|point| OrderedFloat(point.x));

    let mut iso_points: Vec<Point<T>> = Vec::new();
    for point in &mut merged_points.iter() {
        if iso_points.is_empty() || (point.y > iso_points.last().unwrap().y) {
            iso_points.push(*point)
        } else {
            let mut new_point = *point;
            loop {
                if iso_points.is_empty() || (iso_points.last().unwrap().y < (new_point).y) {
                    iso_points.push(new_point);
                    break;
                } else {
                    let last_to_repl = iso_points.pop();
                    new_point.merge_with(&last_to_repl.unwrap());
                }
            }
        }
    }

    return match direction {
        Direction::Ascending => iso_points,
        Direction::Descending => iso_points.iter().map(|p| Point { y: -p.y, ..*p }).collect(),
    };
}




#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usage_example() {
        let points = &[
            Point::<f64>::new(0.0, 1.0),
            Point::<f64>::new(1.0, 2.0),
            Point::<f64>::new(2.0, 1.5),
        ];

        let regression = IsotonicRegression::new_ascending(points);
        assert_eq!(
            regression.interpolate(1.5), 1.75
        );
    }

    #[test]
    fn isotonic_no_points() {
        assert_eq!(isotonic(&[] as &[Point<f64>; 0], Direction::Ascending).is_empty(), true);
    }

    #[test]
    fn isotonic_one_point() {
        assert_eq!(
            isotonic(&[Point::<f64>::new(1.0, 2.0)], Direction::Ascending)
                .pop()
                .unwrap(),
            Point::<f64>::new(1.0, 2.0)
        );
    }

    #[test]
    fn isotonic_simple_merge() {
        assert_eq!(
            isotonic(
                &[Point::<f64>::new(1.0, 2.0), Point::<f64>::new(2.0, 0.0)],
                Direction::Ascending
            )
            .pop()
            .unwrap(),
            Point::new_with_weight(1.5, 1.0, 2.0)
        );
    }

    #[test]
    fn isotonic_one_not_merged() {
        assert_eq!(
            isotonic(
                &[
                    Point::new(0.5, -0.5),
                    Point::new(1.0, 2.0),
                    Point::new(2.0, 0.0),
                ],
                Direction::Ascending
            ),
            [Point::new(0.5, -0.5), Point::new_with_weight(1.5, 1.0, 2.0)]
        );
    }

    #[test]
    fn isotonic_merge_three() {
        assert_eq!(
            isotonic(
                &[
                    Point::new(0.0, 1.0),
                    Point::new(1.0, 2.0),
                    Point::new(2.0, -1.0),
                ],
                Direction::Ascending
            ),
            [Point::new_with_weight(1.0, 2.0 / 3.0, 3.0)]
        );
    }

    #[test]
    fn test_interpolate() {
        let regression =
            IsotonicRegression::new_ascending(&[Point::new(1.0, 5.0), Point::new(2.0, 7.0)]);
        assert!((regression.interpolate(1.5) - 6.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_isotonic_ascending() {
        let points = &[
            Point::new(0.0, 1.0),
            Point::new(1.0, 2.0),
            Point::new(2.0, -1.0),
        ];

        let regression = IsotonicRegression::new_ascending(points);
        assert_eq!(
            regression.get_points(),
            &[Point::new_with_weight(
                (0.0 + 1.0 + 2.0) / 3.0,
                (1.0 + 2.0 - 1.0) / 3.0,
                3.0
            )]
        )
    }

    #[test]
    fn test_isotonic_descending() {
        let points = &[
            Point::new(0.0, -1.0),
            Point::new(1.0, 2.0),
            Point::new(2.0, 1.0),
        ];
        let regression = IsotonicRegression::new_descending(points);
        assert_eq!(
            regression.get_points(),
            &[Point::new_with_weight(1.0, 2.0 / 3.0, 3.0)]
        )
    }

    #[test]
    fn test_descending_interpolation() {
        let regression = IsotonicRegression::new_descending(&[
            Point::new(0.0, 3.0),
            Point::new(1.0, 2.0),
            Point::new(2.0, 1.0),
        ]);
        assert_eq!(regression.interpolate(0.5), 2.5);
    }

    #[test]
    fn test_single_point_regression() {
        let regression = IsotonicRegression::new_ascending(&[
            Point::new(1.0, 3.0),
        ]);
        assert_eq!(regression.interpolate(0.0), 3.0);
    }

    #[test]
    fn test_point_accessors() {
        let point = Point { x: 1.0, y: 2.0 , weight : 3.0};
        assert_eq!(point.x(), 1.0);
        assert_eq!(point.y(), 2.0);
        assert_eq!(point.weight(), 3.0);
    }
}
