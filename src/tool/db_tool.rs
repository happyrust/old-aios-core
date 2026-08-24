use dashmap::DashMap;
use lazy_static::lazy_static;
use memchr::memmem::{find, find_iter};
use once_cell::sync::Lazy;
use std::fs::File;
use std::io::Read;

use crate::types::db_info::PdmsDatabaseInfo;

lazy_static! {
    pub static ref GLOBAL_UDA_NAME_MAP: DashMap<u32, String> = DashMap::new();
    pub static ref GLOBAL_UDA_UKEY_MAP: DashMap<String, u32> = DashMap::new();
}

/// 从bincode数据加载PdmsDatabaseInfo
pub fn read_attr_info_config_from_bin(config_path: &str) -> PdmsDatabaseInfo {
    let mut file = File::open(config_path).unwrap();
    let mut attr_buf: Vec<u8> = Vec::new();
    file.read_to_end(&mut attr_buf);
    bincode::deserialize(&attr_buf).unwrap()
}

/// 从json数据加载PdmsDatabaseInfo
pub fn read_attr_info_config_from_json(config_path: &str) -> PdmsDatabaseInfo {
    let mut file = File::open(config_path).unwrap();
    let mut attr_buf: Vec<u8> = Vec::new();
    file.read_to_end(&mut attr_buf);
    serde_json::from_slice(&attr_buf).unwrap()
}

#[inline]
pub fn convert_to_hash(bytes: &[u8]) -> i32 {
    i32::from_be_bytes(bytes.try_into().unwrap())
}

#[inline]
pub fn is_uda(hash: i32) -> bool {
    hash > 0x171FAD39
}

#[inline]
pub fn is_uda_name(name: &str) -> bool {
    name.starts_with(":")
}

#[inline]
pub fn get_uda_index(hash: u32) -> Option<u32> {
    if hash > 0x171FAD39 {
        Some((hash - 0x171FAD39) >> 24)
    } else {
        None
    }
}

#[inline]
pub fn db1_dehash(hash: u32) -> String {
    let mut result = String::new();
    if hash > 0x171FAD39 {
        let mut k = ((hash - 0x171FAD39) % 0x1000000) as i32;
        result.push(':');
        for _i in 0..6 {
            if k <= 0 {
                break;
            }
            result.push((k % 64 + 32) as u8 as char);
            k /= 64;
        }
    } else {
        if hash <= 0x81BF1 {
            return "".to_string();
        }
        let mut k = (hash - 0x81BF1) as i32;
        while k > 0 {
            result.push((k % 27 + 64) as u8 as char);
            k /= 27;
        }
    }
    result
}

#[inline]
pub fn db1_hash_i32(hash_str: &str) -> i32 {
    db1_hash(hash_str) as _
}

//类似于POSF，这种它的hash是POSI，这里需要强制的把名称缓过来
// static ALIAS_ATT_NAME_MAP: Lazy<DashMap<&'static str, u32>> = Lazy::new(|| {
//     let mut m = DashMap::new();
//     m.insert("POSF", db1_hash("POSI"));
//     m
// });

//todo 处理出错的情况
#[inline]
pub fn db1_hash(hash_str: &str) -> u32 {
    let chars = hash_str.as_bytes();
    if chars.len() < 1 {
        return 0; //出错的暂时用0 表达
    }
    let mut val = 0i64;
    let mut i = (chars.len() - 1) as i32;
    while i >= 0 {
        val = val.overflowing_mul(27).0 + (chars[i as usize] as i64 - 64);
        i -= 1;
    }
    val.saturating_add_unsigned(0x81BF1) as u32
}

#[inline]
pub fn db1_dehash_const(hash: u32) -> String {
    let mut result = String::new();
    if hash > 0x171FAD39 {
        // UDA的情况
        let mut k = ((hash - 0x171FAD39) % 0x1000000) as i32;
        result.push(':');
        for _i in 0..6 {
            if k <= 0 {
                break;
            }
            result.push((k % 64 + 32) as u8 as char);
            k /= 64;
        }
    } else {
        if hash <= 0x81BF1 {
            return "".to_string();
        }
        let mut k = (hash - 0x81BF1) as i32;
        while k > 0 {
            result.push((k % 27 + 64) as u8 as char);
            k /= 27;
        }
    }
    result
}

