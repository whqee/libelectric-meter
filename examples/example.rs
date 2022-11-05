use std::io::{Read, Write};

use electric_meter::{MeterIO, MeterIOError};
use uart_linux::Uart;

fn main() {
    let path = "/dev/ttyUSB1";
    let mut wo = Uart::new_default(path, uart_linux::Permission::RW);

    wo.baudrate = uart_linux::BaudRate::Baud9600;
    wo.apply_settings();

    let mut ro = Uart::new_default(path, uart_linux::Permission::RW);
    // ro.timeout_s = 1;
    ro.timeout_20us = 6000; // 120ms

    let send = move |buf: &[u8]| -> Result<usize, MeterIOError> {
        match wo.write(buf) {
            Ok(sent) => {
                if sent < buf.len() {
                    Err(MeterIOError::IncompleteWrite)
                } else {
                    Ok(sent)
                }
            }
            Err(e) => Err(MeterIOError::STD(e.kind().to_string())),
        }
    };

    let recv_exact = move |buf: &mut [u8]| -> Result<(), MeterIOError> {
        match ro.read_exact(buf) {
            Ok(_) => Ok(()),
            Err(e) => {
                println!("[ErrInfo]: {:?}\n recved = {:X?}", e, buf);
                Err(MeterIOError::TimeOutReadExactBytes)
            }
        }
    };

    let mut io_ops = MeterIO::new(Box::new(send), Box::new(recv_exact));

    println!(
        "Test Result: {:X?}",
        electric_meter::generic_function(
            None,
            electric_meter::FunctionCode::MasterSetSlaveAddr([0x00, 0x00, 0x00, 0x00, 0x00, 0x1]),
            &mut io_ops,
        )
    );

    println!(
        "Test Result: {:X?}",
        electric_meter::generic_function(
            None,
            electric_meter::FunctionCode::MasterQuerySlaveAddr,
            &mut io_ops,
        )
    );

    println!(
        "Test Result: {:?}",
        electric_meter::generic_function(
            Some([0x00, 0x00, 0x00, 0x00, 0x00, 0x1]),
            electric_meter::FunctionCode::M查询A路有功总电量IM1281X,
            &mut io_ops,
        )
    );

    println!(
        "Test Result: {:?}",
        electric_meter::generic_function(
            Some([0x00, 0x00, 0x00, 0x00, 0x00, 0x1]),
            electric_meter::FunctionCode::M查询B路有功总电量IM1281X,
            &mut io_ops,
        )
    );

    println!(
        "Test Result: {:04X?}",
        electric_meter::generic_function(
            Some([0x00, 0x00, 0x00, 0x00, 0x00, 0x1]),
            electric_meter::FunctionCode::M查询A路电流,
            &mut io_ops,
        )
    );

    println!(
        "Test Result: {:04X?}",
        electric_meter::generic_function(
            Some([0x00, 0x00, 0x00, 0x00, 0x00, 0x1]),
            electric_meter::FunctionCode::M查询温度,
            &mut io_ops,
        )
    );

    println!(
        "Test Result: {:X?}",
        electric_meter::generic_function(
            Some([0x00, 0x00, 0x00, 0x00, 0x00, 0x1]),
            electric_meter::FunctionCode::MasterChangeSlaveBaudRate(
                electric_meter::BaudRate::Baud9600
            ),
            &mut io_ops,
        )
    );
    println!(
        "Test Result: {:X?}",
        electric_meter::generic_function(
            Some([0x00, 0x00, 0x00, 0x00, 0x00, 0x1]),
            electric_meter::FunctionCode::MResetMeter,
            &mut io_ops,
        )
    );
}
