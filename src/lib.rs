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
//!     fn fetch<W: Write>(&self, handle: &mut BufWriter<W>) -> Result<()> {
//!         let load = (LoadAverage::new().unwrap().five * 100.0) as isize;
//!         writeln!(handle, "load.value {}", load);
//!         Ok(())
//!     }
//!
//!     // This plugin does not need any setup and will just work, so
//!     // just auto-configure it, if asked for.
//!     fn check_autoconf(&self) -> bool {
//!         true
//!     }
//!
//!     // The acquire function is not needed for a simple plugin that
//!     // only gathers data every 5 minutes (munin standard), but the
//!     // trait requires a stub to be there.
//!     fn acquire(&mut self, config: &Config) -> Result<()> {
//!         unimplemented!()
//!     }
//! }
//!
//! // The actual program start point
//! fn main() -> Result<()> {
//!     // Get our Plugin
//!     let mut load = LoadPlugin;
//!     // And let it do the work.
//!     load.simple_start(String::from("load"))?;
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
// We do not want to write unsafe code
#![forbid(unsafe_code)]

pub mod config;
pub use crate::config::Config;

use anyhow::{anyhow, Result};
// daemonize
use daemonize::Daemonize;
// daemonize
use fs2::FileExt;
use log::{trace, warn};
// daemonize
use spin_sleep::LoopHelper;
use std::{
    env,
    io::{self, BufWriter, Write},
    path::Path,
};
// daemonize
use std::{
    fs::File,
    process::{Command, Stdio},
    thread,
    time::Duration,
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
    /// # fn acquire(&mut self, config: &Config) -> Result<()> { todo!() }
    /// # fn fetch<W: Write>(&self, handle: &mut BufWriter<W>) -> Result<()> { todo!() }
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

    /// Acquire data and store it for later fetching.
    ///
    /// Acquire is called from [MuninPlugin::daemon] once every second
    /// and is expected to do whatever is neccessary to gather the
    /// data, the plugin is supposed to gather. It should then store
    /// it somewhere, where [MuninPlugin::fetch] can read it.
    /// [MuninPlugin::fetch] will be called from munin-node (usually)
    /// every 5 minutes and is expected to output data to stdout, in a
    /// munin compatible way.
    fn acquire(&mut self, config: &Config) -> Result<()>;

    /// Daemonize
    ///
    /// This function will daemonize the process and then start a
    /// loop, run once a second, calling [MuninPlugin::acquire].
    fn daemon(&mut self, config: &Config) -> Result<()> {
        // We want to run as daemon, so prepare
        let daemonize = Daemonize::new()
            .pid_file(&config.pidfile)
            .chown_pid_file(true)
            .working_directory("/tmp");

        // And off into the background we go
        daemonize.start()?;

        // The loop helper makes it easy to repeat a loop once a second
        let mut loop_helper = LoopHelper::builder().build_with_target_rate(1); // Only once a second

        // We run forever
        loop {
            // Let loop helper prepare
            loop_helper.loop_start();

            self.acquire(config)?;

            // Sleep for the rest of the second
            loop_helper.loop_sleep();
        }
    }

    /// Fetch delivers actual data to munin. This is called whenever
    /// the plugin is called without an argument. If the
    /// [config::Config::dirtyconfig] setting is true (auto-detected from
    /// environment set by munin), this will also be called right
    /// after having called [MuninPlugin::config].
    ///
    /// A simple plugin may just gather data here too and then write it to the handle.
    /// A plugin that daemonizes will gather data in [MuninPlugin::acquire] and cache
    /// that in one or more cachefiles and just push it all to the handle (possibly using [std::io::copy]).
    ///
    /// The size of the BufWriter is configurable from [Config::fetchsize].
    ///
    /// # Example 1 - Simple: Calculate some data, output
    /// ```rust
    /// # use munin_plugin::*;
    /// # use anyhow::{anyhow, Result};
    /// # use std::{
    /// # env,
    /// # io::{self, BufWriter, Write},
    /// # };
    /// use procfs::LoadAverage;
    /// # struct LoadPlugin;
    /// # impl MuninPlugin for LoadPlugin {
    /// # fn acquire(&mut self, config: &Config) -> Result<()> { todo!() }
    /// # fn config<W: Write>(&self, handle: &mut BufWriter<W>) -> Result<()> { todo!() }
    /// fn fetch<W: Write>(&self, handle: &mut BufWriter<W>) -> Result<()> {
    ///     let load = (LoadAverage::new().unwrap().five * 100.0) as isize;
    ///     writeln!(handle, "load.value {}", load)?;
    ///     Ok(())
    /// }
    /// # }
    /// ```
    fn fetch<W: Write>(&self, handle: &mut BufWriter<W>) -> Result<()>;

    /// Check whatever is neccessary to decide if the plugin can
    /// auto-configure itself.
    ///
    /// For example a network load plugin may check if network
    /// interfaces exists and then return true, something presenting
    /// values of a daemon like apache or ntp may check if that is
    /// installed - and possibly if fetching values is possible.
    ///
    /// If this function is not overwritten, it defaults to false.
    fn check_autoconf(&self) -> bool {
        false
    }

    /// Tell munin if the plugin supports autoconf.
    ///
    /// Munin expects a simple yes or no on stdout, so we just print
    /// it, depending on the return value of
    /// [MuninPlugin::check_autoconf]. The default of that is a plain
    /// false. If it is possible for your plugin to detect, if it can
    /// autoconfigure itself, then implement the logic in
    /// [MuninPlugin::check_autoconf] and have it return true.
    #[cfg(not(tarpaulin_include))]
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
    /// call the real start function. Only useful for plugins that do
    /// not use daemonization.
    fn simple_start(&mut self, name: String) -> Result<bool> {
        trace!("Simple Start, setting up config");
        let config = Config::new(name);
        trace!("Plugin: {:#?}", config);

        self.start(config)?;
        Ok(true)
    }

    /// The main plugin function, this will deal with parsing
    /// commandline arguments and doing what is expected of the plugin
    /// (present config, fetch values, whatever).
    fn start(&mut self, config: Config) -> Result<bool> {
        trace!("Plugin start");
        trace!("My plugin config: {config:#?}");

        // Store arguments for (possible) later use
        let args: Vec<String> = env::args().collect();

        // Now go over the args and see what we are supposed to do
        match args.len() {
            // no arguments passed, print data
            1 => {
                trace!("No argument, assuming fetch");
                // For daemonization we need to check if a copy of us
                // already runs. We do this by trying to lock our
                // pidfile. If that works, nothing is running, then we
                // need to start us in the background.
                if config.daemonize {
                    let lockfile = !Path::exists(&config.pidfile) || {
                        let lockedfile =
                            File::open(&config.pidfile).expect("Could not open pidfile");
                        lockedfile.try_lock_exclusive().is_ok()
                    };
                    // If we could lock, it appears that acquire isn't running. Start it.
                    if lockfile {
                        trace!("Could lock the pidfile, will spawn acquire now");
                        Command::new(&args[0])
                            .arg("acquire")
                            .stdin(Stdio::null())
                            .stdout(Stdio::null())
                            .stderr(Stdio::null())
                            .spawn()
                            .expect("failed to execute acquire");
                        trace!("Spawned, sleep for 1s, then continue");
                        // Now we wait one second before going on, so the
                        // newly spawned process had a chance to generate us
                        // some data
                        thread::sleep(Duration::from_secs(1));
                    }
                }
                trace!("Calling fetch");
                // We want to write a possibly large amount to stdout, take and lock it
                let stdout = io::stdout();
                // Buffered writer, to gather multiple small writes together
                let mut handle = BufWriter::with_capacity(config.fetchsize, stdout.lock());
                self.fetch(&mut handle)?;
                trace!("Done");
                // And flush the handle, so it can also deal with possible errors
                handle.flush()?;

                return Ok(true);
            }
            // Argument passed, check which one and act accordingly
            2 => match args[1].as_str() {
                "config" => {
                    // We want to write a possibly large amount to stdout, take and lock it
                    let stdout = io::stdout();
                    {
                        // Buffered writer, to gather multiple small writes together
                        let mut handle = BufWriter::with_capacity(config.cfgsize, stdout.lock());
                        self.config(&mut handle)?;
                        // And flush the handle, so it can also deal with possible errors
                        handle.flush()?;
                    }
                    // If munin supports dirtyconfig, send the data now
                    if config.dirtyconfig {
                        trace!("Munin supports dirtyconfig, sending data now");
                        let mut handle = BufWriter::with_capacity(config.fetchsize, stdout.lock());
                        self.fetch(&mut handle)?;
                        // And flush the handle, so it can also deal with possible errors
                        handle.flush()?;
                    }
                    return Ok(true);
                }
                "autoconf" => {
                    self.autoconf();
                    return Ok(true);
                }
                "acquire" => {
                    trace!("Called acquire to gather data");
                    // Only will ever process anything after this line, if
                    // one process has our pidfile already locked, ie. if
                    // another acquire is running. (Or if we can not
                    // daemonize for another reason).
                    if let Err(e) = self.daemon(&config) {
                        return Err(anyhow!(
                            "Could not start plugin {} in daemon mode to gather data: {}",
                            config.plugin_name,
                            e
                        ));
                    };
                }
                &_ => trace!("Unsupported argument: {}", args[1]),
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

    // Our plugin struct
    #[derive(Debug)]
    struct TestPlugin;
    impl MuninPlugin for TestPlugin {
        fn config<W: Write>(&self, handle: &mut BufWriter<W>) -> Result<()> {
            writeln!(handle, "This is a test plugin")?;
            writeln!(handle, "There is no config")?;
            Ok(())
        }
        fn fetch<W: Write>(&self, handle: &mut BufWriter<W>) -> Result<()> {
            writeln!(handle, "This is a value")?;
            writeln!(handle, "And one more value")?;
            Ok(())
        }
        fn check_autoconf(&self) -> bool {
            true
        }
        fn acquire(&mut self, _config: &Config) -> Result<()> {
            unimplemented!()
        }
    }

    #[test]
    fn test_config() {
        let test = TestPlugin;

        // We want to check the output of config contains our test string
        // above, so have it "write" it to a variable, then check if
        // the variable contains what we want
        let checktext = Vec::new();
        let mut handle = BufWriter::new(checktext);
        test.config(&mut handle).unwrap();
        handle.flush().unwrap();

        // And now check what got "written" into the variable
        let (recovered_writer, _buffered_data) = handle.into_parts();
        let output = String::from_utf8(recovered_writer).unwrap();
        assert_eq!(
            output,
            String::from("This is a test plugin\nThere is no config\n")
        );
    }

    #[test]
    fn test_fetch() {
        let test = TestPlugin;

        // We want to check the output of config contains our test string
        // above, so have it "write" it to a variable, then check if
        // the variable contains what we want
        let checktext = Vec::new();
        let mut handle = BufWriter::new(checktext);
        test.fetch(&mut handle).unwrap();
        handle.flush().unwrap();

        // And now check what got "written" into the variable
        let (recovered_writer, _buffered_data) = handle.into_parts();
        let output = String::from_utf8(recovered_writer).unwrap();
        assert_eq!(
            output,
            String::from("This is a value\nAnd one more value\n")
        );
    }
}
