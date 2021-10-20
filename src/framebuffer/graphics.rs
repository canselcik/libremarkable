use crate::framebuffer::cgmath::*;
use crate::framebuffer::common::*;

macro_rules! min {
        ($x: expr) => ($x);
        ($x: expr, $($z: expr),+) => (::std::cmp::min($x, min!($($z),*)));
}

macro_rules! max {
        ($x: expr) => ($x);
        ($x: expr, $($z: expr),+) => (::std::cmp::max($x, max!($($z),*)));
}

pub fn stamp_along_line<F>(stamp: &mut F, start: Point2<i32>, end: Point2<i32>) -> mxcfb_rect
where
    F: FnMut(Point2<i32>),
{
    // Create local variables for moving start point
    let mut x = start.x;
    let mut y = start.y;

    // Get absolute x/y offset
    let dx = if start.x > end.x {
        start.x - end.x
    } else {
        end.x - start.x
    };
    let dy = if start.y > end.y {
        start.y - end.y
    } else {
        end.y - start.y
    };

    // Get slopes
    let sx = if start.x < end.x { 1 } else { -1 };
    let sy = if start.y < end.y { 1 } else { -1 };

    // Initialize error
    let mut err = if dx > dy { dx } else { -dy } / 2;
    let mut err2;

    let (mut min_x, mut max_x, mut min_y, mut max_y) = (x, x, y, y);
    loop {
        // Stamp point
        stamp(Point2 { x, y });

        max_y = max!(max_y, y);
        min_y = min!(min_y, y);
        min_x = min!(min_x, x);
        max_x = max!(max_x, x);

        // Check end condition
        if x == end.x && y == end.y {
            break;
        };

        // Store old error
        err2 = 2 * err;

        // Adjust error and start position
        if err2 > -dx {
            err -= dy;
            x += sx;
        }
        if err2 < dy {
            err += dx;
            y += sy;
        }
    }

    mxcfb_rect {
        top: (min_y) as u32,
        left: (min_x) as u32,
        width: (max_x - min_x) as u32,
        height: (max_y - min_y) as u32,
    }
}
pub fn fill_polygon<F>(write_pixel: &mut F, points: &[Point2<i32>]) -> mxcfb_rect
where
    F: FnMut(Point2<i32>),
{
    // This implementation of polygon rasterisation is based on this article:
    // https://hackernoon.com/computer-graphics-scan-line-polygon-fill-algorithm-3cb47283df6

    // struct to hold edge data
    #[derive(Debug, Copy, Clone)]
    struct EdgeBucket {
        ymax: i32,
        ymin: i32,
        x: i32,
        sign: i32,
        direction: i32,
        dx: i32,
        dy: i32,
        sum: i32,
    }

    // initialise our edge table
    let mut edge_table = Vec::new();
    let num_edges = points.len();
    for i in 0..num_edges {
        let p0 = points[i];
        let p1 = points[(i + 1) % num_edges];
        let (lower, higher, direction) = if p0.y < p1.y {
            (p0, p1, 1)
        } else {
            (p1, p0, -1)
        };
        edge_table.push(EdgeBucket {
            ymax: higher.y,
            ymin: lower.y,
            x: lower.x,
            sign: if lower.x > higher.x { 1 } else { -1 },
            direction,
            dx: (higher.x - lower.x).abs(),
            dy: (higher.y - lower.y).abs(),
            sum: 0,
        });
    }
    // sort the edge table by ymin
    edge_table.sort_unstable_by_key(|p| p.ymin);

    // create active list
    let mut active_list = Vec::<EdgeBucket>::new();

    // initialise scanline with lowest ymin
    let mut scanline = edge_table[0].ymin;

    while !edge_table.is_empty() {
        // remove edges that end on the current scanline
        edge_table.retain(|edge| edge.ymax != scanline);
        active_list.retain(|edge| edge.ymax != scanline);

        // push edges that start on this scanline to the active list
        for edge in edge_table.iter() {
            if edge.ymin == scanline {
                active_list.push(*edge);
            }
        }

        // sort active list by ymin, ascending
        active_list.sort_unstable_by_key(|p| p.x);

        // for every pair of edges on the active list,
        // apply the winding rule of nonzero
        let mut prev_x = 0;
        let mut winding_count = 0;
        for edge in active_list.iter() {
            if winding_count != 0 {
                for x in prev_x..edge.x {
                    write_pixel(Point2 { x, y: scanline });
                }
            }
            prev_x = edge.x;
            winding_count += edge.direction;
        }

        // increment scanline
        scanline += 1;

        // adjust the x of each edge based on its gradient
        for edge in &mut active_list {
            if edge.dx != 0 {
                edge.sum += edge.dx;
            }
            while edge.sum >= edge.dy {
                edge.x -= edge.sign;
                edge.sum -= edge.dy;
            }
        }
    }

    // calculate bounding box
    let (min_xy, max_xy) = points.iter().fold(
        (
            Point2 {
                y: std::i32::MAX,
                x: std::i32::MAX,
            },
            Point2 {
                y: std::i32::MIN,
                x: std::i32::MIN,
            },
        ),
        |acc, p| {
            (
                Point2 {
                    y: min!(acc.0.y, p.y),
                    x: min!(acc.0.x, p.x),
                },
                Point2 {
                    y: max!(acc.1.y, p.y),
                    x: max!(acc.1.x, p.x),
                },
            )
        },
    );
    mxcfb_rect {
        top: min_xy.y as u32,
        left: min_xy.x as u32,
        width: (max_xy.x - min_xy.x) as u32,
        height: (max_xy.y - min_xy.y) as u32,
    }
}

