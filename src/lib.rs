#![no_std]

#[cfg(any(windows, unix))]
extern crate alloc;
#[cfg(any(windows, unix))]
use alloc::string::String;

// use alloc::string::String;
// use alloc::string::ToString;
// use alloc::vec;
// use alloc::vec::Vec;
use core::mem::size_of;
use core::ptr::slice_from_raw_parts;

type Error = MeterError;

const UNINIT_ZERO: u8 = 0;
// 计量模块没有时间等功能，用不上
// const METER_BROADCAST_ADDRESS: [u8; 6] = [0x99, 0x99, 0x99, 0x99, 0x99, 0x99];

const DI_IM1281X_A路有功总电量: u32 = 0x8080_0001;
const DI_IM1281X_B路有功总电量: u32 = 0x8080_0002;
const DI_DTL645通用_A路电压: u32 = 0x0201_0100;
const DI_DTL645通用_A路电流: u32 = 0x0202_0100;
const DI_DTL645通用_温度: u32 = 0x0280_0007;
const DI_IM1281X_波特率: u32 = 0x0400_0703;
// const DI_DTL645通用_地址: u32 = 0x0400_0401;

#[derive(Debug, PartialEq, Eq)]
pub enum FunctionCode {
    /// 读模块地址（仅支持点对点通信）
    MasterQuerySlaveAddr,
    /// 设置模块地址（仅支持点对点通信）
    MasterSetSlaveAddr([u8; 6]),
    /// 更改通信速率
    MasterChangeSlaveBaudRate(BaudRate),
    // /// 冻结电能数据（计量模块没这功能，这个要采集端去实现，挂这先）
    // FreezeCmdReserved,
    // /// 清零（计量数据，不含地址）,计量模块也没有这功能
    MResetMeter,
    // /// 读电压（一个模块接一路测量电压，其他路电压也是这个）
    // M查询电压,
    // /// 读A路电流。 艾瑞达你B路文档呢？。。。
    M查询A路电流,
    M查询A路有功总电量IM1281X,
    M查询B路有功总电量IM1281X,
    M查询温度,
}

/// 电表控制码，目前支持部分功能，生活区抄表够用了
// #[allow(unused)]
#[derive(Debug, PartialEq, Eq)]
enum Code {
    Reserved = 0b00000,
    BroadcastTime = 0b01000,
    ReadData = 0b10001,
    ReadSubsequentData = 0b10010,
    ReadAddr = 0b10011,
    WriteData = 0b10100,
    WriteAddr = 0b10101,
    FreezeCmd = 0b10110,
    ChangeBaudrate = 0b10111,
    ChangPasswd = 0b11000,
    /// 最大需求量清零
    ResetMaxDemand = 0b11001,
    ResetMeter = 0b11010,
    ResetEvent = 0b11011,
    /// Slaver说还有后续帧，继续ReadSubsequentData
    SubsequentDataFollowUp = 1 << 5,
    /// 从站回应说你发的指令不对，一字节的错误码放payload了
    SlaveACKErr = 1 << 6,
    /// 表示这条报文是从站发出的
    SlaveFlag = 1 << 7,
}

#[derive(Debug, PartialEq, Eq)]
pub enum MeterResult {
    /// 地址
    MeterAddr([u8; 6]),

    /// xxxx (0.1 V)
    A路电压单位0_1V(u16),

    /// xxx_xxx (mA)
    A路电流mA(u32),

    // /// xx_xxxx (0.0001KW)
    // A路有功功率(u32),
    /// xxxxxx_xx (0.01 KWh)
    A路有功总电量_单位0_01KWh(u32),

    /// xxxxxx_xx (0.01 KWh)
    B路有功总电量_单位0_01KWh(u32),

    /// xx_xx (0.1 ℃)
    温度_单位0_1C(u16),

    BaudRate(BaudRate),

    SetMeterAddrSuccess([u8; 6]),

    ResetMeterSuccess([u8; 6]),
}

#[derive(Debug, PartialEq, Eq)]
pub enum BaudRate {
    Baud1200,
    Baud2400,
    Baud4800,
    Baud9600,
}

