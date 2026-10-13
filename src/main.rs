mod overlay;
mod ui;

use adw::prelude::*;
fn main() -> gtk::glib::ExitCode {
    let app = adw::Application::builder()
        .application_id("io.github.workcircuit.WorkCircuit")
        .build();
    app.connect_activate(ui::activate);
    app.run()
}
