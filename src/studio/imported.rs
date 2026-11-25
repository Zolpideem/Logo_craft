use eframe::egui::{
    self, Align2, Color32, FontId, Pos2, Rect, RichText, Sense, Shape, Stroke, StrokeKind, Ui,
    Vec2, vec2,
};

use crate::mnp::{Diagram, Network};

use super::{FileAction, Studio, theme::*};

pub(super) struct Imported {
    network: Network,
    selected: Option<usize>,
    zoom: f32,
    pan: Vec2,
    is_network: bool,
    topology: bool,
}

impl Imported {
    pub(super) fn new(network: Network, is_network: bool) -> Self {
        let selected = network
            .devices
            .iter()
            .position(|device| device.diagram.is_some());
        Self {
            network,
            selected,
            zoom: 1.0,
            pan: Vec2::ZERO,
            is_network,
            topology: is_network,
        }
    }

    fn select(&mut self, device: usize) {
        if self.selected != Some(device) {
            self.selected = Some(device);
            self.zoom = 1.0;
            self.pan = Vec2::ZERO;
        }
    }

    fn devices(&mut self, ui: &mut Ui) {
        ui.heading("Projet réseau");
        ui.strong(&self.network.name);
        ui.separator();
        let mut selected = None;
        egui::ScrollArea::vertical().show(ui, |ui| {
            for (index, device) in self.network.devices.iter().enumerate() {
                let response = ui.selectable_label(self.selected == Some(index), &device.name);
                if response.clicked() {
                    selected = Some(index);
                }
                ui.indent(index, |ui| {
                    if !device.address.is_empty() {
                        ui.label(
                            RichText::new(&device.address)
                                .color(MUTED)
                                .monospace()
                                .size(11.0),
                        );
                    }
                    if let Some((name, _)) = &device.diagram {
                        if ui.link(name).clicked() {
                            selected = Some(index);
                        }
                    }
                });
                ui.add_space(5.0);
            }
        });
        if let Some(device) = selected {
            self.select(device);
        }
    }

    fn topology(&mut self, ui: &mut Ui) {
        panel(SHELL, 10, 8).corner_radius(8).show(ui, |ui| {
            ui.strong("Vue réseau");
            let scale = 0.66;
            let card = vec2(105.0, 58.0);
            let controllers = self
                .network
                .devices
                .iter()
                .enumerate()
                .filter(|(_, device)| !device.external);
            let mut right = 310.0_f32;
            let mut bottom = 130.0_f32;
            let mut locations = Vec::new();
            for (row, (index, device)) in controllers.enumerate() {
                let [x, y] = device
                    .position
                    .unwrap_or([44.0 + row as f32 * 180.0, 154.0]);
                let position = vec2(x.max(0.0), y.max(0.0) - 55.0) * scale;
                right = right.max(position.x + card.x + 12.0);
                bottom = bottom.max(position.y + card.y + 26.0);
                locations.push((index, position));
            }
            for [x, y] in self.network.paths.iter().flat_map(|path| &path.points) {
                right = right.max(x * scale + 12.0);
                bottom = bottom.max((y - 55.0) * scale + 8.0);
            }
            let mut selected = None;
            egui::ScrollArea::both()
                .id_salt("mnp-topology")
                .max_height(178.0)
                .show(ui, |ui| {
                    let (rect, _) = ui.allocate_exact_size(
                        vec2(ui.available_width().max(right), bottom),
                        Sense::hover(),
                    );
                    let point = |[x, y]: [f32; 2]| rect.min + vec2(x, y - 55.0) * scale;
                    let painter = ui.painter();
                    for path in &self.network.paths {
                        painter.add(Shape::line(
                            path.points.iter().copied().map(point).collect(),
                            Stroke::new(1.25_f32, if path.local { HIGH } else { ACCENT }),
                        ));
                    }
                    let local_port = self
                        .network
                        .paths
                        .iter()
                        .find(|path| path.local)
                        .and_then(|path| path.points.last())
                        .copied()
                        .unwrap_or([155.0, 115.0]);
                    let local = point(local_port);
                    let computer =
                        Rect::from_min_size(local - vec2(card.x * 0.68, 36.0), vec2(card.x, 36.0));
                    painter.rect_filled(computer, 6, CARD);
                    painter.text(
                        computer.center(),
                        Align2::CENTER_CENTER,
                        "PC local",
                        FontId::proportional(12.0),
                        MUTED,
                    );
                    for (index, position) in locations {
                        let device = &self.network.devices[index];
                        let area = Rect::from_min_size(rect.min + position, card);
                        let response = ui.interact(area, ui.id().with(index), Sense::click());
                        if response.clicked() {
                            selected = Some(index);
                        }
                        painter.rect_filled(
                            area,
                            6,
                            if self.selected == Some(index) {
                                SELECTED
                            } else {
                                CARD
                            },
                        );
                        let text_painter =
                            painter.with_clip_rect(area.shrink(3.0).intersect(ui.clip_rect()));
                        text_painter.text(
                            area.min + vec2(7.0, 5.0),
                            Align2::LEFT_TOP,
                            &device.name,
                            FontId::proportional(12.0),
                            INK,
                        );
                        text_painter.text(
                            area.min + vec2(7.0, 20.0),
                            Align2::LEFT_TOP,
                            &device.address,
                            FontId::monospace(10.0),
                            MUTED,
                        );
                        response.on_hover_text(&device.name);
                    }
                });
            if let Some(device) = selected {
                self.select(device);
            }
        });
        ui.add_space(6.0);
    }

