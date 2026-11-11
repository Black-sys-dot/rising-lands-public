//! Input mapping, ported from `macos_live_driver.c` and the hotkey table.
//!
//! The engine consumes Win32-style messages: `WM_KEYDOWN`/`WM_KEYUP` with the
//! original scan codes and `lParam` bits, and `WM_MOUSEMOVE`..`WM_RBUTTONUP`
//! (`0x200`..`0x205`) in original 800x600 pixels. Physical Windows keys already
//! are virtual keys, but the same 23-key whitelist is kept, and F11 (VK `0x7a`)
//! is never delivered: the renderer composes only 800x600.

use crate::abi::RlInputMessage;

pub const WM_MOUSEMOVE: u32 = 0x0200;
pub const WM_LBUTTONDOWN: u32 = 0x0201;
pub const WM_LBUTTONUP: u32 = 0x0202;
pub const WM_LBUTTONDBLCLK: u32 = 0x0203;
pub const WM_RBUTTONDOWN: u32 = 0x0204;
pub const WM_RBUTTONUP: u32 = 0x0205;
pub const WM_KEYDOWN: u32 = 0x0100;
pub const WM_KEYUP: u32 = 0x0101;

pub const VK_RETURN: u32 = 13;
pub const VK_SHIFT: u32 = 16;
pub const VK_CONTROL: u32 = 17;
pub const VK_MENU: u32 = 18;
pub const VK_ESCAPE: u32 = 27;
pub const VK_SPACE: u32 = 32;
pub const VK_LEFT: u32 = 37;
pub const VK_UP: u32 = 38;
pub const VK_RIGHT: u32 = 39;
pub const VK_DOWN: u32 = 40;
pub const VK_DELETE: u32 = 46;
pub const VK_F11: u32 = 0x7a;
pub const VK_X: u32 = 88;
pub const VK_TAB: u32 = 9;

/// The original input table, in the exact order used by the scan-code table.
pub const ACCEPTED_VKS: [u32; 23] = [
    VK_RETURN,
    VK_SHIFT,
    VK_CONTROL,
    VK_MENU,
    VK_ESCAPE,
    VK_SPACE,
    VK_LEFT,
    VK_UP,
    VK_RIGHT,
    VK_DOWN,
    VK_DELETE,
    VK_X,
    48,
    49,
    50,
    51,
    52,
    53,
    54,
    55,
    56,
    57,
    VK_TAB,
];

/// Original IBM scan codes for `ACCEPTED_VKS`.
pub const SCAN_CODES: [u32; 23] = [
    0x1c, 0x2a, 0x1d, 0x38, 0x01, 0x39, 0x4b, 0x48, 0x4d, 0x50, 0x53, 0x2d, 0x0b, 0x02, 0x03,
    0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0f,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputError {
    InvalidMouseEvent,
    UnsupportedKey,
    QueueCapacityExceeded,
    UnsupportedEvent,
}

/// One scheduled message. `cursor` marks the packets that move the retained
/// host pointer (the initial move and the release), as the C driver does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct QueuedInput {
    pub frame: u64,
    pub message: RlInputMessage,
    pub cursor: bool,
}

pub const QUEUE_CAPACITY: usize = 4096;

pub fn scan_code(vk: u32) -> Option<u32> {
    ACCEPTED_VKS
        .iter()
        .position(|&accepted| accepted == vk)
        .map(|index| SCAN_CODES[index])
}

pub fn is_accepted(vk: u32) -> bool {
    scan_code(vk).is_some()
}

fn is_extended(vk: u32) -> bool {
    matches!(vk, VK_LEFT | VK_UP | VK_RIGHT | VK_DOWN | VK_DELETE)
}

/// Builds the `lParam` of one key message for the *next* state change.
pub fn key_lparam(vk: u32, down: bool, previously_down: bool, control_down: bool) -> Option<u32> {
    let scan = scan_code(vk)?;
    Some(
        1 | (scan << 16)
            | ((is_extended(vk) as u32) << 24)
            | ((control_down as u32) << 29)
            | ((previously_down as u32) << 30)
            | ((!down as u32) << 31),
    )
}

