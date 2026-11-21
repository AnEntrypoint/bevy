#![cfg_attr(not(test), allow(dead_code))]

use std::collections::HashMap;
use std::f64::consts::PI;

use serde_json::{json, Map, Value};

const EDGE_TABLE: [i32; 256] = [
    0x0, 0x109, 0x203, 0x30a, 0x406, 0x50f, 0x605, 0x70c, 0x80c, 0x905, 0xa0f, 0xb06, 0xc0a, 0xd03, 0xe09, 0xf00,
    0x190, 0x99, 0x393, 0x29a, 0x596, 0x49f, 0x795, 0x69c, 0x99c, 0x895, 0xb9f, 0xa96, 0xd9a, 0xc93, 0xf99, 0xe90,
    0x230, 0x339, 0x33, 0x13a, 0x636, 0x73f, 0x435, 0x53c, 0xa3c, 0xb35, 0x83f, 0x936, 0xe3a, 0xf33, 0xc39, 0xd30,
    0x3a0, 0x2a9, 0x1a3, 0xaa, 0x7a6, 0x6af, 0x5a5, 0x4ac, 0xbac, 0xaa5, 0x9af, 0x8a6, 0xfaa, 0xea3, 0xda9, 0xca0,
    0x460, 0x569, 0x663, 0x76a, 0x66, 0x16f, 0x265, 0x36c, 0xc6c, 0xd65, 0xe6f, 0xf66, 0x86a, 0x963, 0xa69, 0xb60,
    0x5f0, 0x4f9, 0x7f3, 0x6fa, 0x1f6, 0xff, 0x3f5, 0x2fc, 0xdfc, 0xcf5, 0xfff, 0xef6, 0x9fa, 0x8f3, 0xbf9, 0xaf0,
    0x650, 0x759, 0x453, 0x55a, 0x256, 0x35f, 0x55, 0x15c, 0xe5c, 0xf55, 0xc5f, 0xd56, 0xa5a, 0xb53, 0x859, 0x950,
    0x7c0, 0x6c9, 0x5c3, 0x4ca, 0x3c6, 0x2cf, 0x1c5, 0xcc, 0xfcc, 0xec5, 0xdcf, 0xcc6, 0xbca, 0xac3, 0x9c9, 0x8c0,
    0x8c0, 0x9c9, 0xac3, 0xbca, 0xcc6, 0xdcf, 0xec5, 0xfcc, 0xcc, 0x1c5, 0x2cf, 0x3c6, 0x4ca, 0x5c3, 0x6c9, 0x7c0,
    0x950, 0x859, 0xb53, 0xa5a, 0xd56, 0xc5f, 0xf55, 0xe5c, 0x15c, 0x55, 0x35f, 0x256, 0x55a, 0x453, 0x759, 0x650,
    0xaf0, 0xbf9, 0x8f3, 0x9fa, 0xef6, 0xfff, 0xcf5, 0xdfc, 0x2fc, 0x3f5, 0xff, 0x1f6, 0x6fa, 0x7f3, 0x4f9, 0x5f0,
    0xb60, 0xa69, 0x963, 0x86a, 0xf66, 0xe6f, 0xd65, 0xc6c, 0x36c, 0x265, 0x16f, 0x66, 0x76a, 0x663, 0x569, 0x460,
    0xca0, 0xda9, 0xea3, 0xfaa, 0x8a6, 0x9af, 0xaa5, 0xbac, 0x4ac, 0x5a5, 0x6af, 0x7a6, 0xaa, 0x1a3, 0x2a9, 0x3a0,
    0xd30, 0xc39, 0xf33, 0xe3a, 0x936, 0x83f, 0xb35, 0xa3c, 0x53c, 0x435, 0x73f, 0x636, 0x13a, 0x33, 0x339, 0x230,
    0xe90, 0xf99, 0xc93, 0xd9a, 0xa96, 0xb9f, 0x895, 0x99c, 0x69c, 0x795, 0x49f, 0x596, 0x29a, 0x393, 0x99, 0x190,
    0xf00, 0xe09, 0xd03, 0xc0a, 0xb06, 0xa0f, 0x905, 0x80c, 0x70c, 0x605, 0x50f, 0x406, 0x30a, 0x203, 0x109, 0x0,
];