    fn diagram(&mut self, ui: &mut Ui) {
        let Some(device) = self.selected.and_then(|id| self.network.devices.get(id)) else {
            ui.label("Ce projet ne contient aucun schéma.");
            return;
        };
        let Some((name, result)) = &device.diagram else {
            ui.heading(&device.name);
            if !device.address.is_empty() {
                ui.label(&device.address);
            }
            for link in &device.connections {
                let role = if link.client { "Client" } else { "Serveur" };
                ui.label(format!("{role} {} · {}", link.protocol, link.remote));
            }
            return;
        };
        panel(SHELL, 10, 6).corner_radius(8).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.strong(name);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("+").clicked() {
                        self.zoom = (self.zoom * 1.2).min(4.0);
                    }
                    ui.label(format!("{:.0} %", self.zoom * 100.0));
                    if ui.button("−").clicked() {
                        self.zoom = (self.zoom / 1.2).max(0.25);
                    }
                    if ui.button("Vue initiale").clicked() {
                        self.zoom = 1.0;
                        self.pan = Vec2::ZERO;
                    }
                });
            });
        });
        match result {
            Ok(diagram) => {
                let (area, response) = ui.allocate_exact_size(ui.available_size(), Sense::drag());
                if response.dragged() {
                    self.pan += response.drag_delta();
                }
                if response.hovered() {
                    let scroll = ui.input(|input| input.smooth_scroll_delta);
                    self.pan += scroll;
                }
                draw(diagram, ui, area, self.pan, self.zoom);
            }
            Err(error) => {
                ui.colored_label(OUTPUT, format!("Impossible d’afficher ce schéma. {error}"));
            }
        }
    }
}

impl Studio {
    pub(super) fn show_imported(&mut self, ctx: &egui::Context) {
        if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::O)) {
            self.open(ctx);
        }
        if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::N)) {
            self.ask(FileAction::New, ctx);
        }
        egui::TopBottomPanel::top("mnp-header")
            .frame(panel(DARK, 15, 8))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Atelier logique").strong().size(20.0));
                    ui.separator();
                    ui.menu_button("Fichier", |ui| {
                        if ui.button("Nouveau   Ctrl+N").clicked() {
                            ui.close();
                            self.ask(FileAction::New, ctx);
                        }
                        if ui.button("Ouvrir…   Ctrl+O").clicked() {
                            ui.close();
                            self.open(ctx);
                        }
                    });
                    if let Some(imported) = &mut self.imported
                        && imported.is_network
                    {
                        ui.toggle_value(&mut imported.topology, "Vue réseau");
                    }
                });
            });
        let Some(imported) = &mut self.imported else {
            ctx.request_repaint();
            return;
        };
        egui::TopBottomPanel::bottom("mnp-status")
            .frame(panel(DARK, 15, 6))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let status = if self.status.is_empty() {
                        if imported.is_network {
                            "Projet Siemens · Lecture seule"
                        } else {
                            "Schéma Siemens · Lecture seule"
                        }
                    } else {
                        &self.status
                    };
                    ui.label(RichText::new(status).color(MUTED).size(12.0));
                    if let Some(diagram) = imported
                        .selected
                        .and_then(|index| imported.network.devices[index].diagram.as_ref())
                        .and_then(|(_, result)| result.as_ref().ok())
                    {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(format!(
                                "{} blocs · {} fils",
                                diagram.blocks.len(),
                                diagram.wires.len()
                            ));
                            if diagram.omitted != 0 {
                                ui.colored_label(
                                    OUTPUT,
                                    format!("{} éléments non affichés", diagram.omitted),
                                );
                            }
                        });
                    }
                });
            });
        if imported.is_network {
            egui::SidePanel::left("mnp-devices")
                .default_width(200.0)
                .min_width(155.0)
                .frame(panel(SHELL, 12, 10))
                .show(ctx, |ui| imported.devices(ui));
        }
        egui::CentralPanel::default()
            .frame(panel(DARK, 8, 8))
            .show(ctx, |ui| {
                if imported.topology {
                    imported.topology(ui);
                }
                imported.diagram(ui);
            });
    }
}