/// Keyboard state for the whitelist. Repeats are accepted; the caller decides
/// whether a repeat matters.
#[derive(Clone, Debug)]
pub struct Keyboard {
    held: [i32; 256],
}

impl Default for Keyboard {
    fn default() -> Self {
        Self { held: [0; 256] }
    }
}

impl Keyboard {
    pub fn held(&self, vk: u32) -> i32 {
        self.held.get(vk as usize).copied().unwrap_or(0)
    }

    pub fn is_held(&self, vk: u32) -> bool {
        self.held(vk) != 0
    }

    /// Schedules one `WM_KEYDOWN`/`WM_KEYUP`. Returns the queued input.
    pub fn key(
        &mut self,
        vk: u32,
        down: bool,
        frame: u64,
        control_down: bool,
    ) -> Result<QueuedInput, InputError> {
        if !is_accepted(vk) {
            return Err(InputError::UnsupportedKey);
        }
        let previous = self.held(vk) != 0;
        let lparam =
            key_lparam(vk, down, previous, control_down).ok_or(InputError::UnsupportedKey)?;
        self.held[vk as usize] = if down { -32768 } else { 0 };
        Ok(QueuedInput {
            frame,
            message: RlInputMessage {
                message: if down { WM_KEYDOWN } else { WM_KEYUP },
                wparam: vk,
                lparam,
            },
            cursor: false,
        })
    }

    /// Every held key, optionally excluding Tab and the digits. The original
    /// phase-2 pump reports held keys except Tab (WndProc-only) and 0..9.
    pub fn held_keys(&self, phase2: bool) -> Vec<(u32, i32)> {
        let mut keys = Vec::new();
        for vk in 0..256u32 {
            if self.held[vk as usize] == 0 {
                continue;
            }
            if phase2 && ((48..=57).contains(&vk) || vk == VK_TAB) {
                continue;
            }
            keys.push((vk, self.held[vk as usize]));
        }
        keys
    }

    pub fn clear(&mut self) {
        self.held = [0; 256];
    }
}

/// Left-button drag state and the scheduled message queue.
#[derive(Clone, Debug, Default)]
pub struct MouseQueue {
    pub left_held: bool,
    pub left_down_frame: u64,
    pub mouse_until: u64,
    pub left_x: i32,
    pub left_y: i32,
    pending: Vec<QueuedInput>,
}

impl MouseQueue {
    pub fn pending(&self) -> &[QueuedInput] {
        &self.pending
    }

    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    pub fn clear(&mut self) {
        self.pending.clear();
        self.left_held = false;
        self.left_down_frame = 0;
        self.mouse_until = 0;
        self.left_x = 0;
        self.left_y = 0;
    }

    /// Ends a held press at its last pixel (focus loss, window close).
    pub fn release_at_last(&mut self, frame: u64) -> Option<QueuedInput> {
        if !self.left_held {
            return None;
        }
        let queued = self.push_release(self.left_x, self.left_y, frame.max(self.mouse_until), 0);
        self.left_held = false;
        Some(queued)
    }

    /// Appends a packet produced outside the mouse gesture (a key message).
    pub fn schedule(&mut self, queued: QueuedInput) -> Result<(), InputError> {
        if self.pending.len() >= QUEUE_CAPACITY {
            return Err(InputError::QueueCapacityExceeded);
        }
        self.pending.push(queued);
        Ok(())
    }

    fn push(&mut self, frame: u64, message: RlInputMessage, cursor: bool) -> QueuedInput {
        let queued = QueuedInput {
            frame,
            message,
            cursor,
        };
        self.pending.push(queued);
        queued
    }

    fn push_release(&mut self, x: i32, y: i32, frame: u64, wparam: u32) -> QueuedInput {
        let lp = ((y as u32) << 16) | x as u32;
        let queued = self.push(
            frame,
            RlInputMessage {
                message: WM_LBUTTONUP,
                wparam: wparam & !3,
                lparam: lp,
            },
            true,
        );
        self.mouse_until = frame;
        self.left_x = x;
        self.left_y = y;
        queued
    }

