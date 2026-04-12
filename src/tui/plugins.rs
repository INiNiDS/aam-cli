// SPDX-FileCopyrightText: 2026 Nikita Goncharov
// SPDX-License-Identifier: GPL-3.0-or-later

use anyhow::Result;
use std::fmt::Debug;

pub trait Plugin: Debug + Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;

    // Lifecycle hooks
    /// # Errorsrr
    /// Returns an error if plugin initialization fails.
    fn on_init(&mut self) -> Result<()> {
        Ok(())
    }
    /// # Errors
    /// Returns an error if command handling fails.
    fn on_command(&mut self, _command: &str, _args: &[&str]) -> Result<bool> {
        Ok(false)
    } // Returns true if handled
}

#[derive(Default)]
pub struct PluginManager {
    plugins: Vec<Box<dyn Plugin>>,
}

impl PluginManager {
    #[must_use] 
    pub fn new() -> Self {
        Self {
            plugins: Vec::new(),
        }
    }

    /// # Errors
    /// Returns an error if the plugin fails to initialize.
    pub fn register<P: Plugin + 'static>(&mut self, mut plugin: P) -> Result<()> {
        plugin.on_init()?;
        self.plugins.push(Box::new(plugin));
        Ok(())
    }

    /// # Errors
    /// Returns an error if command handling fails.
    pub fn handle_command(&mut self, command: &str, args: &[&str]) -> Result<bool> {
        for plugin in &mut self.plugins {
            if plugin.on_command(command, args)? {
                return Ok(true);
            }
        }
        Ok(false)
    }
}
