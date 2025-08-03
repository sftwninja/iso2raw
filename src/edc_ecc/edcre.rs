// Vendored EDCRE implementation
// Based on EDCRE by alex-free/edcre which is built on top of lec.cc from cdrdao
// Copyright (C) 1998-2002 Andreas Mueller <andreas@daneb.de>
//
// This implementation provides bit-accurate EDC/ECC calculation that matches
// the reference implementation used by CD burning software and hardware.
//
// EDCRE source: https://github.com/alex-free/edcre
// Original cdrdao source: https://github.com/cdrdao/cdrdao

use std::sync::OnceLock;

const GF8_PRIM_POLY: u16 = 0x11d; // x^8 + x^4 + x^3 + x^2 + 1
const EDC_POLY: u32 = 0x8001801b; // (x^16 + x^15 + x^2 + 1) * (x^16 + x^2 + x + 1)

static GF8_LOG: OnceLock<[u8; 256]> = OnceLock::new();
static GF8_ILOG: OnceLock<[u8; 256]> = OnceLock::new();
static CRC_TABLE: OnceLock<[u32; 256]> = OnceLock::new();
static GF8_Q_COEFFS_TABLE: OnceLock<[[u16; 256]; 43]> = OnceLock::new();

fn mirror_bits(d: u32, bits: usize) -> u32 {
    let mut r = 0u32;
    let mut d = d;

    for _ in 0..bits {
        r <<= 1;
        if (d & 0x1) != 0 {
            r |= 0x1;
        }
        d >>= 1;
    }

    r
}

fn init_crc_table() -> [u32; 256] {
    let mut table = [0u32; 256];

    for (i, entry) in table.iter_mut().enumerate() {
        let mut r = mirror_bits(i as u32, 8);
        r <<= 24;

        for _ in 0..8 {
            if (r & 0x80000000) != 0 {
                r <<= 1;
                r ^= EDC_POLY;
            } else {
                r <<= 1;
            }
        }

        r = mirror_bits(r, 32);
        *entry = r;
    }

    table
}

fn init_gf8_tables() -> ([u8; 256], [u8; 256]) {
    let mut log_table = [0u8; 256];
    let mut ilog_table = [0u8; 256];

    let mut b = 1u16;

    for log in 0..255u8 {
        log_table[b as usize] = log;
        ilog_table[log as usize] = b as u8;

        b <<= 1;

        if (b & 0x100) != 0 {
            b ^= GF8_PRIM_POLY;
        }
    }

    (log_table, ilog_table)
}

fn gf8_add(a: u8, b: u8) -> u8 {
    a ^ b
}

fn gf8_div(a: u8, b: u8) -> u8 {
    let log_table = GF8_LOG.get().unwrap();
    let ilog_table = GF8_ILOG.get().unwrap();

    if b == 0 {
        panic!("Division by zero in GF(8)");
    }

    if a == 0 {
        return 0;
    }

    let mut sum = log_table[a as usize] as i16 - log_table[b as usize] as i16;

    if sum < 0 {
        sum += 255;
    }

    ilog_table[sum as usize]
}

fn init_gf8_q_coeffs_table() -> [[u16; 256]; 43] {
    let log_table = GF8_LOG.get().unwrap();
    let ilog_table = GF8_ILOG.get().unwrap();

    let mut gf8_coeffs_help = [[0u8; 45]; 2];
    let mut gf8_q_coeffs = [[0u8; 45]; 2];

    // Build matrix H:
    // 1    1   ...  1   1
    // a^44 a^43 ... a^1 a^0
    for j in 0..45 {
        gf8_coeffs_help[0][j] = 1; // e0
        gf8_coeffs_help[1][j] = ilog_table[44 - j]; // e1
    }

    // Resolve equation system for parity byte 0 and 1

    // e1' = e1 + e0
    for j in 0..45 {
        gf8_q_coeffs[1][j] = gf8_add(gf8_coeffs_help[1][j], gf8_coeffs_help[0][j]);
    }

    // e1'' = e1' / (a^1 + 1)
    for j in 0..45 {
        gf8_q_coeffs[1][j] = gf8_div(gf8_q_coeffs[1][j], gf8_q_coeffs[1][43]);
    }

    // e0' = e0 + e1 / a^1
    for j in 0..45 {
        gf8_q_coeffs[0][j] = gf8_add(
            gf8_coeffs_help[0][j],
            gf8_div(gf8_coeffs_help[1][j], ilog_table[1]),
        );
    }

    // e0'' = e0' / (1 + 1 / a^1)
    for j in 0..45 {
        gf8_q_coeffs[0][j] = gf8_div(gf8_q_coeffs[0][j], gf8_q_coeffs[0][44]);
    }

    // Compute the products of 0..255 with all of the Q coefficients
    let mut table = [[0u16; 256]; 43];

    for j in 0..43 {
        table[j][0] = 0;

        for i in 1..256 {
            let mut c = log_table[i] as u16 + log_table[gf8_q_coeffs[0][j] as usize] as u16;
            if c >= 255 {
                c -= 255;
            }
            table[j][i] = ilog_table[c as usize] as u16;

            c = log_table[i] as u16 + log_table[gf8_q_coeffs[1][j] as usize] as u16;
            if c >= 255 {
                c -= 255;
            }
            table[j][i] |= (ilog_table[c as usize] as u16) << 8;
        }
    }

    table
}

