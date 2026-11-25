use std::net::Ipv4Addr;

use eframe::egui::{self, Align2, FontId, RichText, Sense, Stroke, pos2, vec2};

use crate::model::{Block, Connection, Device, Kind, NetworkPort, NetworkSource, Project, Protocol, RemoteBlock};

use super::{Studio, theme::*};

#[derive(Default)]
pub(super) struct NetworkUi {
    pub(super) block_draft: Option<(u32, NetworkPort, String, bool)>,
    selected: Option<u32>,
    device: Option<(Option<u32>, String, String)>,
    connection: Option<(u32, bool, Protocol, Option<u32>)>,
}

impl Studio {
    pub(super) fn network_panel(&mut self, ui: &mut egui::Ui) {
        let max_height = (ui.available_height() - 170.0).max(48.0);
        let initial = if self.project.network_drawing.is_some() {
            210.0
        } else {
            145.0 + self.project.connections.len().min(4) as f32 * 7.0
        };
        let height = self.network_height.unwrap_or(initial).clamp(48.0, max_height);
        panel(SHELL, 12, 8).corner_radius(8).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.strong("Vue réseau");
                if ui
                    .add_enabled(self.editing(), egui::Button::new("Ajouter un appareil"))
                    .clicked()
                {
                    self.network_ui.device = Some((
                        None,
                        format!("LOGO! {}", self.project.devices.len() + 1),
                        String::new(),
                    ));
                }
                if ui.button("Suivi Modbus").clicked() {
                    self.logo.open = true;
                }
            });
            self.network_topology(ui, height);
        });
        let (divider, _) =
            ui.allocate_exact_size(vec2(ui.available_width(), 7.0), Sense::hover());
        let handle = ui
            .interact(divider, ui.id().with("network-divider"), Sense::click_and_drag())
            .on_hover_cursor(egui::CursorIcon::ResizeVertical)
            .on_hover_text("Glisser pour redimensionner les vues");
        if handle.dragged_by(egui::PointerButton::Primary) {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical);
            self.network_height = Some((height + handle.drag_motion().y).clamp(48.0, max_height));
            ui.ctx().request_repaint();
        }
        if handle.double_clicked() {
            self.network_height = None;
        }
        let grip = if handle.hovered() || handle.dragged() { HIGH } else { BORDER };
        for offset in [-1.5, 1.5] {
            let center = divider.center() + vec2(0.0, offset);
            ui.painter().line_segment(
                [center - vec2(14.0, 0.0), center + vec2(14.0, 0.0)],
                Stroke::new(1.0_f32, grip),
            );
        }
        self.network_dialogs(ui.ctx());
    }

    fn network_topology(&mut self, ui: &mut egui::Ui, height: f32) {
        let devices = self.project.devices.clone();
        let connections = self.project.connections.clone();
        let drawing = self.project.network_drawing.clone();
        let mut sheet = None;
        egui::ScrollArea::both()
            .max_height(height)
            .auto_shrink([false, false])
            .id_salt("topologie")
            .show(ui, |ui| {
                let card = vec2(120.0, 54.0);
                let pitch = card.x + 20.0;
                let wires = connections.len().min(4) as f32 * 7.0;
                let scale = 0.75;
                let map = |[x, y]: [f32; 2]| vec2(x, y - 55.0) * scale;
                let external = |id| drawing.as_ref().is_some_and(|d| d.external.contains(&id));
                let mut bounds = vec2(350.0, 145.0 + wires);
                let locations: Vec<_> = devices
                    .iter()
                    .enumerate()
                    .map(|(i, device)| {
                        let pos = drawing
                            .as_ref()
                            .and_then(|d| d.positions.iter().find(|(id, _)| *id == device.id))
                            .map_or(vec2(5.0 + i as f32 * pitch, 77.0), |(_, point)| map(*point));
                        bounds = bounds.max(pos + card + vec2(13.0, 15.0));
                        pos
                    })
                    .collect();
                if let Some(drawing) = &drawing {
                    for &point in drawing.paths.iter().flat_map(|(_, path)| path) {
                        bounds = bounds.max(map(point) + vec2(16.0, 8.0));
                    }
                }
                let (area, _) = ui.allocate_exact_size(
                    vec2(ui.available_width().max(bounds.x), height.max(bounds.y)),
                    Sense::hover(),
                );
                let local = egui::Rect::from_min_size(
                    area.min
                        + drawing
                            .as_ref()
                            .map_or(vec2(5.0, 5.0), |d| map(d.local) - vec2(card.x * 0.7, 33.0)),
                    vec2(card.x, 33.0),
                );
                let painter = ui.painter();
                let location = |i: usize| {
                    let size = if external(devices[i].id) {
                        vec2(8.0, 8.0)
                    } else {
                        card
                    };
                    egui::Rect::from_min_size(area.min + locations[i], size)
                };
                painter.rect_filled(area, 4, DIAGRAM_PAPER);
                if let Some(drawing) = &drawing {
                    for (local, path) in &drawing.paths {
                        if path.len() >= 2 {
                            painter.add(egui::Shape::line(
                                path.iter().map(|&point| area.min + map(point)).collect(),
                                Stroke::new(
                                    1.2_f32,
                                    if *local {
                                        egui::Color32::from_rgb(33, 132, 75)
                                    } else {
                                        WIRE
                                    },
                                ),
                            ));
                        }
                    }
                }
                if drawing
                    .as_ref()
                    .is_none_or(|drawing| drawing.paths.is_empty())
                    && !devices.is_empty()
                {
                    let start = local.center_bottom();
                    let trunk_y = start.y + 10.0;
                    let last = location(devices.len() - 1).center_top();
                    let stroke = Stroke::new(1.0_f32, MUTED);
                    painter.line_segment([start, pos2(start.x, trunk_y)], stroke);
                    painter.line_segment([pos2(start.x, trunk_y), pos2(last.x, trunk_y)], stroke);
                    for i in 0..devices.len() {
                        let port = location(i).center_top();
                        painter.line_segment([pos2(port.x, trunk_y), port], stroke);
                        painter.circle_filled(port, 2.5, MUTED);
                    }
                }
                painter.rect_filled(local, 3, egui::Color32::from_rgb(221, 225, 232));
                painter.text(
                    local.left_center() + vec2(8.0, 0.0),
                    Align2::LEFT_CENTER,
                    "PC local",
                    FontId::proportional(11.0),
                    DARK,
                );
                device_icon(painter, local.right_center() - vec2(23.0, 0.0), true);
                for (i, link) in connections.iter().enumerate() {
                    if drawing
                        .as_ref()
                        .is_some_and(|drawing| !drawing.paths.is_empty())
                    {
                        break;
                    }
                    let client = devices.iter().position(|d| d.id == link.client).unwrap();
                    let server = devices.iter().position(|d| d.id == link.server).unwrap();
                    let a = location(client).center_bottom();
                    let b = location(server).center_bottom();
                    let y = a.y + 8.0 + (i % 4) as f32 * 7.0;
                    let color = if link.protocol == Protocol::S7 {
                        ACCENT
                    } else {
                        HIGH
                    };
                    painter.add(egui::Shape::line(
                        vec![a, pos2(a.x, y), pos2(b.x, y), b],
                        Stroke::new(1.5_f32, color),
                    ));
                    painter.circle_filled(a, 3.0, color);
                    painter.circle_filled(b, 3.0, color);
                }
                for (i, device) in devices.iter().enumerate() {
                    let rect = location(i);
                    let res = ui.interact(rect, ui.id().with(device.id), Sense::click());
                    if res.clicked() || res.secondary_clicked() {
                        self.network_ui.selected = Some(device.id);
                    }
                    if res.clicked() {
                        sheet = self
                            .project
                            .sheets
                            .iter()
                            .position(|sheet| sheet.device == Some(device.id));
                    }
                    let painter = ui.painter();
                    let selected = self.network_ui.selected == Some(device.id)
                        || self
                            .project
                            .active_sheet
                            .and_then(|index| self.project.sheets[index].device)
                            == Some(device.id);
                    let fill = if selected {
                        egui::Color32::from_rgb(190, 207, 234)
                    } else {
                        egui::Color32::from_rgb(221, 225, 232)
                    };
                    painter.rect_filled(rect, 3, fill);
                    if !external(device.id) {
                        let text = painter
                            .with_clip_rect(rect.shrink2(vec2(4.0, 3.0)).intersect(ui.clip_rect()));
                        text.text(
                            rect.min + vec2(7.0, 8.0),
                            Align2::LEFT_TOP,
                            &device.name,
                            FontId::proportional(11.0),
                            DARK,
                        );
                        text.text(
                            rect.min + vec2(7.0, 24.0),
                            Align2::LEFT_TOP,
                            if device.ip.is_unspecified() {
                                "Adresse à définir".into()
                            } else {
                                device.ip.to_string()
                            },
                            FontId::proportional(10.0),
                            DARK,
                        );
                        device_icon(painter, rect.right_center() - vec2(21.0, 0.0), false);
                    }
                    res.on_hover_text(&device.name)
                        .context_menu(|ui| self.device_menu(ui, device));
                }
            });
        if let Some(index) = sheet {
            self.select_sheet(index);
        }
        let Some(device) = self
            .network_ui
            .selected
            .and_then(|id| devices.iter().find(|d| d.id == id))
        else {
            return;
        };
        ui.horizontal(|ui| {
            ui.menu_button(format!("Actions · {}", device.name), |ui| {
                self.device_menu(ui, device)
            });
            for link in connections
                .iter()
                .filter(|link| link.peer(device.id).is_some())
                .take(2)
            {
                let peer = link.peer(device.id).unwrap();
                let name = &devices.iter().find(|d| d.id == peer).unwrap().name;
                let direction = if device.id == link.client {
                    "→"
                } else {
                    "←"
                };
                ui.label(
                    RichText::new(format!("{direction} {name} · {}", link.protocol.label()))
                        .color(MUTED),
                );
            }
        });
    }

    fn device_menu(&mut self, ui: &mut egui::Ui, device: &Device) {
        ui.label(RichText::new(&device.name).strong());
        ui.separator();
        ui.add_enabled_ui(self.editing(), |ui| {
            ui.add_enabled_ui(self.project.devices.len() > 1, |ui| {
                for (label, client) in [
                    ("Connexion en tant que client", true),
                    ("Connexion en tant que serveur", false),
                ] {
                    ui.menu_button(label, |ui| {
                        for protocol in [Protocol::S7, Protocol::ModbusTcp] {
                            if ui
                                .button(format!("Connexion {}", protocol.label()))
                                .clicked()
                            {
                                self.status.clear();
                                self.network_ui.connection =
                                    Some((device.id, client, protocol, None));
                                ui.close();
                            }
                        }
                    });
                }
            });
            let links: Vec<_> = self
                .project
                .connections
                .iter()
                .copied()
                .filter(|link| link.peer(device.id).is_some())
                .collect();
            if !links.is_empty() {
                ui.menu_button("Supprimer une connexion", |ui| {
                    for link in links {
                        let peer = self
                            .project
                            .devices
                            .iter()
                            .find(|d| Some(d.id) == link.peer(device.id))
                            .unwrap();
                        let direction = if link.client == device.id {
                            "→"
                        } else {
                            "←"
                        };
                        if ui
                            .button(format!(
                                "{direction} {} · {}",
                                peer.name,
                                link.protocol.label()
                            ))
                            .clicked()
                        {
                            let mut candidate = self.project.clone();
                            candidate.connections.retain(|c| c.id != link.id);
                            clear_removed_ports(&mut candidate);
                            self.apply_network(candidate);
                            ui.close();
                        }
                    }
                });
            }
        });
        ui.separator();
        if ui
            .add_enabled(
                !device.ip.is_unspecified(),
                egui::Button::new("État des E/S · Modbus TCP"),
            )
            .clicked()
        {
            self.logo.open_for(device.ip);
            ui.close();
        }
        ui.add_enabled_ui(self.editing(), |ui| {
            if ui.button("Nom et adresse de l’appareil…").clicked() {
                self.status.clear();
                self.network_ui.device =
                    Some((Some(device.id), device.name.clone(), device.ip.to_string()));
                ui.close();
            }
            if ui.button("Supprimer l’appareil").clicked() {
                let mut candidate = self.project.clone();
                candidate.devices.retain(|d| d.id != device.id);
                candidate
                    .connections
                    .retain(|link| link.peer(device.id).is_none());
                clear_removed_ports(&mut candidate);
                self.apply_network(candidate);
                self.network_ui.selected = None;
                ui.close();
            }
        });
    }

    fn network_dialogs(&mut self, ctx: &egui::Context) {
        if let Some((id, mut name, mut address)) = self.network_ui.device.take() {
            let mut open = true;
            let mut saved = false;
            let mut error = None;
            egui::Window::new("Appareil réseau")
                .collapsible(false)
                .open(&mut open)
                .show(ctx, |ui| {
                    ui.label("Nom de l’appareil");
                    ui.add(egui::TextEdit::singleline(&mut name).char_limit(60));
                    ui.label("Adresse IPv4");
                    ui.add(
                        egui::TextEdit::singleline(&mut address)
                            .hint_text("192.168.10.10")
                            .char_limit(15),
                    );
                    if ui
                        .add_enabled(self.editing(), egui::Button::new("Valider"))
                        .clicked()
                    {
                        if let Ok(ip) = address.trim().parse::<Ipv4Addr>() {
                            let mut candidate = self.project.clone();
                            let id = id.unwrap_or_else(|| {
                                (1..)
                                    .find(|id| candidate.devices.iter().all(|d| d.id != *id))
                                    .unwrap()
                            });
                            let device = Device {
                                profile: candidate.devices.iter().find(|device| device.id == id).and_then(|device| device.profile),
                                id,
                                name: name.trim().into(),
                                ip,
                            };
                            if let Some(old) = candidate.devices.iter_mut().find(|d| d.id == id) {
                                *old = device;
                            } else {
                                candidate.devices.push(device);
                            }
                            error = self.apply_network(candidate);
                            saved = error.is_none();
                            if saved {
                                self.network_ui.selected = Some(id);
                            }
                        } else {
                            error = Some("Saisissez une adresse IPv4 valide.");
                        }
                    }
                    if let Some(error) = error {
                        self.status = error.into();
                    }
                    if !self.status.is_empty() {
                        ui.colored_label(ui.visuals().error_fg_color, &self.status);
                    }
                });
            if open && !saved {
                self.network_ui.device = Some((id, name, address));
            }
        }
        if let Some((origin, client, protocol, mut peer)) = self.network_ui.connection.take() {
            let mut open = true;
            let mut saved = false;
            egui::Window::new(format!("Connexion {}", protocol.label()))
                .collapsible(false)
                .open(&mut open)
                .show(ctx, |ui| {
                    if let Some(device) = self.project.devices.iter().find(|d| d.id == origin) {
                        let role = if client { "Client" } else { "Serveur" };
                        ui.strong(format!("{role} · {}", device.name));
                    }
                    ui.label(if client {
                        "Appareil serveur"
                    } else {
                        "Appareil client"
                    });
                    device_picker(ui, "peer", &self.project.devices, &mut peer, |id| {
                        id != origin
                    });
                    if ui
                        .add_enabled(
                            self.editing() && peer.is_some(),
                            egui::Button::new("Ajouter"),
                        )
                        .clicked()
                        && let Some(peer) = peer
                    {
                        let (client, server) = if client {
                            (origin, peer)
                        } else {
                            (peer, origin)
                        };
                        let mut candidate = self.project.clone();
                        if candidate.connections.iter().any(|c| {
                            c.client == client && c.server == server && c.protocol == protocol
                        }) {
                            self.status = "Cette connexion existe déjà.".into();
                        } else {
                            let id = (1..)
                                .find(|id| candidate.connections.iter().all(|c| c.id != *id))
                                .unwrap();
                            candidate.connections.push(Connection {
                                id,
                                client,
                                server,
                                protocol,
                            });
                            saved = self.apply_network(candidate).is_none();
                        }
                    }
                    if !self.status.is_empty() {
                        ui.colored_label(ui.visuals().error_fg_color, &self.status);
                    }
                });
            if open && !saved {
                self.network_ui.connection = Some((origin, client, protocol, peer));
            }
        }
    }

    pub(super) fn network_block_summary(&mut self, ui: &mut egui::Ui, block: &Block) {
        if ui.add_enabled(self.editing(), egui::Button::new("Paramètres réseau…")).clicked() {
            self.block_properties = true;
        }
        ui.label(block.network.source.label());
        if block.network.source.live_count().is_some()
            && let Some(device) = self.project.devices.iter().find(|d| Some(d.id) == block.network.device)
        {
            let ip = device.ip;
            if ui.button("Ouvrir le suivi Modbus").clicked() {
                self.logo.open_for(ip);
            }
            let online = self.logo.digital_signal(ip, block.network.source, block.network.address).is_some();
            ui.label(if online { "Lecture en direct" } else { "Hors ligne · valeur 0" });
        }
    }

    pub(super) fn network_block_dialog(&mut self, ctx: &egui::Context, index: usize) {
        let block = &self.project.blocks[index];
        let title = format!("{} [{}]", block.name, block.kind.label());
        let (mut port, mut comment, mut comment_tab) = self.network_ui.block_draft.take()
            .filter(|draft| draft.0 == block.id)
            .map_or_else(|| (block.network, block.message.clone(), false), |(_, port, comment, tab)| (port, comment, tab));
        let mut open = true;
        let mut confirmed = false;
        let mut cancelled = false;
        egui::Window::new(title)
            .id(egui::Id::new("network-block-properties"))
            .anchor(Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .default_width(400.0)
            .collapsible(false)
            .open(&mut open)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut comment_tab, false, "Paramètres");
                    ui.selectable_value(&mut comment_tab, true, "Commentaire");
                });
                ui.separator();
                if comment_tab {
                    ui.add(egui::TextEdit::multiline(&mut comment).desired_rows(6).desired_width(f32::INFINITY).char_limit(1000));
                } else {
                    self.network_port_fields(ui, block.kind, &mut port);
                }
                ui.separator();
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    cancelled = ui.button("Annuler").clicked();
                    let valid = match port.source {
                        NetworkSource::RemoteVm | NetworkSource::RemoteBlock(_) => port.device.is_some() || port.remote_ip.is_some(),
                        _ => true,
                    };
                    confirmed = ui.add_enabled(valid, egui::Button::new("OK")).clicked();
                });
            });
        self.block_properties = open && !confirmed && !cancelled;
        if confirmed && (port != block.network || comment != block.message) {
            let mut candidate = self.project.clone();
            candidate.blocks[index].network = port;
            candidate.blocks[index].message = comment;
            self.apply_network(candidate);
        } else if self.block_properties {
            self.network_ui.block_draft = Some((block.id, port, comment, comment_tab));
        }
    }

    fn network_port_fields(&self, ui: &mut egui::Ui, kind: Kind, port: &mut NetworkPort) {
        let analog = kind.analog_value();
        let remote_default = if kind.network_output() {
            NetworkSource::RemoteVm
        } else if analog {
            NetworkSource::RemoteBlock(RemoteBlock::AnalogInput)
        } else {
            NetworkSource::RemoteBlock(RemoteBlock::Marker)
        };
        let mode = |source| match source {
            NetworkSource::RemoteBlock(_) | NetworkSource::RemoteVm => remote_default,
            _ => source,
        };
        let mut source = mode(port.source);
        ui.strong(if kind.network_input() { "Lire la valeur de" } else { "Écrire la valeur dans" });
        for item in [NetworkSource::LocalVm, remote_default, NetworkSource::Diagnostic] {
            if item != NetworkSource::Diagnostic || kind == Kind::NetInput {
                ui.radio_value(&mut source, item, item.label());
            }
        }
        egui::ComboBox::from_id_salt("network-port-additional")
            .selected_text(if matches!(source, NetworkSource::Simulated | NetworkSource::Coil | NetworkSource::DiscreteInput | NetworkSource::Unsupported) {
                source.label()
            } else {
                "Autres modes"
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut source, NetworkSource::Simulated, NetworkSource::Simulated.label());
                if kind == Kind::NetInput {
                    for item in [NetworkSource::DiscreteInput, NetworkSource::Coil] {
                        ui.selectable_value(&mut source, item, item.label());
                    }
                }
            });
        if source != mode(port.source) {
            port.source = source;
            port.address = u16::from(matches!(source, NetworkSource::RemoteBlock(_)) || source.live_count().is_some());
            port.connection = None;
            port.bit = 0;
            if matches!(source, NetworkSource::LocalVm | NetworkSource::Diagnostic) {
                port.device = None;
                port.remote_ip = None;
            }
        }
        ui.separator();

        match port.source {
            NetworkSource::Diagnostic => {
                ui.label(format!("Type de diagnostic · {}", port.address));
                ui.label("La simulation ne génère pas de défaut matériel.");
            }
            NetworkSource::Unsupported => {
                ui.label("Sélectionnez un mode pour remplacer le paramètre d’origine.");
            }
            NetworkSource::LocalVm => vm_fields(ui, port, analog),
            NetworkSource::RemoteBlock(_) | NetworkSource::RemoteVm => {
                ui.strong("Appareil distant");
                let previous = port.device;
                let device = self.project.devices.iter().find(|d| Some(d.id) == port.device);
                let label = device.map_or_else(
                    || port.remote_ip.map_or("Sélectionner un appareil".into(), |ip| ip.to_string()),
                    |device| format!("{} [{}]", device.name, device.ip),
                );
                ui.horizontal(|ui| {
                    ui.label("Adresse IP / appareil");
                    egui::ComboBox::from_id_salt("remote-device").selected_text(label).show_ui(ui, |ui| {
                        for device in self.project.devices.iter().filter(|device| !device.ip.is_unspecified()) {
                            ui.selectable_value(&mut port.device, Some(device.id), format!("{} [{}]", device.name, device.ip));
                        }
                    });
                });
                let device = self.project.devices.iter().find(|d| Some(d.id) == port.device);
                if previous != port.device {
                    port.remote_ip = device.map(|d| d.ip);
                }
                ui.label(format!("Type d’appareil   {}", device.map_or("—", Device::profile_label)));
                if kind.network_input() {
                    ui.horizontal(|ui| {
                        ui.label("Type de bloc");
                        egui::ComboBox::from_id_salt("remote-block-type")
                            .selected_text(remote_block_label(port.source, analog))
                            .show_ui(ui, |ui| {
                                let kinds = if analog {
                                    [RemoteBlock::AnalogInput, RemoteBlock::AnalogOutput, RemoteBlock::AnalogMarker]
                                } else {
                                    [RemoteBlock::Input, RemoteBlock::Output, RemoteBlock::Marker]
                                };
                                for remote in kinds {
                                    let option = NetworkSource::RemoteBlock(remote);
                                    ui.selectable_value(&mut port.source, option, remote_block_label(option, analog));
                                }
                                ui.selectable_value(&mut port.source, NetworkSource::RemoteVm, remote_block_label(NetworkSource::RemoteVm, analog));
                            });
                    });
                }
                if matches!(port.source, NetworkSource::RemoteBlock(_)) {
                    ui.horizontal(|ui| {
                        ui.label("Numéro de bloc");
                        ui.add(egui::DragValue::new(&mut port.address).range(1..=u16::MAX));
                    });
                } else {
                    vm_fields(ui, port, analog);
                }
            }
            _ => {
                ui.label("Appareil");
                let previous = port.device;
                device_picker(ui, "block-device", &self.project.devices, &mut port.device, |_| true);
                if port.device != previous {
                    port.connection = None;
                }
                if port.source == NetworkSource::Simulated && let Some(device) = port.device {
                    let connections: Vec<_> = self.project.connections.iter().filter_map(|link| {
                        let peer = link.peer(device)?;
                        let name = &self.project.devices.iter().find(|d| d.id == peer)?.name;
                        Some((link.id, format!("{name} · {}", link.protocol.label())))
                    }).collect();
                    ui.label("Connexion simulée");
                    egui::ComboBox::from_id_salt("block-connection")
                        .selected_text(connections.iter().find(|(id, _)| Some(*id) == port.connection)
                            .map_or("Aucune", |(_, label)| label.as_str()))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut port.connection, None, "Aucune");
                            for (id, label) in connections {
                                ui.selectable_value(&mut port.connection, Some(id), label);
                            }
                        });
                }
                ui.horizontal(|ui| {
                    ui.label("Adresse");
                    let range = port.source.live_count().map_or(0..=u16::MAX, |max| 1..=max);
                    ui.add(egui::DragValue::new(&mut port.address).range(range));
                });
            }
        }
    }

    fn apply_network(&mut self, mut candidate: Project) -> Option<&'static str> {
        if let Some(drawing) = &mut candidate.network_drawing
            && (candidate.connections != self.project.connections
                || candidate.devices.len() != self.project.devices.len())
        {
            drawing.paths.clear();
            drawing
                .positions
                .retain(|(id, _)| candidate.devices.iter().any(|d| d.id == *id));
            drawing
                .external
                .retain(|id| candidate.devices.iter().any(|d| d.id == *id));
        }
        if let Err(error) = candidate.document_valid() {
            self.status = error.into();
            return Some(error);
        }
        if candidate != self.project {
            self.remember();
            self.project = candidate;
        }
        self.status.clear();
        None
    }
}

