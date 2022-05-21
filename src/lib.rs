//! munin-plugin - Simple writing of plugins for munin in Rust
//!
//! SPDX-License-Identifier: LGPL-3.0-only
//!
//! Copyright (C) 2022 Joerg Jaspert <joerg@debian.org>
//!
//! # About

//! Simple way to write munin plugins. There are basically two types of plugins,
//! - **Simple** or **standard** ones, those are called once every munin
//! run and gather and output there data at that time. Usually every 5
//! minutes.
//! - **Streaming** ones, those daemonize themself and _continuously_
//! gather data, usually caching it in a file, and when munin comes
//! around after 5 minutes again, they output everything they gathered
//! in the meantime.
//!
//! Those _streaming_ plugins are needed/useful, when graphs with
//! resolutions down to the second, rather than the default 5 minutes,
//! should be created.
//!
//! Both types of plugins have to follow all the usual rules for munin
//! plugins, such as outputting their data to stdout and reacting to
//! the `config` parameter to print their munin graph configuration.
//!
//! # Repositories / plugins using this code
//! - [Simple munin plugin to graph load](https://github.com/Ganneff/munin-load)
//! - [Munin CPU graph with 1second resolution](https://github.com/Ganneff/cpu1sec/)
//! - [Munin Interface graph with 1second resolution](https://github.com/Ganneff/if1sec)
//!
//! # Usage