fn ensure_tables_initialized() {
    static INIT: OnceLock<()> = OnceLock::new();
    INIT.get_or_init(|| {
        let (log_table, ilog_table) = init_gf8_tables();
        let _ = GF8_LOG.set(log_table);
        let _ = GF8_ILOG.set(ilog_table);
        let _ = CRC_TABLE.set(init_crc_table());
        let _ = GF8_Q_COEFFS_TABLE.set(init_gf8_q_coeffs_table());
    });
}

pub fn calc_edc(data: &[u8]) -> u32 {
    ensure_tables_initialized();
    let table = CRC_TABLE.get().unwrap();

    let mut crc = 0u32;

    for &byte in data {
        crc = table[((crc ^ byte as u32) & 0xff) as usize] ^ (crc >> 8);
    }

    crc
}

pub fn calc_mode1_edc(sector: &mut [u8]) {
    let crc = calc_edc(&sector[0..2064]); // sync + header + data

    sector[2064] = (crc & 0xff) as u8;
    sector[2065] = ((crc >> 8) & 0xff) as u8;
    sector[2066] = ((crc >> 16) & 0xff) as u8;
    sector[2067] = ((crc >> 24) & 0xff) as u8;
}

pub fn calc_p_parity(sector: &mut [u8]) {
    ensure_tables_initialized();
    let table = GF8_Q_COEFFS_TABLE.get().unwrap();

    let p_lsb_start = 12; // LEC_HEADER_OFFSET
    let p_parity_offset = 2076; // LEC_MODE1_P_PARITY_OFFSET

    for i in 0..=42 {
        let mut p_lsb_idx = p_lsb_start + i * 2;

        let mut p01_lsb = 0u16;
        let mut p01_msb = 0u16;

        for table_row in table.iter().skip(19).take(24) {
            let d0 = sector[p_lsb_idx];
            let d1 = sector[p_lsb_idx + 1];

            p01_lsb ^= table_row[d0 as usize];
            p01_msb ^= table_row[d1 as usize];

            p_lsb_idx += 2 * 43;
        }

        // P0 (LSB)
        sector[p_parity_offset + 2 * 43 + i * 2] = (p01_lsb & 0xff) as u8;
        sector[p_parity_offset + 2 * 43 + i * 2 + 1] = (p01_msb & 0xff) as u8;

        // P1 (MSB)
        sector[p_parity_offset + i * 2] = (p01_lsb >> 8) as u8;
        sector[p_parity_offset + i * 2 + 1] = (p01_msb >> 8) as u8;
    }
}

pub fn calc_q_parity(sector: &mut [u8]) {
    ensure_tables_initialized();
    let table = GF8_Q_COEFFS_TABLE.get().unwrap();

    let q_lsb_start = 12; // LEC_HEADER_OFFSET
    let q_parity_offset = 2248; // LEC_MODE1_Q_PARITY_OFFSET

    for i in 0..=25 {
        let mut q_lsb_idx = q_lsb_start + i * 2 * 43;

        let mut q01_lsb = 0u16;
        let mut q01_msb = 0u16;

        for table_row in table.iter().take(43) {
            let d0 = sector[q_lsb_idx];
            let d1 = sector[q_lsb_idx + 1];

            q01_lsb ^= table_row[d0 as usize];
            q01_msb ^= table_row[d1 as usize];

            q_lsb_idx += 2 * 44;

            if q_lsb_idx >= q_parity_offset {
                q_lsb_idx -= 2 * 1118;
            }
        }

        // Q0 (LSB)
        sector[q_parity_offset + 2 * 26 + i * 2] = (q01_lsb & 0xff) as u8;
        sector[q_parity_offset + 2 * 26 + i * 2 + 1] = (q01_msb & 0xff) as u8;

        // Q1 (MSB)
        sector[q_parity_offset + i * 2] = (q01_lsb >> 8) as u8;
        sector[q_parity_offset + i * 2 + 1] = (q01_msb >> 8) as u8;
    }
}