fn remote_block_label(source: NetworkSource, analog: bool) -> &'static str {
    match source {
        NetworkSource::RemoteBlock(block) => block.kind().symbol(),
        _ if analog => "VM · mot",
        _ => "VM · bit",
    }
}

fn vm_fields(ui: &mut egui::Ui, port: &mut NetworkPort, analog: bool) {
    ui.horizontal(|ui| {
        ui.label(if analog { "Adresse du mot VM" } else { "Adresse de l’octet VM" });
        ui.add(egui::DragValue::new(&mut port.address).range(0..=if analog { 1022 } else { 1023 }));
        if !analog {
            ui.label("Bit");
            ui.add(egui::DragValue::new(&mut port.bit).range(0..=7));
        }
    });
}

fn device_icon(painter: &egui::Painter, center: egui::Pos2, computer: bool) {
    let body = egui::Rect::from_center_size(
        center,
        if computer {
            vec2(31.0, 22.0)
        } else {
            vec2(28.0, 35.0)
        },
    );
    painter.rect_filled(body, 2, egui::Color32::from_rgb(51, 63, 79));
    let screen = egui::Rect::from_center_size(
        center - vec2(0.0, 2.0),
        if computer {
            vec2(23.0, 14.0)
        } else {
            vec2(16.0, 12.0)
        },
    );
    painter.rect_filled(screen, 1, if computer { CARD } else { OUTPUT });
    let bottom = body.center_bottom() - vec2(0.0, 4.0);
    for x in [-7.0, 0.0, 7.0] {
        painter.circle_filled(bottom + vec2(x, 0.0), 1.4, MUTED);
    }
}

fn device_picker(
    ui: &mut egui::Ui,
    salt: &str,
    devices: &[Device],
    selection: &mut Option<u32>,
    filter: impl Fn(u32) -> bool,
) {
    egui::ComboBox::from_id_salt(salt)
        .selected_text(
            devices
                .iter()
                .find(|d| Some(d.id) == *selection)
                .map_or("Aucun", |d| d.name.as_str()),
        )
        .show_ui(ui, |ui| {
            ui.selectable_value(selection, None, "Aucun");
            for device in devices.iter().filter(|d| filter(d.id)) {
                ui.selectable_value(selection, Some(device.id), &device.name);
            }
        });
}

fn clear_removed_ports(project: &mut Project) {
    for block in project.blocks.iter_mut().chain(
        project
            .sheets
            .iter_mut()
            .flat_map(|sheet| &mut sheet.blocks),
    ) {
        if block
            .network
            .device
            .is_some_and(|id| !project.devices.iter().any(|d| d.id == id))
        {
            block.network = Default::default();
        } else if block
            .network
            .connection
            .is_some_and(|id| !project.connections.iter().any(|c| c.id == id))
        {
            block.network.connection = None;
        }
    }
}
