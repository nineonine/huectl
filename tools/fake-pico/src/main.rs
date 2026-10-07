//! Fake huectl controller: a virtual HID device via /dev/uhid.
//!
//! Stands in for the Pico so the kernel driver and daemon can be built
//! before the hardware exists. Above the transport the kernel treats it
//! like a USB device: same descriptor parsing, same reports.
//!
//! Run in the VM as root (/dev/uhid is root-only). Commands on stdin:
//!
//!   press N | release N   button N (1-6) down/up; 1-3 = knob pushes
//!   turn K D              knob K (1-3) moved D detents (-127..127)
//!   slide V               slide pot at V (0..1023)
//!   quit                  (or end of input)
//!
//! Kernel events (driver bound, device opened, LED and display output
//! reports, decoded) are printed to stderr as they happen.

mod descriptor;
mod uhid;

use std::io::{self, BufRead};
use std::process::ExitCode;
use std::thread;

use descriptor::{Controls, REPORT_DESCRIPTOR, describe_output};
use uhid::{Device, DeviceInfo, Event};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("fake-pico: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> io::Result<()> {
    let mut dev = Device::create(&DeviceInfo {
        name: "huectl fake controller",
        vendor: 0x1209,  // pid.codes
        product: 0x0001, // pid.codes test PID (CLAUDE.md decision 6)
        descriptor: REPORT_DESCRIPTOR,
    })?;
    eprintln!("fake-pico: created 1209:0001; type commands (press/release/turn/slide/quit)");

    let mut events = dev.events()?;
    thread::spawn(move || {
        loop {
            match events.next() {
                Ok(Event::Output { data, .. }) => {
                    eprintln!("kernel -> device: {}", describe_output(&data))
                }
                Ok(ev) => eprintln!("kernel -> device: {ev:?}"),
                Err(e) => {
                    eprintln!("fake-pico: reading events: {e}");
                    break;
                }
            }
        }
    });

    let mut controls = Controls::default();
    for line in io::stdin().lock().lines() {
        let line = line?;
        match apply(&mut controls, &line) {
            Ok(Some(())) => {}
            Ok(None) => break, // quit
            Err(msg) => {
                eprintln!("fake-pico: {msg}");
                continue;
            }
        }
        let report = controls.report();
        dev.send_input(&report)?;
        eprintln!("device -> kernel: {report:02x?}");
        controls.knobs = [0; 3]; // relative: each turn is reported exactly once
    }
    Ok(())
}

/// Update `controls` from one command. `Ok(None)` means quit.
fn apply(controls: &mut Controls, line: &str) -> Result<Option<()>, String> {
    let words: Vec<&str> = line.split_whitespace().collect();
    match words.as_slice() {
        ["quit"] => return Ok(None),
        ["press", n] => controls.buttons |= button_bit(n)?,
        ["release", n] => controls.buttons &= !button_bit(n)?,
        ["turn", k, d] => {
            controls.knobs[parse_in(k, 1, 3)? as usize - 1] = parse_in(d, -127, 127)? as i8
        }
        ["slide", v] => controls.slider = parse_in(v, 0, 1023)? as u16,
        _ => return Err(format!("unknown command: {line:?}")),
    }
    Ok(Some(()))
}

fn button_bit(n: &str) -> Result<u8, String> {
    Ok(1 << (parse_in(n, 1, 6)? - 1))
}

fn parse_in(s: &str, min: i32, max: i32) -> Result<i32, String> {
    match s.parse::<i32>() {
        Ok(v) if (min..=max).contains(&v) => Ok(v),
        _ => Err(format!("expected a number in {min}..={max}, got {s:?}")),
    }
}
