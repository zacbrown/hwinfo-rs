// Predetermined locations where SMBIOS information can be found.
const DEV_MEM: &str = "/dev/mem";
const LINUX_SYSFS_DMI: &str = "/sys/firmware/dmi/tables/DMI";
const LINUX_SYSFS_ENTRY_POINT: &str = "/sys/firmware/dmi/tables/smbios_entry_point";

pub fn get_raw_dmi() -> [u8] {
    if !std::path::Path::new(LINUX_SYSFS_ENTRY_POINT).exists() {
        // Fall back to UNIX /dev/mem if possible.
        if !std::path::Path::new(DEV_MEM).exists() {
            // Nothing to do.
            eprintln!("can't find /dev/mem, aborting");
            std::process::abort();
        }

        return dev_mem_stream();
    }

    let entry_point = std::fs::File::open(LINUX_SYSFS_ENTRY_POINT).unwrap();
    let dmi = std::fs::File::open(LINUX_SYSFS_DMI).unwrap();

}

// https://github.com/mdlayher/smbios-rs/blob/master/src/lib.rs

fn dev_mem_stream() -> Result<(EntryPointType, Vec<Structure>)> {
    let mut mem = fs::File::open(DEV_MEM).map_err(Error::Io)?;

    // Begin searching for the entry point at the location specified in the
    // SMBIOS specification.
    mem.seek(io::SeekFrom::Start(START_ADDRESS))
        .map_err(Error::Io)?;

    let address = find_entry_point(&mem)?;

    // Seek to where the entry point is.
    mem.seek(io::SeekFrom::Start(address)).map_err(Error::Io)?;

    // Discover the SMBIOS table location.
    let entry_point = parse_entry_point(&mem)?;

    let (table_address, table_size) = match &entry_point {
        EntryPointType::Bits32(ep) => ep.table(),
        EntryPointType::Bits64(ep) => ep.table(),
        _ => {
            return Err(Error::Internal(ErrorKind::InvalidEntryPoint));
        }
    };

    // Seek to the start of the SMBIOS stream and decode it.
    mem.seek(io::SeekFrom::Start(table_address as u64))
        .map_err(Error::Io)?;

    let structures = Decoder::new(mem.take(table_size as u64)).decode()?;

    Ok((entry_point, structures))
}
