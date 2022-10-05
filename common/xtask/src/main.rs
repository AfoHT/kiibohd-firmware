use std::{
    env,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
};

use anyhow::anyhow;
use duct::cmd;
use probe_rs::MemoryInterface;
use probe_rs_cli_util::common_options::ProbeOptions;

const CARGO_TARGET: &str = "thumbv7em-none-eabi";
const OPENOCD_INTERFACE: &str = "cmsis-dap";
const OPENOCD_TARGET: &str = "at91sam4sXX";
const PROBE_RUN_CHIP: &str = "ATSAM4S8B";
const RTT_TCP_PORT: u16 = 8765;
const SWD_SPEED: u32 = 2666;

fn main() -> Result<(), anyhow::Error> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let args = args.iter().map(|s| &**s).collect::<Vec<_>>();

    /*
    // For debugging env
    for (key, value) in env::vars() {
        println!("{}: {}", key, value);
    }
    */

    let cargo_target = match env::var("CARGO_TARGET") {
        Ok(val) => val,
        Err(_) => CARGO_TARGET.to_string(),
    };
    let binary_name = match env::var("CARGO_MAKE_BINARY_EXECUTABLE_NAME") {
        Ok(val) => val,
        // Attempt to guess based on the CWD
        Err(_) => current_crate_name()?,
    };
    let openocd_interface = match env::var("OPENOCD_INTERFACE") {
        Ok(val) => val,
        Err(_) => OPENOCD_INTERFACE.to_string(),
    };
    let openocd_target = match env::var("OPENOCD_TARGET") {
        Ok(val) => val,
        Err(_) => OPENOCD_TARGET.to_string(),
    };
    let probe_run_chip = match env::var("PROBE_RUN_CHIP") {
        Ok(val) => val,
        Err(_) => PROBE_RUN_CHIP.to_string(),
    };
    let rtt_tcp_port = match env::var("RTT_TCP_PORT") {
        Ok(val) => val.parse::<u16>().unwrap(),
        Err(_) => RTT_TCP_PORT,
    };
    let swd_speed = match env::var("SWD_SPEED") {
        Ok(val) => val.parse::<u32>().unwrap(),
        Err(_) => SWD_SPEED,
    };

    // Absolute path to the target binary (uses workspace root/target)
    let elf = Path::new(&repo_root()?)
        .join("target")
        .join(cargo_target.clone())
        .join("debug")
        .join(binary_name.clone());

    match &args[..] {
        ["gdb-server"] => gdb_server(
            cargo_target,
            elf,
            openocd_interface,
            openocd_target,
            rtt_tcp_port,
            swd_speed,
        )?,
        ["gdb-client"] => gdb_client(elf)?,
        ["sam4-bootloader"] => sam4_bootloader(probe_run_chip)?,
        _ => println!(
            "Cargo workflows

USAGE:
    cargo xtask [COMMAND]

Environment Variables (effective) (default):
    CARGO_TARGET ({}) ({})
    CRATE_NAME ({}) ({})
    OPENOCD_INTERFACE ({}) ({})
    OPENOCD_TARGET ({}) ({})
    RTT_TCP_PORT ({}) ({})
    SWD_SPEED ({}) ({})

COMMANDS:
    gdb-server      spawns a GDB server; flashes and runs firmware; prints logs
    gdb-client      starts a GDB client and connects to a running GDB server
    sam4-bootloader triggers hard reset and disables halt on reset; useful for use with dfu bootloaders
",
            cargo_target,
            CARGO_TARGET,
            binary_name,
            current_crate_name()?,
            openocd_interface,
            OPENOCD_INTERFACE,
            openocd_target,
            OPENOCD_TARGET,
            rtt_tcp_port,
            RTT_TCP_PORT,
            swd_speed,
            SWD_SPEED,
        ),
    }

    Ok(())
}

fn current_crate_name() -> Result<String, anyhow::Error> {
    Ok(String::from(
        PathBuf::from(env::var("PWD")?)
            // Retrieve the basename
            .file_name()
            .unwrap()
            .to_str()
            .unwrap(),
    ))
}