#[derive(Debug, PartialEq, Eq)]
pub enum MeterError {
    BadData,
    ErrCRC,
    // BadAddress,
    // InvalidAddr,
    UnsupportedYetDI(u32),
    UnsupportedYetChangePasswd,
    UnsupportedYetResetMaxDemand,
    UnsupportedYetResetEvent,
    UnsupportedYetBroadcastTime,
    UnsupportedYetWriteData,
    UnsupportedFreezeCmd,
    UnExpectedNonePayload,
    UnExpectedPayload,
    UnExpectedNoneAddr,
    UnknownBaudrate,
    ParseResultFailed,
    // SlaveAckErr(u8),
    IoErr(MeterIOError),
    NotSlaveMessage,
    SlaveAckErr,
    // 呃这个命名。。。可能永远不用改这个库了，不改了，就先这样。。。
    // UnsupportedYet(String),
    UnsupportedYetSubsequentDataFollowUpFlag,
    UnsupportedYetReadSubsequentData,
    // Unsupported(String),
    UnknownErr,
    ParseAddrFromStringErr,
    // ParseDiErr(String),
    ReservedCode,
}

#[derive(Debug, PartialEq, Eq)]
pub enum MeterIOError {
    TimeOutReadExactBytes,
    // DeviceOrResourceBusy,
    // InputOutputError,
    // NoSuchDevice,
    IncompleteWrite,
    // ReadTimeout,
    #[cfg(any(windows, unix))]
    Std(String),
}

// #[repr(C)]
#[derive(Debug, Clone)]
struct DLT645_2007 {
    addr: [u8; 6],
    code: u8,
    payload_len: u8,
    payload: Option<Payload>,
    // checksum: u8,
}

pub struct DLT645_2007Raw {
    len: u8,
    // 够用了
    container: [u8; 31],
}

#[derive(Debug, Clone)]
struct Payload {
    data_identifiers: Option<u32>,
    data_len: u8,
    // 目前最大也就4+4
    data: [u8; 11],
}

// impl TryFrom<&str> for MeterAddr {
//     type Error = MeterError;

//     fn try_from(value: &str) -> Result<Self, Self::Error> {
//         if value.is_ascii() && value.len() == 12 {
//             let mut chars = value.chars();
//             let mut addr = [0u8; 6];
//             for i in 0..6 {
//                 addr[i] = (chars.next().unwrap().to_digit(16).unwrap() * 16
//                     + chars.next().unwrap().to_digit(16).unwrap()) as u8;
//             }
//             return Ok(MeterAddr { raw: addr });
//         }
//         Err(MeterError::ParseAddrFromStringErr)
//     }
// }

// impl MeterAddr {
//     pub fn to_string(&self) -> String {
//         self.raw
//             .iter()
//             .fold("".to_string(), |acc, x| acc + &x.to_string())
//     }
// }

impl Code {
    fn val(self) -> u8 {
        self as u8
    }
}

impl From<u8> for Code {
    fn from(value: u8) -> Self {
        match value {
            0 => Code::Reserved,
            0b01000 => Code::BroadcastTime,
            0b10001 => Code::ReadData,
            0b10010 => Code::ReadSubsequentData,
            0b10011 => Code::ReadAddr,
            0b10100 => Code::WriteData,
            0b10101 => Code::WriteAddr,
            0b10110 => Code::FreezeCmd,
            0b10111 => Code::ChangeBaudrate,
            0b11000 => Code::ChangPasswd,
            0b11001 => Code::ResetMaxDemand,
            0b11010 => Code::ResetMeter,
            0b11011 => Code::ResetEvent,
            _ => Code::Reserved,
        }
    }
}

impl Payload {
    #[inline]
    fn len(&self) -> u8 {
        if self.data_identifiers.is_some() {
            size_of::<u32>() as u8 + self.data_len + 1
        } else {
            self.data_len + 1
        }
    }
}

impl DLT645_2007Raw {
    pub fn as_ref(&self) -> &[u8] {
        &self.container[..self.len as usize]
    }
}

impl DLT645_2007 {
    fn new(addr: [u8; 6], code: Code, payload: Option<Payload>) -> DLT645_2007 {
        let (payload_len, payload) = if let Some(p) = payload {
            (p.len(), Some(p))
        } else {
            (0, None)
        };
        DLT645_2007 {
            addr,
            code: code.val(),
            payload_len,
            payload,
            // checksum: UNINIT_ZERO,
        }
    }

