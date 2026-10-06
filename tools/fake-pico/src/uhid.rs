//! Minimal /dev/uhid client (see include/uapi/linux/uhid.h).
//!
//! Every message in either direction is one `struct uhid_event`: a u32
//! type followed by a union of request structs. All structs are packed
//! (no padding), so field offsets are just running sums of field sizes.
//! We build and parse the bytes by hand to keep the wire format visible.
//! Integers are little-endian (x86 VM).

use std::fs::{File, OpenOptions};
use std::io::{self, Read, Write};

// enum uhid_event_type (gaps are legacy events we never use).
const DESTROY: u32 = 1;
const START: u32 = 2;
const STOP: u32 = 3;
const OPEN: u32 = 4;
const CLOSE: u32 = 5;
const OUTPUT: u32 = 6;
const GET_REPORT: u32 = 9;
const GET_REPORT_REPLY: u32 = 10;
const CREATE2: u32 = 11;
const INPUT2: u32 = 12;
const SET_REPORT: u32 = 13;
const SET_REPORT_REPLY: u32 = 14;

/// UHID_DATA_MAX and HID_MAX_DESCRIPTOR_SIZE.
const DATA_MAX: usize = 4096;
/// sizeof(struct uhid_event): u32 type + the largest union member,
/// uhid_create2_req = 128 + 64 + 64 + 2 + 2 + 4*4 + 4096 = 4372 bytes.
const EVENT_SIZE: usize = 4 + 4372;
/// Payload starts right after the u32 type.
const U: usize = 4;

const BUS_USB: u16 = 0x03;
const EIO: u16 = 5;

pub struct DeviceInfo<'a> {
    pub name: &'a str,
    pub vendor: u32,
    pub product: u32,
    pub descriptor: &'a [u8],
}

/// Something the kernel told us. Debug output is what the user sees.
// Fields are only read by printing them, which dead-code analysis ignores.
#[allow(dead_code)]
#[derive(Debug)]
pub enum Event {
    /// A HID driver bound to the device.
    Start,
    Stop,
    /// Someone opened the input device (e.g. evtest); reports matter now.
    Open,
    Close,
    /// Host -> device report (our LED report ends up here).
    Output { report_type: u8, data: Vec<u8> },
    /// Kernel asked to read a report; we answer with an error.
    GetReport { id: u32, report_num: u8 },
    /// Kernel asked to write a report; we answer with an error.
    SetReport { id: u32, report_num: u8, data: Vec<u8> },
    Unknown(u32),
}

/// A virtual HID device. Dropping it closes /dev/uhid, which destroys it.
pub struct Device {
    file: File,
}

impl Device {
    pub fn create(info: &DeviceInfo) -> io::Result<Self> {
        if info.descriptor.len() > DATA_MAX {
            return Err(io::Error::other("report descriptor too long"));
        }
        let file = OpenOptions::new().read(true).write(true).open("/dev/uhid")?;

        // struct uhid_create2_req, at offset U:
        //   name[128] phys[64] uniq[64] rd_size:u16 bus:u16
        //   vendor:u32 product:u32 version:u32 country:u32 rd_data[4096]
        let mut ev = event(CREATE2);
        let name = info.name.as_bytes();
        let name = &name[..name.len().min(127)]; // keep a NUL terminator
        ev[U..U + name.len()].copy_from_slice(name);
        put_u16(&mut ev, U + 256, info.descriptor.len() as u16);
        put_u16(&mut ev, U + 258, BUS_USB);
        put_u32(&mut ev, U + 260, info.vendor);
        put_u32(&mut ev, U + 264, info.product);
        // version (U+268) and country (U+272) stay 0.
        ev[U + 276..U + 276 + info.descriptor.len()].copy_from_slice(info.descriptor);

        let mut dev = Self { file };
        dev.write_event(&ev)?;
        Ok(dev)
    }

    /// Send an input report (device -> host), report ID included.
    pub fn send_input(&mut self, report: &[u8]) -> io::Result<()> {
        // struct uhid_input2_req: size:u16 data[4096]
        let mut ev = event(INPUT2);
        put_u16(&mut ev, U, report.len() as u16);
        ev[U + 2..U + 2 + report.len()].copy_from_slice(report);
        self.write_event(&ev)
    }

    /// A second handle for reading kernel events on another thread.
    pub fn events(&self) -> io::Result<Events> {
        Ok(Events { file: self.file.try_clone()? })
    }

    fn write_event(&mut self, ev: &[u8]) -> io::Result<()> {
        self.file.write_all(ev)
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        // Closing the fd would do this too; being explicit documents it.
        let _ = self.write_event(&event(DESTROY));
    }
}

pub struct Events {
    file: File,
}

impl Events {
    /// Block until the kernel sends the next event. Report requests are
    /// answered with EIO so the kernel doesn't sit waiting for a timeout.
    pub fn next(&mut self) -> io::Result<Event> {
        let mut ev = vec![0u8; EVENT_SIZE];
        let n = self.file.read(&mut ev)?;
        if n < 4 {
            return Err(io::Error::other(format!("short read from /dev/uhid: {n} bytes")));
        }
        Ok(match get_u32(&ev, 0) {
            START => Event::Start,
            STOP => Event::Stop,
            OPEN => Event::Open,
            CLOSE => Event::Close,
            OUTPUT => {
                // struct uhid_output_req: data[4096] size:u16 rtype:u8
                let size = get_u16(&ev, U + DATA_MAX) as usize;
                let report_type = ev[U + DATA_MAX + 2];
                Event::Output { report_type, data: ev[U..U + size.min(DATA_MAX)].to_vec() }
            }
            GET_REPORT => {
                // struct uhid_get_report_req: id:u32 rnum:u8 rtype:u8
                let id = get_u32(&ev, U);
                self.reply(GET_REPORT_REPLY, id)?;
                Event::GetReport { id, report_num: ev[U + 4] }
            }
            SET_REPORT => {
                // struct uhid_set_report_req: id:u32 rnum:u8 rtype:u8 size:u16 data[4096]
                let id = get_u32(&ev, U);
                let size = get_u16(&ev, U + 6) as usize;
                self.reply(SET_REPORT_REPLY, id)?;
                Event::SetReport {
                    id,
                    report_num: ev[U + 4],
                    data: ev[U + 8..U + 8 + size.min(DATA_MAX)].to_vec(),
                }
            }
            other => Event::Unknown(other),
        })
    }

    /// Both reply structs start with id:u32 err:u16.
    fn reply(&mut self, kind: u32, id: u32) -> io::Result<()> {
        let mut ev = event(kind);
        put_u32(&mut ev, U, id);
        put_u16(&mut ev, U + 4, EIO);
        self.file.write_all(&ev)
    }
}

fn event(kind: u32) -> Vec<u8> {
    let mut ev = vec![0u8; EVENT_SIZE];
    put_u32(&mut ev, 0, kind);
    ev
}

fn put_u16(buf: &mut [u8], at: usize, v: u16) {
    buf[at..at + 2].copy_from_slice(&v.to_le_bytes());
}

fn put_u32(buf: &mut [u8], at: usize, v: u32) {
    buf[at..at + 4].copy_from_slice(&v.to_le_bytes());
}

fn get_u16(buf: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([buf[at], buf[at + 1]])
}

fn get_u32(buf: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(buf[at..at + 4].try_into().unwrap())
}