//! This library tries to abstract as much of the details away, so you
//! can concentrate on the actual task - defining how the graph should
//! appear and gathering the data. For that, you need to implement the
//! [MuninPlugin] trait and provide the two functions `config` and
//! `acquire`, all the rest are provided with a (hopefully) useful
//! default implementation.
//!
//! ## config()
//! The _config_ function will be called whenever the plugin gets
//! called with the config argument from munin. This happens on every
//! munin run, which usually happens every 5 minutes. It is expected
//! to print out a munin graph configuration and you can find details
//! on possible values to print at [the Munin Plugin
//! Guide](http://guide.munin-monitoring.org/en/latest/plugin/writing.html).
//! For some basics you can also look into the examples throughout
//! this lib.
//!
//! **Note**: Streaming plugins should take care of correctly setting
//! munins `graph_data_size` and `update_rate` option. Those is the
//! difference in their configuration compared to standard plugins!
//!
//! ## acquire()
//!
//! The _acquire_ function will be called whenever the plugin needs to
//! gather data. For a _standard_ plugin that will be once every 5
//! minutes. A _streaming_ plugin will call this function once every
//! second.
//!
//! In both cases, _standard_ and _streaming_, you should do whatever
//! is needed to gather the data and then write it to the provided
//! handle, this library will take care of either handing it directly
//! to munin on stdout (_standard_) or storing it in a cache file
//! (_streaming_), to hand it out whenever munin comes around to fetch
//! the data.
//!
//! The format to write the data in is the one munin expects,
//! - _standard_: fieldname.value VALUE
//! - _streaming_: fieldname.value EPOCH:VALUE
//! where fieldname matches the config output, EPOCH is the
//! unix epoch in seconds and VALUE is whatever value got
//! calculated.
//!
//! # Example
//! The following implements the **load** plugin from munin, graphing
//! the load average of the system, using the 5-minute value. As
//! implemented, it expects to be run by munin every 5 minutes,
//! usually munin will first run it with the config parameter,
//! followed by no parameter to fetch data. If munin-node supports it
//! and the capability _dirtyconfig_ is set, config will also print
//! out data (this library handles that for you).
//!
//! It is a shortened version of the plugin linked above (Simple munin
//! plugin to graph load), with things like logging dropped.
//!
//! For more example code look into the actual [MuninPlugin] trait and
//! its function definitions.
//!
//! ```rust
//! use anyhow::Result;
//! use munin_plugin::{Config, MuninPlugin};
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
//!     // Calculate data (we want the 5-minute load average) and write it to the handle.
//!     fn acquire<W: Write>(&self, handle: &mut BufWriter<W>, _config: &Config, _epoch: u64) -> Result<()> {
//!         let load = (LoadAverage::new().unwrap().five * 100.0) as isize;
//!         writeln!(handle, "load.value {}", load)?;
//!         Ok(())
//!     }
//! }
//!
//! // The actual program start point
//! fn main() -> Result<()> {
//!     // Get our Plugin
//!     let load = LoadPlugin;
//!     // And let it do the work.
//!     load.simple_start(String::from("load"))?;
//!     Ok(())
//! }
//! ```
//!
//! # Logging
//! This crate uses the default [log] crate to output log messages of
//! level trace. If you want to see them, select a log framework you
//! like and ensure its level will display trace messages. See
//! that frameworks documentation on how to setup/include it.
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
    fs::{rename, File, OpenOptions},
    process::{Command, Stdio},
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
// daemonize
use tempfile::NamedTempFile;

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
    /// for many graphs.
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
    /// # fn acquire<W: Write>(&self, handle: &mut BufWriter<W>, config: &Config, epoch: u64) -> Result<()> { todo!() }
    /// # fn fetch<W: Write>(&self, handle: &mut BufWriter<W>, config: &Config) -> Result<()> { todo!() }
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
    /// Acquire is called whenever data should be gathered. For a
    /// _standard_ plugin this will be every 5 minutes, a _streaming_
    /// plugin will call acquire once a second. Acquire is expected to
    /// do whatever is neccessary to gather the data that the plugin
    /// is supposed to gather. It should writeln!() it to the provided
    /// handle, which - depending on the plugin type - will either be
    /// connected to stdout or a cachefile. The data written out has
    /// to be in munin compatible format:
    /// - _standard_ plugin: fieldname.value VALUE
    /// - _streaming_ plugin: fieldname.value EPOCH:VALUE
    /// where fieldname matches the config output, EPOCH is the unix
    /// epoch in seconds and VALUE is whatever value got calculated.
    ///
    /// # Example 1, _standard_ plugin
    /// ```rust
    /// # pub use munin_plugin::*;
    /// # use procfs::LoadAverage;
    /// # use anyhow::{anyhow, Result};
    /// # use std::{
    /// # env,
    /// # fs::{rename, OpenOptions},
    /// # io::{self, BufWriter, Write},
    /// # path::{Path, PathBuf},
    /// # time::{SystemTime, UNIX_EPOCH},
    /// # };
    /// # struct InterfacePlugin {
    /// #   interface: String,
    /// #   cache: PathBuf,
    /// #   if_txbytes: PathBuf,
    /// #   if_rxbytes: PathBuf,
    /// # };
    /// # impl MuninPlugin for InterfacePlugin {
    /// # fn fetch<W: Write>(&self, handle: &mut BufWriter<W>, config: &Config) -> Result<()> { todo!() }
    /// # fn config<W: Write>(&self, handle: &mut BufWriter<W>) -> Result<()> { todo!() }
    /// fn acquire<W: Write>(&self, handle: &mut BufWriter<W>, config: &Config, epoch: u64) -> Result<()> {
    ///     let load = (LoadAverage::new().unwrap().five * 100.0) as isize;
    ///     writeln!(handle, "load.value {}", load)?;
    ///     Ok(())
    /// }
    /// # }
    /// ```
    ///
    /// # Example 2, _streaming_ plugin
    /// ```rust
    /// # pub use munin_plugin::*;
    /// # use anyhow::{anyhow, Result};
    /// # use std::{
    /// # env,
    /// # fs::{rename, OpenOptions},
    /// # io::{self, BufWriter, Write},
    /// # path::{Path, PathBuf},
    /// # time::{SystemTime, UNIX_EPOCH},
    /// # };
    /// # struct InterfacePlugin {
    /// #   interface: String,
    /// #   cache: PathBuf,
    /// #   if_txbytes: PathBuf,
    /// #   if_rxbytes: PathBuf,
    /// # };
    /// # impl MuninPlugin for InterfacePlugin {
    /// # fn fetch<W: Write>(&self, handle: &mut BufWriter<W>, config: &Config) -> Result<()> { todo!() }
    /// # fn config<W: Write>(&self, handle: &mut BufWriter<W>) -> Result<()> { todo!() }
    /// fn acquire<W: Write>(&self, handle: &mut BufWriter<W>, config: &Config, epoch: u64) -> Result<()> {
    ///     // Read in the received and transferred bytes, store as u64
    ///     let rx: u64 = std::fs::read_to_string(&self.if_rxbytes)?.trim().parse()?;
    ///     let tx: u64 = std::fs::read_to_string(&self.if_txbytes)?.trim().parse()?;
    ///
    ///     // And now write out values
    ///     writeln!(handle, "{0}_tx.value {1}:{2}", self.interface, epoch, tx)?;
    ///     writeln!(handle, "{0}_rx.value {1}:{2}", self.interface, epoch, rx)?;
    ///
    ///     Ok(())
    /// }
    /// # }
    /// ```
    fn acquire<W: Write>(
        &self,
        handle: &mut BufWriter<W>,
        config: &Config,
        epoch: u64,
    ) -> Result<()>;

    /// Daemonize
    ///
    /// This function will daemonize the process and then start a
    /// loop, run once a second, calling [MuninPlugin::acquire].
    fn daemon(&self, config: &Config) -> Result<()> {
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

            // Streaming plugins need the epoch, so provide it
            let epoch = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("Time gone broken, what?")
                .as_secs(); // without the nanosecond part

            // Own scope, so file is closed before we sleep. Ensures
            // we won't have a file open, that fetch just moved away
            // to send out to munin.
            {
                // Open the munin cachefile to store our values,
                // using a BufWriter to "collect" the two writeln
                // together
                let mut handle = BufWriter::with_capacity(
                    config.fetchsize,
                    OpenOptions::new()
                        .create(true) // If not there, create
                        .write(true) // We want to write
                        .append(true) // We want to append
                        .open(&config.plugin_cache)?,
                );

                self.acquire(&mut handle, config, epoch)?;
            }
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
    /// The size of the BufWriter this function uses is configurable
    /// from [Config::fetchsize].
    ///
    /// This function will adjust its behaviour based on the plugin
    /// being a _standard_ or _streaming_ plugin. For _standard_ plugins
    /// it will simply call acquire, so data is gathered and written
    /// to the provided handle (and as such, to stdout where munin
    /// expects it).
    ///
    /// For _streaming_ plugins it will create a temporary file beside
    /// the [config::Config::plugin_cache], will rename the
    /// [config::Config::plugin_cache] and then use [std::io::copy] to
    /// "copy" the data to the provided handle.
    ///
    /// # Overriding this function
    /// If you want to override this function, you should ensure that
    /// (for _streaming_ plugins) you ensure that the cache file is
    /// reset, whenever `fetch()` runs, or old data may be given to
    /// munin needlessly. You also need to ensure to not accidently
    /// deleting data when dealing with your cachefile. For example:
    /// You read the whole cachefile, then output it to munin, then
    /// delete it - and during the halfsecond this took, new data
    /// appeared in the file, now lost.
    fn fetch<W: Write>(&self, handle: &mut BufWriter<W>, config: &Config) -> Result<()> {
        // Daemonize means plugin writes a cachefile, so lets output that
        if config.daemonize {
            // We need a temporary file
            let fetchpath = NamedTempFile::new_in(
                config
                    .plugin_cache
                    .parent()
                    .expect("Could not find useful temp path"),
            )?;
            // Rename the cache file, to ensure that acquire doesn't add data
            // between us outputting data and deleting the file
            rename(&config.plugin_cache, &fetchpath)?;
            // Want to read the tempfile now
            let mut fetchfile = std::fs::File::open(&fetchpath)?;
            // And ask io::copy to just take it all and shove it into the handle
            io::copy(&mut fetchfile, handle)?;
        } else {
            // Not daemonizing, plugin gathers data and wants to output it directly.
            // So we just call acquire, which is expected to write its data to handle.
            self.acquire(handle, config, 0)?;
        }
        Ok(())
    }

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
    /// not use daemonization or need other config changes to run
    /// successfully..
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
                self.fetch(&mut handle, &config)?;
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
                        self.fetch(&mut handle, &config)?;
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
                    // Will only ever process anything after this line, if
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
        fn fetch<W: Write>(&self, handle: &mut BufWriter<W>, _config: &Config) -> Result<()> {
            writeln!(handle, "This is a value")?;
            writeln!(handle, "And one more value")?;
            Ok(())
        }
        fn check_autoconf(&self) -> bool {
            true
        }
        fn acquire<W: Write>(
            &self,
            _handle: &mut BufWriter<W>,
            _config: &Config,
            _epoch: u64,
        ) -> Result<()> {
            Ok(())
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
        test.fetch(&mut handle, &config::Config::new("test".to_string()))
            .unwrap();
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