const TRI_TABLE: [&[i8]; 256] = [
    &[-1],
    &[0, 8, 3, -1],
    &[0, 1, 9, -1],
    &[1, 8, 3, 9, 8, 1, -1],
    &[1, 2, 10, -1],
    &[0, 8, 3, 1, 2, 10, -1],
    &[9, 2, 10, 0, 2, 9, -1],
    &[2, 8, 3, 2, 10, 8, 10, 9, 8, -1],
    &[3, 11, 2, -1],
    &[0, 11, 2, 8, 11, 0, -1],
    &[1, 9, 0, 2, 3, 11, -1],
    &[1, 11, 2, 1, 9, 11, 9, 8, 11, -1],
    &[3, 10, 1, 11, 10, 3, -1],
    &[0, 10, 1, 0, 8, 10, 8, 11, 10, -1],
    &[3, 9, 0, 3, 11, 9, 11, 10, 9, -1],
    &[9, 8, 10, 10, 8, 11, -1],
    &[4, 7, 8, -1],
    &[4, 3, 0, 7, 3, 4, -1],
    &[0, 1, 9, 8, 4, 7, -1],
    &[4, 1, 9, 4, 7, 1, 7, 3, 1, -1],
    &[1, 2, 10, 8, 4, 7, -1],
    &[3, 4, 7, 3, 0, 4, 1, 2, 10, -1],
    &[9, 2, 10, 9, 0, 2, 8, 4, 7, -1],
    &[2, 10, 9, 2, 9, 7, 2, 7, 3, 7, 9, 4, -1],
    &[8, 4, 7, 3, 11, 2, -1],
    &[11, 4, 7, 11, 2, 4, 2, 0, 4, -1],
    &[9, 0, 1, 8, 4, 7, 2, 3, 11, -1],
    &[4, 7, 11, 9, 4, 11, 9, 11, 2, 9, 2, 1, -1],
    &[3, 10, 1, 3, 11, 10, 7, 8, 4, -1],
    &[1, 11, 10, 1, 4, 11, 1, 0, 4, 7, 11, 4, -1],
    &[4, 7, 8, 9, 0, 11, 9, 11, 10, 11, 0, 3, -1],
    &[4, 7, 11, 4, 11, 9, 9, 11, 10, -1],
    &[9, 5, 4, -1],
    &[9, 5, 4, 0, 8, 3, -1],
    &[0, 5, 4, 1, 5, 0, -1],
    &[8, 5, 4, 8, 3, 5, 3, 1, 5, -1],
    &[1, 2, 10, 9, 5, 4, -1],
    &[3, 0, 8, 1, 2, 10, 4, 9, 5, -1],
    &[5, 2, 10, 5, 4, 2, 4, 0, 2, -1],
    &[2, 10, 5, 3, 2, 5, 3, 5, 4, 3, 4, 8, -1],
    &[9, 5, 4, 2, 3, 11, -1],
    &[0, 11, 2, 0, 8, 11, 4, 9, 5, -1],
    &[0, 5, 4, 0, 1, 5, 2, 3, 11, -1],
    &[2, 1, 5, 2, 5, 8, 2, 8, 11, 4, 8, 5, -1],
    &[10, 3, 11, 10, 1, 3, 9, 5, 4, -1],
    &[4, 9, 5, 0, 8, 1, 8, 10, 1, 8, 11, 10, -1],
    &[5, 4, 0, 5, 0, 11, 5, 11, 10, 11, 0, 3, -1],
    &[5, 4, 8, 5, 8, 10, 10, 8, 11, -1],
    &[9, 7, 8, 5, 7, 9, -1],
    &[9, 3, 0, 9, 5, 3, 5, 7, 3, -1],
    &[0, 7, 8, 0, 1, 7, 1, 5, 7, -1],
    &[1, 5, 3, 3, 5, 7, -1],
    &[9, 7, 8, 9, 5, 7, 10, 1, 2, -1],
    &[10, 1, 2, 9, 5, 0, 5, 3, 0, 5, 7, 3, -1],
    &[8, 0, 2, 8, 2, 5, 8, 5, 7, 10, 5, 2, -1],
    &[2, 10, 5, 2, 5, 3, 3, 5, 7, -1],
    &[7, 9, 5, 7, 8, 9, 3, 11, 2, -1],
    &[9, 5, 7, 9, 7, 2, 9, 2, 0, 2, 7, 11, -1],
    &[2, 3, 11, 0, 1, 8, 1, 7, 8, 1, 5, 7, -1],
    &[11, 2, 1, 11, 1, 7, 7, 1, 5, -1],
    &[9, 5, 8, 8, 5, 7, 10, 1, 3, 10, 3, 11, -1],
    &[5, 7, 0, 5, 0, 9, 7, 11, 0, 1, 0, 10, 11, 10, 0, -1],
    &[11, 10, 0, 11, 0, 3, 10, 5, 0, 8, 0, 7, 5, 7, 0, -1],
    &[11, 10, 5, 7, 11, 5, -1],
    &[10, 6, 5, -1],
    &[0, 8, 3, 5, 10, 6, -1],
    &[9, 0, 1, 5, 10, 6, -1],
    &[1, 8, 3, 1, 9, 8, 5, 10, 6, -1],
    &[1, 6, 5, 2, 6, 1, -1],
    &[1, 6, 5, 1, 2, 6, 3, 0, 8, -1],
    &[9, 6, 5, 9, 0, 6, 0, 2, 6, -1],
    &[5, 9, 8, 5, 8, 2, 5, 2, 6, 3, 2, 8, -1],
    &[2, 3, 11, 10, 6, 5, -1],
    &[11, 0, 8, 11, 2, 0, 10, 6, 5, -1],
    &[0, 1, 9, 2, 3, 11, 5, 10, 6, -1],
    &[5, 10, 6, 1, 9, 2, 9, 11, 2, 9, 8, 11, -1],
    &[6, 3, 11, 6, 5, 3, 5, 1, 3, -1],
    &[0, 8, 11, 0, 11, 5, 0, 5, 1, 5, 11, 6, -1],
    &[3, 11, 6, 0, 3, 6, 0, 6, 5, 0, 5, 9, -1],
    &[6, 5, 9, 6, 9, 11, 11, 9, 8, -1],
    &[5, 10, 6, 4, 7, 8, -1],
    &[4, 3, 0, 4, 7, 3, 6, 5, 10, -1],
    &[1, 9, 0, 5, 10, 6, 8, 4, 7, -1],
    &[10, 6, 5, 1, 9, 7, 1, 7, 3, 7, 9, 4, -1],
    &[6, 1, 2, 6, 5, 1, 4, 7, 8, -1],
    &[1, 2, 5, 5, 2, 6, 3, 0, 4, 3, 4, 7, -1],
    &[8, 4, 7, 9, 0, 5, 0, 6, 5, 0, 2, 6, -1],
    &[7, 3, 9, 7, 9, 4, 3, 2, 9, 5, 9, 6, 2, 6, 9, -1],
    &[3, 11, 2, 7, 8, 4, 10, 6, 5, -1],
    &[5, 10, 6, 4, 7, 2, 4, 2, 0, 2, 7, 11, -1],
    &[0, 1, 9, 4, 7, 8, 2, 3, 11, 5, 10, 6, -1],
    &[9, 2, 1, 9, 11, 2, 9, 4, 11, 7, 11, 4, 5, 10, 6, -1],
    &[8, 4, 7, 3, 11, 5, 3, 5, 1, 5, 11, 6, -1],
    &[5, 1, 11, 5, 11, 6, 1, 0, 11, 7, 11, 4, 0, 4, 11, -1],
    &[0, 5, 9, 0, 6, 5, 0, 3, 6, 11, 6, 3, 8, 4, 7, -1],
    &[6, 5, 9, 6, 9, 11, 4, 7, 9, 7, 11, 9, -1],
    &[10, 4, 9, 6, 4, 10, -1],
    &[4, 10, 6, 4, 9, 10, 0, 8, 3, -1],
    &[10, 0, 1, 10, 6, 0, 6, 4, 0, -1],
    &[8, 3, 1, 8, 1, 6, 8, 6, 4, 6, 1, 10, -1],
    &[1, 4, 9, 1, 2, 4, 2, 6, 4, -1],
    &[3, 0, 8, 1, 2, 9, 2, 4, 9, 2, 6, 4, -1],
    &[0, 2, 4, 4, 2, 6, -1],
    &[8, 3, 2, 8, 2, 4, 4, 2, 6, -1],
    &[10, 4, 9, 10, 6, 4, 11, 2, 3, -1],
    &[0, 8, 2, 2, 8, 11, 4, 9, 10, 4, 10, 6, -1],
    &[3, 11, 2, 0, 1, 6, 0, 6, 4, 6, 1, 10, -1],
    &[6, 4, 1, 6, 1, 10, 4, 8, 1, 2, 1, 11, 8, 11, 1, -1],
    &[9, 6, 4, 9, 3, 6, 9, 1, 3, 11, 6, 3, -1],
    &[8, 11, 1, 8, 1, 0, 11, 6, 1, 9, 1, 4, 6, 4, 1, -1],
    &[3, 11, 6, 3, 6, 0, 0, 6, 4, -1],
    &[6, 4, 8, 11, 6, 8, -1],
    &[7, 10, 6, 7, 8, 10, 8, 9, 10, -1],
    &[0, 7, 3, 0, 10, 7, 0, 9, 10, 6, 7, 10, -1],
    &[10, 6, 7, 1, 10, 7, 1, 7, 8, 1, 8, 0, -1],
    &[10, 6, 7, 10, 7, 1, 1, 7, 3, -1],
    &[1, 2, 6, 1, 6, 8, 1, 8, 9, 8, 6, 7, -1],
    &[2, 6, 9, 2, 9, 1, 6, 7, 9, 0, 9, 3, 7, 3, 9, -1],
    &[7, 8, 0, 7, 0, 6, 6, 0, 2, -1],
    &[7, 3, 2, 6, 7, 2, -1],
    &[2, 3, 11, 10, 6, 8, 10, 8, 9, 8, 6, 7, -1],
    &[2, 0, 7, 2, 7, 11, 0, 9, 7, 6, 7, 10, 9, 10, 7, -1],
    &[1, 8, 0, 1, 7, 8, 1, 10, 7, 6, 7, 10, 2, 3, 11, -1],
    &[11, 2, 1, 11, 1, 7, 10, 6, 1, 6, 7, 1, -1],
    &[8, 9, 6, 8, 6, 7, 9, 1, 6, 11, 6, 3, 1, 3, 6, -1],
    &[0, 9, 1, 11, 6, 7, -1],
    &[7, 8, 0, 7, 0, 6, 3, 11, 0, 11, 6, 0, -1],
    &[7, 11, 6, -1],
    &[7, 6, 11, -1],
    &[3, 0, 8, 11, 7, 6, -1],
    &[0, 1, 9, 11, 7, 6, -1],
    &[8, 1, 9, 8, 3, 1, 11, 7, 6, -1],
    &[10, 1, 2, 6, 11, 7, -1],
    &[1, 2, 10, 3, 0, 8, 6, 11, 7, -1],
    &[2, 9, 0, 2, 10, 9, 6, 11, 7, -1],
    &[6, 11, 7, 2, 10, 3, 10, 8, 3, 10, 9, 8, -1],
    &[7, 2, 3, 6, 2, 7, -1],
    &[7, 0, 8, 7, 6, 0, 6, 2, 0, -1],
    &[2, 7, 6, 2, 3, 7, 0, 1, 9, -1],
    &[1, 6, 2, 1, 8, 6, 1, 9, 8, 8, 7, 6, -1],
    &[10, 7, 6, 10, 1, 7, 1, 3, 7, -1],
    &[10, 7, 6, 1, 7, 10, 1, 8, 7, 1, 0, 8, -1],
    &[0, 3, 7, 0, 7, 10, 0, 10, 9, 6, 10, 7, -1],
    &[7, 6, 10, 7, 10, 8, 8, 10, 9, -1],
    &[6, 8, 4, 11, 8, 6, -1],
    &[3, 6, 11, 3, 0, 6, 0, 4, 6, -1],
    &[8, 6, 11, 8, 4, 6, 9, 0, 1, -1],
    &[9, 4, 6, 9, 6, 3, 9, 3, 1, 11, 3, 6, -1],
    &[6, 8, 4, 6, 11, 8, 2, 10, 1, -1],
    &[1, 2, 10, 3, 0, 11, 0, 6, 11, 0, 4, 6, -1],
    &[4, 11, 8, 4, 6, 11, 0, 2, 9, 2, 10, 9, -1],
    &[10, 9, 3, 10, 3, 2, 9, 4, 3, 11, 3, 6, 4, 6, 3, -1],
    &[8, 2, 3, 8, 4, 2, 4, 6, 2, -1],
    &[0, 4, 2, 4, 6, 2, -1],
    &[1, 9, 0, 2, 3, 4, 2, 4, 6, 4, 3, 8, -1],
    &[1, 9, 4, 1, 4, 2, 2, 4, 6, -1],
    &[8, 1, 3, 8, 6, 1, 8, 4, 6, 6, 10, 1, -1],
    &[10, 1, 0, 10, 0, 6, 6, 0, 4, -1],
    &[4, 6, 3, 4, 3, 8, 6, 10, 3, 0, 3, 9, 10, 9, 3, -1],
    &[10, 9, 4, 6, 10, 4, -1],
    &[4, 9, 5, 7, 6, 11, -1],
    &[0, 8, 3, 4, 9, 5, 11, 7, 6, -1],
    &[5, 0, 1, 5, 4, 0, 7, 6, 11, -1],
    &[11, 7, 6, 8, 3, 4, 3, 5, 4, 3, 1, 5, -1],
    &[9, 5, 4, 10, 1, 2, 7, 6, 11, -1],
    &[6, 11, 7, 1, 2, 10, 0, 8, 3, 4, 9, 5, -1],
    &[7, 6, 11, 5, 4, 10, 4, 2, 10, 4, 0, 2, -1],
    &[3, 4, 8, 3, 5, 4, 3, 2, 5, 10, 5, 2, 11, 7, 6, -1],
    &[7, 2, 3, 7, 6, 2, 5, 4, 9, -1],
    &[9, 5, 4, 0, 8, 6, 0, 6, 2, 6, 8, 7, -1],
    &[3, 6, 2, 3, 7, 6, 1, 5, 0, 5, 4, 0, -1],
    &[6, 2, 8, 6, 8, 7, 2, 1, 8, 4, 8, 5, 1, 5, 8, -1],
    &[9, 5, 4, 10, 1, 6, 1, 7, 6, 1, 3, 7, -1],
    &[1, 6, 10, 1, 7, 6, 1, 0, 7, 8, 7, 0, 9, 5, 4, -1],
    &[4, 0, 10, 4, 10, 5, 0, 3, 10, 6, 10, 7, 3, 7, 10, -1],
    &[7, 6, 10, 7, 10, 8, 5, 4, 10, 4, 8, 10, -1],
    &[6, 9, 5, 6, 11, 9, 11, 8, 9, -1],
    &[3, 6, 11, 0, 6, 3, 0, 5, 6, 0, 9, 5, -1],
    &[0, 11, 8, 0, 5, 11, 0, 1, 5, 5, 6, 11, -1],
    &[6, 11, 3, 6, 3, 5, 5, 3, 1, -1],
    &[1, 2, 10, 9, 5, 11, 9, 11, 8, 11, 5, 6, -1],
    &[0, 11, 3, 0, 6, 11, 0, 9, 6, 5, 6, 9, 1, 2, 10, -1],
    &[11, 8, 5, 11, 5, 6, 8, 0, 5, 10, 5, 2, 0, 2, 5, -1],
    &[6, 11, 3, 6, 3, 5, 2, 10, 3, 10, 5, 3, -1],
    &[5, 8, 9, 5, 2, 8, 5, 6, 2, 3, 8, 2, -1],
    &[9, 5, 6, 9, 6, 0, 0, 6, 2, -1],
    &[1, 5, 8, 1, 8, 0, 5, 6, 8, 3, 8, 2, 6, 2, 8, -1],
    &[1, 5, 6, 2, 1, 6, -1],
    &[1, 3, 6, 1, 6, 10, 3, 8, 6, 5, 6, 9, 8, 9, 6, -1],
    &[10, 1, 0, 10, 0, 6, 9, 5, 0, 5, 6, 0, -1],
    &[0, 3, 8, 5, 6, 10, -1],
    &[10, 5, 6, -1],
    &[11, 5, 10, 7, 5, 11, -1],
    &[11, 5, 10, 11, 7, 5, 8, 3, 0, -1],
    &[5, 11, 7, 5, 10, 11, 1, 9, 0, -1],
    &[10, 7, 5, 10, 11, 7, 9, 8, 1, 8, 3, 1, -1],
    &[11, 1, 2, 11, 7, 1, 7, 5, 1, -1],
    &[0, 8, 3, 1, 2, 7, 1, 7, 5, 7, 2, 11, -1],
    &[9, 7, 5, 9, 2, 7, 9, 0, 2, 2, 11, 7, -1],
    &[7, 5, 2, 7, 2, 11, 5, 9, 2, 3, 2, 8, 9, 8, 2, -1],
    &[2, 5, 10, 2, 3, 5, 3, 7, 5, -1],
    &[8, 2, 0, 8, 5, 2, 8, 7, 5, 10, 2, 5, -1],
    &[9, 0, 1, 5, 10, 3, 5, 3, 7, 3, 10, 2, -1],
    &[9, 8, 2, 9, 2, 1, 8, 7, 2, 10, 2, 5, 7, 5, 2, -1],
    &[1, 3, 5, 3, 7, 5, -1],
    &[0, 8, 7, 0, 7, 1, 1, 7, 5, -1],
    &[9, 0, 3, 9, 3, 5, 5, 3, 7, -1],
    &[9, 8, 7, 5, 9, 7, -1],
    &[5, 8, 4, 5, 10, 8, 10, 11, 8, -1],
    &[5, 0, 4, 5, 11, 0, 5, 10, 11, 11, 3, 0, -1],
    &[0, 1, 9, 8, 4, 10, 8, 10, 11, 10, 4, 5, -1],
    &[10, 11, 4, 10, 4, 5, 11, 3, 4, 9, 4, 1, 3, 1, 4, -1],
    &[2, 5, 1, 2, 8, 5, 2, 11, 8, 4, 5, 8, -1],
    &[0, 4, 11, 0, 11, 3, 4, 5, 11, 2, 11, 1, 5, 1, 11, -1],
    &[0, 2, 5, 0, 5, 9, 2, 11, 5, 4, 5, 8, 11, 8, 5, -1],
    &[9, 4, 5, 2, 11, 3, -1],
    &[2, 5, 10, 3, 5, 2, 3, 4, 5, 3, 8, 4, -1],
    &[5, 10, 2, 5, 2, 4, 4, 2, 0, -1],
    &[3, 10, 2, 3, 5, 10, 3, 8, 5, 4, 5, 8, 0, 1, 9, -1],
    &[5, 10, 2, 5, 2, 4, 1, 9, 2, 9, 4, 2, -1],
    &[8, 4, 5, 8, 5, 3, 3, 5, 1, -1],
    &[0, 4, 5, 1, 0, 5, -1],
    &[8, 4, 5, 8, 5, 3, 9, 0, 5, 0, 3, 5, -1],
    &[9, 4, 5, -1],
    &[4, 11, 7, 4, 9, 11, 9, 10, 11, -1],
    &[0, 8, 3, 4, 9, 7, 9, 11, 7, 9, 10, 11, -1],
    &[1, 10, 11, 1, 11, 4, 1, 4, 0, 7, 4, 11, -1],
    &[3, 1, 4, 3, 4, 8, 1, 10, 4, 7, 4, 11, 10, 11, 4, -1],
    &[4, 11, 7, 9, 11, 4, 9, 2, 11, 9, 1, 2, -1],
    &[9, 7, 4, 9, 11, 7, 9, 1, 11, 2, 11, 1, 0, 8, 3, -1],
    &[11, 7, 4, 11, 4, 2, 2, 4, 0, -1],
    &[11, 7, 4, 11, 4, 2, 8, 3, 4, 3, 2, 4, -1],
    &[2, 9, 10, 2, 7, 9, 2, 3, 7, 7, 4, 9, -1],
    &[9, 10, 7, 9, 7, 4, 10, 2, 7, 8, 7, 0, 2, 0, 7, -1],
    &[3, 7, 10, 3, 10, 2, 7, 4, 10, 1, 10, 0, 4, 0, 10, -1],
    &[1, 10, 2, 8, 7, 4, -1],
    &[4, 9, 1, 4, 1, 7, 7, 1, 3, -1],
    &[4, 9, 1, 4, 1, 7, 0, 8, 1, 8, 7, 1, -1],
    &[4, 0, 3, 7, 4, 3, -1],
    &[4, 8, 7, -1],
    &[9, 10, 8, 10, 11, 8, -1],
    &[3, 0, 9, 3, 9, 11, 11, 9, 10, -1],
    &[0, 1, 10, 0, 10, 8, 8, 10, 11, -1],
    &[3, 1, 10, 11, 3, 10, -1],
    &[1, 2, 11, 1, 11, 9, 9, 11, 8, -1],
    &[3, 0, 9, 3, 9, 11, 1, 2, 9, 2, 11, 9, -1],
    &[0, 2, 11, 8, 0, 11, -1],
    &[3, 2, 11, -1],
    &[2, 3, 8, 2, 8, 10, 10, 8, 9, -1],
    &[9, 10, 2, 0, 9, 2, -1],
    &[2, 3, 8, 2, 8, 10, 0, 1, 8, 1, 10, 8, -1],
    &[1, 10, 2, -1],
    &[1, 3, 8, 9, 1, 8, -1],
    &[0, 9, 1, -1],
    &[0, 3, 8, -1],
    &[-1],
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CaveAxis {
    X,
    Y,
    Z,
}

impl CaveAxis {
    pub fn parse(name: &str) -> Self {
        match name {
            "x" => Self::X,
            "z" => Self::Z,
            _ => Self::Y,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::X => "x",
            Self::Y => "y",
            Self::Z => "z",
        }
    }
}

fn tunnel_hash(seed: i32, index: i32) -> f64 {
    let mut h = seed as u32;
    h = (h ^ index as u32).wrapping_mul(0x27d4eb2d);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2c1b3c6d);
    h ^= h >> 12;
    h = h.wrapping_mul(0x297a2d39);
    h ^= h >> 15;
    h as f64 / 4294967296.0
}