    fn checksum(raw: &[u8]) -> u8 {
        let mut checksum = 0;
        for val in raw {
            checksum = (checksum as usize + *val as usize) as u8;
        }
        checksum
    }

    // 转换成报文的裸字节形式，用于发送
    fn to_dlt645_2007raw(mut self) -> DLT645_2007Raw {
        // capacity = HEAD.len + ADDR.len + HEAD.len + CODE.len + DATA_LEN.len + DATA.len + CS.len + TAIL.len
        //  = 12 + DATA.len
        // if self.payload_len > 0 {
        //     self.payload_len -= 1;
        // }
        // let capacity = 12 + self.payload_len;
        let mut container = [0u8; 31];
        let mut i = 0;

        container[i] = 0x68;
        i += 1;
        // reverse (Big Endain)
        for j in (0..6).rev() {
            container[i] = self.addr[j];
            i += 1;
        }
        container[i] = 0x68;
        i += 1;
        container[i] = self.code;
        i += 1;

        if let Some(payload) = self.payload {
            let payload_len = payload.len() - 1;
            self.payload_len = payload_len + 1;
            container[i] = payload_len;
            i += 1;

            if let Some(di) = payload.data_identifiers {
                container[i..i + 4].clone_from_slice(&(di + 0x3333_3333_u32).to_le_bytes());
                i += 4;
            }
            // v.extend_from_slice(&payload.data);
            container[i..i + payload.data.len()].clone_from_slice(&payload.data);
            i += payload.data_len as usize;
        } else {
            // payload len = 0
            container[i] = 0;
            i += 1;
        }
        container[i] = DLT645_2007::checksum(&container[..i]);
        i += 1;
        container[i] = 0x16;
        i += 1;

        assert!(i <= container.len());

        // Debug Message
        // println!("[DebugInfo] DTL645 to raw = {:02X?}", v);
        // print_frame(&v);
        // todo!();

        DLT645_2007Raw {
            len: i as u8,
            container,
        }
    }

    // 从裸字节报文里解析信息成结构体
    fn parse_from_raw(raw: &[u8]) -> Result<DLT645_2007, Error> {
        // 合规的最少12字节。Master收到的至少13字节。
        if raw.len() < 12 || raw[0] != 0x68 || raw[7] != 0x68 {
            // println!("[DebugInfo] BadData {:02X?}", raw);
            return Err(MeterError::BadData);
        }

        // println!("[DebugInfo] raw.len = {}, {:02X?}", raw.len(), raw);
        // check 'C'
        // 算了。。没必要

        let code = raw[8];
        let mut payload_len = raw[9];

        // len before checksum
        let len = raw.len() - 2;

        let buf_ptr = slice_from_raw_parts(raw.as_ptr(), len);

        // Safe
        if Self::checksum(unsafe { &*buf_ptr }) != raw[raw.len() - 2] {
            // println!("[DebugInfo] CRC Err {:02X?}", raw);
            return Err(MeterError::ErrCRC);
        }

        if raw[raw.len() - 1] != 0x16 {
            // println!("[DebugInfo] BadData {:02X?}", raw);
            return Err(MeterError::BadData);
        }

        let payload = if payload_len == 0 {
            None
        } else {
            let mut data_len = payload_len;
            payload_len += 1;
            let data_identifiers;
            let mut data = [0u8; 11];
            if Code::ReadData.val() | 0x80 == code
                // || Code::WriteData.val() | 0x80 == code
                || Code::ReadSubsequentData.val() | 0x80 == code
            {
                data_identifiers = Some(
                    &u32::from_le_bytes(unsafe { *(raw[10..=13].as_ptr() as *mut [u8; 4]) })
                        - 0x3333_3333,
                );
                data_len -= 4;
                data[..data_len as usize].clone_from_slice(&raw[14..14 + data_len as usize]);
            } else {
                data_identifiers = None;
                data[..data_len as usize].clone_from_slice(&raw[10..10 + data_len as usize]);
            }
            Some(Payload {
                data_identifiers,
                data_len,
                data,
            })
        };

        let mut addr = [0u8; 6];
        addr.clone_from_slice(&raw[1..7]);
        addr.reverse();
        // for (i, j) in (1..7).rev().enumerate() {
        //     // println!("raw[{}]={:X}", j, raw[j]);
        //     addr[i] = raw[j];
        // }

        Ok(DLT645_2007 {
            addr,
            code,
            payload_len,
            payload,
            // checksum: raw[len],
        })
    }
}

