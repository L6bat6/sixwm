







mod state;
mod handlers;

use state::{SixWM, ClientState};
use smithay::{
    backend::{
        renderer::{
            damage::OutputDamageTracker,
            gles::GlesRenderer,
            utils::on_commit_buffer_handler,
            element::surface::WaylandSurfaceRenderElement,
        },
        winit::{self, WinitEvent},
        input::{
            InputEvent,
            ButtonState,
            AbsolutePositionEvent,
            PointerButtonEvent,
            KeyboardKeyEvent,
            Event,
        }, 
    },
    input::{
        Seat, SeatState, SeatHandler, 
        keyboard::{XkbConfig, FilterResult, Keysym}, 
        pointer::{CursorImageStatus, ButtonEvent},
    },
    delegate_compositor, delegate_shm, delegate_seat,
    desktop::{Space, space::render_output},
    output::{Output, PhysicalProperties, Mode, Subpixel, Scale},
    reexports::{
        calloop::EventLoop,
        wayland_server::{
            protocol::{wl_surface::WlSurface},
            Display, Client, ListeningSocket,
        },
    },
    // Importante: rola
    wayland::{
        seat::WaylandFocus, 
        buffer::BufferHandler,
        compositor::{CompositorClientState, CompositorHandler, CompositorState},
        shm::{ShmHandler, ShmState},
        shell::xdg::XdgShellState,
    },
    utils::{Transform, Serial},
};
use std::time::Duration;
use std::process::Command;

// Confia
const TERM_CMD: &str = "foot"; 
const MENU_CMD: &[&str] = &["bemenu-run", "-p", "Run:"];

/
enum KeyAction {
    None,
    Quit,
    Close,
    Run(String),
    RunArgs(String, Vec<String>),
}

impl SeatHandler for SixWM {
    type KeyboardFocus = WlSurface;
    type PointerFocus = WlSurface;
    type TouchFocus = WlSurface;
    fn seat_state(&mut self) -> &mut SeatState<Self> { &mut self.seat_state }
    fn focus_changed(&mut self, _seat: &Seat<Self>, _focused: Option<&WlSurface>) {}
    fn cursor_image(&mut self, _seat: &Seat<Self>, _image: CursorImageStatus) {}
}

impl CompositorHandler for SixWM {
    fn compositor_state(&mut self) -> &mut CompositorState { &mut self.compositor_state }
    fn client_compositor_state<'a>(&self, c: &'a Client) -> &'a CompositorClientState {
        &c.get_data::<ClientState>().unwrap().compositor_state
    }
    fn commit(&mut self, surface: &WlSurface) { on_commit_buffer_handler::<Self>(surface); }
}

impl ShmHandler for SixWM { fn shm_state(&self) -> &ShmState { &self.shm_state } }
impl BufferHandler for SixWM { fn buffer_destroyed(&mut self, _: &smithay::reexports::wayland_server::protocol::wl_buffer::WlBuffer) {} }