pub const fn db1_hash_const(hash_str: &str) -> u32 {
    let chars = hash_str.as_bytes();
    if chars.len() < 1 {
        return 0; //出错的暂时用0 表达
    }
    let mut val = 0i64;
    let mut i = (chars.len() - 1) as i32;
    while i >= 0 {
        val = val.overflowing_mul(27).0 + (chars[i as usize] as i64 - 64);
        i -= 1;
    }
    val.saturating_add_unsigned(0x81BF1) as u32
}

#[test]
fn db1_dehash_test() {
    //USER
    //0xd943a
    let name = db1_dehash(0xd943a);
    println!("name={:?}", name);

    let name = db1_dehash(0x95B0C);
    println!("name={:?}", name);

    let name = db1_dehash(0x743f49);
    println!("name={:?}", name);

    //0x107684ca
    let name = db1_dehash(0x107684ca);
    println!("name={:?}", name);

    //0xd53c73e
    let name = db1_dehash(0xd53c73e);
    println!("name={:?}", name);

    // 0xd943a
    let name = db1_dehash(0xd943a);
    println!("name={:?}", name);

    //0x10767608
    let name = db1_dehash(0x10767608);
    println!("name={:?}", name);

    //0xe578d83
    let name = db1_dehash(0xe578d83);
    println!("name={:?}", name);

    // 0x743f49
    let name = db1_dehash(0x743f49);
    println!("name={:?}", name);

    //0x16e27bf9
    let name = db1_dehash(0x16e27bf9);
    println!("name={:?}", name);

    //10cd4f90
    let name = db1_dehash(0x10cd4f90);
    println!("0x10cd4f90 name={:?}", name);

    //0x10769b0c
    let name = db1_dehash(0x10769b0c);
    println!("name={:?}", name);

    //0x10ceccbb
    let name = db1_dehash(0x10ceccbb);
    println!("name={:?}", name);

    let name = db1_dehash(0xcc6b3F);
    println!("0xcc6b3F={:?}", name);

    //0x107684ca
    let name = db1_dehash(0x107684ca);
    println!("name={:?}", name);

    //10767607
    let name = db1_dehash(0x10767607);
    println!("name={:?}", name);

    //0x77fff4
    let name = db1_dehash(0x77fff4);
    println!("name={:?}", name);

    //0xc80b935
    let name = db1_dehash(0xc80b935);
    println!("name={:?}", name);

    //0xc9d2755
    let name = db1_dehash(0xc9d2755);
    println!("name={:?}", name);

    let val = db1_hash("SCTN");
    println!("{:#4X}", val);
}

fn convert_to_le_i32(table: &[u8], dw_offset: usize) -> i32 {
    i32::from_le_bytes(table[dw_offset * 4..dw_offset * 4 + 4].try_into().unwrap())
}

fn get_mapped_value(table: &[u8], v: i32) -> i32 {
    let mut res = -1;
    if v < 0xFFFF {
        if v < 0 {
            res = -1;
        } else if v <= 127 {
            res = v;
        } else if v <= 255 {
            res = convert_to_le_i32(table, v as usize) as i32;
        } else {
            res = convert_to_le_i32(table, (v / 0x100) as usize);
        }
    }

    if res == -99999 || v < 0x40 || v == 0xFF {
        return -1;
    }

    if res < 0 {
        let c = res * -764i32;
        //println!("{:#4X}", c);
        let a = v & 0xFF;
        // //println!("{:#4X}", a);
        //println!("{}", (a + (c / 4) as i32 + 1) as usize);
        res = convert_to_le_i32(table, (a + (c / 4) as i32 + 1) as usize) as i32;
        //从1开始
    }
    res
}

pub fn convert_to_u8_vec(v: i64) -> Vec<u8> {
    let mut res = vec![];
    if v < 0 {
    } else if v <= 127 {
        res.push(v as u8);
    } else if v <= 2047 {
        res.push((v / 0x40 + 0xC0) as u8);
        res.push((v % 0x40 + 0x80) as u8);
    } else if v <= 0xFFFF {
        res.push((v / 0x1000 + 0xE0) as u8);
        res.push(((v & 0x80000FFF) / 0x40 + 0x80) as u8);
        res.push((v % 0x40 + 0x80) as u8);
    } else if v > 0x10FFFF {
        //todo
    } else {
        res.push((v / 0x40000 + 240) as u8);
        res.push(((v & 0x8003FFFF) / 0x1000 + 0x80) as u8);
        res.push(((v & 0x80000FFF) / 0x40 + 0x80) as u8);
        res.push((v % 0x40 + 0x80) as u8);
    }
    return res;
}

