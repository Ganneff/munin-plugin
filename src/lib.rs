//! munin-plugin - Simple writing of plugins for munin in Rust
//!
//! SPDX-License-Identifier: LGPL-3.0-only
//!
//! Copyright (C) 2022 Joerg Jaspert <joerg@debian.org>
//!
//! # About
//! Simple way to write munin plugins.
//!
//! # Example code
//! - [Simple munin plugin to graph load ](https://github.com/Ganneff/munin-load)
//!
//! # Usage
//! To implement a standard munin plugin, which munin runs every 5 minutes when fetching data, you load this library, create an empty struct named for your plugin and then implement `MuninPlugin` for your struct. You need to write out the functions `config` and `fetch`, the rest can have the magic `unimplemented!()`, and you call start() on your Plugin.
//!
//! # Example
//! The following implements the **load** plugin from munin, graphing the load average of the system, using the 5-minute value. As implemented, it expects to be run by munin every 5 minutes, usually munin will first run it with the config parameter, followed by no parameter to fetch data. If munin-node supports it and the capability _dirtyconfig_ is set, config will also print out data.
//!
//! ```rust
//! use anyhow::Result;
//! use flexi_logger::Logger;
//! use log::{debug, info};
//! use munin_plugin::{config::Config, MuninPlugin};
//! use procfs::LoadAverage;
//! use std::io::{self, BufWriter, Write};
//! #[derive(Debug)]
//! struct LoadPlugin;
//!
//! impl MuninPlugin for LoadPlugin {
//!     fn config(&self) -> Result<()> {
//!         // We want to write a possibly large amount to stdout, take and lock it
//!        let stdout = io::stdout();
//!        let bufsize = 8192;
//!        // Buffered writer, to gather multiple small writes together
//!        let mut handle = BufWriter::with_capacity(bufsize, stdout.lock());
//!
//!        writeln!(handle, "graph_title Load average")?;
//!        writeln!(handle, "graph_args --base 1000 -l 0")?;
//!        writeln!(handle, "graph_vlabel load")?;
//!        writeln!(handle, "graph_scale no")?;
//!        writeln!(handle, "graph_category system")?;
//!        writeln!(handle, "load.label load")?;
//!        writeln!(handle, "load.warning 10")?;
//!        writeln!(handle, "load.critical 120")?;
//!        writeln!(handle, "graph_info The load average of the machine describes how many processes are in the run-queue (scheduled to run immediately.")?;
//!        writeln!(handle, "load.info Average load for the five minutes.")?;
//!        // And flush the handle, so it can also deal with possible errors
//!        handle.flush()?;
//!        Ok(())
//!     }
//!     fn fetch(&self) {
//!         let load = (LoadAverage::new().unwrap().five * 100.0) as isize;
//!         println!("load.value {}", load);
//!     }
//!     fn run(&self) {
//!         unimplemented!()
//!     }
//!     fn daemonize(&self) {
//!         unimplemented!()
//!     }
//!     fn acquire(&self) {
//!         unimplemented!()
//!     }
//!     /// Check autoconf
//!     fn check_autoconf(&self) -> bool {
//!         true
//!     }
//! }
//!
//! fn main() -> Result<()> {
//!     Logger::try_with_env_or_str("trace")
//!         .unwrap()
//!         .set_palette("b1;3;2;4;6".to_owned())
//!         .start()
//!         .unwrap();
//!     info!("load started");
//!     let config = Config::new("load".to_string());
//!     debug!("Plugin: {:#?}", config);
//!
//!     let load = LoadPlugin;
//!     load.start(config)?;
//!     Ok(())
//! }

//! ```

#![warn(missing_docs)]

pub mod config;

pub use crate::config::Config;

use anyhow::{anyhow, Result};
use log::{info, trace, warn};
use std::env;

/// Document this trait
pub trait MuninPlugin {
    /// Config
    fn config(&self) -> Result<()>;

    /// Run
    fn run(&self);

    /// Daemonize
    fn daemonize(&self);

    /// Acquire
    fn acquire(&self);

    /// Fetch
    fn fetch(&self);

    /// Check autoconf
    fn check_autoconf(&self) -> bool {
        false
    }

    /// Autoconf
    fn autoconf(&self) {
        if self.check_autoconf() {
            println!("yes")
        } else {
            println!("no")
        }
    }

    /// Start
    fn start(&self, config: Config) -> Result<bool> {
        trace!("Plugin start");
        trace!("My plugin config: {config:#?}");

        // Store arguments for (possible) later use
        let args: Vec<String> = env::args().collect();

        // Now go over the args and see what we are supposed to do
        match args.len() {
            // no arguments passed, print data
            1 => {
                self.fetch();
                return Ok(true);
            }
            // Argument passed, check which one and act accordingly
            2 => match args[1].as_str() {
                "config" => {
                    self.config()?;
                    // If munin supports dirtyconfig, send the data now
                    if config.dirtyconfig {
                        trace!("Munin supports dirtyconfig, sending data now");
                        self.fetch();
                    }
                    return Ok(true);
                }
                "autoconf" => {
                    self.autoconf();
                    return Ok(true);
                }
                &_ => info!("Found an argument: {}", args[1]),
            },
            // Whatever else
            _ => return Err(anyhow!("No argument given")),
        }
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_config() {
        // Whole set of defaults
        let config = config::Config {
            ..Default::default()
        };
        assert_eq!(
            config.plugin_name,
            String::from("Simple munin plugin in Rust")
        );

        // Use defaults (except for name)
        let mut config2 = config::Config {
            plugin_name: String::from("Lala"),
            ..Default::default()
        };
        assert_eq!(config2.plugin_name, String::from("Lala"));
        assert_eq!(config2.daemonize, false);

        config2.pidfile = PathBuf::new();
        config2.pidfile.push(&config2.plugin_statedir);
        config2.pidfile.push(String::from("Lala.pid"));

        let config3 = config::Config::new(String::from("Lala"));
        assert_eq!(config2, config3);
    }
}
