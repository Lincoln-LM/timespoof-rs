use dll_syringe::process::OwnedProcess;
use dll_syringe::Syringe;
use iced::widget::{button, checkbox, column, container, text, text_input};
use iced::{Element, Length, Padding, Sandbox, Settings};
use std::io::{Read, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::path::PathBuf;
use std::str::FromStr;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use time::format_description::well_known::Iso8601;
use time::OffsetDateTime;

struct Server {
    stream: TcpStream,
    real_time: bool,
    move_forward: bool,
    update_base_time: bool,
    datetime: String,
}

#[derive(Debug, Clone)]
enum Message {
    RealTimeChanged(bool),
    MoveForwardChanged(bool),
    UpdateBaseTimeChanged(bool),
    DateTimeChanged(String),
    Submit,
}

impl Sandbox for Server {
    type Message = Message;

    fn new() -> Self {
        let address = SocketAddr::new(Ipv4Addr::new(127, 0, 0, 1).into(), 63463);
        let listener = TcpListener::bind(address).expect("Failed to find to port 63463");

        let mut process_name = String::new();
        println!("Portion of Process Executable Name (ex. EmuHawk): ");
        std::io::stdin().read_line(&mut process_name).unwrap();

        let process_name = process_name.trim_end_matches("\r\n");
        let target_process =
            OwnedProcess::find_first_by_name(process_name).expect("Application not found running!");

        let path = if cfg!(debug_assertions) {
            PathBuf::from_str("./target/debug/libtimespoof.dll").unwrap()
        } else {
            PathBuf::from_str("./target/release/libtimespoof.dll").unwrap()
        };

        if !path.exists() {
            panic!("DLL not build! Run 'cargo build' or 'cargo build --release'.")
        }

        let syringe = Syringe::for_process(target_process);
        syringe.inject(path).expect("Failed to inject dll!");

        println!("DLL Injected");

        let (stream, _) = listener.accept().unwrap();

        Self {
            stream,
            real_time: false,
            move_forward: false,
            update_base_time: false,
            datetime: "2000-01-01T00:00:00.000000-04:00".to_string(),
        }
    }

    fn title(&self) -> String {
        String::from("timespoofer-rs server")
    }

    fn update(&mut self, message: Self::Message) {
        match message {
            Message::RealTimeChanged(b) => {
                self.real_time = b;
            }
            Message::MoveForwardChanged(b) => {
                self.move_forward = b;
            }
            Message::UpdateBaseTimeChanged(b) => {
                self.update_base_time = b;
            }
            Message::DateTimeChanged(datetime) => {
                self.datetime = datetime;
            }
            Message::Submit => {
                let target = OffsetDateTime::parse(&self.datetime, &Iso8601::DEFAULT).unwrap();
                let timestamp = (SystemTime::from(target)
                    .duration_since(UNIX_EPOCH - Duration::from_secs(11644473600))
                    .unwrap()
                    .as_nanos()
                    / 100) as u64;
                let _ = self.stream.write(
                    format!(
                        "{timestamp} {} {} {}",
                        u8::from(self.real_time),
                        u8::from(self.move_forward),
                        u8::from(self.update_base_time)
                    )
                    .as_bytes(),
                );
                let mut resp = [0; 1024];
                let resp_len = self.stream.read(&mut resp).unwrap();
                println!("Response: {}", String::from_utf8_lossy(&resp[0..resp_len]));
            }
        }
    }

    fn view(&self) -> Element<'_, Self::Message> {
        let real_time = container(checkbox(
            "Real Time",
            self.real_time,
            Message::RealTimeChanged,
        ))
        .width(Length::Fill);
        let move_forward = container(checkbox(
            "Move Forward",
            self.move_forward,
            Message::MoveForwardChanged,
        ))
        .width(Length::Fill);
        let update_base_time = container(checkbox(
            "Update Base Time",
            self.update_base_time,
            Message::UpdateBaseTimeChanged,
        ))
        .width(Length::Fill);
        let datetime = container(text_input("", &self.datetime, Message::DateTimeChanged))
            .width(Length::Fill)
            .padding(Padding::from([0, 15]));

        let label = container(text("Date String (ISO 8601)")).width(Length::Fill);

        let submit = container(button("Submit").on_press(Message::Submit))
            .width(Length::Fill)
            .padding([0, 15]);

        container(
            column![
                real_time.center_x(),
                move_forward.center_x(),
                update_base_time.center_x(),
                label.center_x(),
                datetime.center_x(),
                submit.center_x()
            ]
            .spacing(25),
        )
        .height(Length::Fill)
        .width(Length::Fill)
        .center_x()
        .center_y()
        .into()
    }
}

fn main() -> iced::Result {
    let mut settings = Settings::default();
    settings.window.size = (300, 300);
    settings.window.resizable = false;
    Server::run(settings)
}
