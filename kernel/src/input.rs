//! Input subsystem for RaeenOS
//! Provides real-time input processing for keyboard, mouse, and other input devices

use crate::drivers;
use crate::graphics;
use crate::slo::{with_slo_harness, SloCategory};
use crate::slo_measure;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use alloc::string::ToString;
use spin::Mutex;
use lazy_static::lazy_static;

// Input latency tracking for SLO measurements
lazy_static! {
    static ref INPUT_TIMESTAMPS: Mutex<BTreeMap<u8, u64>> = Mutex::new(BTreeMap::new());
    static ref LATENCY_SAMPLES: Mutex<Vec<f64>> = Mutex::new(Vec::new());
}

/// Record timestamp when keyboard interrupt occurs
pub fn record_input_interrupt_timestamp(scancode: u8, timestamp_ns: u64) {
    let mut timestamps = INPUT_TIMESTAMPS.lock();
    timestamps.insert(scancode, timestamp_ns);
}

/// Record input latency measurement when event is delivered to application
fn record_input_latency(scancode: u8) {
    let delivery_time = crate::time::get_timestamp_ns();
    
    if let Some(interrupt_time) = INPUT_TIMESTAMPS.lock().remove(&scancode) {
        let latency_ns = delivery_time.saturating_sub(interrupt_time);
        let latency_us = latency_ns as f64 / 1000.0; // Convert to microseconds
        
        // Collect samples for periodic SLO measurement
        let mut samples = LATENCY_SAMPLES.lock();
        samples.push(latency_us);
        
        // Report to SLO harness every 100 samples
        if samples.len() >= 100 {
            let sample_data = samples.clone();
            samples.clear();
            
            // Record SLO measurement
            with_slo_harness(|harness| {
                slo_measure!(
                    harness,
                    SloCategory::InputLatency,
                    "keyboard_interrupt_to_delivery",
                    "microseconds",
                    sample_data.len() as u64,
                    sample_data
                );
            });
        }
    }
}

/// Process input events for real-time input thread
/// This function is called by the input RT thread to handle low-latency input processing
pub fn process_input_events() {
    // Process keyboard events with improved handling
    while let Some(key) = drivers::keyboard::get_key() {
        let pressed = (key & 0x80) == 0; // Check if key is pressed or released
        let key_code = (key & 0x7F) as u32; // Remove press/release bit
        
        // Enhanced keyboard event routing
        route_keyboard_event(key_code, pressed);
    }
    
    // Process mouse events with improved tracking
    if let Some((x, y, buttons)) = drivers::mouse::get_mouse_state() {
        // Enhanced mouse state tracking
        static mut LAST_MOUSE_STATE: (i32, i32, u8) = (0, 0, 0);
        unsafe {
            let (last_x, last_y, last_buttons) = LAST_MOUSE_STATE;
            
            // Handle mouse movement
            if x != last_x || y != last_y {
                route_mouse_move_event(x, y, last_x, last_y);
            }
            
            // Handle button state changes
            if buttons != last_buttons {
                for button in 0..3 {
                    let button_mask = 1 << button;
                    let was_pressed = (last_buttons & button_mask) != 0;
                    let is_pressed = (buttons & button_mask) != 0;
                    
                    if was_pressed != is_pressed {
                        route_mouse_button_event(x, y, button, is_pressed);
                    }
                }
            }
            
            LAST_MOUSE_STATE = (x, y, buttons);
        }
    }
}

/// Enhanced keyboard event routing with focus management
fn route_keyboard_event(key_code: u32, pressed: bool) {
    // Record input latency measurement for pressed keys
    if pressed {
        record_input_latency(key_code as u8);
    }
    
    // Handle global hotkeys first
    if pressed {
        match key_code {
            0x3B => { // F1 - Show help
                graphics::show_help_overlay();
                return;
            }
            0x3C => { // F2 - Toggle performance overlay
                graphics::toggle_performance_overlay();
                return;
            }
            0x3D => { // F3 - Open task manager
                match graphics::create_task_manager_window() {
                    Ok(_window_id) => {
                        crate::serial::_print(format_args!("[Input] Opened task manager\n"));
                    }
                    Err(e) => {
                        crate::serial::_print(format_args!("[Input] Failed to open task manager: {}\n", e));
                    }
                }
                return;
            }
            0x01 => { // ESC - Cancel current operation
                graphics::cancel_current_operation();
                return;
            }
            _ => {}
        }
    }
    
    // Route to focused window
    graphics::handle_keyboard_event(key_code, pressed);
}

/// Enhanced mouse movement event routing
fn route_mouse_move_event(x: i32, y: i32, last_x: i32, last_y: i32) {
    // Calculate movement delta
    let delta_x = x - last_x;
    let delta_y = y - last_y;
    
    // Update cursor position
    graphics::update_cursor_position(x, y);
    
    // Handle window dragging if in drag mode
    graphics::handle_window_drag(x, y, delta_x, delta_y);
    
    // Route hover events to windows
    graphics::handle_mouse_hover(x, y);
}

/// Enhanced mouse button event routing
fn route_mouse_button_event(x: i32, y: i32, button: u8, pressed: bool) {
    if pressed {
        // Handle window focus on click
        if let Some(window_id) = graphics::get_window_at_point(x, y) {
            let _ = graphics::set_input_focus(window_id);
        }
        
        // Check for window title bar clicks (for dragging)
        if button == 0 { // Left mouse button
            graphics::start_window_drag_if_title_bar(x, y);
        }
    } else {
        // Handle drag end
        if button == 0 {
            graphics::end_window_drag();
        }
    }
    
    // Route to appropriate window
    graphics::handle_mouse_event(x, y, button, pressed);
}