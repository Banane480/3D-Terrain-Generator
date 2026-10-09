use std::collections::HashSet;
use winit::keyboard::KeyCode;

#[derive(Default)]
pub struct InputState {
    pub keys_pressed: HashSet<KeyCode>,
    pub mouse_delta: (f32, f32),
    pub cursor_locked: bool,
    pub toggle_freecam_requested: bool,
    pub toggle_invert_y_requested: bool,
}

impl InputState {
    pub fn new() -> Self {
        Self {
            keys_pressed: HashSet::new(),
            mouse_delta: (0.0, 0.0),
            cursor_locked: true,
            toggle_freecam_requested: false,
            toggle_invert_y_requested: false,
        }
    }

    pub fn on_key_event(&mut self, key: KeyCode, pressed: bool) {
        if pressed {
            self.keys_pressed.insert(key);
            if key == KeyCode::KeyF {
                self.toggle_freecam_requested = true;
            } else if key == KeyCode::KeyI {
                self.toggle_invert_y_requested = true;
            }
        } else {
            self.keys_pressed.remove(&key);
        }
    }

    pub fn on_mouse_move(&mut self, dx: f64, dy: f64) {
        if self.cursor_locked {
            self.mouse_delta.0 += dx as f32;
            self.mouse_delta.1 += dy as f32;
        }
    }

    pub fn is_key_down(&self, key: KeyCode) -> bool {
        self.keys_pressed.contains(&key)
    }

    // Support des claviers ZQSD (AZERTY) et WASD (QWERTY)
    pub fn is_forward(&self) -> bool {
        self.is_key_down(KeyCode::KeyW) || self.is_key_down(KeyCode::KeyZ)
    }

    pub fn is_backward(&self) -> bool {
        self.is_key_down(KeyCode::KeyS)
    }

    pub fn is_left(&self) -> bool {
        self.is_key_down(KeyCode::KeyA) || self.is_key_down(KeyCode::KeyQ)
    }

    pub fn is_right(&self) -> bool {
        self.is_key_down(KeyCode::KeyD)
    }

    pub fn is_jump(&self) -> bool {
        self.is_key_down(KeyCode::Space)
    }

    pub fn is_sprint(&self) -> bool {
        self.is_key_down(KeyCode::ShiftLeft) || self.is_key_down(KeyCode::ShiftRight)
    }

    pub fn is_crouch(&self) -> bool {
        self.is_key_down(KeyCode::ControlLeft) || self.is_key_down(KeyCode::KeyC)
    }

    pub fn pop_mouse_delta(&mut self) -> (f32, f32) {
        let delta = self.mouse_delta;
        self.mouse_delta = (0.0, 0.0);
        delta
    }

    pub fn pop_freecam_toggle(&mut self) -> bool {
        let val = self.toggle_freecam_requested;
        self.toggle_freecam_requested = false;
        val
    }

    pub fn pop_invert_y_toggle(&mut self) -> bool {
        let val = self.toggle_invert_y_requested;
        self.toggle_invert_y_requested = false;
        val
    }
}