pub fn make_sphere_cave_sdf(cx: f64, cy: f64, cz: f64, radius: f64) -> impl Fn(f64, f64, f64) -> f64 {
    move |x, y, z| {
        let (dx, dy, dz) = (x - cx, y - cy, z - cz);
        (dx * dx + dy * dy + dz * dz).sqrt() - radius
    }
}

pub fn make_cylinder_cave_sdf(cx: f64, cy: f64, cz: f64, radius: f64, half_height: f64, axis: CaveAxis) -> impl Fn(f64, f64, f64) -> f64 {
    move |x, y, z| {
        let (dx, dy, dz) = (x - cx, y - cy, z - cz);
        let (radial, along) = match axis {
            CaveAxis::X => (dy.hypot(dz), dx),
            CaveAxis::Y => (dx.hypot(dz), dy),
            CaveAxis::Z => (dx.hypot(dy), dz),
        };
        let d_radial = radial - radius;
        let d_along = along.abs() - half_height;
        let outside = d_radial.max(0.0).hypot(d_along.max(0.0));
        let inside = d_radial.max(d_along).min(0.0);
        outside + inside
    }
}

pub fn make_tunnel_cave_sdf(seed: i32, radius: f64, length: f64, axis: CaveAxis, warp_amp: f64, warp_freq: f64) -> impl Fn(f64, f64, f64) -> f64 {
    move |x, y, z| {
        let (along, u, v) = match axis {
            CaveAxis::X => (x, y, z),
            CaveAxis::Y => (y, x, z),
            CaveAxis::Z => (z, x, y),
        };
        let t = (along / (length * 0.5)).clamp(-1.0, 1.0);
        let sample = ((t + 1.0) * 0.5 * 1000.0).round() as i32;
        let warp_u = (tunnel_hash(seed, sample * 2) * 2.0 - 1.0) * warp_amp * (t * PI * warp_freq + tunnel_hash(seed, 1) * 6.28318).sin();
        let warp_v = (tunnel_hash(seed, sample * 2 + 1) * 2.0 - 1.0) * warp_amp * (t * PI * warp_freq + tunnel_hash(seed, 2) * 6.28318).cos();
        let d_radial = (u - warp_u).hypot(v - warp_v) - radius;
        let d_along = along.abs() - length * 0.5;
        let outside = d_radial.max(0.0).hypot(d_along.max(0.0));
        let inside = d_radial.max(d_along).min(0.0);
        outside + inside
    }
}

