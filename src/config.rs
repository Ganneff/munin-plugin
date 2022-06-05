//! Config data for a munin plugin

// We do not want to write unsafe code
#![forbid(unsafe_code)]

use fastrand;
use log::trace;
use std::{
    env,
    iter::repeat_with,
    path::{Path, PathBuf},
};

/// Plugin configuration.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Config {
    /// The name of the plugin.
    ///
    /// Default is "Simple munin plugin in Rust"
    pub plugin_name: String,

    /// Plugins state directory
    ///
    /// Fallback to /tmp if environment variable MUNIN_PLUGSTATE is
    /// not set.
    pub plugin_statedir: PathBuf,

    /// Cachefile for the plugin
    ///
    /// Plugins that daemonize and continuously fetch data need to
    /// write them somewhere, so that the
    /// [MuninPlugin::fetch](super::MuninPlugin::fetch) function can
    /// output them. The default is a combination of
    /// [Config::plugin_statedir] and a random string, with _munin_ and
    /// _value_ added, in [std::format!] syntax: `"{}.munin.{}.value",
    /// [Config::plugin_statedir], randomstring`.
    pub plugin_cache: PathBuf,

    /// Does munin support dirtyconfig? (Send data after sending config)
    ///
    /// Checks MUNIN_CAP_DIRTYCONFIG environment variable, if set to 1,
    /// this is true, otherwise false.
    pub dirtyconfig: bool,

    /// Does this plugin need to run in background, continuously fetching data?
    ///
    /// Default to false
    pub daemonize: bool,

    /// If plugin uses daemonize, whats the pidfile name?
    ///
    /// Defaults to [Config::plugin_statedir] plus "munin-plugin.pid", using
    /// [Config::new] will set it to
    /// [Config::plugin_statedir]/[Config::plugin_name].pid
    pub pidfile: PathBuf,

    /// Size of buffer for BufWriter for [MuninPlugin::config](super::MuninPlugin::config).
    ///
    /// Defaults to 8192, but if the plugin outputs huge munin
    /// configuration (trivial with multigraph plugins), you may want
    /// to increase this.
    pub config_size: usize,

    /// Size of buffer for BufWriter for [MuninPlugin::fetch](super::MuninPlugin::fetch).
    ///
    /// Defaults to 8192, but if the plugin outputs large datasets, it
    /// is useful to increase this.
    pub fetch_size: usize,
}

impl Config {
    /// Return the plugin state directory as munin wants it - or /tmp
    /// if no environment variable is set.
    fn get_statedir() -> PathBuf {
        PathBuf::from(env::var("MUNIN_PLUGSTATE").unwrap_or_else(|_| String::from("/tmp")))
    }

    /// Create a new Config with defined plugin_name, also setting
    /// [Config::pidfile] and [Config::plugin_cache] to a sensible
    /// value using the [Config::plugin_name].
    ///
    /// # Examples
    ///
    /// ```
    /// # use munin_plugin::config::Config;
    /// let config = Config::new(String::from("great-plugin"));
    /// println!("My pidfile is {:?}", config.pidfile);
    /// ```
    pub fn new(plugin_name: String) -> Self {
        Config::realnew(plugin_name, false)
    }

    /// Create a new Config for a streaming (daemonizing) plugin with
    /// defined plugin_name, also setting [Config::pidfile] and
    /// [Config::plugin_cache] to a sensible value using the
    /// [Config::plugin_name].
    ///
    /// # Examples
    ///
    /// ```
    /// # use munin_plugin::config::Config;
    /// let config = Config::new_daemon(String::from("great-plugin"));
    /// println!("My pidfile is {:?}", config.pidfile);
    /// ```
    pub fn new_daemon(plugin_name: String) -> Self {
        Config::realnew(plugin_name, true)
    }

    /// Actually do the work of creating the config element
    fn realnew(plugin_name: String, daemonize: bool) -> Self {
        trace!("Creating new config for plugin {plugin_name}, daemon: {daemonize}");
        let pd = plugin_name.clone();
        Self {
            plugin_name,
            daemonize: daemonize,
            pidfile: Config::get_statedir().join(format!("{}.pid", pd)),
            plugin_cache: Config::get_statedir().join(format!("munin.{}.value", pd)),
            ..Default::default()
        }
    }
}

/// Useful defaults, if possible based on munin environment.
impl Default for Config {
    /// Set default values, try to read munin environment variables to
    /// fill [Config::plugin_statedir] and [Config::dirtyconfig].
    /// [Config::plugin_statedir] falls back to _/tmp_ if no munin
    /// environment variables are present.
    fn default() -> Self {
        let statedir = Config::get_statedir();
        let insert: String = repeat_with(fastrand::alphanumeric).take(10).collect();
        let cachename = Path::new(&statedir).join(format!("munin.{}.value", insert));
        Self {
            plugin_name: String::from("Simple munin plugin in Rust"),
            plugin_statedir: statedir.clone(),
            plugin_cache: cachename,
            dirtyconfig: match env::var("MUNIN_CAP_DIRTYCONFIG") {
                Ok(val) => val.eq(&"1"),
                Err(_) => false,
            },
            daemonize: false,
            pidfile: statedir.join("munin-plugin.pid"),
            config_size: 8192,
            fetch_size: 8192,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_modconfig() {
        // Whole set of defaults
        let config = Config {
            ..Default::default()
        };
        assert_eq!(
            config.plugin_name,
            String::from("Simple munin plugin in Rust")
        );

        // Use defaults (except for name)
        let mut config2 = Config {
            plugin_name: String::from("Lala"),
            ..Default::default()
        };
        // Is plugin name as given?
        assert_eq!(config2.plugin_name, String::from("Lala"));
        // Defaults as expected?
        assert!(!config2.daemonize);
        assert_eq!(config2.fetch_size, 8192);

        config2.pidfile = PathBuf::new();
        config2.pidfile.push(&config2.plugin_statedir);
        config2.pidfile.push(String::from("Lala.pid"));

        let config3 = Config::new(String::from("Lala"));
        // At this point, the plugin_cache should be different
        assert_ne!(config2, config3);
        config2.plugin_cache = config2.plugin_statedir.join("munin.Lala.value");
        assert_eq!(config2, config3);
    }

    #[test]
    fn test_new_daemon() {
        let config = Config::new_daemon(String::from("great-plugin"));
        assert_eq!(config.plugin_name, String::from("great-plugin"));
        assert_eq!(
            config.plugin_cache,
            PathBuf::from(String::from("/tmp/munin.great-plugin.value"))
        );
        assert!(config.daemonize);
    }
}
