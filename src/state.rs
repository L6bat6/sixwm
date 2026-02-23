use smithay::{
    reexports::wayland_server::{
        DisplayHandle,
        backend::{ClientData, ClientId, DisconnectReason},
    },
    wayland::{
        compositor::CompositorState,
        shm::ShmState,
        shell::xdg::XdgShellState,
    },
    input::{Seat, SeatState}, 
    desktop::{Space, Window}, 
    utils::Point,
};

pub struct SixWM {
    pub compositor_state: CompositorState,
    pub shm_state: ShmState,
    pub xdg_shell_state: XdgShellState,
    pub seat_state: SeatState<SixWM>,
    pub seat: Seat<SixWM>,
    pub display_handle: DisplayHandle,
    pub space: Space<Window>, // rola agora com espaco
    pub pointer_location: Point<f64, smithay::utils::Logical>,
    pub width: u32,
    pub height: u32,
    
    //offset) dentro dela
    pub grabbed_window: Option<(Window, Point<f64, smithay::utils::Logical>)>,
}

pub struct ClientState {
    pub compositor_state: smithay::wayland::compositor::CompositorClientState,
}

impl ClientData for ClientState {
    fn initialized(&self, _client_id: ClientId) {}
    fn disconnected(&self, _client_id: ClientId, _reason: DisconnectReason) {}
}