pub struct CaveMesh {
    pub positions: Vec<f32>,
    pub indices: Vec<u32>,
}

impl CaveMesh {
    pub fn vertex_count(&self) -> usize {
        self.positions.len() / 3
    }
}

const EDGE_ENDPOINTS: [(usize, usize); 12] = [(0, 1), (1, 2), (2, 3), (3, 0), (4, 5), (5, 6), (6, 7), (7, 4), (0, 4), (1, 5), (2, 6), (3, 7)];

fn lerp_edge(p1: [f64; 3], p2: [f64; 3], v1: f32, v2: f32) -> [f64; 3] {
    let (v1, v2) = (v1 as f64, v2 as f64);
    let t = if (v1 - v2).abs() < 1e-10 { 0.0 } else { -v1 / (v2 - v1) };
    [0, 1, 2].map(|k| p1[k] + t * (p2[k] - p1[k]))
}

fn vertex_id(p: [f64; 3], positions: &mut Vec<f32>, by_key: &mut HashMap<i64, u32>) -> u32 {
    let quant = |c: f64| (c * 100.0) as i64 + 512;
    let key = (quant(p[0]) << 20) | (quant(p[1]) << 10) | quant(p[2]);
    if let Some(id) = by_key.get(&key) {
        return *id;
    }
    let id = (positions.len() / 3) as u32;
    positions.extend(p.map(|c| c as f32));
    by_key.insert(key, id);
    id
}

