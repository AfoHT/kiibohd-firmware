// Copyright 2021-2022 Jacob Alexander
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use const_env::from_env;

// ----- Constants -----

pub const CSIZE: usize = 19; // Number of columns
pub const RSIZE: usize = 6; // Number of rows
pub const MSIZE: usize = RSIZE * CSIZE; // Total matrix size

// Remap lookup
// 0 mapped keys are ignored
pub const SWITCH_REMAP: &[u8] = &[
    1,  // C1;R1:0
    20, // C1;R2:1
    38, // C1;R3:2
    56, // C1;R4:3
    72, // C1;R5:4
    89, // C1;R6:5
    2,  // C2;R1:6
    21, // C2;R2:7
    39, // C2;R3:8
    57, // C2;R4:9
    73, // C2;R5:10
    90, // C2;R6:11
    3,  // C3;R1:12
    22, // C3;R2:13
    40, // C3;R3:14
    58, // C3;R4:15
    74, // C3;R5:16
    91, // C3;R6:17
    4,  // C4;R1:18
    23, // C4;R2:19
    41, // C4;R3:20
    59, // C4;R4:21
    75, // C4;R5:22
    0,  // C4;R6:23
    5,  // C5;R1:24
    24, // C5;R2:25
    42, // C5;R3:26
    60, // C5;R4:27
    76, // C5;R5:28
    0,  // C5;R6:29
    6,  // C6;R1:30
    25, // C6;R2:31
    43, // C6;R3:32
    61, // C6;R4:33
    77, // C6;R5:34
    92, // C6;R6:35
    7,  // C7;R1:36
    26, // C7;R2:37
    44, // C7;R3:38
    62, // C7;R4:39
    78, // C7;R5:40
    0,  // C7;R6:41
    8,  // C8;R1:42
    27, // C8;R2:43
    45, // C8;R3:44
    63, // C8;R4:45
    79, // C8;R5:46
    0,  // C8;R6:47
    9,  // C9;R1:48
    28, // C9;R2:49
    46, // C9;R3:50
    64, // C9;R4:51
    80, // C9;R5:52
    0,  // C9;R6:53
    10, // C10;R1:54
    29, // C10;R2:55
    47, // C10;R3:56
    65, // C10;R4:57
    81, // C10;R5:58
    93, // C10;R6:59
    11, // C11;R1:60
    30, // C11;R2:61
    48, // C11;R3:62
    66, // C11;R4:63
    82, // C11;R5:64
    94, // C11;R6:65
    12, // C12;R1:66
    31, // C12;R2:67
    49, // C12;R3:68
    67, // C12;R4:69
    0,  // C12;R5:70
    0,  // C12;R6:71
    13, // C13;R1:72
    32, // C13;R2:73
    50, // C13;R3:74
    0,  // C13;R4:75
    0,  // C13;R5:76
    0,  // C13;R6:77
    14, // C14;R1:78
    33, // C14;R2:79
    51, // C14;R3:80
    68, // C14;R4:81
    83, // C14;R5:82
    95, // C14;R6:83
    15, // C15;R1:84
    0,  // C15;R2:85
    0,  // C15;R3:86
    0,  // C15;R4:87
    84, // C15;R5:88
    96, // C15;R6:89
    16, // C16;R1:90
    34, // C16;R2:91
    52, // C16;R3:92
    69, // C16;R4:93
    85, // C16;R5:94
    97, // C16;R6:95
    17, // C17;R1:96
    35, // C17;R2:97
    53, // C17;R3:98
    70, // C17;R4:99
    86, // C17;R5:100
    98, // C17;R6:101
    18, // C18;R1:102
    36, // C18;R2:103
    54, // C18;R3:104
    71, // C18;R4:105
    87, // C18;R5:106
    99, // C18;R6:107
    19, // C19;R1:108
    37, // C19;R2:109
    55, // C19;R3:110
    0,  // C19;R4:111
    88, // C19;R5:112
    0,  // C19;R6:113
];

pub const SCAN_PERIOD_US: u32 = 1000 / CSIZE as u32; // Scan all strobes within 1 ms (1000 Hz) for USB

#[from_env]
pub const VERGEN_GIT_SEMVER: &str = "N/A";
#[from_env]
pub const VERGEN_GIT_COMMIT_COUNT: &str = "0";