impl From<&FunctionCode> for Code {
    fn from(value: &FunctionCode) -> Self {
        match value {
            FunctionCode::MasterQuerySlaveAddr => Self::ReadAddr,
            FunctionCode::MasterSetSlaveAddr(_) => Self::WriteAddr,
            FunctionCode::MasterChangeSlaveBaudRate(_) => Self::ChangeBaudrate,
            FunctionCode::M查询A路有功总电量IM1281X => Self::ReadData,
            FunctionCode::M查询B路有功总电量IM1281X => Self::ReadData,
            FunctionCode::M查询温度 => Self::ReadData,
            FunctionCode::M查询A路电流 => Self::ReadData,
            FunctionCode::MResetMeter => Self::ResetMeter,
        }
    }
}

// for Master to Parse the recved result
impl From<DLT645_2007> for Result<MeterResult, Error> {
    fn from(value: DLT645_2007) -> Self {
        let code = value.code;
        if code & Code::SlaveFlag.val() == 0 {
            Err(MeterError::NotSlaveMessage)
        } else if code & Code::SlaveACKErr.val() != 0 {
            Err(MeterError::SlaveAckErr)
        } else if code & Code::SubsequentDataFollowUp.val() != 0 {
            // unimplemented!()
            Err(MeterError::UnsupportedYetSubsequentDataFollowUpFlag)
        } else {
            // println!("Code {:X}", code);
            match (code & !Code::SlaveFlag.val()).into() {
                Code::Reserved => Err(MeterError::ReservedCode),
                Code::BroadcastTime => Err(MeterError::UnsupportedYetBroadcastTime),
                Code::ReadData => {
                    if let Some(p) = value.payload {
                        p.into()
                    } else {
                        Err(MeterError::UnExpectedNonePayload)
                    }
                }
                Code::ReadSubsequentData => Err(MeterError::UnsupportedYetReadSubsequentData),
                Code::ReadAddr => {
                    if let Some(p) = value.payload {
                        p.into()
                    } else {
                        Err(MeterError::UnExpectedNoneAddr)
                    }
                }
                Code::WriteData => Err(MeterError::UnsupportedYetWriteData),
                Code::WriteAddr => {
                    if let Some(p) = value.payload {
                        Err(MeterError::UnExpectedPayload)
                    } else {
                        Ok(MeterResult::SetMeterAddrSuccess(value.addr))
                    }
                }
                Code::FreezeCmd => Err(MeterError::UnsupportedFreezeCmd),
                Code::ChangeBaudrate => {
                    if let Some(p) = value.payload {
                        match p.data[0] - 0x33 {
                            0x04 => Ok(MeterResult::BaudRate(BaudRate::Baud1200)),
                            0x08 => Ok(MeterResult::BaudRate(BaudRate::Baud2400)),
                            0x10 => Ok(MeterResult::BaudRate(BaudRate::Baud4800)),
                            0x20 => Ok(MeterResult::BaudRate(BaudRate::Baud9600)),
                            _ => Err(MeterError::UnknownBaudrate),
                        }
                    } else {
                        Err(MeterError::UnExpectedNonePayload)
                    }
                }
                Code::ChangPasswd => Err(MeterError::UnsupportedYetChangePasswd),
                Code::ResetMaxDemand => Err(MeterError::UnsupportedYetResetMaxDemand),
                Code::ResetMeter => Ok(MeterResult::ResetMeterSuccess(value.addr)),
                Code::ResetEvent => Err(MeterError::UnsupportedYetResetEvent),
                _ => Err(MeterError::UnknownErr),
            }
        }
    }
}