pub fn polygonize_cave_sdf(sdf: impl Fn(f64, f64, f64) -> f64, res: usize) -> CaveMesh {
    const LO: f64 = -1.5;
    const HI: f64 = 1.5;
    let step = (HI - LO) / (res as f64 - 1.0);
    let at = |i: usize| LO + i as f64 * step;
    let mut field = vec![0.0_f32; res * res * res];
    for k in 0..res {
        for j in 0..res {
            for i in 0..res {
                field[k * res * res + j * res + i] = sdf(at(i), at(j), at(k)) as f32;
            }
        }
    }
    let mut positions: Vec<f32> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();
    let mut by_key: HashMap<i64, u32> = HashMap::new();
    for k in 0..res - 1 {
        for j in 0..res - 1 {
            for i in 0..res - 1 {
                let (x, y, z) = (at(i), at(j), at(k));
                let (x1, y1, z1) = (x + step, y + step, z + step);
                let corners = [[x, y, z], [x1, y, z], [x1, y1, z], [x, y1, z], [x, y, z1], [x1, y, z1], [x1, y1, z1], [x, y1, z1]];
                let base = k * res * res + j * res + i;
                let vals = [
                    field[base],
                    field[base + 1],
                    field[base + 1 + res],
                    field[base + res],
                    field[base + res * res],
                    field[base + 1 + res * res],
                    field[base + 1 + res + res * res],
                    field[base + res + res * res],
                ];
                let mut case = 0_usize;
                for (n, v) in vals.iter().enumerate() {
                    if *v < 0.0 {
                        case |= 1 << n;
                    }
                }
                let edges = EDGE_TABLE[case];
                if edges == 0 {
                    continue;
                }
                let mut verts = [[0.0_f64; 3]; 12];
                for (e, (a, b)) in EDGE_ENDPOINTS.iter().enumerate() {
                    if edges & (1_i32 << e) != 0 {
                        verts[e] = lerp_edge(corners[*a], corners[*b], vals[*a], vals[*b]);
                    }
                }
                let tri = TRI_TABLE[case];
                let mut t = 0;
                while tri[t] != -1 {
                    let ids = [tri[t], tri[t + 1], tri[t + 2]].map(|e| vertex_id(verts[e as usize], &mut positions, &mut by_key));
                    let (ia, ib, ic) = (ids[0], ids[1], ids[2]);
                    if ia != ib && ib != ic && ic != ia {
                        indices.extend([ia, ic, ib]);
                    }
                    t += 3;
                }
            }
        }
    }
    CaveMesh { positions, indices }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CaveShape {
    Sphere,
    Cylinder { half_height: f64, axis: CaveAxis },
    Tunnel { length: f64, axis: CaveAxis, seed: f64, warp_amp: f64, warp_freq: f64 },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CaveVolume {
    pub world_x: f64,
    pub world_y: f64,
    pub world_z: f64,
    pub world_radius: f64,
    pub shape: CaveShape,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CaveCarveLayer {
    volumes: Vec<CaveVolume>,
}

fn finite(spec: &Value, key: &str) -> Option<f64> {
    spec.get(key).and_then(Value::as_f64).filter(|v| v.is_finite())
}

fn finite_or(spec: &Value, key: &str, fallback: f64) -> f64 {
    finite(spec, key).unwrap_or(fallback)
}

fn parse_anchor(spec: &Value) -> Option<(f64, f64, f64, f64)> {
    let world_radius = finite(spec, "worldRadius").filter(|r| *r > 0.0)?;
    Some((finite(spec, "worldX")?, finite(spec, "worldY")?, finite(spec, "worldZ")?, world_radius))
}

fn parse_shape(spec: &Value, world_radius: f64) -> Option<CaveShape> {
    let axis = CaveAxis::parse(spec.get("axis").and_then(Value::as_str).unwrap_or("y"));
    match spec.get("kind").and_then(Value::as_str) {
        Some("cylinder") => {
            let half_height = finite_or(spec, "halfHeight", 1.0);
            (half_height > 0.0).then_some(CaveShape::Cylinder { half_height, axis })
        }
        Some("tunnel") => {
            let length = finite_or(spec, "length", world_radius * 2.0) / world_radius;
            (length > 0.0).then_some(CaveShape::Tunnel {
                length,
                axis,
                seed: finite_or(spec, "seed", 0.0),
                warp_amp: finite_or(spec, "warpAmp", 0.15),
                warp_freq: finite_or(spec, "warpFreq", 2.5),
            })
        }
        _ => Some(CaveShape::Sphere),
    }
}

fn vertical_span(vol: &CaveVolume, horizontal: f64) -> f64 {
    match vol.shape {
        CaveShape::Sphere => (vol.world_radius * vol.world_radius - horizontal * horizontal).max(0.0).sqrt(),
        CaveShape::Cylinder { half_height, .. } => {
            if horizontal > vol.world_radius {
                0.0
            } else {
                vol.world_radius * half_height
            }
        }
        CaveShape::Tunnel { .. } => {
            if horizontal > vol.world_radius {
                0.0
            } else {
                vol.world_radius
            }
        }
    }
}

impl CaveCarveLayer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_cave_volume(&mut self, spec: &Value) -> bool {
        let Some((world_x, world_y, world_z, world_radius)) = parse_anchor(spec) else {
            return false;
        };
        let Some(shape) = parse_shape(spec, world_radius) else {
            return false;
        };
        self.volumes.push(CaveVolume { world_x, world_y, world_z, world_radius, shape });
        true
    }

    pub fn volume_count(&self) -> usize {
        self.volumes.len()
    }

    pub fn height_delta_at(&self, x: f64, z: f64, surface_y: f64) -> f64 {
        self.volumes.iter().fold(0.0, |delta, vol| {
            let horizontal = (x - vol.world_x).hypot(z - vol.world_z);
            if horizontal > vol.world_radius {
                return delta;
            }
            let span = vertical_span(vol, horizontal);
            if span <= 0.0 {
                return delta;
            }
            let cave_top = vol.world_y + span;
            if cave_top < surface_y {
                return delta;
            }
            delta - ((cave_top - surface_y) + span)
        })
    }

    pub fn to_json(&self) -> Value {
        let volumes: Vec<Value> = self.volumes.iter().map(volume_json).collect();
        json!({ "version": 2, "volumes": volumes })
    }
}

fn volume_json(vol: &CaveVolume) -> Value {
    let mut out = Map::new();
    out.insert("worldX".into(), json!(vol.world_x));
    out.insert("worldY".into(), json!(vol.world_y));
    out.insert("worldZ".into(), json!(vol.world_z));
    out.insert("worldRadius".into(), json!(vol.world_radius));
    match vol.shape {
        CaveShape::Sphere => {
            out.insert("kind".into(), json!("sphere"));
        }
        CaveShape::Cylinder { half_height, axis } => {
            out.insert("kind".into(), json!("cylinder"));
            out.insert("halfHeight".into(), json!(half_height));
            out.insert("axis".into(), json!(axis.name()));
        }
        CaveShape::Tunnel { length, axis, seed, warp_amp, warp_freq } => {
            out.insert("kind".into(), json!("tunnel"));
            out.insert("axis".into(), json!(axis.name()));
            out.insert("length".into(), json!(length * vol.world_radius));
            out.insert("seed".into(), json!(seed));
            out.insert("warpAmp".into(), json!(warp_amp));
            out.insert("warpFreq".into(), json!(warp_freq));
        }
    }
    Value::Object(out)
}

pub fn load_cave_carve_layer(json: &Value) -> CaveCarveLayer {
    let mut layer = CaveCarveLayer::new();
    if let Some(volumes) = json.get("volumes").and_then(Value::as_array) {
        for spec in volumes {
            layer.add_cave_volume(spec);
        }
    }
    layer
}

#[cfg(test)]
mod tests {
    use super::*;

    const TOLERANCE: f64 = 1e-9;
    const MESH_TOLERANCE: f64 = 1e-6;

    fn fixture() -> Value {
        serde_json::from_str(include_str!("../testdata/parity/spoint-cave-sdf.json")).unwrap()
    }

    fn num(v: &Value, key: &str) -> f64 {
        v[key].as_f64().unwrap()
    }

    fn assert_close(got: f64, want: &Value, what: &str) {
        let want = want.as_f64().unwrap();
        assert!((got - want).abs() <= TOLERANCE, "{what}: {got} against spoint {want}");
    }

    fn assert_values(points: &Value, sdf: impl Fn(f64, f64, f64) -> f64, want: &Value, what: &str) {
        let points = points.as_array().unwrap();
        let want = want.as_array().unwrap();
        assert_eq!(points.len(), want.len(), "{what}: probe count");
        for (p, w) in points.iter().zip(want) {
            assert_close(sdf(p[0].as_f64().unwrap(), p[1].as_f64().unwrap(), p[2].as_f64().unwrap()), w, what);
        }
    }

    fn assert_json_close(got: &Value, want: &Value, what: &str) {
        match (got, want) {
            (Value::Object(g), Value::Object(w)) => {
                assert_eq!(g.len(), w.len(), "{what}: key count");
                for (key, want_value) in w {
                    let got_value = g.get(key).unwrap_or_else(|| panic!("{what}: missing {key}"));
                    assert_json_close(got_value, want_value, &format!("{what}.{key}"));
                }
            }
            (Value::Array(g), Value::Array(w)) => {
                assert_eq!(g.len(), w.len(), "{what}: length");
                for (g, w) in g.iter().zip(w) {
                    assert_json_close(g, w, what);
                }
            }
            (Value::Number(_), Value::Number(_)) => {
                let (g, w) = (got.as_f64().unwrap(), want.as_f64().unwrap());
                assert!((g - w).abs() <= TOLERANCE, "{what}: {g} against spoint {w}");
            }
            _ => assert_eq!(got, want, "{what}"),
        }
    }

    #[test]
    fn sdf_closures_match_spoint_cave_sdf() {
        let fixture = fixture();
        let probes = &fixture["sdfProbes"];
        let points = &probes["points"];
        let sphere = &probes["sphere"];
        assert_values(points, make_sphere_cave_sdf(num(sphere, "cx"), num(sphere, "cy"), num(sphere, "cz"), num(sphere, "radius")), &sphere["values"], "sphere");
        for c in probes["cylinders"].as_array().unwrap() {
            let axis = CaveAxis::parse(c["axis"].as_str().unwrap());
            let sdf = make_cylinder_cave_sdf(num(c, "cx"), num(c, "cy"), num(c, "cz"), num(c, "radius"), num(c, "halfHeight"), axis);
            assert_values(points, sdf, &c["values"], "cylinder");
        }
        for t in probes["tunnels"].as_array().unwrap() {
            let axis = CaveAxis::parse(t["axis"].as_str().unwrap());
            let sdf = make_tunnel_cave_sdf(num(t, "seed") as i32, num(t, "radius"), num(t, "length"), axis, num(t, "warpAmp"), num(t, "warpFreq"));
            assert_values(points, sdf, &t["values"], "tunnel");
        }
    }

    #[test]
    fn polygonizer_matches_spoint_polygonize_cave_sdf() {
        let fixture = fixture();
        for mesh in fixture["meshes"].as_array().unwrap() {
            let name = mesh["name"].as_str().unwrap();
            let res = mesh["res"].as_u64().unwrap() as usize;
            let got = match name {
                "sphere" => polygonize_cave_sdf(make_sphere_cave_sdf(0.0, 0.0, 0.0, 1.0), res),
                "cylinder-y" => polygonize_cave_sdf(make_cylinder_cave_sdf(0.0, 0.0, 0.0, 1.0, 0.6, CaveAxis::Y), res),
                "tunnel-y" => polygonize_cave_sdf(make_tunnel_cave_sdf(7, 1.0, 2.5, CaveAxis::Y, 0.15, 2.5), res),
                other => panic!("unknown mesh {other}"),
            };
            assert_eq!(got.vertex_count() as u64, mesh["vc"].as_u64().unwrap(), "{name}: vertex count");
            assert_eq!(got.indices.len() as u64, mesh["ic"].as_u64().unwrap(), "{name}: index count");
            let checksum: i64 = got.positions.iter().map(|p| (*p as f64 * 1000.0 + 0.5).floor() as i64).sum();
            assert_eq!(checksum, mesh["checksum"].as_i64().unwrap(), "{name}: checksum");
            let want_positions = mesh["positions"].as_array().unwrap();
            assert_eq!(got.positions.len(), want_positions.len(), "{name}: position count");
            for (g, w) in got.positions.iter().zip(want_positions) {
                assert!((*g as f64 - w.as_f64().unwrap()).abs() <= MESH_TOLERANCE, "{name}: position {g} against spoint {w}");
            }
            let got_indices: Vec<u64> = got.indices.iter().map(|i| *i as u64).collect();
            let want_indices: Vec<u64> = mesh["indices"].as_array().unwrap().iter().map(|i| i.as_u64().unwrap()).collect();
            assert_eq!(got_indices, want_indices, "{name}: indices");
        }
    }

    #[test]
    fn carve_layer_round_trip_matches_spoint_load_cave_carve_layer() {
        let fixture = fixture();
        let carve = &fixture["carve"];
        let mut layer = CaveCarveLayer::new();
        for (spec, want) in carve["specs"].as_array().unwrap().iter().zip(carve["addResults"].as_array().unwrap()) {
            assert_eq!(layer.add_cave_volume(spec), want["added"].as_bool().unwrap(), "add {spec}");
        }
        assert_eq!(layer.volume_count() as u64, carve["volumeCount"].as_u64().unwrap());
        assert_json_close(&layer.to_json(), &carve["layerJson"], "layer");
        let loaded = load_cave_carve_layer(&carve["layerJson"]);
        assert_eq!(loaded.volume_count() as u64, carve["loadedVolumeCount"].as_u64().unwrap());
        assert_json_close(&loaded.to_json(), &carve["loadedJson"], "loaded");
        for (probe, want) in carve["heightProbes"].as_array().unwrap().iter().zip(carve["heightDelta"].as_array().unwrap()) {
            let (x, z, y) = (probe[0].as_f64().unwrap(), probe[1].as_f64().unwrap(), probe[2].as_f64().unwrap());
            assert_close(layer.height_delta_at(x, z, y), want, "height delta");
            assert_close(loaded.height_delta_at(x, z, y), want, "loaded height delta");
        }
    }
}
