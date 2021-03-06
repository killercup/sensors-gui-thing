#![recursion_limit = "1024"]

use std::{io::Error as IoError, process::Command, time::Duration};

use vgtk::ext::*;
use vgtk::lib::gio::{prelude::ApplicationExtManual, ActionExt, ApplicationFlags, SimpleAction};
use vgtk::lib::glib::object::Cast;
use vgtk::lib::gtk::*;
use vgtk::{gtk, Component, UpdateAction, VNode};

use anyhow::{Context, Result};

mod sensors;
use sensors::{Error, Sensors};

fn main() {
    pretty_env_logger::init();
    let (app, scope) = vgtk::start::<Model>();

    let _worker = std::thread::spawn(move || loop {
        scope.send_message(Message::UpdateSensors(Sensors::fetch()));
        std::thread::sleep(Duration::from_millis(250));
    });

    let args: Vec<String> = std::env::args().collect();
    let exit_status = app.run(&args);
    std::process::exit(exit_status);
}

#[derive(Clone, Debug, Default)]
struct Model {
    sensors: Sensors,
}

#[derive(Clone, Debug)]
enum Message {
    Init,
    UpdateSensors(Result<Sensors, Error>),
    Exit,
}

impl Component for Model {
    type Message = Message;
    type Properties = ();

    fn update(&mut self, msg: Self::Message) -> UpdateAction<Self> {
        match msg {
            Message::Exit => {
                vgtk::quit();
                UpdateAction::None
            }
            Message::Init => {
                log::info!("hello");
                UpdateAction::None
            }
            Message::UpdateSensors(fetch_sensors) => {
                if let Ok(sensors) = fetch_sensors {
                    log::debug!("{:?}", sensors);
                    UpdateAction::Render
                } else {
                    UpdateAction::None
                }
            }
        }
    }

    fn view(&self) -> VNode<Model> {
        let model: Option<TreeModel> = None;

        gtk! {
            <Application::new_unwrap(Some("space.deterinistic.sensors-gui-thingy"), ApplicationFlags::empty())>
                <SimpleAction::new("quit", None) Application::accels=["<Ctrl>q"].as_ref() enabled=true on activate=|a, _| Message::Exit/>

                <ApplicationWindow default_width=800 default_height=480 border_width=20 on destroy=|_| Message::Exit>
                    <HeaderBar title="Sensors" show_close_button=true />
                    <Box orientation=Orientation::Vertical>
                        <ScrolledWindow Box::expand=true Box::fill=true>
                            <TreeView::new()
                                model=model
                                headers_clickable=true
                                enable_search=true
                                tooltip_column=0
                                on show=|tree_view| {
                                    // for column in Dataset::to_treeview_columns() {
                                    //     tree_view.append_column(&column);
                                    // }
                                    Message::Init
                                }>
                            </TreeView>
                        </ScrolledWindow>
                    </Box>
                </ApplicationWindow>
            </Application>
        }
    }
}
