//! Rising Lands Windows client.
//!
//! `host/` owns the portable orchestration ported from `macos_live_driver.c`:
//! frame phases, input mapping, pacing, viewport geometry, durable saves and
//! the event transport. `platform/` replaces the AppKit/AVFoundation layer with
//! Win32, Direct2D and WASAPI. Tests exercise the portable pieces without a
//! window, a device or any game data.

// Link the ABI implementation; its symbols are referenced through `abi`.
use rising_engine as _;

pub mod abi;
pub mod host;
pub mod platform;
