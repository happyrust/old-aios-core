use crate::RefU64;
use chrono::DateTime;
use chrono::TimeZone;
use chrono::Utc;
use deku::bitvec::*;
use deku::ctx::Endian;
use deku::prelude::*;
use derivative::Derivative;
use serde::{Deserialize, Serialize};
use std::convert::{TryFrom, TryInto};

pub const PAGE_SIZE: usize = 0x800;

#[derive(Default, Clone, Debug, PartialEq, DekuRead, DekuWrite, Serialize, Deserialize)]
#[deku(endian = "big")]
pub struct PdmsHeader {
    //开头两个未知
    pub unknown_0: [i32; 2],
    pub db_num: i32,
    pub unknown_1: [i32; 5], //然后是 00 00 00 01
    pub noun: i32,
    pub unknown_2: i32, // 0xFF FF FF FF
    pub latest_ses_pgno: u32,
    pub ext_no: u32,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct DbPageBasicInfo {
    pub pdms_header: PdmsHeader,
    pub latest_ses_pageno: u32,
    pub latest_ses_data: SessionPageData,
    //暂时通过记录file的大小来实现增量更新
    pub file_size: u64,
    // pub timestamp: DateTime<Utc>,
}

///会话层的定位信息
#[derive(Default, Clone, Debug, PartialEq, DekuRead, DekuWrite, Serialize, Deserialize)]
#[deku(endian = "big")]
pub struct SessionPageData {
    #[deku(skip, default = "0")]
    pub pgno: usize,
    pub page_type: i32,
    pub last_ses_pageno: i32,
    pub last_ses_extno: i32,
    //会话id
    pub sesno: i32,
    pub unknown_0: i32, // 0xFF FF FF FF

    //最后一页的页号
    pub end_pgno: u32,
    pub end_extno: u32,

    pub index_root_pageno: u32,
    pub index_root_extno: u32,
    pub claim_pageno: u32,
    pub claim_extno: u32,

    pub unknown_1: i32,
    pub unknown_2: i32,

    pub year: u32,
    pub month: u32,
    pub hours: u32,
    pub seconds: u32,

    pub unknown_u32: [i32; 13],
    pub name_words_len: u32,
    #[deku(count = "name_words_len * 4")]
    pub name_bytes: Vec<u8>,
    #[deku(count = "(9 - name_words_len) * 4")]
    pub empty_bytes: Vec<u8>,

    pub comments_words_len: u32,
    #[deku(count = "comments_words_len * 4")]
    pub comments_bytes: Vec<u8>,

    #[deku(count = "deku::rest.len()/8")]
    pub remain_bytes: Vec<u8>, //剩余的余量bytes
}

impl SessionPageData {
    #[inline]
    pub fn get_id(&self, project: &str, dbnum: i32) -> String {
        format!("{}_{}_{:0>6}", project, dbnum, self.sesno)
    }

    // pub fn gen_sur_json(&self, project: &str, dbnum: i32) -> String{
    //     //id 需要拿 sesno 和 dbnum 组合？还是和文件名组合？
    //     let id = self.get_id(project, dbnum);
    //     let mut json = serde_json::json!({
    //         "id": id,
    //         "sesno": self.sesno,
    //         "pgno": self.pgno,
    //         "dbnum": dbnum,
    //         "index_pgno": self.index_root_pageno,
    //         "claim_pgno": self.claim_pageno,
    //         "end_pgno": self.end_pgno,
    //         "computer_name": self.get_computer_name(),
    //         "comments": self.get_comments_name(),
    //         "date": self.get_timestamp().to_rfc3339(),
    //     });
    //     json.to_string()
    // }

    #[inline]
    pub fn get_timestamp(&self) -> DateTime<Utc> {
        let year = self.year;
        let month = self.month;
        let days = self.hours / 24;
        let hours = self.hours % 24;
        let minutes = self.seconds / 60;
        let seconds = self.seconds % 60;
        Utc.with_ymd_and_hms(
            year as i32,
            month as u32,
            days,
            hours as u32,
            minutes,
            seconds,
        )
        .latest()
        .unwrap()
    }

    // #[inline]
    // pub fn get_computer_name(&self) -> String {
    //     if self.name_words_len == 0 {
    //         return String::new();
    //     }
    //     //去掉后面为 0 的 bytes
    //     let i = (self.name_words_len as usize - 1) * 4;
    //     // dbg!(&self.name_bytes[i as usize..]);
    //     let rpos = self.name_bytes[i..].into_iter().rev().position(|&x| x != 0).unwrap_or(0);
    //     // dbg!(rpos);
    //     decode_chars_data(&self.name_bytes[..(i+4-rpos)]).0
    // }