/// Helper function to sample pixels on the bezier curve.
fn sample_bezier(
    startpt: Point2<f32>,
    ctrlpt: Point2<f32>,
    endpt: Point2<f32>,
    samples: i32,
) -> Vec<(f32, Point2<f32>)> {
    let mut points = Vec::new();
    for i in 0..samples {
        let t = (i as f32) / (samples - 1) as f32;
        let precisept = Point2 {
            x: (1.0 - t).powf(2.0) * startpt.x
                + 2.0 * (1.0 - t) * t * ctrlpt.x
                + t.powf(2.0) * endpt.x,
            y: (1.0 - t).powf(2.0) * startpt.y
                + 2.0 * (1.0 - t) * t * ctrlpt.y
                + t.powf(2.0) * endpt.y,
        };
        points.push((t, precisept));
    }
    points
}

pub fn draw_dynamic_bezier<F>(
    write_pixel: &mut F,
    startpt: (Point2<f32>, f32),
    ctrlpt: (Point2<f32>, f32),
    endpt: (Point2<f32>, f32),
    samples: i32,
) -> mxcfb_rect
where
    F: FnMut(Point2<i32>),
{
    let mut left_edge = Vec::<Point2<i32>>::new();
    let mut right_edge = Vec::<Point2<i32>>::new();
    let mut prev_left_pt = Point2 {
        x: std::i32::MIN,
        y: std::i32::MIN,
    };
    let mut prev_right_pt = Point2 {
        x: std::i32::MIN,
        y: std::i32::MIN,
    };
    for (t, pt) in sample_bezier(startpt.0, ctrlpt.0, endpt.0, samples) {
        // interpolate width
        let width = 2.0
            * if t < 0.5 {
                startpt.1 * (0.5 - t) + ctrlpt.1 * t
            } else {
                ctrlpt.1 * (1.0 - t) + endpt.1 * (t - 0.5)
            };

        // calculate tangent
        let velocity = 2.0 * (1.0 - t) * (ctrlpt.0 - startpt.0) + 2.0 * t * (endpt.0 - ctrlpt.0);
        let speed = velocity.magnitude();
        let tangent = if speed > 0.0 {
            velocity / speed
        } else {
            // handle case where control point == start/end point
            let extent = startpt.0 - endpt.0;
            if extent.magnitude() > 0.0 {
                extent / extent.magnitude()
            } else {
                // all points are the same, so no tangent exists
                Vector2 { x: 0.0, y: 0.0 }
            }
        };
        let left_pt = (pt
            + Vector2 {
                x: -tangent.y * width / 2.0,
                y: tangent.x * width / 2.0,
            })
        .cast()
        .unwrap();
        if left_pt != prev_left_pt {
            left_edge.push(left_pt);
            prev_left_pt = left_pt;
        }
        let right_pt = (pt
            + Vector2 {
                x: tangent.y * width / 2.0,
                y: -tangent.x * width / 2.0,
            })
        .cast()
        .unwrap();
        if right_pt != prev_right_pt {
            right_edge.push(right_pt);
            prev_right_pt = right_pt;
        }
    }
    right_edge.reverse();
    left_edge.append(&mut right_edge);
    if left_edge.len() > 2 {
        fill_polygon(write_pixel, &left_edge)
    } else {
        mxcfb_rect::invalid()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    struct Mock<'a> {
        pixel_writes: &'a mut Vec<Point2<i32>>,
    }
    impl<'a> Mock<'a> {
        fn write_pixel(&mut self, point: Point2<i32>) {
            self.pixel_writes.push(point)
        }
    }

    #[test]
    fn test_draw_1px_square_polygon() {
        let mut mock = Mock {
            pixel_writes: &mut Vec::new(),
        };
        let points = vec![
            Point2 { x: 100, y: 100 },
            Point2 { x: 100, y: 101 },
            Point2 { x: 101, y: 101 },
            Point2 { x: 101, y: 100 },
        ];
        fill_polygon(&mut |p| mock.write_pixel(p), &points);
        assert_eq!(mock.pixel_writes, &vec![Point2 { x: 100, y: 100 }]);
    }

    #[test]
    fn test_draw_2x1px_triangle_pair() {
        let mut mock = Mock {
            pixel_writes: &mut Vec::new(),
        };
        let points = vec![
            Point2 { x: 100, y: 100 },
            Point2 { x: 100, y: 101 },
            Point2 { x: 102, y: 100 },
        ];
        fill_polygon(&mut |p| mock.write_pixel(p), &points);
        let points = vec![
            Point2 { x: 100, y: 101 },
            Point2 { x: 102, y: 100 },
            Point2 { x: 102, y: 101 },
        ];
        fill_polygon(&mut |p| mock.write_pixel(p), &points);
        assert_eq!(
            mock.pixel_writes,
            &vec![Point2 { x: 100, y: 100 }, Point2 { x: 101, y: 100 }]
        );
    }
}
