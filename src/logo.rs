use std::{
    io::{self, Read, Write},
    net::{IpAddr, SocketAddr, TcpStream},
    sync::{
        Arc, Mutex,
        mpsc::{self, RecvTimeoutError, Sender},
    },
    thread,
    time::Duration,
};

use eframe::egui::{self, RichText};

use crate::{model::NetworkSource, studio::theme::HIGH};

const TIMEOUT: Duration = Duration::from_secs(2);
const REFRESH: Duration = Duration::from_millis(500);
#[derive(Debug, PartialEq, Eq)]
struct Signals {
    inputs: [u8; 3],
    outputs: [u8; 3],
}

struct LogoConnection {
    socket: TcpStream,
    unit: u8,
    transaction: u16,
}

impl LogoConnection {
    fn connect(address: SocketAddr, unit: u8) -> io::Result<Self> {
        let socket = TcpStream::connect_timeout(&address, TIMEOUT)?;
        socket.set_read_timeout(Some(TIMEOUT))?;
        socket.set_write_timeout(Some(TIMEOUT))?;
        Ok(Self {
            socket,
            unit,
            transaction: 0,
        })
    }

    fn read_signals(&mut self) -> io::Result<Signals> {
        // Siemens LOGO!Soft Comfort help §2.8.1.20, converted to zero-based offsets.
        Ok(Signals {
            inputs: self.read_bits(2, 0, 24)?,
            outputs: self.read_bits(1, 8192, 20)?,
        })
    }

    fn read_bits(&mut self, function: u8, offset: u16, count: u8) -> io::Result<[u8; 3]> {
        self.transaction = self.transaction.wrapping_add(1);
        let [tx_hi, tx_lo] = self.transaction.to_be_bytes();
        let [offset_hi, offset_lo] = offset.to_be_bytes();
        let request = [
            tx_hi, tx_lo, 0, 0, 0, 6, self.unit, function, offset_hi, offset_lo, 0, count,
        ];
        self.socket.write_all(&request)?;

        let mut header = [0; 7];
        self.socket.read_exact(&mut header)?;
        let length = u16::from_be_bytes([header[4], header[5]]);
        if header[..4] != request[..4] || header[6] != self.unit || !matches!(length, 3 | 6) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "En-tête Modbus invalide.",
            ));
        }
        let mut body = [0; 5];
        self.socket
            .read_exact(&mut body[..usize::from(length - 1)])?;
        match (length, body) {
            (6, [code, 3, first, second, third]) if code == function => Ok([first, second, third]),
            (3, [code, exception, ..]) if code == (function | 0x80) => Err(io::Error::other(
                format!("Le LOGO! a refusé la lecture (exception Modbus 0x{exception:02X})."),
            )),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Données Modbus invalides.",
            )),
        }
    }
}

type LatestReading = Arc<Mutex<Option<io::Result<Signals>>>>;

struct Session {
    latest: LatestReading,
    _stop: Sender<()>,
}

impl Session {
    fn start(address: SocketAddr, unit: u8, ctx: egui::Context) -> io::Result<Self> {
        let (stop, stopped) = mpsc::channel();
        let latest = LatestReading::default();
        let readings = Arc::clone(&latest);
        thread::Builder::new()
            .name("logo-modbus".into())
            .spawn(move || {
                let result = LogoConnection::connect(address, unit).and_then(|mut connection| {
                    loop {
                        let signals = connection.read_signals()?;
                        *readings.lock().unwrap() = Some(Ok(signals));
                        ctx.request_repaint();
                        if stopped.recv_timeout(REFRESH) != Err(RecvTimeoutError::Timeout) {
                            return Ok(());
                        }
                    }
                });
                if let Err(error) = result {
                    *readings.lock().unwrap() = Some(Err(error));
                    ctx.request_repaint();
                }
            })?;
        Ok(Self {
            latest,
            _stop: stop,
        })
    }
}

pub struct LogoMonitor {
    pub open: bool,
    ip: String,
    port: u16,
    unit: u8,
    session: Option<Session>,
    signals: Option<Signals>,
    error: Option<String>,
}

impl Default for LogoMonitor {
    fn default() -> Self {
        Self {
            open: false,
            ip: String::new(),
            port: 502,
            unit: 255,
            session: None,
            signals: None,
            error: None,
        }
    }
}

impl LogoMonitor {
    pub fn digital_signal(
        &self,
        ip: std::net::Ipv4Addr,
        source: NetworkSource,
        address: u16,
    ) -> Option<bool> {
        if self.ip.trim().parse::<std::net::Ipv4Addr>().ok() != Some(ip) {
            return None;
        }
        let signals = self.signals.as_ref()?;
        let data = match source {
            NetworkSource::DiscreteInput => signals.inputs,
            NetworkSource::Coil => signals.outputs,
            _ => return None,
        };
        let index = usize::from(address.checked_sub(1)?);
        Some(data.get(index / 8)? & (1 << (index % 8)) != 0)
    }

    pub fn open_for(&mut self, ip: std::net::Ipv4Addr) {
        if self.open && self.ip.trim().parse::<std::net::Ipv4Addr>().ok() == Some(ip) {
            return;
        }
        self.session = None;
        self.signals = None;
        self.error = None;
        self.ip = ip.to_string();
        self.port = 502;
        self.unit = 255;
        self.open = true;
    }

