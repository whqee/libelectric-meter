#![no_std]
extern crate alloc;
use alloc::boxed::Box;
use alloc::string::String;
use alloc::string::ToString;
use alloc::vec;
use alloc::vec::Vec;
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
    // UnsupportedDataIdentity,
    ParseResultFailed,
    // SlaveAckErr(u8),
    IoErr(MeterIOError),
    NotSlaveMessage,
    SlaveAckErr,
    // 呃这个命名。。。可能永远不用改这个库了，不改了，就先这样。。。
    UnsupportedYet(String),
    Unsupported(String),
    UnknownErr(String),
    ParseAddrFromStringErr,
    ParseDiErr(String),
}

#[derive(Debug, PartialEq, Eq)]
pub enum MeterIOError {
    TimeOutReadExactBytes,
    // DeviceOrResourceBusy,
    // InputOutputError,
    // NoSuchDevice,
    IncompleteWrite,
    // ReadTimeout,
    STD(String),
}

// pub enum MeterIO {
//     Closure(MeterIoClosure),
//     Fn(MeterIoFn),
// }

// pub struct MeterAddr {
//     raw: [u8; 6],
// }

/// See examples/example.rs
pub struct MeterIO {
    /// Function. return sent bytes. This should support concurrency.
    send: Box<dyn FnMut(&[u8]) -> Result<usize, MeterIOError>>,

    /// Function. read exact! bytes
    recv_exact: Box<dyn FnMut(&mut [u8]) -> Result<(), MeterIOError>>,
    // /// Function. change master's baudrate
    // change_baud: Box<dyn FnMut(BaudRate) -> Result<(), MeterIOError>>,
}

// pub struct MeterIO {
//     /// Function. return recieved bytes. This should support concurrency.
//     pub recv_exact_bytes: fn(&mut [u8]) -> Result<(), MeterIOError>,

//     /// Function. return sent bytes. This should support concurrency.
//     pub send_bytes: fn(&[u8]) -> Result<usize, MeterIOError>,
// }

// #[repr(C)]
#[derive(Debug, Clone)]
struct DLT645_2007 {
    addr: [u8; 6],
    code: u8,
    data_len: u8,
    data: Option<PayloadData>,
    // checksum: u8,
}

