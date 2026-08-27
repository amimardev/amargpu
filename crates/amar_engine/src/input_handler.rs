use std::collections::HashSet;
use winit::dpi::PhysicalPosition;
use winit::event::MouseButton;
use winit::keyboard::KeyCode;

/// system generation of pressed events can be slower then frame rendering speeds, 
/// so we should keep the state of pressed active untill released event is captured
#[derive(Default)]
pub struct InputHandler {
    keys_held: HashSet<KeyCode>,
    keys_pressed_this_frame: Vec<KeyCode>,
    keys_released_this_frame: Vec<KeyCode>,

    mouse_held: HashSet<MouseButton>,
    mouse_pressed_this_frame: Vec<MouseButton>,
    mouse_released_this_frame: Vec<MouseButton>,

    mouse_pos: PhysicalPosition<f64>,
    mouse_delta: (f64, f64),
    last_frame_mouse_pos: Option<PhysicalPosition<f64>>,
}

impl InputHandler {
    pub fn handle_key(&mut self, code: KeyCode, is_pressed: bool) {
        if is_pressed {
            if self.keys_held.insert(code) {
                // only fires the "just pressed" event on the transition,
                // not on OS key-repeat (winit repeats held keys)
                self.keys_pressed_this_frame.push(code); 
            }
        } else {
            self.keys_held.remove(&code);
            self.keys_released_this_frame.push(code);
        }
    }

    pub fn handle_mouse_key(&mut self, button: MouseButton, is_pressed: bool) {
        if is_pressed {
            if self.mouse_held.insert(button) {
                self.mouse_pressed_this_frame.push(button);
            }
        } else {
            self.mouse_held.remove(&button);
            self.mouse_released_this_frame.push(button);
        }
    }

    pub fn handle_mouse_moved(&mut self, position: PhysicalPosition<f64>) {
        self.mouse_pos = position;
    }

    // --- queries, used during update/game logic ---

    pub fn is_key_held(&self, code: KeyCode) -> bool {
        self.keys_held.contains(&code)
    }

    /// returns true if key went from released to pressed this frame
    pub fn was_key_just_pressed(&self, code: KeyCode) -> bool {
        self.keys_pressed_this_frame.contains(&code)
    }

    pub fn was_key_released(&self, code: KeyCode) -> bool {
        self.keys_released_this_frame.contains(&code)
    }

    pub fn mouse_position(&self) -> PhysicalPosition<f64> {
        self.mouse_pos
    }

    pub fn mouse_delta(&self) -> (f64, f64) {
        self.mouse_delta
    }

    // --- call once per frame, AFTER game logic has read this frame's events ---

    pub fn flush(&mut self) {
        self.keys_pressed_this_frame.clear();
        self.keys_released_this_frame.clear();
        self.mouse_pressed_this_frame.clear();
        self.mouse_released_this_frame.clear();

        self.mouse_delta = match self.last_frame_mouse_pos {
            Some(prev) => (self.mouse_pos.x - prev.x, self.mouse_pos.y - prev.y),
            None => (0.0, 0.0),
        };
        self.last_frame_mouse_pos = Some(self.mouse_pos);
    }
}