    pub fn show(&mut self, ctx: &egui::Context) {
        if !self.open {
            return;
        }
        let reading = self
            .session
            .as_ref()
            .and_then(|s| s.latest.lock().unwrap().take());
        match reading {
            Some(Ok(signals)) => self.signals = Some(signals),
            Some(Err(error)) => {
                self.session = None;
                self.signals = None;
                self.error = Some(format!("Communication interrompue. {error}"));
            }
            None => {}
        }

        let mut open = self.open;
        egui::Window::new("Réseau LOGO!")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_width(360.0)
            .show(ctx, |ui| {
                ui.strong("Suivi des entrées et sorties · Modbus TCP");
                ui.add_enabled_ui(self.session.is_none(), |ui| {
                    egui::Grid::new("logo-endpoint").show(ui, |ui| {
                        ui.label("Adresse IP");
                        ui.add(egui::TextEdit::singleline(&mut self.ip).desired_width(160.0));
                        ui.end_row();
                        ui.label("Port TCP");
                        ui.add(egui::DragValue::new(&mut self.port).range(1..=65535));
                        ui.end_row();
                        ui.label("Identifiant Modbus");
                        ui.add(egui::DragValue::new(&mut self.unit));
                        ui.end_row();
                    });
                });
                ui.horizontal(|ui| {
                    if self.session.is_some() {
                        if ui.button("Déconnecter").clicked() {
                            self.session = None;
                            self.signals = None;
                        } else if self.signals.is_some() {
                            ui.label("Connecté · Actualisation en direct");
                        } else {
                            ui.spinner();
                            ui.label("Connexion en cours…");
                        }
                    } else if ui.button("Connecter").clicked() {
                        self.error = match self.ip.trim().parse::<IpAddr>() {
                            Ok(ip) => match Session::start(
                                SocketAddr::new(ip, self.port),
                                self.unit,
                                ctx.clone(),
                            ) {
                                Ok(session) => {
                                    self.session = Some(session);
                                    None
                                }
                                Err(error) => Some(format!("Connexion impossible. {error}")),
                            },
                            Err(_) => Some("Saisissez une adresse IP valide.".into()),
                        };
                    }
                });
                if let Some(error) = &self.error {
                    ui.colored_label(ui.visuals().error_fg_color, error);
                }
                if let Some(signals) = &self.signals {
                    show_bits(ui, "Entrées numériques", 'I', signals.inputs, 24);
                    show_bits(ui, "Sorties numériques", 'Q', signals.outputs, 20);
                }
            });
        self.open = open;
        if !open {
            self.session = None;
            self.signals = None;
            self.error = None;
        }
    }
}

fn show_bits(ui: &mut egui::Ui, title: &str, prefix: char, data: [u8; 3], count: usize) {
    ui.separator();
    ui.strong(title);
    egui::Grid::new(prefix).show(ui, |ui| {
        for index in 0..count {
            let active = data[index / 8] & (1 << (index % 8)) != 0;
            let label = format!("{prefix}{:02} {}", index + 1, u8::from(active));
            let text = RichText::new(label).monospace();
            ui.label(if active {
                text.strong().color(HIGH)
            } else {
                text
            });
            if index % 6 == 5 {
                ui.end_row();
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;

    #[test]
    fn reads_logo_digital_io_and_reports_modbus_exception() -> io::Result<()> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let address = listener.local_addr()?;
        let server = thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            socket.set_read_timeout(Some(TIMEOUT)).unwrap();
            for (request, response) in [
                (
                    [0, 1, 0, 0, 0, 6, 255, 2, 0, 0, 0, 24],
                    &[0, 1, 0, 0, 0, 6, 255, 2, 3, 1, 2, 128][..],
                ),
                (
                    [0, 2, 0, 0, 0, 6, 255, 1, 32, 0, 0, 20],
                    &[0, 2, 0, 0, 0, 6, 255, 1, 3, 0, 4, 8][..],
                ),
                (
                    [0, 3, 0, 0, 0, 6, 255, 2, 0, 0, 0, 24],
                    &[0, 3, 0, 0, 0, 3, 255, 0x82, 2][..],
                ),
            ] {
                let mut actual = [0; 12];
                socket.read_exact(&mut actual).unwrap();
                assert_eq!(actual, request);
                socket.write_all(&response[..5]).unwrap();
                socket.write_all(&response[5..]).unwrap();
            }
        });

        let mut client = LogoConnection::connect(address, 255)?;
        let signals = client.read_signals()?;
        assert_eq!(
            signals,
            Signals {
                inputs: [1, 2, 128],
                outputs: [0, 4, 8]
            }
        );
        let monitor = LogoMonitor {
            ip: "127.0.0.1".into(),
            signals: Some(signals),
            ..Default::default()
        };
        assert_eq!(
            monitor.digital_signal(
                std::net::Ipv4Addr::LOCALHOST,
                NetworkSource::DiscreteInput,
                24
            ),
            Some(true)
        );
        assert_eq!(
            monitor.digital_signal(std::net::Ipv4Addr::LOCALHOST, NetworkSource::Coil, 10),
            Some(false)
        );
        assert_eq!(
            monitor.digital_signal(std::net::Ipv4Addr::LOCALHOST, NetworkSource::Coil, 11),
            Some(true)
        );
        assert_eq!(
            monitor.digital_signal(std::net::Ipv4Addr::UNSPECIFIED, NetworkSource::Coil, 11),
            None
        );
        assert!(
            client
                .read_signals()
                .unwrap_err()
                .to_string()
                .contains("0x02")
        );
        server.join().unwrap();
        Ok(())
    }
}