fn repo_root() -> Result<PathBuf, anyhow::Error> {
    // path to this crate (the directory that contains this crate's Cargo.toml)
    Ok(PathBuf::from(env::var("CARGO_MANIFEST_DIR")?)
        // from there go two levels up
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_owned())
}

fn gdb_server(
    cargo_target: String,
    elf: PathBuf,
    openocd_interface: String,
    openocd_target: String,
    rtt_tcp_port: u16,
    swd_speed: u32,
) -> Result<(), anyhow::Error> {
    const BP_LENGTH: u8 = 2; // breakpoint length
    const RTT_BLOCK_IF_FULL: u32 = 2; // bit in `flags` field
    const RTT_FLAGS: u32 = 44; // offset of `flags` field in control block
    const RTT_ID: &str = "SEGGER RTT"; // control block ID
    const RTT_SIZE: u8 = 48; // control block size
    const THUMB_BIT: u32 = 1;

    cmd!("cargo", "build", "--target", cargo_target).run()?;

    // get symbol addresses from ELF
    let nm = cmd!("nm", "-C", &elf).read()?;
    let mut rtt = None;
    let mut main = None;
    for line in nm.lines() {
        if line.ends_with("_SEGGER_RTT") {
            rtt = Some(line.split_once(' ').map_or(line, |x| x.0));
        } else if line.ends_with("main") {
            main = Some(line.split_once(' ').map_or(line, |x| x.0));
        }
    }

    let rtt = u32::from_str_radix(
        rtt.ok_or_else(|| anyhow!("RTT control block not found"))?,
        16,
    )?;
    let main = u32::from_str_radix(
        main.ok_or_else(|| anyhow!("`main` function not found"))?,
        16,
    )? & !THUMB_BIT;

    #[rustfmt::skip]
    let openocd = cmd!(
        "openocd",
        "-d0",
        "-c", format!("source [find interface/{}.cfg]", openocd_interface),
        "-c", "transport select swd",
        "-c", format!("source [find target/{}.cfg]", openocd_target),
        "-c", format!("adapter speed {}", swd_speed),
        "-c", "init",
        "-c", format!("rtt server start {} 0", rtt_tcp_port),
        "-c", "reset init",
        "-c", format!("flash write_image erase {}", elf.display()),
        "-c", "reset halt",
        "-c", format!("rtt setup {} {} {:?}", rtt, RTT_SIZE, RTT_ID),
        "-c", format!("bp {} {} hw", main, BP_LENGTH),
        "-c", "resume",
        "-c", format!("mww {} {}", rtt + RTT_FLAGS, RTT_BLOCK_IF_FULL),
        "-c", "rtt start",
    )
    .stderr_to_stdout()
    .reader()?;

    let mut lines = BufReader::new(openocd).lines();

    for line in lines.by_ref() {
        let line = line?;
        println!("{}", line);

        if line.contains("wrote") {
            break;
        }
    }

    cmd!("nc", "localhost", RTT_TCP_PORT.to_string())
        .pipe(cmd!("defmt-print", "-e", &elf))
        .run()?;

    // close `openocd` *after* `nc`
    drop(lines);

    Ok(())
}

fn gdb_client(elf: PathBuf) -> Result<(), anyhow::Error> {
    // Run gdb
    cmd!("arm-none-eabi-gdb", format!("{}", elf.display()),).run()?;
    Ok(())
}

/// Force sam4 reset to bootloader even if halt on reset is enabled
///
/// This is a common scenario when a debug probe is attached and it prevents
/// the normal use of internal bootloaders.
fn sam4_bootloader(probe_run_chip: String) -> Result<(), anyhow::Error> {
    let common = ProbeOptions {
        allow_erase_all: false,
        chip: Some(probe_run_chip),
        chip_description_path: None,
        connect_under_reset: false,
        dry_run: false,
        probe_selector: None,
        protocol: None,
        speed: None,
    };
    let mut session = common.simple_attach()?;
    let mut core = session.core(0)?;

    // Disable halt on reset (DEMCR) and other reset debugging
    core.write_word_32(0xE000EDFC, 0)?;

    // Force nRST (RSTC_CR) - atsam4
    core.write_word_32(0x400E1400, 0xA500_0008)?;

    Ok(())
}