    // #[inline]
    // pub fn get_comments_name(&self) -> String {
    //     if self.comments_words_len == 0 {
    //         return String::new();
    //     }
    //     //去掉后面为 0 的 bytes
    //     let i = (self.comments_words_len as usize - 1) * 4;
    //     // dbg!(&self.comments_bytes[i as usize..]);
    //     let rpos = self.comments_bytes[i..].into_iter().rev().position(|&x| x != 0).unwrap_or(0);
    //     // dbg!(rpos);
    //     decode_chars_data(&self.comments_bytes[..(i+4-rpos)]).0
    // }
    //
    // //是否需要要检测有无变化？先拿到最新的数据试试看里面的参考号，和之前的比有无变化
    // pub fn get_session_saved_refnos() {
    //
    // }
}

///内含有的几个index part，名称表等等
#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "big")]
pub struct SesIndexesData {
    #[deku(assert_eq = "0x3")]
    pub page_type: i32,
    pub last_ses_pageno: u32,
    pub last_ses_extno: u32,
    pub sesno: i32,
    pub unknown_0: i32, // 0xFF FF FF FF

    pub claim_data_pageno: u32,
    pub claim_data_extno: u32,

    pub index_root_pageno: u32,
    pub index_root_extno: u32,
    pub claim_root_pageno: u32,
    pub claim_root_extno: u32,
}

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "big")]
pub struct RefnoIndexPgId {
    pub refno_0: u32,
    pub refno_1: u32,
    pub page_no: u32,
    pub ext_no: u32,
}

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
pub struct RootIndexPage {
    #[deku(endian = "big")]
    pub page_type: i32,
    #[deku(endian = "big")]
    pub noun: i32,
    //00 00 00 02 00 00 00 02 00 00 00 02 00 00 00 00
    #[deku(endian = "big")]
    pub unknowns_0: [i32; 4],
    //00 00 01 ED
    #[deku(endian = "big")]
    pub residual_num: u32, //要用0x200 - residual_num 得到剩余的值
    //80 00 00 01 80 00 00 01
    #[deku(endian = "big")]
    pub lock: [i32; 2], //可能是lock

    #[deku(endian = "big")]
    pub last_pageno: u32,
    #[deku(endian = "big")]
    pub last_extno: u32,

    pub lower_root: RefnoIndexPgId,
    pub upper_root: RefnoIndexPgId,
}

///Index 里的数据条目
#[derive(Debug, PartialEq, DekuRead, DekuWrite, Clone)]
#[deku(endian = "big")]
pub struct RefnoDataLoc {
    pub refno_0: u32,
    pub refno_1: u32,
    pub pgno: u32,
    #[deku(bits = "20")]
    pub offset: u32,
    #[deku(bits = "12")]
    pub other: u16,
}

impl RefnoDataLoc {
    #[inline]
    pub fn get_refno(&self) -> RefU64 {
        RefU64::from_two_nums(self.refno_0, self.refno_1)
    }

    #[inline]
    pub fn get_att_offset(&self) -> u64 {
        self.pgno as u64 * 0x800 + self.offset as u64 * 2
    }
}

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
pub struct RefnoIndexPage {
    #[deku(endian = "big")]
    pub page_type: i32,
    #[deku(endian = "big")]
    pub noun: i32,
    //00 00 00 02 00 00 00 02 00 00 00 02 00 00 00 00
    #[deku(endian = "big")]
    pub unknowns_0: [i32; 4],

    #[deku(endian = "big")]
    pub pfno: u32, //还需要搞的清楚一点，这个值到底怎么来的

    #[deku(reader = "read_refno_index_pgid(deku::rest, )")]
    pub data_locs: Vec<RefnoIndexPgId>,
}

//DekuWrite
#[derive(Derivative, PartialEq, DekuRead)]
#[derivative(Debug)]
#[deku(endian = "big")]
pub struct IndexPageData {
    pub page_type: i32,
    #[deku(assert_eq = "0xCC47DF")]
    pub noun: i32,
    pub level: u32,
    pub unknowns: [u32; 3],
    pub pfno: u32,

    #[deku(reader = "read_refno_data_loc(deku::rest, *level)")]
    pub refno_locs: Vec<RefnoDataLoc>,
    #[derivative(Debug = "ignore")]
    #[deku(count = "deku::rest.len()/8")]
    pub remain_zero_bytes: Vec<u8>, //剩余的余量bytes
}

impl IndexPageData {
    #[inline]
    pub fn get_max_pgno(&self) -> u32 {
        self.refno_locs
            .iter()
            .map(|x| x.pgno)
            .max()
            .unwrap_or_default()
    }
}