#[derive(Debug, Clone)]
struct PayloadData {
    data_identifiers: Option<u32>,
    data: Vec<u8>,
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

impl PayloadData {
    #[inline]
    fn len(&self) -> u8 {
        if self.data_identifiers.is_some() {
            (size_of::<u32>() + self.data.len()) as u8
        } else {
            self.data.len() as u8
        }
    }
}

impl DLT645_2007 {
    fn new(addr: [u8; 6], code: Code, data: Option<PayloadData>) -> DLT645_2007 {
        DLT645_2007 {
            addr,
            code: code.val(),
            data_len: UNINIT_ZERO,
            data,
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
    fn to_raw(mut self) -> Vec<u8> {
        // capacity = HEAD.len + ADDR.len + HEAD.len + CODE.len + DATA_LEN.len + DATA.len + CS.len + TAIL.len
        //  = 12 + DATA.len
        let capacity = 12 + self.data_len;
        let mut v = Vec::with_capacity(capacity.into());

        v.push(0x68);
        // reverse (Big Endain)
        for i in (0..6).rev() {
            v.push(self.addr[i])
        }
        v.push(0x68);
        v.push(self.code);

        if let Some(payload_data) = self.data {
            self.data_len = payload_data.len();
            v.push(self.data_len);

            if let Some(di) = payload_data.data_identifiers {
                v.extend_from_slice(&(di + 0x3333_3333_u32).to_le_bytes());
            }
            v.extend_from_slice(&payload_data.data);
        } else {
            v.push(self.data_len);
        }

        v.push(DLT645_2007::checksum(&v));
        v.push(0x16);

        // Debug Message
        // println!("[DebugInfo] DTL645 to raw = {:02X?}", v);
        // print_frame(&v);
        // todo!();

        v
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

        let data_len = raw[9] as usize;

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

        let data = if data_len == 0 {
            None
        } else {
            let data_identifiers;
            let data;
            if Code::ReadData.val() | 0x80 == raw[8]
                // || Code::WriteData.val() | 0x80 == raw[8]
                || Code::ReadSubsequentData.val() | 0x80 == raw[8]
            {
                data_identifiers = Some(
                    &u32::from_le_bytes(unsafe { *(raw[10..=13].as_ptr() as *mut [u8; 4]) })
                        - 0x3333_3333,
                );
                data =
                    unsafe { &*slice_from_raw_parts(raw.as_ptr().add(14), data_len - 4) }.to_vec()
            } else {
                data_identifiers = None;
                data = unsafe { &*slice_from_raw_parts(raw.as_ptr().add(10), data_len) }.to_vec()
            }
            Some(PayloadData {
                data_identifiers,
                data,
            })
        };

        let mut addr = [0u8; 6];
        for (i, j) in (1..7).rev().enumerate() {
            // println!("raw[{}]={:X}", j, raw[j]);
            addr[i] = raw[j];
        }

        Ok(DLT645_2007 {
            addr,
            code: raw[8],
            data_len: data_len as u8,
            data,
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
        if value.code & Code::SlaveFlag.val() == 0 {
            Err(MeterError::NotSlaveMessage)
        } else if value.code & Code::SlaveACKErr.val() != 0 {
            Err(MeterError::SlaveAckErr)
        } else if value.code & Code::SubsequentDataFollowUp.val() != 0 {
            // unimplemented!()
            Err(MeterError::UnsupportedYet(
                "SubsequentDataFollowUp".to_string(),
            ))
        } else {
            // println!("Code {:X}", value.code);
            match (value.code & !Code::SlaveFlag.val()).into() {
                Code::Reserved => Err(MeterError::UnsupportedYet("Reserved".to_string())),
                Code::BroadcastTime => Err(MeterError::UnsupportedYet("BroadcastTime".to_string())),
                Code::ReadData => {
                    if value.data.is_some() {
                        value.data.unwrap().into()
                    } else {
                        Err(MeterError::UnknownErr("ReadData, but data is none".into()))
                    }
                }
                Code::ReadSubsequentData => {
                    Err(MeterError::UnsupportedYet("ReadSubsequentData".to_string()))
                }
                Code::ReadAddr => {
                    if value.data.is_some() {
                        value.data.unwrap().into()
                    } else {
                        Err(MeterError::UnknownErr("ReadAddr, but data is none".into()))
                    }
                }
                Code::WriteData => Err(MeterError::UnsupportedYet("to do".to_string())),
                Code::WriteAddr => {
                    if value.data.is_none() {
                        Ok(MeterResult::SetMeterAddrSuccess(value.addr))
                    } else {
                        Err(MeterError::UnknownErr("WriteAddr, but data is_some".into()))
                    }
                }
                Code::FreezeCmd => Err(MeterError::UnsupportedYet("FreezeCmd".to_string())),
                Code::ChangeBaudrate => {
                    if value.data.is_some() {
                        match value.data.unwrap().data[0] - 0x33 {
                            0x04 => Ok(MeterResult::BaudRate(BaudRate::Baud1200)),
                            0x08 => Ok(MeterResult::BaudRate(BaudRate::Baud2400)),
                            0x10 => Ok(MeterResult::BaudRate(BaudRate::Baud4800)),
                            0x20 => Ok(MeterResult::BaudRate(BaudRate::Baud9600)),
                            _ => Err(MeterError::UnknownErr("BaudRate".to_string())),
                        }
                    } else {
                        Err(MeterError::UnknownErr(
                            "ChangeBaudrate returned data is none ¿".to_string(),
                        ))
                    }
                }
                Code::ChangPasswd => Err(MeterError::UnsupportedYet("ChangPasswd".to_string())),
                Code::ResetMaxDemand => {
                    Err(MeterError::UnsupportedYet("ResetMaxDemand".to_string()))
                }
                Code::ResetMeter => Ok(MeterResult::ResetMeterSuccess(value.addr)),
                Code::ResetEvent => Err(MeterError::UnsupportedYet("ResetEvent".to_string())),
                _ => Err(MeterError::UnknownErr(
                    "UnknownErr Meter Code: ".to_string() + &value.code.to_string(),
                )),
            }
        }
    }
}

impl From<PayloadData> for Result<MeterResult, MeterError> {
    fn from(value: PayloadData) -> Self {
        match value.data_identifiers {
            Some(DI_IM1281X_A路有功总电量) => {
                Ok(MeterResult::A路有功总电量_单位0_01KWh(
                    // safe
                    u32::from_le_bytes(unsafe { *(value.data.as_ptr() as *mut [u8; 4]) })
                        - 0x3333_3333,
                ))
            }

            Some(DI_IM1281X_B路有功总电量) => {
                Ok(MeterResult::B路有功总电量_单位0_01KWh(
                    // safe
                    u32::from_le_bytes(unsafe { *(value.data.as_ptr() as *mut [u8; 4]) })
                        - 0x3333_3333,
                ))
            }

            Some(DI_DTL645通用_A路电压) => Ok(MeterResult::A路电压单位0_1V(
                // safe
                u16::from_le_bytes(unsafe { *(value.data.as_ptr() as *mut [u8; 2]) }) - 0x3333,
            )),

            // 还没测，睡觉先,这版本不用就先不测了
            Some(DI_DTL645通用_A路电流) => Ok(MeterResult::A路电流mA(
                value
                    .data
                    .into_iter()
                    // .rev()
                    .enumerate()
                    .fold(0, |acc, i| acc + (((i.1 - 0x33) as u32) << i.0 * 8)),
            )),

            Some(DI_DTL645通用_温度) => Ok(MeterResult::温度_单位0_1C(
                // safe
                // u16::from_be_bytes(unsafe { *(value.data.as_ptr() as *mut [u8; 2]) }) - 0x3333,
                unsafe { *(value.data.as_ptr() as *mut u16) } - 0x3333,
                // u16::from(value.data[0])
            )),

            Some(DI_IM1281X_波特率) => {
                Ok(MeterResult::BaudRate(match value.data.first().unwrap() {
                    // safe to unwrap
                    0x04 => BaudRate::Baud1200,
                    0x08 => BaudRate::Baud2400,
                    0x10 => BaudRate::Baud4800,
                    0x20 => BaudRate::Baud9600,
                    _ => return Err(MeterError::Unsupported("BaudRate".to_string())),
                }))
            }
            // This version, it's returned Meter's 6 Bytes address
            None => {
                // println!("Payload.data {:02X?}", value.data);
                // let mut addr: [u8; 6] = unsafe { *(value.data.as_ptr() as *mut [u8; 6]) };
                // addr.reverse();
                // for ptr in addr.iter_mut() {
                //     *ptr -= 0x33;
                // }
                let mut addr = [0u8; 6];
                for (i, j) in value.data.iter().rev().enumerate() {
                    addr[i] = (*j as usize - 0x33) as u8;
                }
                // data 6 bytes
                Ok(MeterResult::MeterAddr(addr))
            }
            _ => Err(MeterError::ParseDiErr(
                "Unsupported yet, DI(u32)=".to_string()
                    + &value.data_identifiers.unwrap().to_string(),
            )),
        }
    }
}

impl From<&FunctionCode> for Option<PayloadData> {
    fn from(value: &FunctionCode) -> Self {
        match value {
            FunctionCode::MasterQuerySlaveAddr => Self::None,
            // FunctionCode::MasterQuerySlaveAddr => Self::Some(PayloadData {
            //     data_identifiers: Some(DI_DTL645通用_地址),
            //     data: vec![],
            // }),
            FunctionCode::MasterSetSlaveAddr(addr) => Self::Some(PayloadData {
                // data_identifiers: Some(DI_DTL645通用_地址),
                data_identifiers: None,
                data: addr.iter().rev().map(|&x| x + 0x33).collect(),
            }),
            FunctionCode::MasterChangeSlaveBaudRate(baud) => {
                let mut v = Vec::with_capacity(1);
                match baud {
                    BaudRate::Baud1200 => v.push(0x04 + 0x33),
                    BaudRate::Baud2400 => v.push(0x08 + 0x33),
                    BaudRate::Baud4800 => v.push(0x10 + 0x33),
                    BaudRate::Baud9600 => v.push(0x20 + 0x33),
                };
                Self::Some(PayloadData {
                    // data_identifiers: Some(DI_IM1281X_波特率),
                    data_identifiers: None,
                    data: v,
                })
            }
            FunctionCode::M查询A路有功总电量IM1281X => Self::Some(PayloadData {
                data_identifiers: Some(DI_IM1281X_A路有功总电量),
                data: Vec::with_capacity(0),
            }),
            FunctionCode::M查询B路有功总电量IM1281X => Self::Some(PayloadData {
                data_identifiers: Some(DI_IM1281X_B路有功总电量),
                data: Vec::with_capacity(0),
            }),
            FunctionCode::M查询温度 => Self::Some(PayloadData {
                data_identifiers: Some(DI_DTL645通用_温度),
                data: Vec::with_capacity(0),
            }),
            FunctionCode::M查询A路电流 => Self::Some(PayloadData {
                data_identifiers: Some(DI_DTL645通用_A路电流),
                data: Vec::with_capacity(0),
            }),
            FunctionCode::MResetMeter => Self::Some(PayloadData {
                data_identifiers: None,
                data: vec![0x33, 0x33, 0x33, 0x33, 0x33, 0x33, 0x33, 0x33],
            }),
        }
    }
}

impl MeterIO {
    pub fn new(
        send: Box<dyn FnMut(&[u8]) -> Result<usize, MeterIOError>>,
        recv_exact: Box<dyn FnMut(&mut [u8]) -> Result<(), MeterIOError>>,
    ) -> Self {
        Self { recv_exact, send }
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
pub fn generate_raw_frame(addr: Option<[u8; 6]>, fc: &FunctionCode) -> Vec<u8> {
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
    // let addr = if fc == &FunctionCode::MasterQuerySlaveAddr {
    //     [0xAA, 0xAA, 0xAA, 0xAA, 0xAA, 0xAA]
    // } else if let Some(addr) = addr {
    //     addr
    // } else {
    //     [0x11, 0x11, 0x11, 0x11, 0x11, 0x11]
    // };
    DLT645_2007::new(addr, fc.into(), fc.into()).to_raw()
}

/// Generic Function ! Enough !
///
/// It's simple to use it directly without reading Doc.
///
pub fn generic_function(
    addr: Option<[u8; 6]>,
    fc: FunctionCode,
    io_ops: &mut MeterIO,
) -> Result<MeterResult, MeterError> {
    // send the raw frame
    (io_ops.send)(&generate_raw_frame(addr, &fc))?;

    let len = __bytes_should_recv(fc);

    let mut buf = Vec::with_capacity(len);
    unsafe { buf.set_len(len) };
    // buf[0] = 0; // uneeded
    // buf.fill(0); // 只要是read_exact, 这个就没必要

    // recv 'len' bytes
    (io_ops.recv_exact)(&mut buf)?;

    // thanks to __bytes_should_recv(), safe to unwrap() here
    parse_result_from_raw_frame(&buf)
}

// pub fn print_frame(frame: &Vec<u8>) {
//     print!("[DebugInfo] Debug print a raw frame:");
//     for i in frame {
//         print!("{:02X} ", i)
//     }
//     println!("");
// }
