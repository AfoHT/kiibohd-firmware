// Copyright 2021-2022 Jacob Alexander
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use const_env::from_env;

// ----- Constants -----

pub const CSIZE: usize = 22; // Number of columns
pub const RSIZE: usize = 6; // Number of rows
pub const MSIZE: usize = RSIZE * CSIZE; // Total matrix size
                                        // Size of ADC buffer per strobe (plus 1 for the previous strobe's last sample)
pub const ADC_BUF_SIZE: usize = kiibohd_atsam4s::constants::ADC_SAMPLES * RSIZE + 1;

// Remap lookup
// 0 mapped keys are ignored
pub const SWITCH_REMAP: &[u8] = &[
    1,   // C1;R1:0
    21,  // C1;R2:1
    43,  // C1;R3:2
    64,  // C1;R4:3
    81,  // C1;R5:4
    100, // C1;R6:5
    0,   // C2;R1:6
    22,  // C2;R2:7
    44,  // C2;R3:8
    0,   // C2;R4:9
    82,  // C2;R5:10
    101, // C2;R6:11
    2,   // C3;R1:12
    23,  // C3;R2:13
    45,  // C3;R3:14
    65,  // C3;R4:15
    83,  // C3;R5:16
    102, // C3;R6:17
    3,   // C4;R1:18
    24,  // C4;R2:19
    46,  // C4;R3:20
    66,  // C4;R4:21
    84,  // C4;R5:22
    103, // C4;R6:23
    4,   // C5;R1:24
    25,  // C5;R2:25
    47,  // C5;R3:26
    67,  // C5;R4:27
    85,  // C5;R5:28
    0,   // C5;R6:29
    5,   // C6;R1:30
    26,  // C6;R2:31
    48,  // C6;R3:32
    68,  // C6;R4:33
    86,  // C6;R5:34
    0,   // C6;R6:35
    0,   // C7;R1:36
    27,  // C7;R2:37
    49,  // C7;R3:38
    69,  // C7;R4:39
    87,  // C7;R5:40
    104, // C7;R6:41
    6,   // C8;R1:42
    28,  // C8;R2:43
    50,  // C8;R3:44
    70,  // C8;R4:45
    88,  // C8;R5:46
    105, // C8;R6:47
    7,   // C9;R1:48
    29,  // C9;R2:49
    51,  // C9;R3:50
    71,  // C9;R4:51
    89,  // C9;R5:52
    0,   // C9;R6:53
    8,   // C10;R1:54
    30,  // C10;R2:55
    52,  // C10;R3:56
    72,  // C10;R4:57
    90,  // C10;R5:58
    106, // C10;R6:59
    9,   // C11;R1:60
    31,  // C11;R2:61
    53,  // C11;R3:62
    73,  // C11;R4:63
    91,  // C11;R5:64
    107, // C11;R6:65
    10,  // C12;R1:66
    32,  // C12;R2:67
    54,  // C12;R3:68
    74,  // C12;R4:69
    92,  // C12;R5:70
    108, // C12;R6:71
    11,  // C13;R1:72
    33,  // C13;R2:73
    55,  // C13;R3:74
    75,  // C13;R4:75
    0,   // C13;R5:76
    0,   // C13;R6:77
    12,  // C14;R1:78
    34,  // C14;R2:79
    0,   // C14;R3:80
    76,  // C14;R4:81
    93,  // C14;R5:82
    109, // C14;R6:83
    13,  // C15;R1:84
    35,  // C15;R2:85
    56,  // C15;R3:86
    77,  // C15;R4:87
    94,  // C15;R5:88
    110, // C15;R6:89
    14,  // C16;R1:90
    36,  // C16;R2:91
    57,  // C16;R3:92
    0,   // C16;R4:93
    0,   // C16;R5:94
    110, // C16;R6:95
    15,  // C17;R1:96
    37,  // C17;R2:97
    58,  // C17;R3:98
    0,   // C17;R4:99
    95,  // C17;R5:100
    112, // C17;R6:101
    16,  // C18;R1:102
    38,  // C18;R2:103
    59,  // C18;R3:104
    0,   // C18;R4:105
    0,   // C18;R5:106
    113, // C18;R6:107
    17,  // C19;R1:108
    39,  // C19;R2:109
    60,  // C19;R3:110
    78,  // C19;R4:111
    96,  // C19;R5:112
    114, // C19;R6:113
    18,  // C20;R1:114
    40,  // C20;R2:115
    61,  // C20;R3:116
    79,  // C20;R4:117
    97,  // C20;R5:118
    0,   // C20;R6:119
    19,  // C21;R1:120
    41,  // C21;R2:121
    62,  // C21;R3:122
    80,  // C21;R4:123
    98,  // C21;R5:124
    115, // C21;R6:125
    20,  // C22;R1:126
    42,  // C22;R2:127
    63,  // C22;R3:128
    0,   // C22;R4:129
    99,  // C22;R5:130
    0,   // C22;R6:131
];

#[from_env]
pub const VERGEN_GIT_SEMVER: &str = "N/A";
#[from_env]
pub const VERGEN_GIT_COMMIT_COUNT: &str = "0";