fn draw(diagram: &Diagram, ui: &Ui, area: Rect, pan: Vec2, zoom: f32) {
    let painter = ui.painter().with_clip_rect(area.intersect(ui.clip_rect()));
    painter.rect_filled(area, 6, DIAGRAM_PAPER);
    let transform = |[x, y]: [f32; 2]| area.min + vec2(16.0, 16.0) + pan + vec2(x, y) * zoom;
    let ink = Color32::from_rgb(31, 42, 62);
    for path in &diagram.wires {
        painter.add(Shape::line(
            path.iter().copied().map(transform).collect(),
            Stroke::new(1.0_f32, WIRE),
        ));
    }
    for block in &diagram.blocks {
        let rect = Rect::from_min_size(
            transform(block.position),
            vec2(block.size[0], block.size[1]) * zoom,
        );
        let symbol_size =
            11.0_f32.min(block.size[0] / (0.65 * block.symbol.chars().count().max(1) as f32));
        let symbol = FontId::monospace((symbol_size * zoom).clamp(2.0, 44.0));
        painter.rect_filled(rect, 2, Color32::from_rgb(255, 223, 150));
        painter.rect_stroke(rect, 2, Stroke::new(1.0_f32, ink), StrokeKind::Inside);
        for input in &block.inputs {
            let [a, b] = input.ends.map(transform);
            if input.inverted {
                painter.circle_filled(a.lerp(b, 0.5), (a.x - b.x).abs() * 0.5, Color32::BLACK);
            } else {
                painter.line_segment([a, b], Stroke::new(1.0_f32, ink));
            }
        }
        for &output in &block.outputs {
            painter.line_segment(output.map(transform), Stroke::new(1.0_f32, ink));
        }
        painter.text(
            rect.center(),
            Align2::CENTER_CENTER,
            &block.symbol,
            symbol,
            ink,
        );
        let label = if block.description.is_empty() {
            block.number.clone()
        } else {
            format!("{} ({})", block.number, block.description)
        };
        painter.text(
            rect.left_top() - vec2(0.0, 2.0),
            Align2::LEFT_BOTTOM,
            label,
            FontId::proportional((10.0 * zoom).clamp(5.0, 40.0)),
            ink,
        );
    }
    for (path, label) in &diagram.connectors {
        let mut points: Vec<Pos2> = path.iter().copied().map(transform).collect();
        let bounds = Rect::from_points(&points);
        points.push(points[0]);
        let mut fill = egui::Mesh::default();
        let green = Color32::from_rgb(137, 207, 171);
        fill.colored_vertex(bounds.center(), green);
        for &point in &points {
            fill.colored_vertex(point, green);
        }
        for index in 1..points.len() as u32 {
            fill.add_triangle(0, index, index + 1);
        }
        painter.add(Shape::mesh(fill));
        painter.add(Shape::line(points, Stroke::new(1.0_f32, ink)));
        painter.text(
            bounds.left_top(),
            Align2::LEFT_BOTTOM,
            label,
            FontId::proportional((9.0 * zoom).clamp(5.0, 36.0)),
            ink,
        );
    }
    for (point, text) in &diagram.comments {
        painter.text(
            transform(*point),
            Align2::LEFT_TOP,
            text,
            FontId::proportional((10.0 * zoom).clamp(5.0, 40.0)),
            DIAGRAM_MUTED,
        );
    }
}
