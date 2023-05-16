use std::io::{Read, Write};

use electric_meter::MeterIOError;
use uart_linux::Uart;

fn main() {
    let path = "/dev/ttyUSB0";
    let mut uart: Uart = Uart::new_default_locked(path, uart_linux::Permission::RW);

    uart.baudrate = uart_linux::BaudRate::Baud9600;
    uart.timeout_20us = 6000; // 120ms
    // uart.timeout_s = 1;
    uart.apply_settings();

    struct UartIo {
        uart: uart_linux::Uart,
    }

    impl electric_meter::Io for UartIo {
        fn write(&mut self, buf: &[u8]) -> Result<usize, MeterIOError> {
            std::thread::sleep(std::time::Duration::from_millis(100));
            match self.uart.write(buf) {
                Ok(sent) => {
                    if sent < buf.len() {
                        Err(MeterIOError::IncompleteWrite)
                    } else {
                        Ok(sent)
                    }
                }
                Err(e) => Err(MeterIOError::Std(e.kind().to_string())),
            }
        }

        fn recv_exact(&mut self, buf: &mut [u8]) -> Result<(), MeterIOError> {
            std::thread::sleep(std::time::Duration::from_millis(50));
            match self.uart.read_exact(buf) {
                Ok(_) => Ok(()),
                Err(e) => {
                    println!("[ErrInfo]: {:?} \\\n   recved = {:X?}", e, buf);
                    Err(MeterIOError::TimeOutReadExactBytes)
                }
            }
        }
    }

    let mut io_ops = UartIo { uart };

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