impl From<Payload> for Result<MeterResult, MeterError> {
    fn from(payload: Payload) -> Self {
        if let Some(di) = payload.data_identifiers {
            match di {
                DI_IM1281X_A路有功总电量 => {
                    Ok(MeterResult::A路有功总电量_单位0_01KWh(
                        // safe
                        u32::from_le_bytes(unsafe { *(payload.data.as_ptr() as *mut [u8; 4]) })
                            - 0x3333_3333,
                    ))
                }

                DI_IM1281X_B路有功总电量 => {
                    Ok(MeterResult::B路有功总电量_单位0_01KWh(
                        // safe
                        u32::from_le_bytes(unsafe { *(payload.data.as_ptr() as *mut [u8; 4]) })
                            - 0x3333_3333,
                    ))
                }

                DI_DTL645通用_A路电压 => Ok(MeterResult::A路电压单位0_1V(
                    // safe
                    u16::from_le_bytes(unsafe { *(payload.data.as_ptr() as *mut [u8; 2]) })
                        - 0x3333,
                )),

                //
                DI_DTL645通用_A路电流 => Ok(MeterResult::A路电流mA(
                    payload.data[..3]
                        .into_iter()
                        // .rev()
                        .enumerate()
                        .fold(0, |acc, i| acc + (((i.1 - 0x33) as u32) << i.0 * 8)),
                )),

                DI_DTL645通用_温度 => Ok(MeterResult::温度_单位0_1C(
                    // safe
                    // u16::from_be_bytes(unsafe { *(payload.data.as_ptr() as *mut [u8; 2]) }) - 0x3333,
                    unsafe { *(payload.data.as_ptr() as *mut u16) } - 0x3333,
                    // u16::from(payload.data[0])
                )),

                DI_IM1281X_波特率 => {
                    Ok(MeterResult::BaudRate(match payload.data.first().unwrap() {
                        // safe to unwrap
                        0x04 => BaudRate::Baud1200,
                        0x08 => BaudRate::Baud2400,
                        0x10 => BaudRate::Baud4800,
                        0x20 => BaudRate::Baud9600,
                        _ => return Err(MeterError::UnknownBaudrate),
                    }))
                }

                _ => Err(MeterError::UnsupportedYetDI(
                    payload.data_identifiers.unwrap(),
                )),
            }
        } else {
            // DI is None
            // This version, it's returned Meter's 6 Bytes address

            // println!("Payload.data {:02X?}", payload.data);
            // let mut addr: [u8; 6] = unsafe { *(payload.data.as_ptr() as *mut [u8; 6]) };
            // addr.reverse();
            // for ptr in addr.iter_mut() {
            //     *ptr -= 0x33;
            // }
            let mut addr = [0u8; 6];
            // addr.clone_from_slice(&payload.data[..6]);
            for (i, j) in payload.data[..6].iter().rev().enumerate() {
                addr[i] = (*j as usize - 0x33) as u8;
            }
            // data 6 bytes
            Ok(MeterResult::MeterAddr(addr))
        }
    }
}

impl From<&FunctionCode> for Option<Payload> {
    fn from(value: &FunctionCode) -> Self {
        match value {
            FunctionCode::MasterQuerySlaveAddr => Self::None,
            // FunctionCode::MasterQuerySlaveAddr => Self::Some(PayloadData {
            //     data_identifiers: Some(DI_DTL645通用_地址),
            //     data: vec![],
            // }),
            FunctionCode::MasterSetSlaveAddr(addr) => Self::Some(Payload {
                // data_identifiers: Some(DI_DTL645通用_地址),
                data_identifiers: None,
                data_len: 6,
                data: {
                    let mut data = [0u8; 11];
                    let mut addr = addr.clone();
                    addr.reverse();
                    for x in &mut addr {
                        *x += 0x33;
                    }
                    data[..6].clone_from_slice(&addr);
                    data
                },
            }),
            FunctionCode::MasterChangeSlaveBaudRate(baud) => {
                // let mut v = Vec::with_capacity(1);
                let mut data = [0u8; 11];
                match baud {
                    BaudRate::Baud1200 => data[0] = 0x04 + 0x33,
                    BaudRate::Baud2400 => data[0] = 0x08 + 0x33,
                    BaudRate::Baud4800 => data[0] = 0x10 + 0x33,
                    BaudRate::Baud9600 => data[0] = 0x20 + 0x33,
                };
                Self::Some(Payload {
                    // data_identifiers: Some(DI_IM1281X_波特率),
                    data_identifiers: None,
                    data_len: 1,
                    data,
                })
            }
            FunctionCode::M查询A路有功总电量IM1281X => Self::Some(Payload {
                data_identifiers: Some(DI_IM1281X_A路有功总电量),
                data_len: 0,
                data: [0u8; 11],
            }),
            FunctionCode::M查询B路有功总电量IM1281X => Self::Some(Payload {
                data_identifiers: Some(DI_IM1281X_B路有功总电量),
                data_len: 0,
                data: [0u8; 11],
            }),
            FunctionCode::M查询温度 => Self::Some(Payload {
                data_identifiers: Some(DI_DTL645通用_温度),
                data_len: 0,
                data: [0u8; 11],
            }),
            FunctionCode::M查询A路电流 => Self::Some(Payload {
                data_identifiers: Some(DI_DTL645通用_A路电流),
                data_len: 0,
                data: [0u8; 11],
            }),
            FunctionCode::MResetMeter => Self::Some(Payload {
                data_identifiers: None,
                data_len: 8,
                data: [0x33, 0x33, 0x33, 0x33, 0x33, 0x33, 0x33, 0x33, 0, 0, 0],
            }),
        }
    }
}

