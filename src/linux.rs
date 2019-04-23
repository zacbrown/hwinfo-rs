// Predetermined locations where SMBIOS information can be found.
const DEV_MEM: &str = "/dev/mem";
const LINUX_SYSFS_DMI: &str = "/sys/firmware/dmi/tables/DMI";
const LINUX_SYSFS_ENTRY_POINT: &str = "/sys/firmware/dmi/tables/smbios_entry_point";

/// Detects the entry point and location of an SMBIOS stream on this system,
/// returning the entry point found and all available SMBIOS structures.
// TODO(mdlayher): is this signature idiomatic?  Should this function just
// decode the stream instead?
pub fn stream() -> Result<(EntryPointType, Vec<Structure>)> {
    // Try the standard Linux sysfs location.
    // TODO(mdlayher): figure out cross-platform support.
    if !path::Path::new(LINUX_SYSFS_ENTRY_POINT).exists() {
        // Fall back to UNIX /dev/mem if possible.
        if !path::Path::new(DEV_MEM).exists() {
            // Nothing to do.
            return Err(Error::Internal(ErrorKind::EntryPointNotFound));
        }

        return dev_mem_stream();
    }

    let entry_point = fs::File::open(LINUX_SYSFS_ENTRY_POINT).map_err(Error::Io)?;
    let dmi = fs::File::open(LINUX_SYSFS_DMI).map_err(Error::Io)?;

    let structures = Decoder::new(dmi).decode()?;

    Ok((parse_entry_point(entry_point)?, structures))
}