delegate_compositor!(SixWM);
delegate_shm!(SixWM);
delegate_seat!(SixWM);

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt().init();

    let mut event_loop: EventLoop<SixWM> = EventLoop::try_new()?;
    let mut display: Display<SixWM> = Display::new()?;
    let display_handle = display.handle();

    let mut seat_state = SeatState::new();
    let mut seat = seat_state.new_wl_seat(&display_handle, "winit");

    let xkb_config = XkbConfig { layout: "us", variant: "intl", ..XkbConfig::default() };
    seat.add_keyboard(xkb_config, 180, 45).expect("Erro ao iniciar teclado");
    seat.add_pointer();

    let mut state = SixWM {
        compositor_state: CompositorState::new::<SixWM>(&display_handle),
        shm_state: ShmState::new::<SixWM>(&display_handle, vec![]),
        xdg_shell_state: XdgShellState::new::<SixWM>(&display_handle),
        seat_state,
        seat,
        display_handle: display_handle.clone(),
        space: Space::default(),
        pointer_location: (0.0, 0.0).into(),
        width: 1280,
        height: 720,
        grabbed_window: None,
    };

    let output = Output::new("winit".into(), PhysicalProperties {
        size: (0,0).into(), subpixel: Subpixel::Unknown, make: "Smithay".into(),
        model: "Winit".into(), serial_number: "001".into(), 
    });
    let mode = Mode { size: (1280, 720).into(), refresh: 60_000 };
    output.change_current_state(Some(mode), Some(Transform::Normal), Some(Scale::Fractional(1.0)), Some((0,0).into()));
    state.space.map_output(&output, (0,0));

    let mut damage_tracker = OutputDamageTracker::from_output(&output);
    let listener = ListeningSocket::bind_auto("wayland", 1..32)?;
    let socket_name = listener.socket_name().unwrap().to_string_lossy().into_owned();

    println!("SixWM rodando na socket: {}!", socket_name);

    let (mut backend, winit) = winit::init::<GlesRenderer>()?;
    let output_cloned = output.clone();

    event_loop.handle().insert_source(winit, move |event, _, state| {
        match event {
            WinitEvent::CloseRequested => std::process::exit(0),
            WinitEvent::Resized { size, .. } => {
                state.width = size.w as u32;
                state.height = size.h as u32;
                let mode = Mode { size: (size.w, size.h).into(), refresh: 60_000 };
                output_cloned.change_current_state(Some(mode), None, None, None);
            },
            WinitEvent::Input(ev) => match ev {
                InputEvent::PointerMotionAbsolute { event, .. } => {
                    let pos = event.position_transformed((state.width as i32, state.height as i32).into()); 
                    state.pointer_location = pos;
                    let serial = Serial::from(0);
                    let pointer = state.seat.get_pointer().unwrap();

                    if let Some((window, offset)) = &state.grabbed_window {
                        state.space.map_element(window.clone(), (pos - *offset).to_i32_round(), true);
                    }

                    pointer.motion(state, None, &smithay::input::pointer::MotionEvent { location: pos, serial, time: 0 });

                    if state.grabbed_window.is_none() {
                        if let Some((window, loc)) = state.space.element_under(pos).map(|(w, l)| (w.clone(), l)) {
                            if let Some(surface) = window.wl_surface() {
                                let surface = surface.into_owned();
                                state.seat.get_keyboard().unwrap().set_focus(state, Some(surface.clone()), serial);
                                pointer.motion(state, Some((surface, (pos - loc.to_f64()).to_i32_round())), &smithay::input::pointer::MotionEvent { location: pos, serial, time: 0 });
                            }
                        }
                    }
                },
                
                InputEvent::PointerButton { event, .. } => {
                     let pointer = state.seat.get_pointer().unwrap();
                     let serial = Serial::from(0);
                     let button = event.button_code();
                     let state_btn = event.state();
                     let modifiers = state.seat.get_keyboard().unwrap().modifier_state();

                     // + Alt = Mover Janela
                     if button == 272 && state_btn == ButtonState::Pressed {
                        let pos = state.pointer_location;
                        if let Some((window, loc)) = state.space.element_under(pos).map(|(w, l)| (w.clone(), l)) {
                            state.space.raise_element(&window, true);
                            if modifiers.alt {
                                state.grabbed_window = Some((window, pos - loc.to_f64()));
                            }
                        }
                     } else if button == 272 && state_btn == ButtonState::Released {
                        state.grabbed_window = None;
                     }

                     pointer.button(state, &ButtonEvent { button, state: state_btn, serial, time: 0 });
                },

                InputEvent::Keyboard { event, .. } => {
                    let keyboard = state.seat.get_keyboard().unwrap();
                    let serial = Serial::from(0);
                    let time = event.time();
                    let key_code = event.key_code();
                    let key_state = event.state();

                    // (retorna o enum KeyAction)
                    let action = keyboard.input(state, key_code, key_state, serial, time as u32, |_, modifiers, handle| {
                        if key_state == smithay::backend::input::KeyState::Pressed && modifiers.alt {
                            let sym = handle.raw_syms().get(0).cloned().unwrap_or(Keysym::from(0));
                            match sym {
                                Keysym::Return => FilterResult::Intercept(KeyAction::Run(TERM_CMD.into())),
                                Keysym::d => FilterResult::Intercept(KeyAction::RunArgs(MENU_CMD[0].into(), MENU_CMD[1..].iter().map(|s| s.to_string()).collect())),
                                Keysym::Q if modifiers.shift => FilterResult::Intercept(KeyAction::Quit),
                                Keysym::C if modifiers.shift => FilterResult::Intercept(KeyAction::Close),
                                _ => FilterResult::Forward,
                            }
                        } else {
                            FilterResult::Forward
                        }
                    });

                
                    match action {
                        Some(KeyAction::Quit) => std::process::exit(0),
                        Some(KeyAction::Run(cmd)) => { Command::new(cmd).spawn().ok(); },
                        Some(KeyAction::RunArgs(cmd, args)) => { Command::new(cmd).args(args).spawn().ok(); },
                        Some(KeyAction::Close) => {
                             if let Some(surface) = state.seat.get_keyboard().unwrap().current_focus() {
                                if let Some(window) = state.space.elements().find(|w| w.wl_surface().as_deref() == Some(&surface)) {
                                    if let Some(toplevel) = window.toplevel() {
                                        toplevel.send_close();
                                    }
                                }
                             }
                        },
                        _ => {},
                    }
                }
                _ => {}
            },
            _ => (),
        }
    })?;

    loop {
        event_loop.dispatch(Duration::from_millis(16), &mut state)?;
        display.flush_clients()?;

        // RENDERIZAÇÃO
        {
            let (renderer, mut target) = backend.bind().unwrap();
            
            // rola
            render_output(
                &output,
                renderer,
                &mut target,
                1.0, 
                0,
                [&state.space],
                &[] as &[WaylandSurfaceRenderElement<GlesRenderer>], // rola q agora funciona
                &mut damage_tracker,
                [0.1, 0.1, 0.1, 1.0]
            ).unwrap();
        }
        backend.submit(None).unwrap();
    }
}