    /// Schedules the host mouse event exactly as the validated macOS driver:
    /// a press is held across frames until the real release; the fixed click
    /// gesture still spans k, k+1 and k+3; right clicks ignore the real release.
    pub fn mouse(
        &mut self,
        message: u32,
        x: i32,
        y: i32,
        wparam: u32,
        frame: u64,
    ) -> Result<Vec<QueuedInput>, InputError> {
        if !(WM_MOUSEMOVE..=WM_RBUTTONUP).contains(&message)
            || !(0..800).contains(&x)
            || !(0..600).contains(&y)
        {
            return Err(InputError::InvalidMouseEvent);
        }
        if message == WM_RBUTTONUP || (message == WM_LBUTTONUP && !self.left_held) {
            return Ok(Vec::new());
        }
        if self.pending.len() + 3 > QUEUE_CAPACITY {
            return Err(InputError::QueueCapacityExceeded);
        }
        let mut emitted = Vec::with_capacity(3);
        let left = message == WM_LBUTTONDOWN || message == WM_LBUTTONDBLCLK;
        let release = message == WM_LBUTTONUP;
        let count = if message == WM_MOUSEMOVE || release {
            1
        } else if left {
            2
        } else {
            3
        };
        let mut frame = frame;
        if left && self.left_held {
            // A press without a delivered host release closes the old press first.
            emitted.push(self.push_release(self.left_x, self.left_y, frame, 0));
            self.left_held = false;
        }
        if frame <= self.mouse_until {
            frame = self.mouse_until + 1;
        }
        let lp = ((y as u32) << 16) | x as u32;
        let button = u32::from(self.left_held);
        if release {
            if frame < self.left_down_frame + 2 {
                frame = self.left_down_frame + 2;
            }
            emitted.push(self.push(
                frame,
                RlInputMessage {
                    message: WM_LBUTTONUP,
                    wparam: wparam & !3,
                    lparam: lp,
                },
                true,
            ));
            self.mouse_until = frame;
            self.left_held = false;
            self.left_x = x;
            self.left_y = y;
        } else {
            emitted.push(self.push(
                frame,
                RlInputMessage {
                    message: WM_MOUSEMOVE,
                    wparam: (wparam & !3) | button,
                    lparam: lp,
                },
                true,
            ));
            if left {
                emitted.push(self.push(
                    frame + 1,
                    RlInputMessage {
                        message,
                        wparam: (wparam & !3) | 1,
                        lparam: lp,
                    },
                    false,
                ));
                self.mouse_until = frame + 1;
                self.left_down_frame = frame + 1;
                self.left_held = true;
                self.left_x = x;
                self.left_y = y;
            } else if count == 3 {
                emitted.push(self.push(
                    frame + 1,
                    RlInputMessage {
                        message,
                        wparam: (wparam & !3) | 2 | button,
                        lparam: lp,
                    },
                    false,
                ));
                emitted.push(self.push(
                    frame + 3,
                    RlInputMessage {
                        message: WM_RBUTTONUP,
                        wparam: (wparam & !3) | button,
                        lparam: lp,
                    },
                    false,
                ));
                self.mouse_until = frame + 3;
            } else if button != 0 {
                self.left_x = x;
                self.left_y = y;
            }
        }
        Ok(emitted)
    }

    /// Removes every message due at `frame`, in order, updating the retained
    /// host cursor for packets that carry one.
    pub fn take_due(&mut self, frame: u64, cursor: &mut (i32, i32)) -> Vec<RlInputMessage> {
        let mut due = Vec::new();
        let mut keep = Vec::with_capacity(self.pending.len());
        for queued in self.pending.drain(..) {
            if queued.frame <= frame {
                if queued.cursor {
                    cursor.0 = queued.message.lparam as u16 as i32;
                    cursor.1 = (queued.message.lparam >> 16) as u16 as i32;
                }
                due.push(queued.message);
            } else {
                keep.push(queued);
            }
        }
        self.pending = keep;
        due
    }
}
