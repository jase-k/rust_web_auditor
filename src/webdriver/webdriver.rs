use std::process::Child;
use std::process::Command;

pub struct DriverHandle {
    process: Child,
}

pub enum WebDriver {
    GeckoDriver,
}

#[derive(Debug)]
pub enum WebDriverError {
    UnableToCloseProgram(String),
}

impl DriverHandle {
    pub fn new(driver_type: WebDriver) -> Self {
        println!("Creating WebDriver");
        if cfg!(target_os = "windows") {
            match driver_type {
                WebDriver::GeckoDriver => {
                    return DriverHandle {
                        process: Command::new("geckodriver")
                            .spawn()
                            .expect("command failed to start"),
                    }
                },
                //TODO: add more compatible Drivers
            }
        } else if cfg!(target_os = "linux") {
            println!("Running configuration for linux");
            match driver_type {
                WebDriver::GeckoDriver => {
                    return DriverHandle {
                        process: Command::new("geckodriver")
                            .spawn()
                            .expect("command failed to start"),
                    }
                },
                //Add more compatible Drivers later
            }
        } else {
            panic!("Didn't recognize os system!");
        }
    }

    pub fn kill(&mut self) -> Result<(), WebDriverError> {
        println!("Closing Webdriver");
        if let Ok(_) = self.process.kill() {
            Ok(())
        } else {
            Err(WebDriverError::UnableToCloseProgram(String::from(
                "Driver wasn't running!",
            )))
        }
    }
}
