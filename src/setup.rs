use std::{fs, process::Command};
use crate::error::{self, HypercError};

pub fn setup_hyperc() {
    match setup::proceed() {
        Ok(_) => println!("Installation finished!"),
        Err(e) => error::report_setup_error(&e),
    }
}

const OS_RELEASE_DIR: &str = "/etc/os-release";

#[cfg(target_os = "linux")]
mod setup {
    use crate::{error::HypercError, setup::{
        get_distro_name, 
        setup_arch, 
        setup_debian, 
        setup_fedora
    }};

    pub fn proceed() -> Result<(), HypercError> {
        let id = get_distro_name()?;

        match id.trim() {
            "arch" => setup_arch(),
            "ubuntu" | "debian" => setup_debian(),
            "fedora" => setup_fedora(),
            _ => Err(HypercError::SetupError {
                message: "This distro is not supported. (yet)".to_string()
            })
        }
    }
}

#[cfg(target_os = "windows")]
mod setup {
    use std::process::Command;
  
    pub fn proceed() -> Result<(), HypercError> {
        let mut cmd = Command::new("winget");
        cmd.args(["install", "-e", "--id", "LLVM.LLVM"]);
        proceed_cmd(&mut cmd)?;
        Ok(())
    }
}

fn setup_arch() -> Result<(), HypercError> {
    let mut cmd = Command::new("pacman");
    cmd.args(["-S", "--noconfirm", "llvm"]);
    proceed_cmd(&mut cmd)?;
    Ok(())
}

fn setup_debian() -> Result<(), HypercError> {

    let mut cmd = Command::new("apt-get");
    cmd.args(["update"]);
    proceed_cmd(&mut cmd)?;

    let mut cmd = Command::new("apt-get");
    cmd.args([
        "install", "-y", 
        "lsb-release", "wget"
    ]);
    proceed_cmd(&mut cmd)?;

    let mut cmd = Command::new("wget");
    cmd.args(["-O", "/tmp/llvm.sh", "https://apt.llvm.org/llvm.sh"]);
    proceed_cmd(&mut cmd)?;

    let mut cmd = Command::new("bash");
    cmd.args(["/tmp/llvm.sh"]);
    proceed_cmd(&mut cmd)?;
    
    let mut cmd = Command::new("rm");
    cmd.args(["-f", "/tmp/llvm.sh"]);
    proceed_cmd(&mut cmd)?;

    Ok(())
}

fn setup_fedora() -> Result<(), HypercError> {
    let mut cmd = Command::new("dnf");
    cmd.args(["install", "-y", "llvm"]);
    proceed_cmd(&mut cmd)?;
    Ok(())
}

fn proceed_cmd(cmd: &mut Command) -> Result<(), HypercError> {
    match cmd.status() {
        Ok(status) => {
            if !status.success() {
                return Err(HypercError::SetupError {
                    message: format!("Exited with code: {:?}", status.code())
                })
            }
        }
        Err(e) => {
            return Err(HypercError::SetupError {
                message: format!("Error emmited while downloading llvm: {}", e)
            })
        }
    }
    Ok(())
}

fn get_distro_name() -> Result<String, HypercError> {
    let content = match fs::read_to_string(OS_RELEASE_DIR) {
        Ok(c) => c,
        Err(e) => return Err(HypercError::SetupError {
            message: format!("Error emmited while getting distro name: {}", e)
        })
    };

    for ln in content.lines() {
        if let Some(id) = ln.strip_prefix("ID=") {
            return Ok(id.trim_matches('"').to_owned());
        }
    }

    Err(HypercError::SetupError {
        message: "Distro name not found.".to_string()
    })
}