//! munin-plugin - Simple writing of plugins for munin in Rust
//!
//! SPDX-License-Identifier: LGPL-3.0-only
//!
//! Copyright (C) 2022 Joerg Jaspert <joerg@debian.org>
//!
//! # About
//! Simple way to write munin plugins.
//!
//! # Repository / plugin using this code
//! - [Simple munin plugin to graph load](https://github.com/Ganneff/munin-load)
//!
//! # Usage
//! To implement a standard munin plugin, which munin runs every 5 minutes when fetching data, you load this library, create an empty struct named for your plugin and then implement `MuninPlugin` for your struct. You need to write out the functions `config` and `fetch`, the rest can have the magic `unimplemented!()`, and you call start() on your Plugin.
//!
//! # Example
//! The following implements the **load** plugin from munin, graphing the load average of the system, using the 5-minute value. As implemented, it expects to be run by munin every 5 minutes, usually munin will first run it with the config parameter, followed by no parameter to fetch data. If munin-node supports it and the capability _dirtyconfig_ is set, config will also print out data.
//!
//! It is a shortened version of the plugin linked above (Simple munin plugin to graph load), with things like logging dropped.
//!
//! ```rust
//! use anyhow::Result;
//! use munin_plugin::{config::Config, MuninPlugin};
//! use procfs::LoadAverage;
//! use std::io::{self, BufWriter, Write};
//!
//! // Our plugin struct
//! #[derive(Debug)]
//! struct LoadPlugin;
//!
//! // Implement the needed functions
//! impl MuninPlugin for LoadPlugin {
//!     // Write out munin config. handle is setup as a bufwriter to stdout.
//!     fn config<W: Write>(&self, handle: &mut BufWriter<W>) -> Result<()> {
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
//!        Ok(())
//!     }
//!
//!     // Fetch and display data
//!     fn fetch(&self) {
//!         let load = (LoadAverage::new().unwrap().five * 100.0) as isize;
//!         println!("load.value {}", load);
//!     }
//!
//!     // This plugin does not need any setup and will just work, so
//!     // just auto-configure it, if asked for.
//!     fn check_autoconf(&self) -> bool {
//!         true
//!     }
//!
//!     // The other functions are not needed for a simple plugin that
//!     // only gathers data every 5 minutes (munin standard), but the
//!     // trait requires stubs to be there.
//!     fn run(&self) {
//!         unimplemented!()
//!     }
//!     fn daemonize(&self) {
//!         unimplemented!()
//!     }
//!     fn acquire(&self) {
//!         unimplemented!()
//!     }
//! }
//!
//! // The actual program start point
//! fn main() -> Result<()> {
//!     // Setup our config, needs our name, rest the defaults will work.
//!     let config = Config::new("load".to_string());
//!     // Get our Plugin
//!     let load = LoadPlugin;
//!     // And let it do the work.
//!     load.start(config)?;
//!     Ok(())
//! }
//! ```
//!
//! # Logging
//! This crate uses the default [log] crate to output log messages of
//! level trace or debug. No other levels will be used. If you want to
//! see them, select a log framework you like and ensure its level
//! will display trace/debug messages. See that frameworks
//! documentation on how to setup/include it.
//!
//! If you do not want/need log output, just do nothing.

// Tell us if we forget to document things
#![warn(missing_docs)]

pub mod config;
pub use crate::config::Config;

use anyhow::{anyhow, Result};
use log::{info, trace, warn};
use std::{
    env,
    io::{self, BufWriter, Write},
};

/// Defines a Munin Plugin and the needed functions
pub trait MuninPlugin {
    /// Write out a munin config, read the [Developing
    /// plugins](http://guide.munin-monitoring.org/en/latest/develop/plugins/index.html)
    /// guide from munin for everything you can print out here.
    ///
    /// Note that munin expects this to appear on stdout, so the
    /// plugin gives you a handle to write to, which is setup as a
    /// [std::io::BufWriter] to stdout. The [std::io::BufWriter]
    /// capacity defaults to 8192 bytes, but if you need more, its
    /// size can be set using [Config::cfgsize]. An example where this
    /// may be useful is a munin multigraph plugin that outputs config
    /// for a many graphs.
    ///
    /// # Example
    /// ```rust
    /// # pub use munin_plugin::*;
    /// # use anyhow::{anyhow, Result};
    /// # use std::{
    /// # env,
    /// # io::{self, BufWriter, Write},
    /// # };
    /// # struct LoadPlugin;
    /// # impl MuninPlugin for LoadPlugin {
    /// # fn run(&self) { todo!() }
    /// # fn daemonize(&self) { todo!() }
    /// # fn acquire(&self) { todo!() }
    /// # fn fetch(&self) { todo!() }
    /// fn config<W: Write>(&self, handle: &mut BufWriter<W>) -> Result<()> {
    ///     writeln!(handle, "graph_title Load average")?;
    ///     writeln!(handle, "graph_args --base 1000 -l 0")?;
    ///     writeln!(handle, "graph_vlabel load")?;
    ///     writeln!(handle, "graph_scale no")?;
    ///     writeln!(handle, "graph_category system")?;
    ///     writeln!(handle, "load.label load")?;
    ///     writeln!(handle, "load.warning 10")?;
    ///     writeln!(handle, "load.critical 120")?;
    ///     writeln!(handle, "graph_info The load average of the machine describes how many processes are in the run-queue (scheduled to run immediately.")?;
    ///     writeln!(handle, "load.info Average load for the five minutes.")?;
    ///     Ok(())
    /// }
    /// # }
    /// ```
    fn config<W: Write>(&self, handle: &mut BufWriter<W>) -> Result<()>;

    /// Run
    fn run(&self);

    /// Daemonize
    fn daemonize(&self);

    /// Acquire
    fn acquire(&self);

    /// Fetch delivers actual data to munin. This is called whenever
    /// the plugin is called without an argument. If the
    /// [config::Config::dirtyconfig] setting is true (auto-detected from
    /// environment set by munin), this will also be called right
    /// after having called [MuninPlugin::config].
    ///
    /// For a simple plugin, this may gather data and just print it to
    /// stdout in a format that munin accepts. A plugin that
    /// daemonizes itself may just write out the cached data here.
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

    /// A simplified start, only need a name, for the rest, defaults are fine.
    ///
    /// This is just a tiny bit of "being lazy is good" and will
    /// create the [MuninPlugin::config] with the given name, then
    /// call the real start function.
    fn simple_start(&self, name: String) -> Result<bool> {
        trace!("Simple Start, setting up config");
        let config = Config::new(name);
        trace!("Plugin: {:#?}", config);

        self.start(config)?;
        Ok(true)
    }

    /// The main plugin function, this will deal with parsing
    /// commandline arguments and doing what is expected of the plugin
    /// (present config, fetch values, whatever).
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
                    // We want to write a possibly large amount to stdout, take and lock it
                    let stdout = io::stdout();
                    // Buffered writer, to gather multiple small writes together
                    let mut handle = BufWriter::with_capacity(config.cfgsize, stdout.lock());
                    self.config(&mut handle)?;
                    // And flush the handle, so it can also deal with possible errors
                    handle.flush()?;
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
