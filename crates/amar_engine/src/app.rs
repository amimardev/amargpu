use std::sync::Arc;

use crate::{
    input_handler::InputHandler,
    plugins::{Plugin, PluginHandler},
    registry::static_ptr::StaticRef,
    state::State,
};
use winit::{
    application::ApplicationHandler,
    event::*,
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::PhysicalKey,
    window::Window,
};

pub struct App {
    state: Option<State>,
    plugins: Option<Vec<Box<dyn Plugin>>>,
}

impl App {
    pub fn new() -> Self {
        Self {
            state: None,
            plugins: Some(vec![]),
        }
    }
    pub fn register<P: Plugin + 'static>(mut self, plugin: P) -> Self {
        self.plugins
            .get_or_insert_with(Vec::new)
            .push(Box::new(plugin));
        self
    }
    pub fn run(mut self) -> anyhow::Result<()> {
        env_logger::init();

        let event_loop = EventLoop::with_user_event().build()?;

        event_loop.run_app(&mut self)?;

        Ok(())
    }
}

impl ApplicationHandler<State> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        #[allow(unused_mut)]
        let mut window_attributes = Window::default_attributes();

        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());

        let mut state = pollster::block_on(State::new(
            window,
            self.plugins.take().unwrap_or(Vec::new()),
        ))
        .unwrap();
        state.init_all();
        self.state = Some(state);
    }

    #[allow(unused_mut)]
    fn user_event(&mut self, _event_loop: &ActiveEventLoop, mut event: State) {
        self.state = Some(event);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        let state = match &mut self.state {
            Some(canvas) => canvas,
            None => return,
        };

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => state.handle_resize(size.width, size.height),
            WindowEvent::RedrawRequested => {
                state
                    .registry
                    .insert_res(unsafe { StaticRef::new(event_loop) });
                state.run_update_all();
                state
                    .registry
                    .get_res_mut::<InputHandler>()
                    .map(|i_h| i_h.flush());
                
                match state.render() {
                    Ok(_) => {}
                    Err(e) => {
                        // Log the error and exit gracefully
                        log::error!("{e}");
                        event_loop.exit();
                    }
                }
                state.registry.remove_res::<StaticRef<ActiveEventLoop>>();
            }
            WindowEvent::CursorMoved { position, .. } => {
                state
                    .registry
                    .get_res_mut::<InputHandler>()
                    .map(|i_h| i_h.handle_mouse_moved(position));
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(code),
                        state: key_state,
                        ..
                    },
                ..
            } => {
                state
                    .registry
                    .get_res_mut::<InputHandler>()
                    .map(|i_h| i_h.handle_key(code, key_state.is_pressed()));
            }
            WindowEvent::MouseInput {
                state: key_state,
                button: code,
                ..
            } => {
                state
                    .registry
                    .get_res_mut::<InputHandler>()
                    .map(|i_h| i_h.handle_mouse_key(code, key_state.is_pressed()));
            }
            _ => {}
        }
    }
}
