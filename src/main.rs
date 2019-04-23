#![allow(unused_imports)]
extern crate CoreFoundation_sys;
extern crate IOKit_sys;
extern crate dmidecode;
extern crate mach;
#[macro_use] extern crate scopeguard;

use IOKit_sys::{
    IOMasterPort,
    IOServiceGetMatchingService,
    IOServiceMatching,
    IORegistryEntryCreateCFProperty,
    IOObjectRelease,
    io_service_t
};

use CoreFoundation_sys::{
    CFAllocatorGetDefault,
    CFDataGetLength,
    CFDataGetBytePtr,
    CFDataRef,
    CFMutableDictionaryRef,
    CFRelease,
    CFStringCreateWithCString,
    CFTypeRef,
    kCFAllocatorDefault,
    kCFStringEncodingASCII,
};

use mach::port::*;

use std::ffi::CString;

#[allow(non_upper_case_globals)]
const kNilOptions: u32 = 0;

fn main() {
    let master_port = get_master_port();
    let service = get_io_service(master_port);
    let buffer_entrypoint = get_smbios_eps_data(service);
    let buffer_entrypoint_slice = buffer_entrypoint.as_slice();
    let buffer_data = get_smbios_data(service);
    let buffer_data_slice = buffer_data.as_slice();

    let entry_point = dmidecode::EntryPoint::search(buffer_entrypoint_slice).unwrap();
    for s in entry_point.structures(&buffer_data_slice) {
        let table = s.unwrap();
        match table {
            dmidecode::Structure::System(
                dmidecode::system::System {
                    manufacturer,
                    serial,
                    version,
                    product,
                    uuid,
                    sku,
                    family,
                    .. }) => {
                println!("== SYSTEM ==");
                println!("manufacturer: {}", manufacturer);
                println!("serial: {}", serial);
                println!("version: {}", version);
                println!("product: {}", product);
                println!("sku: {:?}", sku);
                println!("uuid: {:?}", uuid);
                println!("family: {:?}", family);
                println!("\n");
            }
            dmidecode::Structure::BaseBoard(dmidecode::baseboard::BaseBoard {
                manufacturer,
                product,
                version,
                serial,
                asset,
                feature_flags,
                location_in_chassis,
                board_type,
                ..}) => {
                println!("== BASEBOARD ==");
                println!("manufacturer: {}", manufacturer);
                println!("product: {}", product);
                println!("version: {}", version);
                println!("serial: {}", serial);
                println!("asset: {}", asset);
                println!("feature_flags: {:?}", feature_flags);
                println!("location_in_chassis: {}", location_in_chassis);
                println!("board_type: {:?}", board_type);
                println!("\n");
            }
            dmidecode::Structure::Processor(
                dmidecode::processor::Processor {
                    processor_type,
                    processor_version,
                    serial_number,
                    asset_tag,
                    ..
                }) => {
                println!("== PROCESSOR ==");
                println!("processor_type: {:?}", processor_type);
                println!("processor_version: {}", processor_version);
                println!("serial_number: {:?}", serial_number);
                println!("asset_tag: {:?}", asset_tag);
                println!("\n");
            }
            _ => {}
        }
    }
}

fn get_master_port() -> mach_port_t {
    let mut master_port: mach_port_t = 0;
    let master_port_ptr = &mut master_port as *mut _;
    unsafe { IOMasterPort(mach::port::MACH_PORT_NULL, master_port_ptr) };
    if master_port == MACH_PORT_NULL {
        eprintln!("Call to IOMasterPort failed");
        std::process::abort();
    }

    master_port
}

fn get_io_service(master_port: mach_port_t) -> io_service_t {
    let service_name = CString::new("AppleSMBIOS").expect("Why would this ever fail?");
    let service_name_ptr = service_name.as_ptr() as *const i8;
    let service = unsafe { IOServiceGetMatchingService(master_port, IOServiceMatching(service_name_ptr)) };
    if service == MACH_PORT_NULL {
        eprintln!("AppleSMBIOS service is unreachable, sorry.");
        std::process::abort();
    }

    service
}

fn get_raw_data(service: io_service_t, property_name: *const std::os::raw::c_char) -> Vec<u8> {
    let property_name_cfstring = unsafe {
        CFStringCreateWithCString(
            CFAllocatorGetDefault(),
            property_name,
            kCFStringEncodingASCII)
    };
    if property_name_cfstring == std::ptr::null() {
        eprintln!("Failed to create CFString of 'SMBIOS' c-string. Wat.");
        std::process::abort();
    }

    let smbios = unsafe { IORegistryEntryCreateCFProperty(service, property_name_cfstring, kCFAllocatorDefault, kNilOptions) };
    if smbios == std::ptr::null() {
        eprintln!("No data in AppleSMBIOS IOService, sorry.");
        std::process::abort();
    }

    let smbios_data = unsafe { CFDataGetBytePtr(smbios as CFDataRef) };
    let smbios_data = smbios_data as CFDataRef;

    let length = unsafe { CFDataGetLength(smbios as CFDataRef) } as usize;
    if smbios_data == std::ptr::null()  || length == 0 {
        eprintln!("Data is null.");
        std::process::abort();
    }

    let mut buffer = vec![0u8; length];
    let smbios_data_slice = unsafe {
        std::slice::from_raw_parts(smbios_data as *const _ as *const u8, length)
    };

    buffer.copy_from_slice(smbios_data_slice);

    buffer
}

fn get_smbios_eps_data(service: io_service_t) -> Vec<u8> {
    let key = b"SMBIOS-EPS\0".as_ptr() as *const std::os::raw::c_char;
    get_raw_data(service, key)
}

fn get_smbios_data(service: io_service_t) -> Vec<u8> {
    let key = b"SMBIOS\0".as_ptr() as *const std::os::raw::c_char;
    get_raw_data(service, key)
}