//如果 level 为 2，格式还有点不一样
fn read_refno_data_loc(
    mut rest: &BitSlice<u8, Msb0>,
    level: u32,
) -> Result<(&BitSlice<u8, Msb0>, Vec<RefnoDataLoc>), DekuError> {
    let mut vec = Vec::new();
    if level == 2 {
        //4 个未知数
        //80 00 00 01 80 00 00 01 00 00 7D 3E 00 00 00 01
        rest = u32::read(rest, ())?.0;
        rest = u32::read(rest, ())?.0;
        rest = u32::read(rest, ())?.0;
        rest = u32::read(rest, ())?.0;
    }
    loop {
        let (next_rest, peek) = u32::read(rest, ())?;
        if peek == 0x0 {
            rest = next_rest;
            break;
        }
        let (next_rest, d) = RefnoDataLoc::read(rest, ())?;
        vec.push(d);
        rest = next_rest;
    }
    Ok((rest, vec))
}

fn read_refno_index_pgid(
    rest: &BitSlice<u8, Msb0>,
) -> Result<(&BitSlice<u8, Msb0>, Vec<RefnoIndexPgId>), DekuError> {
    let mut pgids = Vec::new();
    let mut rest = rest;
    loop {
        let (next_rest, peek) = u32::read(rest, ())?;
        if peek == 0x0 {
            rest = next_rest;
            break;
        }
        let (next_rest, pgid) = RefnoIndexPgId::read(rest, ())?;
        pgids.push(pgid);
        rest = next_rest;
    }
    Ok((rest, pgids))
}

//todo 需要处理跨页的数据
#[derive(Clone, Debug, PartialEq, Default, DekuRead, DekuWrite)]
#[deku(ctx = "_endian: Endian")]
pub struct EleMembers {
    #[deku(endian = "big")]
    pub flag: u16,
    #[deku(endian = "big")]
    pub len: u16,
    #[deku(endian = "big")]
    pub refno: (u32, u32),
    #[deku(endian = "big")]
    pub unknown_0: (u32, u32),
    #[deku(count = "(len-4)/2")]
    #[deku(endian = "big")]
    pub children: Vec<(u32, u32)>,
}

#[derive(Clone, Debug, PartialEq, DekuRead, DekuWrite)]
// #[deku(endian = "big")]
pub struct ElePageData {
    //0x7
    pub flag: u32,
    #[deku(reader = "read_eles(deku::rest)")]
    pub eles_vec: Vec<EleRawData>,
    #[deku(count = "deku::rest.len()/8")]
    pub remain_bytes: Vec<u8>, //剩余的余量bytes
}

#[derive(Clone, Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "big")]
pub struct EleRawData {
    //00 00
    pub implicit_flag: u16,
    //00 2E
    pub implicit_count: u16,

    // pub implicit_size: i32,
    //00 00 00 02 00 00 00 02 00 00 00 02 00 00 00 00
    pub ref0: i32,
    pub ref1: i32,
    pub noun: i32, //还需要搞的清楚一点，这个值到底怎么来的

    pub parent_ref0: i32,
    pub parent_ref1: i32,
    pub page_no: u32,

    #[deku(cond = "*implicit_flag == 0", count = "(implicit_count - 7) * 4")]
    pub implicit_data: Vec<u8>,

    #[deku(reader = "read_members(deku::rest)")]
    pub members: Option<EleMembers>,

    pub explicit_flag: u16,
    pub explicit_count: u16,

    #[deku(cond = "*explicit_flag == 1", count = "(explicit_count - 1) * 4")]
    pub explicit_data: Option<Vec<u8>>,
}

impl EleRawData {}

fn read_members(
    rest: &BitSlice<u8, Msb0>,
) -> Result<(&BitSlice<u8, Msb0>, Option<EleMembers>), DekuError> {
    let (_next_rest, peek) = u16::read(rest, Endian::Big)?;
    if peek != 0x2 {
        return Ok((rest, None));
    }
    let (next_rest, membs) = EleMembers::read(rest, Endian::Big)?;
    Ok((next_rest, Some(membs)))
}

fn read_eles(
    rest: &BitSlice<u8, Msb0>,
) -> Result<(&BitSlice<u8, Msb0>, Vec<EleRawData>), DekuError> {
    let mut vec = Vec::new();
    let mut rest = rest;
    loop {
        let (next_rest, peek) = u32::read(rest, ())?;
        if peek == 0x0 {
            rest = next_rest;
            break;
        }
        let (next_rest, d) = EleRawData::read(rest, ())?;
        vec.push(d);
        rest = next_rest;
    }
    Ok((rest, vec))
}