impl From<MeterIOError> for MeterError {
    fn from(value: MeterIOError) -> Self {
        Self::IoErr(value)
    }
}

pub fn __bytes_should_recv(fc: FunctionCode) -> usize {
    // 0x68 + addr + 0x68 + Code + DataLen + Data + CS + 0x16
    // 1 + 6 + 1 + 1 + 1 + DataLen + 1 + 1 = 12 + DataLen
    // DataLen 参考计量模块技术手册
    match fc {
        FunctionCode::MasterQuerySlaveAddr => 12 + 6,
        FunctionCode::MasterSetSlaveAddr(_) => 12,
        FunctionCode::MasterChangeSlaveBaudRate(_) => 12 + 1,
        FunctionCode::M查询A路有功总电量IM1281X => 12 + 4 + 4,
        FunctionCode::M查询B路有功总电量IM1281X => 12 + 4 + 4,
        FunctionCode::M查询温度 => 12 + 4 + 2,
        FunctionCode::M查询A路电流 => 12 + 4 + 3,
        FunctionCode::MResetMeter => 12,
    }
}

/// Parse Meter Result from a raw DLT645 frame
///
/// prepared if you don't want to use 'generic_function()'
///
/// Simply use it directly
///
#[inline]
// #[allow(unused)]
pub fn parse_result_from_raw_frame(buf: &[u8]) -> Result<MeterResult, MeterError> {
    // let t = DLT645_2007::parse_from_raw(&buf)?;
    // if let Some(data) = t.data {
    //     Ok(data.into())
    // } else {
    //     Err(MeterError::ParseResultFailed)
    // }
    DLT645_2007::parse_from_raw(&buf)?.into()
}

/// Generate raw DTL645 frame, prepared if you don't want to use 'generic_function()'
///
/// Simply use it directly
///
pub fn generate_raw_frame(addr: Option<[u8; 6]>, fc: &FunctionCode) -> DLT645_2007Raw {
    let addr = match fc {
        &FunctionCode::MasterQuerySlaveAddr => [0xAA, 0xAA, 0xAA, 0xAA, 0xAA, 0xAA],
        // &FunctionCode::MasterChangeSlaveBaudRate(_) => [0xAA, 0xAA, 0xAA, 0xAA, 0xAA, 0xAA],
        &FunctionCode::MasterSetSlaveAddr(_) => [0xAA, 0xAA, 0xAA, 0xAA, 0xAA, 0xAA],
        _ => {
            if let Some(addr) = addr {
                addr
            } else {
                [0x11, 0x11, 0x11, 0x11, 0x11, 0x11]
            }
        }
    };
    DLT645_2007::new(addr, fc.into(), fc.into()).to_dlt645_2007raw()
}

/// Generic Function ! Enough !
///
/// It's simple to use it directly without reading Doc.
///
pub fn generic_function(
    addr: Option<[u8; 6]>,
    fc: FunctionCode,
    io: &mut dyn Io,
) -> Result<MeterResult, MeterError> {
    // send the raw frame
    io.write(&generate_raw_frame(addr, &fc).as_ref())?;

    // 目前最大: 20
    let len = __bytes_should_recv(fc);
    let mut buf = [0u8; 20];

    // recv 'len' bytes
    io.recv_exact(&mut buf[..len])?;

    // thanks to __bytes_should_recv(), safe to unwrap() here
    parse_result_from_raw_frame(&buf[..len])
}

pub trait Io {
    fn write(&mut self, buf: &[u8]) -> Result<usize, MeterIOError>;
    fn recv_exact(&mut self, buf: &mut [u8]) -> Result<(), MeterIOError>;
}
