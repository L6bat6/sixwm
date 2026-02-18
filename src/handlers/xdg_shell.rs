use crate::state::SixWM;
use smithay::{
    delegate_xdg_shell,
    desktop::Window,
    reexports::wayland_server::protocol::wl_seat::WlSeat,
    utils::Serial,
    wayland::shell::xdg::{
        PopupSurface, PositionerState, ToplevelSurface, XdgShellHandler, XdgShellState,
    },
};

impl XdgShellHandler for SixWM {
    fn xdg_shell_state(&mut self) -> &mut XdgShellState {
        &mut self.xdg_shell_state
    }

    fn new_toplevel(&mut self, surface: ToplevelSurface) {
        // Usando o construtor correto para a sua versão do Smithay
        let window = Window::new_wayland_window(surface);
        
        // Spawn das janelas em (50, 50) para não ficarem escondidas no canto
        self.space.map_element(window, (50, 50), true);
        println!("Janela aberta com sucesso!");
    }

    fn new_popup(&mut self, _surface: PopupSurface, _positioner: PositionerState) {}
    fn grab(&mut self, _surface: PopupSurface, _seat: WlSeat, _serial: Serial) {}
    fn reposition_request(&mut self, _surface: PopupSurface, _positioner: PositionerState, _token: u32) {}
}

delegate_xdg_shell!(SixWM);