///处理这种数据 [0x26, 0x7E, 0xxx, 0xxx, 0x20, 0x26]
pub fn decode_chi_chars(table: &[u8], data: &[u8]) -> String {
    if data.len() < 6 {
        return "unset".to_string();
    }
    let mut str_data = vec![];
    let d = &data[2..data.len() - 2];
    //println!("{:#4X?}", d);
    let mut i = 0;
    while i < d.len() - 1 {
        //todo 原来是 d.len() 加了个 -1 ，该方法可能需要调整
        let d0 = d[i] as u64;
        //println!("{:#4X?}", d0);
        let d1 = d[i + 1] as i32;
        //println!("{:#4X?}", d1);
        let val = (d0 << 8) as i32 + d1 + 0x8080;
        //println!("{:#4X?}", val);
        let code = get_mapped_value(table, val) as i64;
        //println!("Code: {:#4X?}", code);
        let chars = convert_to_u8_vec(code);
        //println!("Code: {:#4X?}", &chars);
        str_data.extend(chars);
        i += 2;
    }
    String::from_utf8_lossy(&str_data).to_string()
}

///返回（解密后的utf8字符串， 是否包含中文）
pub fn decode_chars_data(input: &[u8]) -> (String, bool) {
    let table_data = include_bytes!("../../encode_char_table.bin");
    let start_iter = find_iter(&input, &[0x26, 0x7E]);
    let mut contains_chi = false;
    let mut res = vec![];
    let mut prev_pos = 0;
    for p in start_iter {
        if input.len() > p && prev_pos <= p {
            // todo 这个地方也需要调整
            res.extend_from_slice(&input[prev_pos..p]);
            if let Some(len) = find(&input[p..], &[0x20, 0x26]) {
                // dbg!(&input[p..p + len]);
                let decode_str = decode_chi_chars(table_data, &input[p..p + len + 2]);
                res.extend(decode_str.bytes());
                prev_pos = p + len + 2;
                contains_chi = true;
            } else {
                res.extend_from_slice(&input[p..]);
                break;
            }
        }
    }
    res.extend_from_slice(&input[prev_pos..]);

    let r = String::from_utf8_lossy(&res).to_string();
    (r, contains_chi)
}

#[test]
fn test_chinese_data() {
    // //println!("Hello, world!");
    //26 7E 37 27 43 45 20 26
    let _test_code = vec![
        0x26, 0x7E, 0x37, 0x56, 0x56, 0x27, 0x32, 0x62, 0x4A, 0x54, 0x20, 0x26,
    ];
    let _test_code = vec![0x26, 0x7E, 0x37, 0x27, 0x43, 0x45, 0x20, 0x26];
    let _test_code = vec![0x26, 0x7E, 0x39, 0x5C, 0x20, 0x26];
    let _test_code = vec![
        0x2F, 0x26, 0x7E, 0x39, 0x5C, 0x20, 0x26, 0x31, 0x32, 0x26, 0x7E, 0x35, 0x40, 0x20, 0x26,
    ];
    let _test_code = vec![
        0x2F, 0x26, 0x7E, 0x39, 0x5C, 0x20, 0x26, 0x31, 0x32, 0x26, 0x7E, 0x35, 0x40, 0x20, 0x26,
        0x74, 0x65, 0x73, 0x74,
    ];
    let test_code = vec![0x2F, 0x31, 0x30, 0x30, 0x2D, 0x42, 0x2D, 0x31];
    // let table_data = include_bytes!("../encode_char_table.bin");

    let name = decode_chars_data(&test_code);
    dbg!(name);
}

#[test]
fn test_db1_dehash() {
    let hash = db1_dehash(688051936);
    assert_eq!(":CNPEspco".to_string(), hash);
    let hash = db1_dehash(3832756588);
    assert_eq!(":3D_SJZT".to_string(), hash);
    let hash = db1_dehash(642951949);
    assert_eq!(":3D_SJRY".to_string(), hash);
}
