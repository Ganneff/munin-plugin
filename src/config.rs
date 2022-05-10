//! Config data for a munin plugin
use log::trace;
use std::{env, path::PathBuf};

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
    pub cfgsize: usize,

    /// Size of buffer for BufWriter for [MuninPlugin::fetch](super::MuninPlugin::fetch).
    ///
    /// Defaults to 8192, but if the plugin outputs large datasets, it
    /// is useful to increase this.
    pub fetchsize: usize,
}

impl Config {
    /// Create a new Config with defined plugin_name, also setting
    /// [Config::pidfile] to a sensible value using the [Config::plugin_name].
    ///
    /// # Examples
    ///
    /// ```
    /// # use munin_plugin::config::Config;
    /// let config = Config::new(String::from("great-plugin"));
    /// println!("My pidfile is {:?}", config.pidfile);
    /// ```
    pub fn new(plugin_name: String) -> Self {
        trace!("Creating new config for plugin {plugin_name}");
        let pd = plugin_name.clone();
        let mut cfg = Self {
            plugin_name,
            ..Default::default()
        };
        cfg.pidfile = cfg.plugin_statedir.join(pd + ".pid");
        cfg
    }
}

/// Useful defaults, if possible based on munin environment.
impl Default for Config {
    /// Set default values, try to read munin environment variables to fill [Config::plugin_statedir] and [Config::dirtyconfig]. [Config::plugin_statedir] falls back to _/tmp_ if no munin environment variables are present.
    fn default() -> Self {
        let statedir =
            PathBuf::from(env::var("MUNIN_PLUGSTATE").unwrap_or_else(|_| String::from("/tmp")));
        Self {
            plugin_name: String::from("Simple munin plugin in Rust"),
            plugin_statedir: statedir.clone(),
            dirtyconfig: match env::var("MUNIN_CAP_DIRTYCONFIG") {
                Ok(val) => val.eq(&"1"),
                Err(_) => false,
            },
            daemonize: false,
            pidfile: statedir.join("munin-plugin.pid"),
            cfgsize: 8192,
            fetchsize: 8192,
        }
    }
}
