use std::{
    collections::HashSet,
    fs,
    io::Write,
    path::PathBuf,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use eframe::egui::{
    self, Align, Align2, Color32, FontId, Frame, Id, Key, Layout, PointerButton, Pos2, Rect,
    RichText, Sense, Stroke, StrokeKind, Ui, Vec2, pos2, vec2,
};

use crate::logo::LogoMonitor;
use crate::mnp;
use crate::model::{Block, Kind, Project, ProjectSimulation, SCAN_MS, Signal, Wire};

mod drawing;
mod network;
mod panels;
pub(crate) mod theme;

use theme::*;

const NODE_W: f32 = 25.0;
const GRID: f32 = 20.0;

enum FileAction {
    New,
    Open(PathBuf),
    Example,
    NetworkExample,
    Close,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Port {
    Input(u32, usize),
    Output(u32),
}

pub struct Studio {
    project: Project,
    saved: Project,
    path: Option<PathBuf>,
    undo: Vec<Project>,
    redo: Vec<Project>,
    selected: HashSet<u32>,
    selection_start: Option<Pos2>,
    pan_anchor: Option<(PointerButton, Pos2)>,
    place: Option<Kind>,
    wire_start: Option<Port>,
    port_press: Option<(Port, Pos2)>,
    selected_wire: Option<Wire>,
    block_properties: bool,
    cut_tool: bool,
    last_input_click: Option<((u32, usize), f64)>,
    show_palette: bool,
    show_inspector: bool,
    show_network: bool,
    network_height: Option<f32>,
    network_ui: network::NetworkUi,
    grid: bool,
    offset: Vec2,
    zoom: f32,
    simulation: Option<(Instant, ProjectSimulation)>,
    logo: LogoMonitor,
    pending: Option<FileAction>,
    allow_close: bool,
    status: String,
    show_import_warnings: bool,
}

impl Studio {
    pub fn new() -> Self {
        let project = Project::example();
        Self {
            saved: project.clone(),
            project,
            path: None,
            undo: Vec::new(),
            redo: Vec::new(),
            selected: HashSet::new(),
            selection_start: None,
            pan_anchor: None,
            place: None,
            wire_start: None,
            port_press: None,
            selected_wire: None,
            block_properties: false,
            cut_tool: false,
            last_input_click: None,
            show_palette: true,
            show_inspector: true,
            show_network: false,
            network_height: None,
            network_ui: network::NetworkUi::default(),
            grid: false,
            offset: Vec2::ZERO,
            zoom: 1.0,
            simulation: None,
            logo: LogoMonitor::default(),
            pending: None,
            allow_close: false,
            status: String::new(),
            show_import_warnings: false,
        }
    }

    pub fn load_network_example(&mut self) {
        self.load(Project::network_example(), None);
    }

    fn editing(&self) -> bool {
        self.simulation.is_none()
    }
    fn dirty(&self) -> bool {
        if self.project.active_sheet == self.saved.active_sheet {
            return self.project != self.saved;
        }
        let mut saved = self.saved.clone();
        if let Some(sheet) = self.project.active_sheet {
            saved.switch_sheet(sheet);
        }
        self.project != saved
    }

    fn remember(&mut self) {
        if self.undo.len() == 60 {
            self.undo.remove(0);
        }
        self.undo.push(self.project.clone());
        self.redo.clear();
    }

    fn undo(&mut self, redo: bool) {
        let (from, to) = if redo {
            (&mut self.redo, &mut self.undo)
        } else {
            (&mut self.undo, &mut self.redo)
        };
        if let Some(project) = from.pop() {
            to.push(std::mem::replace(&mut self.project, project));
            self.reset_selection();
            self.network_ui = network::NetworkUi::default();
        }
    }

    fn remove_selected(&mut self) {
        if !self.selected.is_empty() {
            self.remember();
            self.project
                .blocks
                .retain(|b| !self.selected.contains(&b.id));
            self.project
                .wires
                .retain(|w| !self.selected.contains(&w.from) && !self.selected.contains(&w.to));
            if let Some(drawing) = &mut self.project.drawing {
                drawing.routes.retain(|route| {
                    !self.selected.contains(&route.wire.from)
                        && !self.selected.contains(&route.wire.to)
                });
            }
            self.selected.clear();
            self.cancel_wiring();
        }
    }

    fn select_block(&mut self, id: u32) {
        self.selected_wire = None;
        self.selected.clear();
        self.selected.insert(id);
    }

    fn select_sheet(&mut self, index: usize) {
        if self.project.active_sheet == Some(index) {
            return;
        }
        if let Some((_, simulation)) = &mut self.simulation
            && let Err(error) = simulation.select(index)
        {
            self.status = error.into();
            return;
        }
        self.project.switch_sheet(index);
        self.saved.switch_sheet(index);
        self.reset_selection();
        self.offset = vec2(16.0, 16.0);
        self.zoom = 1.0;
    }

    fn reset_selection(&mut self) {
        self.selected.clear();
        self.selection_start = None;
        self.pan_anchor = None;
        self.place = None;
        self.cancel_wiring();
        self.selected_wire = None;
        self.block_properties = false;
        self.cut_tool = false;
    }

    fn load(&mut self, project: Project, path: Option<PathBuf>) {
        self.show_import_warnings = false;
        self.saved = project.clone();
        self.project = project;
        self.path = path;
        self.reset_selection();
        self.undo.clear();
        self.redo.clear();
        self.simulation = None;
        self.offset = if self.project.drawing.is_some() {
            vec2(16.0, 16.0)
        } else {
            Vec2::ZERO
        };
        self.zoom = 1.0;
        self.status.clear();
        self.show_network = !self.project.devices.is_empty();
        self.network_ui = network::NetworkUi::default();
    }

    fn ask(&mut self, action: FileAction, ctx: &egui::Context) {
        if self.dirty() {
            self.pending = Some(action);
        } else {
            self.file_action(action, ctx);
        }
    }

    fn file_action(&mut self, action: FileAction, ctx: &egui::Context) {
        match action {
            FileAction::New => self.load(Project::empty(), None),
            FileAction::Example => self.load(Project::example(), None),
            FileAction::NetworkExample => self.load_network_example(),
            FileAction::Close => {
                self.allow_close = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            FileAction::Open(path) => {
                self.open_path(path);
            }
        }
    }

    pub fn open_path(&mut self, path: PathBuf) {
        if mnp::is_project(&path) || mnp::is_diagram(&path) {
            match mnp::open_editable(&path) {
                Ok(project) => self.load(project, None),
                Err(error) => self.status = format!("Ouverture impossible. {error}"),
            }
        } else {
            match Self::read_project(&path) {
                Ok(project) => self.load(project, Some(path)),
                Err(error) => self.status = format!("Ouverture impossible. {error}"),
            }
        }
    }

    fn open(&mut self, ctx: &egui::Context) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter(
                "Fichiers compatibles",
                &["logique", mnp::EXTENSION, mnp::DIAGRAM_EXTENSION],
            )
            .add_filter("Projet Atelier logique", &["logique"])
            .add_filter("Projet réseau (.mnp)", &[mnp::EXTENSION])
            .add_filter("Schéma (.lsc)", &[mnp::DIAGRAM_EXTENSION])
            .pick_file()
        else {
            return;
        };
        self.ask(FileAction::Open(path), ctx);
    }

    fn pick_project() -> Option<PathBuf> {
        rfd::FileDialog::new()
            .add_filter("Projet Atelier logique", &["logique"])
            .pick_file()
    }

    fn read_project(path: &std::path::Path) -> Result<Project, Box<dyn std::error::Error>> {
        if fs::metadata(path)?.len() > 4_000_000 {
            return Err("Le fichier est trop volumineux.".into());
        }
        let project: Project = serde_json::from_slice(&fs::read(path)?)?;
        project.document_valid()?;
        Ok(project)
    }

    fn pick_subcircuit(&mut self, block: usize) {
        let Some(path) = Self::pick_project() else {
            return;
        };
        let result: Result<Project, Box<dyn std::error::Error>> = (|| {
            let circuit = Self::read_project(&path)?;
            let mut updated = self.project.clone();
            updated.blocks[block].subcircuit = Some(Box::new(circuit));
            updated.order()?;
            Ok(updated)
        })();
        match result {
            Ok(updated) => {
                self.remember();
                self.project = updated;
                self.status = "Schéma associé à la fonction utilisateur.".into();
            }
            Err(error) => self.status = format!("Import impossible. {error}"),
        }
    }

    fn save(&mut self, choose_path: bool) -> bool {
        let path = if !choose_path && self.path.is_some() {
            self.path.clone()
        } else {
            rfd::FileDialog::new()
                .add_filter("Projet Atelier logique", &["logique"])
                .set_file_name(
                    self.path
                        .as_ref()
                        .and_then(|path| path.file_name())
                        .and_then(|name| name.to_str())
                        .map(str::to_owned)
                        .unwrap_or_else(|| {
                            let name = std::path::Path::new(&self.project.name)
                                .file_stem()
                                .and_then(|name| name.to_str())
                                .unwrap_or("schema");
                            let name =
                                name.replace(['<', '>', ':', '"', '/', '\\', '|', '?', '*'], "_");
                            format!("{name}.logique")
                        }),
                )
                .save_file()
        };
        let Some(path) = path else { return false };
        if path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| !extension.eq_ignore_ascii_case("logique"))
        {
            self.status =
                "Choisissez un fichier .logique pour enregistrer le projet Atelier.".into();
            return false;
        }
        let path = if path.extension().is_none() {
            path.with_extension("logique")
        } else {
            path
        };
        self.save_to(path)
    }

    fn save_to(&mut self, path: PathBuf) -> bool {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let temp = path.with_file_name(format!(".atelier-{}-{stamp}.tmp", std::process::id()));
        let result: Result<(), Box<dyn std::error::Error>> = (|| {
            self.project.document_valid()?;
            let bytes = serde_json::to_vec_pretty(&self.project)?;
            if bytes.len() > 4_000_000 {
                return Err("Le fichier est trop volumineux.".into());
            }
            let mut file = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temp)?;
            let result = file.write_all(&bytes).and_then(|()| file.sync_all());
            drop(file);
            let result = result.and_then(|()| fs::rename(&temp, &path));
            if result.is_err() {
                let _ = fs::remove_file(&temp);
            }
            result?;
            Ok(())
        })();
        match result {
            Ok(()) => {
                self.path = Some(path);
                self.saved = self.project.clone();
                self.status = "Projet enregistré.".into();
                true
            }
            Err(error) => {
                self.status = format!("Enregistrement impossible. {error}");
                false
            }
        }
    }

    fn toggle_simulation(&mut self) {
        if self.simulation.take().is_some() {
            self.status.clear();
            return;
        }
        match ProjectSimulation::new(&self.project) {
            Ok(mut sim) => {
                sim.scan(0.0, |ip, source, address| self.logo.digital_signal(ip, source, address));
                self.simulation = Some((Instant::now(), sim));
                self.cancel_wiring();
                self.selected_wire = None;
                self.block_properties = false;
                self.place = None;
                self.cut_tool = false;
                self.status.clear();
            }
            Err(error) => self.status = error.into(),
        }
    }

    fn signal(&self, index: usize) -> Signal {
        self.simulation
            .as_ref()
            .map_or_else(Signal::default, |(_, sim)| sim.signal(index))
    }

    fn switch_input(&mut self, index: usize) {
        if let Some((start, sim)) = &mut self.simulation {
            sim.signals()[index].input = !sim.signal(index).input;
            sim.scan(start.elapsed().as_secs_f64(), |ip, source, address| self.logo.digital_signal(ip, source, address));
        }
    }

    fn connect(&mut self, from: u32, to: u32, pin: usize) {
        let wire = Wire::new(from, to, pin);
        if self.project.wires.contains(&wire) {
            self.status.clear();
            return;
        }
        let mut candidate = self.project.clone();
        candidate.wires.retain(|w| w.to != to || w.pin != pin);
        candidate.wires.push(wire);
        if let Some(drawing) = &mut candidate.drawing {
            drawing
                .routes
                .retain(|route| route.wire.to != to || route.wire.pin != pin);
        }
        let result = if candidate.drawing.is_some() {
            let source = candidate.block(from).unwrap();
            let target = candidate.block(to).unwrap();
            if from == to || source.kind.analog_value() != target.kind.analog_pin(pin) {
                Err("Choisissez deux blocs distincts et des broches de même type.")
            } else {
                candidate.document_valid()
            }
        } else {
            candidate.order().map(|_| ())
        };
        if let Err(error) = result {
            self.status = error.into();
            return;
        }
        self.remember();
        self.project = candidate;
        self.status.clear();
    }

    fn cancel_wiring(&mut self) {
        self.wire_start = None;
        self.port_press = None;
        self.last_input_click = None;
    }

    fn join_ports(&mut self, first: Port, second: Port) -> bool {
        match (first, second) {
            (Port::Output(from), Port::Input(to, pin))
            | (Port::Input(to, pin), Port::Output(from)) => {
                self.cancel_wiring();
                self.select_block(to);
                self.connect(from, to, pin);
                true
            }
            _ => false,
        }
    }

    fn click_port(&mut self, port: Port, now: f64, double_click_delay: f64) {
        if let Some(start) = self.wire_start
            && self.join_ports(start, port)
        {
            return;
        }
        let id = match port {
            Port::Input(id, pin) => {
                let key = (id, pin);
                if let Some(block) = self.project.block(id)
                    && block.kind.can_invert_pin(pin)
                {
                    if self.last_input_click.is_some_and(|(previous, time)| {
                        previous == key && now - time < double_click_delay
                    }) {
                        self.remember();
                        self.project.block_mut(id).unwrap().inverted_pins ^= 1 << pin;
                        self.cancel_wiring();
                        self.status.clear();
                        return;
                    }
                    self.last_input_click = Some((key, now));
                }
                id
            }
            Port::Output(id) => {
                self.last_input_click = None;
                id
            }
        };
        self.select_block(id);
        self.wire_start = (self.wire_start != Some(port)).then_some(port);
        self.status.clear();
    }

    fn disconnect(&mut self, wire: Wire) {
        self.remember();
        self.project.wires.retain(|w| *w != wire);
        if let Some(drawing) = &mut self.project.drawing {
            drawing.routes.retain(|route| {
                (route.wire.from, route.wire.to, route.wire.pin) != (wire.from, wire.to, wire.pin)
            });
        }
        self.selected_wire = None;
        self.status.clear();
    }

    pub fn show(&mut self, ctx: &egui::Context) {
        if ctx.input(|i| i.viewport().close_requested()) && self.dirty() && !self.allow_close {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.pending = Some(FileAction::Close);
        }
        if let Some((start, sim)) = &mut self.simulation {
            sim.scan(start.elapsed().as_secs_f64(), |ip, source, address| self.logo.digital_signal(ip, source, address));
            ctx.request_repaint_after(Duration::from_millis(SCAN_MS));
        }
        if self.pending.is_none() {
            self.keys(ctx);
        }
        self.header(ctx);
        self.status_bar(ctx);
        let max_dock_width = (ctx.screen_rect().width() * 0.30).clamp(190.0, 360.0);
        egui::SidePanel::left("palette")
            .default_width(202.0)
            .width_range(180.0..=max_dock_width)
            .frame(panel(SHELL, 12, 13))
            .show_animated(ctx, self.show_palette, |ui| self.palette(ui));
        egui::SidePanel::right("inspector")
            .default_width(219.0)
            .width_range(190.0..=max_dock_width)
            .frame(panel(SHELL, 13, 13))
            .show_animated(ctx, self.show_inspector, |ui| self.inspector(ui));
        egui::CentralPanel::default()
            .frame(Frame::new().fill(DARK).inner_margin(egui::Margin::same(8)))
            .show(ctx, |ui| {
                if self.show_network {
                    self.network_panel(ui);
                }
                if !self.project.sheets.is_empty() {
                    egui::ScrollArea::horizontal()
                        .id_salt("diagram-tabs")
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                let mut next = None;
                                for (index, sheet) in self.project.sheets.iter().enumerate() {
                                    if ui.selectable_label(self.project.active_sheet == Some(index), &sheet.name).clicked()
                                    {
                                        next = Some(index);
                                    }
                                }
                                if let Some(index) = next {
                                    self.select_sheet(index);
                                }
                            });
                        });
                    ui.add_space(4.0);
                }
                self.canvas(ui);
            });
        let network_block = self.selected.iter().next()
            .and_then(|&id| self.project.blocks.iter().position(|block| block.id == id))
            .filter(|&i| self.selected.len() == 1 && {
                let kind = self.project.blocks[i].kind;
                kind.network_input() || kind.network_output()
            });
        if self.block_properties && let Some(index) = network_block {
            self.network_block_dialog(ctx, index);
        } else if self.block_properties {
            let mut open = true;
            egui::Window::new("Paramètres du bloc")
                .id(Id::new("block-properties"))
                .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
                .default_width(340.0)
                .max_height(ctx.screen_rect().height() * 0.72)
                .open(&mut open)
                .show(ctx, |ui| self.inspector_contents(ui));
            self.block_properties &= open && self.selected.len() == 1;
        }
        if !self.block_properties || network_block.is_none() {
            self.network_ui.block_draft = None;
        }
        self.logo.show(ctx);
        if self.show_import_warnings {
            egui::Window::new("Détails de l’import")
                .open(&mut self.show_import_warnings)
                .show(ctx, |ui| {
                    ui.label(
                        "Les modifications sont enregistrées dans un projet Atelier (.logique).",
                    );
                    egui::ScrollArea::vertical()
                        .max_height(320.0)
                        .show(ui, |ui| {
                            for warning in &self.project.import_warnings {
                                ui.label(warning);
                            }
                        });
                });
        }
        self.confirm(ctx);
    }

    fn keys(&mut self, ctx: &egui::Context) {
        let shortcuts = ctx.input(|i| {
            (
                i.modifiers.command,
                i.modifiers.shift,
                i.key_pressed(Key::S),
                i.key_pressed(Key::O),
                i.key_pressed(Key::N),
                i.key_pressed(Key::F5),
                i.key_pressed(Key::Escape),
                i.key_pressed(Key::Delete),
                i.key_pressed(Key::Z),
                i.key_pressed(Key::Y),
            )
        });
        let (ctrl, shift, save, open, new, run, escape, delete, undo, redo) = shortcuts;
        if ctrl && save {
            self.save(shift);
        }
        if ctrl && open {
            self.open(ctx);
        }
        if ctrl && new {
            self.ask(FileAction::New, ctx);
        }
        if run {
            self.toggle_simulation();
        }
        if escape {
            self.block_properties = false;
            self.selected.clear();
            self.selection_start = None;
            self.place = None;
            self.cancel_wiring();
            self.selected_wire = None;
            self.cut_tool = false;
            self.status.clear();
        }
        if self.editing() && !ctx.wants_keyboard_input() {
            if delete {
                if let Some(wire) = self.selected_wire {
                    self.disconnect(wire);
                } else {
                    self.remove_selected();
                }
            }
            if ctrl && (undo || redo) {
                self.undo(redo || shift);
            }
        }
    }

    fn canvas(&mut self, ui: &mut Ui) {
        let mut fit_view = false;
        let mut zoom_factor = 1.0;
        panel(SHELL, 14, 8).corner_radius(8).show(ui, |ui| {
            ui.horizontal(|ui| {
                let title = self.project.active_name();
                let title = if title.trim().is_empty() {
                    "Schéma sans nom"
                } else {
                    title
                };
                let modified = if self.dirty() { " *" } else { "" };
                let width = (ui.available_width() - 284.0).max(25.0);
                ui.allocate_ui_with_layout(
                    vec2(width, 27.0),
                    Layout::left_to_right(Align::Center),
                    |ui| {
                        ui.set_min_width(width);
                        ui.add(
                            egui::Label::new(
                                RichText::new(format!("{title}{modified}"))
                                    .strong()
                                    .size(17.0),
                            )
                            .truncate(),
                        );
                    },
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.button("+").on_hover_text("Agrandir").clicked() {
                        zoom_factor = 1.15;
                    }
                    ui.label(
                        RichText::new(format!("{:.0} %", self.zoom * 100.0))
                            .monospace()
                            .color(MUTED)
                            .size(11.0),
                    );
                    if ui.button("−").on_hover_text("Réduire").clicked() {
                        zoom_factor = 1.0 / 1.15;
                    }
                    if ui
                        .button("Cadrer")
                        .on_hover_text("Afficher tout le schéma")
                        .clicked()
                    {
                        fit_view = true;
                    }
                    if ui
                        .add_enabled(
                            self.editing(),
                            egui::Button::new("Couper / Joindre").selected(self.cut_tool),
                        )
                        .on_hover_text(
                            "Afficher une liaison sous forme de renvois ou rétablir son fil",
                        )
                        .clicked()
                    {
                        self.cut_tool = !self.cut_tool;
                        self.place = None;
                        self.cancel_wiring();
                        self.status.clear();
                    }
                });
            });
        });
        let (board, painter) =
            ui.allocate_painter(ui.available_size_before_wrap(), Sense::click_and_drag());
        let area = board.rect;
        let painter = painter.with_clip_rect(area.intersect(ui.clip_rect()));
        painter.rect_filled(area, 8, DIAGRAM_PAPER);
        painter.rect_stroke(area, 8, Stroke::new(1.0_f32, BORDER), StrokeKind::Inside);
        if fit_view {
            self.fit(area.size());
            ui.ctx().request_repaint();
        }
        if zoom_factor != 1.0 {
            self.zoom_at(area.center() - area.min, zoom_factor);
            ui.ctx().request_repaint();
        }

        if let Some((button, previous)) = self.pan_anchor.take() {
            let (pointer, down) =
                ui.input(|i| (i.pointer.interact_pos(), i.pointer.button_down(button)));
            if let Some(pointer) = pointer {
                self.offset += pointer - previous;
                if down {
                    self.pan_anchor = Some((button, pointer));
                }
            }
        } else if board.contains_pointer() {
            self.pan_anchor = ui.input(|i| {
                let button = [PointerButton::Secondary, PointerButton::Middle]
                    .into_iter()
                    .find(|&button| {
                        i.pointer.button_pressed(button) && i.pointer.button_down(button)
                    })?;
                Some((button, i.pointer.interact_pos()?))
            });
        }
        if self.pan_anchor.is_some() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
        }
        if let Some(pointer) = ui
            .input(|i| i.pointer.hover_pos())
            .filter(|_| board.contains_pointer())
        {
            let scroll = ui.input(|i| i.smooth_scroll_delta.y);
            if scroll.abs() > 0.01 {
                self.zoom_at(pointer - area.min, (scroll * 0.003).exp());
            }
        }
        let origin = area.min + self.offset;
        let zoom = self.zoom;
        if self.grid {
            let pitch = GRID * zoom;
            let start_x = area.left() + (origin.x - area.left()).rem_euclid(pitch);
            let start_y = area.top() + (origin.y - area.top()).rem_euclid(pitch);
            let mut x = start_x;
            while x < area.right() {
                let mut y = start_y;
                while y < area.bottom() {
                    painter.circle_filled(pos2(x, y), 0.85, DIAGRAM_GRID);
                    y += pitch;
                }
                x += pitch;
            }
        }

        let to_screen = |b: &Block| {
            Rect::from_min_size(
                origin + vec2(b.x, b.y) * zoom,
                drawing::block_size(b) * zoom,
            )
        };
        let mut nodes = std::collections::HashMap::new();
        for (i, block) in self.project.blocks.iter().enumerate() {
            nodes.insert(block.id, (i, to_screen(block)));
        }

        let port_position = |port| match port {
            Port::Input(id, pin) => {
                let (index, rect) = nodes[&id];
                drawing::input_position(&self.project.blocks[index], rect, pin, zoom)
            }
            Port::Output(id) => {
                let (index, rect) = nodes[&id];
                drawing::output_position(&self.project.blocks[index], rect, zoom)
            }
        };
        let area = area.intersect(ui.clip_rect());
        let ports: Vec<_> = self
            .project
            .blocks
            .iter()
            .filter(|_| self.editing() && !self.cut_tool && self.place.is_none())
            .flat_map(|block| {
                (0..block.kind.pins().len())
                    .filter(|&pin| drawing::has_input(block, pin))
                    .map(|pin| Port::Input(block.id, pin))
                    .chain(block.has_output().then_some(Port::Output(block.id)))
            })
            .map(|port| (port, port_position(port)))
            .collect();
        let port_hit = |point: Pos2| {
            if !area.contains(point) || ui.ctx().layer_id_at(point) != Some(ui.layer_id()) {
                return None;
            }
            ports
                .iter()
                .rev()
                .filter(|&&(port, at)| {
                    area.contains(at) && port_bounds(at, port, zoom).contains(point)
                })
                .min_by(|(_, a), (_, b)| a.distance_sq(point).total_cmp(&b.distance_sq(point)))
                .map(|&(port, _)| port)
        };
        let hovered_port = ui.input(|i| i.pointer.hover_pos()).and_then(port_hit);
        if hovered_port.is_some() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
        }
        let primary_events = ui.input(|i| {
            i.events
                .iter()
                .filter_map(|event| match *event {
                    egui::Event::PointerButton {
                        pos,
                        button: PointerButton::Primary,
                        pressed,
                        ..
                    } => Some((pos, pressed)),
                    _ => None,
                })
                .collect::<Vec<_>>()
        });
        let mut port_used = self.port_press.is_some();
        for (point, pressed) in primary_events {
            let hit = port_hit(point);
            if pressed {
                self.port_press = hit.map(|port| (port, point));
                port_used |= self.port_press.is_some();
                if hit.is_none() {
                    self.last_input_click = None;
                }
            } else if let Some((start, press_point)) = self.port_press.take() {
                if let Some(end) = hit
                    && self.join_ports(start, end)
                {
                    continue;
                }
                if hit == Some(start) && press_point.distance(point) <= 6.0 {
                    self.click_port(
                        start,
                        ui.input(|i| i.time),
                        ui.ctx().options(|o| o.input_options.max_double_click_delay),
                    );
                } else {
                    self.cancel_wiring();
                }
            }
        }
        if !ui.input(|i| i.pointer.primary_down()) {
            self.port_press = None;
        }

        if board.drag_started_by(PointerButton::Primary)
            && self.place.is_none()
            && !self.cut_tool
            && self.pan_anchor.is_none()
            && !port_used
        {
            self.selection_start = ui.input(|i| i.pointer.press_origin());
            self.cancel_wiring();
            self.selected_wire = None;
            self.status.clear();
        }
        let selection = self
            .selection_start
            .zip(board.interact_pointer_pos())
            .map(|(start, end)| Rect::from_two_pos(start, end).intersect(area));
        if let Some(rect) = selection {
            self.selected = nodes
                .iter()
                .filter(|(_, (_, bounds))| rect.intersects(*bounds))
                .map(|(&id, _)| id)
                .collect();
        }
        if !board.dragged_by(PointerButton::Primary) {
            self.selection_start = None;
        }

        let wire_pointer = ui.input(|i| i.pointer.hover_pos()).filter(|p| {
            self.editing()
                && self.place.is_none()
                && self.wire_start.is_none()
                && self.port_press.is_none()
                && hovered_port.is_none()
                && area.contains(*p)
                && !nodes.values().any(|(_, rect)| rect.contains(*p))
        });
        if let Some(drawing) = &self.project.drawing {
            drawing::background(&painter, drawing, origin, zoom);
        }
        let routes: std::collections::HashMap<_, _> = self
            .project
            .drawing
            .iter()
            .flat_map(|drawing| &drawing.routes)
            .map(|route| ((route.wire.from, route.wire.to, route.wire.pin), route))
            .collect();
        let mut wire_hit = None;
        let mut hit_distance = 6.0;
        for (wire_index, &w) in self.project.wires.iter().enumerate() {
            let (index, start) = nodes[&w.from];
            let (target, end) = nodes[&w.to];
            let source_block = &self.project.blocks[index];
            let target_block = &self.project.blocks[target];
            let a = drawing::output_position(source_block, start, zoom);
            let b = drawing::input_position(target_block, end, w.pin, zoom);
            let appearance = routes.get(&(w.from, w.to, w.pin));
            let anchors = [
                [source_block.x, source_block.y],
                [target_block.x, target_block.y],
            ];
            let high = self.signal(index).high;
            let selected = self.selected_wire == Some(w);
            let color = if selected {
                Color32::from_rgb(37, 101, 198)
            } else if self.project.blocks[index].kind.analog_value() {
                Color32::from_rgb(107, 71, 158)
            } else if high {
                Color32::from_rgb(0, 122, 92)
            } else {
                WIRE
            };
            let stroke = Stroke::new(if selected || high { 1.5_f32 } else { 0.9_f32 }, color);
            let mut consider_hit = |distance| {
                if distance < hit_distance {
                    hit_distance = distance;
                    wire_hit = Some(wire_index);
                }
            };
            if w.cut {
                if let Some(route) = appearance
                    && let Some(connectors) = &route.connectors
                {
                    for (index, connector) in connectors.iter().enumerate() {
                        let [x, y] = anchors[index];
                        let [old_x, old_y] = route.anchors[index];
                        let marker_origin = origin + vec2(x - old_x, y - old_y) * zoom;
                        if let Some(stubs) = &route.cut_stubs {
                            let mut points: Vec<_> = stubs[index]
                                .iter()
                                .map(|&[x, y]| marker_origin + vec2(x, y) * zoom)
                                .collect();
                            if index == 0 {
                                points.insert(0, a);
                            } else {
                                points.push(b);
                            }
                            if let Some(pointer) = wire_pointer {
                                consider_hit(drawing::path_distance(pointer, &points));
                            }
                            painter.add(egui::Shape::line(points, stroke));
                        }
                        let center =
                            drawing::connector(&painter, connector, marker_origin, zoom, stroke);
                        if let Some(pointer) = wire_pointer {
                            consider_hit(center.distance(pointer) / zoom.max(1.0));
                        }
                    }
                    continue;
                }
                let marker = |end, direction| {
                    let center = end + vec2(12.0 * zoom * direction, 0.0);
                    let tip = center + vec2(7.0 * zoom, 0.0);
                    let back = center - vec2(2.0 * zoom, 0.0);
                    painter.line_segment([end, if direction > 0.0 { back } else { tip }], stroke);
                    painter.add(egui::Shape::convex_polygon(
                        vec![
                            back - vec2(0.0, 4.0 * zoom),
                            center - vec2(0.0, 4.0 * zoom),
                            tip,
                            center + vec2(0.0, 4.0 * zoom),
                            back + vec2(0.0, 4.0 * zoom),
                        ],
                        if high {
                            HIGH
                        } else {
                            Color32::from_rgb(137, 207, 171)
                        },
                        stroke,
                    ));
                    center
                };
                let from = marker(a, 1.0);
                let to = marker(b, -1.0);
                let target_label = target_block.kind.pins()[w.pin];
                for (center, label) in [
                    (from, format!("{}/{target_label}", target_block.name)),
                    (to, source_block.name.clone()),
                ] {
                    painter.text(
                        center - vec2(0.0, 5.0 * zoom),
                        Align2::CENTER_BOTTOM,
                        label,
                        FontId::proportional(8.0 * zoom),
                        color,
                    );
                    if let Some(pointer) = wire_pointer {
                        consider_hit(center.distance(pointer));
                    }
                }
                continue;
            }
            let points = if let Some(route) = appearance
                && route.anchors == anchors
                && !route.route.is_empty()
            {
                std::iter::once(a)
                    .chain(route.route.iter().map(|&[x, y]| origin + vec2(x, y) * zoom))
                    .chain(std::iter::once(b))
                    .collect()
            } else if b.x >= a.x + 16.0 * zoom {
                let mid = (a.x + b.x) * 0.5;
                vec![a, pos2(mid, a.y), pos2(mid, b.y), b]
            } else {
                let lower = start.bottom().max(end.bottom()) + 18.0 * zoom;
                vec![
                    a,
                    a + vec2(13.0 * zoom, 0.0),
                    pos2(a.x + 13.0 * zoom, lower),
                    pos2(b.x - 13.0 * zoom, lower),
                    b - vec2(13.0 * zoom, 0.0),
                    b,
                ]
            };
            if let Some(pointer) = wire_pointer {
                consider_hit(drawing::path_distance(pointer, &points));
            }
            painter.add(egui::Shape::line(points, stroke));
        }
        if wire_hit.is_some() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        if board.secondary_clicked() {
            self.selected.clear();
            self.selected_wire = wire_hit.map(|index| self.project.wires[index]);
        }
        if let Some(wire) = self.selected_wire {
            board.context_menu(|ui| {
                if ui.button("Supprimer la liaison").clicked() {
                    self.disconnect(wire);
                    ui.close();
                }
            });
        }
        if let Some(start) = self.port_press.map(|(port, _)| port).or(self.wire_start)
            && let Some(pointer) = ui
                .input(|i| i.pointer.hover_pos())
                .filter(|p| area.contains(*p))
            && let Some(&(_, from)) = ports.iter().find(|&&(port, _)| port == start)
        {
            painter.line_segment([from, pointer], Stroke::new(1.5_f32, HIGH));
        }

        let mut node_used = port_used;
        let mut block_to_remove = None;
        for i in 0..self.project.blocks.len() {
            let block = self.project.blocks[i].clone();
            let r = to_screen(&block);
            let id = Id::new(("block", block.id));
            if !r.expand(15.0 * zoom).intersects(area) && ui.ctx().dragged_id() != Some(id) {
                continue;
            }
            let signal = self.signal(i);
            let interaction = if self.editing() {
                Sense::click_and_drag()
            } else {
                Sense::click()
            };
            let response = ui
                .interact(r.intersect(area), id, interaction)
                .on_hover_text(format!("{} · {}", block.name, block.kind.label()));
            if response.clicked() && !port_used {
                node_used = true;
                self.select_block(block.id);
                if block.kind.simulated_key() && !self.editing() {
                    self.switch_input(i);
                }
            }
            if response.double_clicked() && self.editing() && !port_used {
                self.select_block(block.id);
                self.block_properties = true;
            }
            if self.editing() {
                response.context_menu(|ui| {
                    if ui.button("Paramètres du bloc…").clicked() {
                        self.select_block(block.id);
                        self.block_properties = true;
                        ui.close();
                    }
                    if ui.button("Supprimer le bloc").clicked() {
                        block_to_remove = Some(block.id);
                        ui.close();
                    }
                });
            }
            if response.dragged_by(PointerButton::Primary)
                && self.editing()
                && !self.cut_tool
                && self.place.is_none()
                && self.pan_anchor.is_none()
                && !port_used
            {
                let delta = if response.drag_started() {
                    self.remember();
                    if !self.selected.contains(&block.id) {
                        self.select_block(block.id);
                    }
                    ui.input(|i| {
                        i.pointer
                            .interact_pos()
                            .zip(i.pointer.press_origin())
                            .map_or(Vec2::ZERO, |(current, start)| current - start)
                    })
                } else {
                    ui.input(|i| i.pointer.delta())
                } / zoom;
                for b in &mut self.project.blocks {
                    if self.selected.contains(&b.id) {
                        b.x += delta.x;
                        b.y += delta.y;
                    }
                }
                ui.ctx().request_repaint();
            }

            let selected = self.selected.contains(&block.id);
            let network = block.kind.network_input() || block.kind.network_output();
            painter.rect_filled(
                r,
                0,
                if signal.high {
                    HIGH
                } else if network {
                    Color32::from_rgb(141, 219, 167)
                } else {
                    Color32::from_rgb(255, 222, 139)
                },
            );
            painter.rect_stroke(
                r,
                0,
                Stroke::new(
                    if selected { 2.0_f32 } else { 0.9_f32 },
                    if selected {
                        Color32::from_rgb(59, 81, 179)
                    } else {
                        DARK
                    },
                ),
                StrokeKind::Outside,
            );
            let caption = block
                .network
                .device
                .filter(|_| block.drawing.is_none())
                .and_then(|id| self.project.devices.iter().find(|d| d.id == id))
                .map_or_else(
                    || block.name.clone(),
                    |d| format!("{} · {}", block.name, d.name),
                );
            drawing::symbol(&painter, &block, r, zoom);
            painter.text(
                r.left_top() - vec2(0.0, 2.0 * zoom),
                Align2::LEFT_BOTTOM,
                caption,
                FontId::proportional(9.5 * zoom),
                DARK,
            );

            drawing::extra_pins(&painter, &block, r, zoom);
            for (pin, label) in block.kind.pins().iter().enumerate() {
                if !drawing::has_input(&block, pin) {
                    continue;
                }
                let port = Port::Input(block.id, pin);
                let at = drawing::input_position(&block, r, pin, zoom);
                let wired = self
                    .project
                    .wires
                    .iter()
                    .any(|w| w.to == block.id && w.pin == pin);
                let can_invert = block.kind.can_invert_pin(pin);
                let mut inverted = block.pin_inverted(pin);
                if self.editing() && !self.cut_tool && area.contains(at) {
                    let response = ui.interact(
                        port_bounds(at, port, zoom).intersect(area),
                        Id::new(("in", block.id, pin)),
                        Sense::click(),
                    );
                    response.context_menu(|ui| {
                        if can_invert && ui.checkbox(&mut inverted, "Inverser l’entrée").changed()
                        {
                            self.remember();
                            self.project.blocks[i].inverted_pins ^= 1 << pin;
                            ui.close();
                        }
                        if let Some(wire) = self
                            .project
                            .wires
                            .iter()
                            .find(|w| w.to == block.id && w.pin == pin)
                            .copied()
                            && ui.button("Débrancher l’entrée").clicked()
                        {
                            self.disconnect(wire);
                            ui.close();
                        }
                    });
                    if hovered_port == Some(port) && can_invert {
                        let action = if inverted { "rétablir" } else { "inverser" };
                        response.on_hover_text(format!(
                            "Entrée {label} · Double-cliquez pour {action}"
                        ));
                    } else if hovered_port == Some(port) {
                        response.on_hover_text(format!("Entrée {label}"));
                    }
                }
                if inverted {
                    let radius = (3.0 * zoom).max(1.0);
                    painter.circle_filled(at - vec2(radius, 0.0), radius, Color32::BLACK);
                } else {
                    self.paint_port(
                        &painter,
                        at,
                        wired || self.wire_start == Some(port) || hovered_port == Some(port),
                    );
                }
            }
            if block.has_output() {
                let port = Port::Output(block.id);
                let at = drawing::output_position(&block, r, zoom);
                if hovered_port == Some(port) {
                    ui.interact(
                        port_bounds(at, port, zoom).intersect(area),
                        Id::new(("out", block.id)),
                        Sense::hover(),
                    )
                    .on_hover_text(if block.kind.analog_value() {
                        "Sortie analogique"
                    } else {
                        "Sortie numérique"
                    });
                }
                self.paint_port(
                    &painter,
                    at,
                    self.wire_start == Some(port) || signal.high || hovered_port == Some(port),
                );
            }

            if !self.editing() && block.kind.analog_value() {
                painter.text(
                    r.center_bottom() - vec2(0.0, 3.0) * zoom,
                    Align2::CENTER_BOTTOM,
                    signal.analog.to_string(),
                    FontId::monospace(11.0 * zoom),
                    DARK,
                );
            } else if !self.editing() {
                let at = r.center_bottom() - vec2(0.0, 7.0) * zoom;
                painter.text(
                    at,
                    Align2::CENTER_CENTER,
                    if signal.high { "1" } else { "0" },
                    FontId::monospace(9.0 * zoom),
                    DARK,
                );
            }
        }
        if let Some(id) = block_to_remove {
            self.select_block(id);
            self.remove_selected();
            return;
        }
        if let Some(rect) = selection.filter(|_| self.selection_start.is_some()) {
            painter.rect_filled(rect, 0, ACCENT.linear_multiply(0.12));
            painter.rect_stroke(rect, 0, Stroke::new(1.0_f32, ACCENT), StrokeKind::Inside);
        }
        if board.clicked() && !node_used {
            if self.cut_tool {
                if let Some(wire) = wire_hit {
                    self.remember();
                    self.project.wires[wire].cut ^= true;
                    self.selected_wire = None;
                    ui.ctx().request_repaint();
                }
            } else if let Some(kind) = self.place.take() {
                if self.project.blocks.len() >= 2000 {
                    self.status = "Le schéma a atteint la limite de 2 000 blocs.".into();
                    return;
                }
                if let Some(pointer) = board.interact_pointer_pos() {
                    let point = (pointer - origin) / zoom - vec2(NODE_W, node_height(kind)) * 0.5;
                    let snap = |v: f32| {
                        if self.grid {
                            (v / GRID).round() * GRID
                        } else {
                            v
                        }
                    };
                    let (x, y) = (snap(point.x), snap(point.y));
                    self.remember();
                    let id = self.project.add(kind, x, y);
                    self.select_block(id);
                    self.status.clear();
                }
            } else if let Some(index) = wire_hit {
                self.selected.clear();
                self.selected_wire = Some(self.project.wires[index]);
                self.status.clear();
            } else {
                self.selected.clear();
                self.selected_wire = None;
                self.cancel_wiring();
                self.status.clear();
            }
        }
        if self.project.blocks.is_empty() {
            painter.text(
                area.center(),
                Align2::CENTER_CENTER,
                "Choisissez un bloc dans la bibliothèque.",
                FontId::proportional(15.0),
                DIAGRAM_MUTED,
            );
        }
    }

    fn zoom_at(&mut self, anchor: Vec2, factor: f32) {
        let previous = self.zoom;
        self.zoom = (self.zoom * factor).clamp(0.35, 2.5);
        self.offset = anchor - (anchor - self.offset) * (self.zoom / previous);
    }

    fn fit(&mut self, available: Vec2) {
        if self.project.blocks.is_empty() {
            self.zoom = 1.0;
            self.offset = Vec2::ZERO;
            return;
        }
        let bounds = self.project.blocks.iter().fold(Rect::NOTHING, |bounds, b| {
            bounds.union(Rect::from_min_size(pos2(b.x, b.y), drawing::block_size(b)))
        });
        let scale = (available - Vec2::splat(44.0)).max(Vec2::splat(1.0)) / bounds.size();
        self.zoom = scale.x.min(scale.y).clamp(0.35, 1.25);
        self.offset =
            (available - bounds.size() * self.zoom) * 0.5 - bounds.min.to_vec2() * self.zoom;
    }

    fn paint_port(&self, painter: &egui::Painter, at: Pos2, active: bool) {
        let radius = (2.0 * self.zoom).clamp(1.0, 4.0);
        painter.circle(
            at,
            radius,
            if active { HIGH } else { DIAGRAM_PAPER },
            Stroke::new(0.8_f32, DIAGRAM_MUTED),
        );
    }

    fn confirm(&mut self, ctx: &egui::Context) {
        if self.pending.is_some() {
            egui::Modal::new(Id::new("unsaved")).show(ctx, |ui| {
                ui.heading("Modifications non enregistrées");
                ui.label("Enregistrer le schéma avant de continuer ?");
                ui.horizontal(|ui| {
                    if ui.button("Enregistrer").clicked()
                        && self.save(false)
                        && let Some(action) = self.pending.take()
                    {
                        self.file_action(action, ctx);
                    }
                    if ui.button("Ignorer les modifications").clicked()
                        && let Some(action) = self.pending.take()
                    {
                        self.file_action(action, ctx);
                    }
                    if ui.button("Annuler").clicked() {
                        self.pending = None;
                    }
                });
            });
        }
    }
}

fn port_bounds(at: Pos2, port: Port, zoom: f32) -> Rect {
    let outside = (8.0 * zoom).max(7.0);
    let inside = (4.0 * zoom).max(5.0);
    let height = (4.0 * zoom).max(5.0);
    let (left, right) = match port {
        Port::Input(..) => (outside, inside),
        Port::Output(..) => (inside, outside),
    };
    Rect::from_min_max(at - vec2(left, height), at + vec2(right, height))
}

fn node_height(kind: Kind) -> f32 {
    if kind.pins().len() > 1 { 39.0 } else { 35.0 }
}

fn in_pos(r: Rect, pin: usize, kind: Kind, zoom: f32) -> Pos2 {
    let middle = (kind.pins().len() as f32 - 1.0) * 0.5;
    pos2(r.left(), r.center().y + (pin as f32 - middle) * 9.0 * zoom)
}

#[cfg(test)]
mod tests {
    use crate::model::Simulation;
    use super::*;
    use egui::{Event, Modifiers, RawInput, Shape};

    fn frame(ctx: &egui::Context, studio: &mut Studio, events: Vec<Event>) -> egui::FullOutput {
        ctx.run(
            RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(800.0, 600.0))),
                events,
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| studio.canvas(ui));
            },
        )
    }

    fn button(pos: Pos2, button: PointerButton, pressed: bool) -> Event {
        Event::PointerButton {
            pos,
            button,
            pressed,
            modifiers: Modifiers::NONE,
        }
    }

    #[test]
    fn dragging_network_divider_resizes_both_views() {
        let ctx = egui::Context::default();
        let mut studio = Studio::new();
        studio.load_network_example();
        let mut draw = |events| {
            let mut divider = Pos2::ZERO;
            let mut program_height = 0.0;
            let _ = ctx.run(
                RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1000.0, 700.0))),
                    events,
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        studio.network_panel(ui);
                        let id = ui.id().with("network-divider");
                        divider = ctx.read_response(id).unwrap().rect.center();
                        program_height = ui.available_height();
                    });
                },
            );
            (divider, program_height)
        };
        let (start, before) = draw(vec![]);
        draw(vec![Event::PointerMoved(start)]);
        draw(vec![button(start, PointerButton::Primary, true)]);
        let end = start + vec2(0.0, 45.0);
        draw(vec![Event::PointerMoved(end)]);
        draw(vec![button(end, PointerButton::Primary, false)]);
        let (after, program_height) = draw(vec![]);
        assert!((after.y - start.y - 45.0).abs() < 1.0, "{start:?} → {after:?}");
        assert!((before - program_height - 45.0).abs() < 1.0);
    }

    #[test]
    #[ignore = "nécessite le projet Siemens de référence via ATELIER_MNP_TEST"]
    fn edits_and_saves_the_imported_maison_project() {
        let source = PathBuf::from(std::env::var_os("ATELIER_MNP_TEST").expect("ATELIER_MNP_TEST"));
        let original = fs::read(&source).unwrap();
        let mut studio = Studio::new();
        studio.open_path(source.clone());
        assert_eq!(studio.project.name, "Project_Maison", "{}", studio.status);
        assert_eq!(studio.project.sheets.len(), 2);
        assert!(!studio.project.import_warnings.is_empty());
        assert_eq!(studio.project.active_name(), "Domotique_Salon.lsc");
        assert!(studio.show_network && studio.show_inspector && studio.path.is_none());
        assert_eq!(
            studio.project.wires.iter().filter(|wire| wire.cut).count(),
            39
        );
        assert_eq!(
            studio.project.sheets[1]
                .wires
                .iter()
                .filter(|wire| wire.cut)
                .count(),
            45
        );
        let drawing = studio.project.drawing.as_ref().unwrap();
        assert!(drawing.loose_wires.is_empty() && drawing.loose_connectors.is_empty());
        let clock = studio.project.sheets[1]
            .blocks
            .iter()
            .find(|block| block.kind == Kind::WeeklyTimer)
            .unwrap();
        let cam = clock.schedule.as_ref().unwrap().cams[0];
        assert_eq!((cam.days, cam.start_minute, cam.end_minute), (85, 30, 270));
        let delay = studio.project.sheets[1]
            .blocks
            .iter()
            .find(|block| block.kind == Kind::OnDelay && block.name == "B025")
            .unwrap();
        assert_eq!(delay.duration, 7200.0);
        let gate = studio
            .project
            .blocks
            .iter()
            .position(|block| block.name == "B043")
            .unwrap();
        assert_eq!(studio.project.blocks[gate].kind, Kind::And);
        assert!(
            studio
                .project
                .wires
                .iter()
                .any(|wire| wire.to == studio.project.blocks[gate].id && wire.pin == 1)
        );

        let ctx = egui::Context::default();
        let first_frame = frame(&ctx, &mut studio, vec![]);
        let origin = first_frame
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                Shape::Rect(rect) if rect.fill == DIAGRAM_PAPER => Some(rect.rect.min),
                _ => None,
            })
            .unwrap();
        let block = &studio.project.blocks[gate];
        let gate_rect = Rect::from_min_size(
            origin + studio.offset + vec2(block.x, block.y),
            drawing::block_size(block),
        );
        let pin = drawing::input_position(block, gate_rect, 0, 1.0);
        assert!(first_frame.shapes.iter().any(|shape| matches!(
            &shape.shape, Shape::Circle(circle) if circle.center == pin && circle.fill == HIGH
        )));
        let center = gate_rect.center();
        let moved = center + vec2(16.0, 10.0);
        frame(
            &ctx,
            &mut studio,
            vec![
                Event::PointerMoved(center),
                button(center, PointerButton::Primary, true),
            ],
        );
        frame(&ctx, &mut studio, vec![Event::PointerMoved(moved)]);
        frame(
            &ctx,
            &mut studio,
            vec![button(moved, PointerButton::Primary, false)],
        );
        assert_eq!(
            studio.project.blocks[gate].x,
            gate_rect.left() - origin.x - studio.offset.x + 16.0
        );
        assert_eq!(
            studio.project.blocks[gate].y,
            gate_rect.top() - origin.y - studio.offset.y + 10.0
        );
        studio.undo(false);
        let cut = drawing_cut_at(&studio.project, "2/NI9");
        let marker = cut.connectors.as_ref().unwrap().get(1).unwrap();
        let bounds = Rect::from_points(
            &marker
                .path
                .iter()
                .map(|&[x, y]| pos2(x, y))
                .collect::<Vec<_>>(),
        );
        let point = origin + studio.offset + bounds.center().to_vec2();
        let cut_wire = cut.wire;
        for pressed in [true, false] {
            frame(
                &ctx,
                &mut studio,
                vec![
                    Event::PointerMoved(point),
                    button(point, PointerButton::Primary, pressed),
                ],
            );
        }
        assert_eq!(studio.selected_wire, Some(cut_wire));
        studio.disconnect(cut_wire);
        assert!(!studio.project.wires.contains(&cut_wire));
        assert!(
            !studio
                .project
                .drawing
                .as_ref()
                .unwrap()
                .routes
                .iter()
                .any(|route| route.wire == cut_wire)
        );
        studio.undo(false);
        assert_eq!(drawing_cut_at(&studio.project, "2/NI9").wire, cut_wire);

        // Repeat the B043–B044 connection from the screen recording.
        let gate_output = drawing::output_position(&studio.project.blocks[gate], gate_rect, 1.0);
        let next = studio
            .project
            .blocks
            .iter()
            .find(|b| b.name == "B044")
            .unwrap();
        let next_rect = Rect::from_min_size(
            origin + studio.offset + vec2(next.x, next.y),
            drawing::block_size(next),
        );
        let gate_id = studio.project.blocks[gate].id;
        let next_id = next.id;
        let occupied_input = drawing::input_position(next, next_rect, 1, 1.0);
        let free_input = drawing::input_position(next, next_rect, 3, 1.0);
        let from_gate = Wire::new(gate_id, next_id, 1);
        let added = Wire::new(gate_id, next_id, 3);
        assert!(studio.project.wires.contains(&from_gate));
        studio.disconnect(from_gate);
        let drag = |studio: &mut Studio, from, to| {
            frame(
                &ctx,
                studio,
                vec![
                    Event::PointerMoved(from),
                    button(from, PointerButton::Primary, true),
                ],
            );
            // Fast drags can deliver motion and release in the same frame.
            frame(
                &ctx,
                studio,
                vec![
                    Event::PointerMoved(to),
                    button(to, PointerButton::Primary, false),
                ],
            );
        };
        drag(&mut studio, occupied_input, gate_output);
        assert!(
            studio.project.wires.contains(&from_gate),
            "{}",
            studio.status
        );
        drag(&mut studio, gate_output, free_input + vec2(-5.0, 2.0));
        assert!(studio.project.wires.contains(&added), "{}", studio.status);
        assert!(studio.project.wires.contains(&from_gate));
        studio.undo(false);
        assert!(!studio.project.wires.contains(&added));
        for point in [free_input, gate_output] {
            frame(
                &ctx,
                &mut studio,
                vec![
                    Event::PointerMoved(point),
                    button(point, PointerButton::Primary, true),
                    button(point, PointerButton::Primary, false),
                ],
            );
        }
        assert!(studio.project.wires.contains(&added), "{}", studio.status);
        studio.undo(false);
        studio.undo(false);
        studio.undo(false);
        assert!(studio.project.wires.contains(&from_gate));
        assert!(!studio.dirty());

        studio.remember();
        let new_block_id = studio.project.add(Kind::Or, 310.0, 175.0);
        studio.zoom = 0.55;
        let new_block = studio.project.block(new_block_id).unwrap();
        let new_rect = Rect::from_min_size(
            origin + studio.offset + vec2(new_block.x, new_block.y) * studio.zoom,
            drawing::block_size(new_block) * studio.zoom,
        );
        let imported_output =
            origin + studio.offset + (gate_output - origin - studio.offset) * studio.zoom;
        let new_input = drawing::input_position(new_block, new_rect, 2, studio.zoom);
        drag(&mut studio, imported_output, new_input);
        assert!(
            studio
                .project
                .wires
                .contains(&Wire::new(gate_id, new_block_id, 2)),
            "{}",
            studio.status
        );
        studio.zoom = 1.0;
        studio.undo(false);
        studio.undo(false);
        assert!(!studio.dirty());

        let block = &studio.project.blocks[gate];
        let bounds = Rect::from_min_size(
            origin + studio.offset + vec2(block.x, block.y),
            drawing::block_size(block),
        );
        let pin = drawing::input_position(block, bounds, 1, 1.0);
        let was_inverted = block.pin_inverted(1);
        for _ in 0..2 {
            frame(
                &ctx,
                &mut studio,
                vec![
                    Event::PointerMoved(pin),
                    button(pin, PointerButton::Primary, true),
                ],
            );
            frame(
                &ctx,
                &mut studio,
                vec![button(pin, PointerButton::Primary, false)],
            );
        }
        assert_ne!(studio.project.blocks[gate].pin_inverted(1), was_inverted);
        let ctx = egui::Context::default();
        frame(&ctx, &mut studio, vec![]);
        let center = bounds.center();
        for _ in 0..2 {
            frame(
                &ctx,
                &mut studio,
                vec![
                    Event::PointerMoved(center),
                    button(center, PointerButton::Primary, true),
                ],
            );
            frame(
                &ctx,
                &mut studio,
                vec![button(center, PointerButton::Primary, false)],
            );
        }
        assert!(studio.block_properties);
        let edited_gate = studio.project.blocks[gate].clone();
        let source_block = studio
            .project
            .blocks
            .iter()
            .find(|block| block.name == "B045")
            .unwrap();
        let source_id = source_block.id;
        let source_rect = Rect::from_min_size(
            origin + studio.offset + vec2(source_block.x, source_block.y),
            drawing::block_size(source_block),
        );
        let from = drawing::output_position(source_block, source_rect, 1.0) + vec2(3.0, 0.0);
        let to = drawing::input_position(&edited_gate, bounds, 0, 1.0) - vec2(4.0, 0.0);
        studio.block_properties = false;
        frame(&ctx, &mut studio, vec![Event::PointerMoved(from)]);
        frame(
            &ctx,
            &mut studio,
            vec![button(from, PointerButton::Primary, true)],
        );
        frame(&ctx, &mut studio, vec![Event::PointerMoved(to)]);
        frame(
            &ctx,
            &mut studio,
            vec![button(to, PointerButton::Primary, false)],
        );
        let changed_wire = Wire::new(source_id, edited_gate.id, 0);
        assert!(
            studio.project.wires.contains(&changed_wire),
            "{}",
            studio.status
        );
        assert!(
            !studio
                .project
                .drawing
                .as_ref()
                .unwrap()
                .routes
                .iter()
                .any(|route| { route.wire.to == edited_gate.id && route.wire.pin == 0 })
        );
        studio.select_sheet(1);
        assert_eq!(studio.project.active_name(), "Domotique_Cellier.lsc");
        assert!(studio.dirty());

        let saved_path =
            std::env::temp_dir().join(format!("atelier-maison-{}.logique", std::process::id()));
        assert!(studio.save_to(saved_path.clone()), "{}", studio.status);
        assert!(studio.save_to(saved_path.clone()), "{}", studio.status);
        let mut reopened = Studio::new();
        reopened.open_path(saved_path.clone());
        fs::remove_file(saved_path).unwrap();
        assert_eq!(reopened.project, studio.project, "{}", reopened.status);
        assert_eq!(reopened.show_inspector, studio.show_inspector);
        reopened.select_sheet(0);
        assert_eq!(reopened.project.blocks[gate], edited_gate);
        assert!(reopened.project.wires.contains(&changed_wire));
        assert!(!reopened.dirty());
        assert_eq!(fs::read(source).unwrap(), original);
    }

    fn drawing_cut_at<'a>(project: &'a Project, label: &str) -> &'a crate::model::WireDrawing {
        project
            .drawing
            .as_ref()
            .unwrap()
            .routes
            .iter()
            .find(|route| {
                route
                    .connectors
                    .as_ref()
                    .is_some_and(|connectors| connectors[1].label == label)
            })
            .unwrap()
    }

    #[test]
    fn double_click_inverts_only_the_target_input_and_survives_saving() {
        let ctx = egui::Context::default();
        let mut studio = Studio::new();
        let output = frame(&ctx, &mut studio, vec![]);
        let origin = output
            .shapes
            .into_iter()
            .find_map(|shape| match shape.shape {
                Shape::Rect(rect) if rect.fill == DIAGRAM_PAPER => Some(rect.rect.min),
                _ => None,
            })
            .unwrap();
        let gate = &studio.project.blocks[2];
        let bounds = Rect::from_min_size(
            origin + vec2(gate.x, gate.y),
            vec2(NODE_W, node_height(gate.kind)),
        );
        let first = in_pos(bounds, 0, gate.kind, 1.0);
        let second = in_pos(bounds, 1, gate.kind, 1.0);
        let click = |studio: &mut Studio, at| {
            frame(
                &ctx,
                studio,
                vec![
                    Event::PointerMoved(at),
                    button(at, PointerButton::Primary, true),
                ],
            );
            frame(
                &ctx,
                studio,
                vec![button(at, PointerButton::Primary, false)],
            );
        };

        let original = studio.project.clone();
        click(&mut studio, first);
        click(&mut studio, second);
        assert_eq!(studio.project, original);
        click(&mut studio, second);
        assert!(!studio.project.blocks[2].pin_inverted(0));
        assert!(studio.project.blocks[2].pin_inverted(1));
        assert_eq!(studio.project.wires, original.wires);

        let saved = serde_json::to_vec(&studio.project).unwrap();
        let restored = serde_json::from_slice(&saved).unwrap();
        let mut sim = Simulation::new(&restored).unwrap();
        for (first, second) in [(false, false), (true, false), (true, true), (false, true)] {
            sim.signals[0].input = first;
            sim.signals[1].input = second;
            sim.scan(&restored, 0.0);
            assert_eq!(sim.signals[3].high, first && !second);
        }
        studio.undo(false);
        assert_eq!(studio.project, original);
        studio.undo(true);
        assert_eq!(studio.project, restored);
        click(&mut studio, second - vec2(3.0, 0.0));
        click(&mut studio, second - vec2(3.0, 0.0));
        assert_eq!(studio.project, original);
    }

    #[test]
    fn cut_wire_round_trips_and_can_be_joined_without_interrupting_signal() {
        let ctx = egui::Context::default();
        let mut studio = Studio::new();
        studio.cut_tool = true;
        let origin = frame(&ctx, &mut studio, vec![])
            .shapes
            .into_iter()
            .find_map(|shape| match shape.shape {
                Shape::Rect(rect) if rect.fill == DIAGRAM_PAPER => Some(rect.rect.min),
                _ => None,
            })
            .unwrap();
        let click = |studio: &mut Studio, at| {
            frame(
                &ctx,
                studio,
                vec![
                    Event::PointerMoved(at),
                    button(at, PointerButton::Primary, true),
                ],
            );
            frame(
                &ctx,
                studio,
                vec![button(at, PointerButton::Primary, false)],
            );
        };
        let original = studio.project.clone();
        let source = &original.blocks[0];
        let output = origin + vec2(source.x + NODE_W, source.y + node_height(source.kind) * 0.5);
        click(&mut studio, output + vec2(27.0, 0.0));
        assert!(studio.project.wires[0].cut);
        assert!(studio.project.wires[1..].iter().all(|wire| !wire.cut));
        let saved = serde_json::to_vec(&studio.project).unwrap();
        let restored = serde_json::from_slice(&saved).unwrap();
        assert_eq!(studio.project, restored);
        let mut sim = Simulation::new(&restored).unwrap();
        sim.signals[0].input = true;
        sim.signals[1].input = true;
        sim.scan(&restored, 0.0);
        assert!(sim.signals[3].high);
        click(&mut studio, output + vec2(12.0, 0.0));
        assert_eq!(studio.project, original);
        studio.undo(false);
        assert_eq!(studio.project, restored);
    }

    #[test]
    fn canvas_selects_and_moves_multiple_blocks_and_pans_at_zoom() {
        let ctx = egui::Context::default();
        let mut studio = Studio::new();
        let output = frame(&ctx, &mut studio, vec![]);
        let origin = output
            .shapes
            .into_iter()
            .find_map(|shape| match shape.shape {
                Shape::Rect(rect) if rect.fill == DIAGRAM_PAPER => Some(rect.rect.min),
                _ => None,
            })
            .unwrap();

        let start = origin + vec2(20.0, 25.0);
        let end = origin + vec2(70.0, 165.0);
        frame(
            &ctx,
            &mut studio,
            vec![
                Event::PointerMoved(start),
                button(start, PointerButton::Primary, true),
            ],
        );
        frame(&ctx, &mut studio, vec![Event::PointerMoved(end)]);
        frame(
            &ctx,
            &mut studio,
            vec![button(end, PointerButton::Primary, false)],
        );
        let original = studio.project.clone();
        assert_eq!(
            studio.selected,
            HashSet::from([original.blocks[0].id, original.blocks[1].id])
        );

        let block = &original.blocks[0];
        let start = origin + vec2(block.x + NODE_W * 0.5, block.y + 12.0);
        let delta = vec2(24.0, 16.0);
        let end = start + delta;
        frame(
            &ctx,
            &mut studio,
            vec![
                Event::PointerMoved(start),
                button(start, PointerButton::Primary, true),
            ],
        );
        frame(&ctx, &mut studio, vec![Event::PointerMoved(end)]);
        frame(
            &ctx,
            &mut studio,
            vec![button(end, PointerButton::Primary, false)],
        );
        for (index, (before, after)) in original
            .blocks
            .iter()
            .zip(&studio.project.blocks)
            .enumerate()
        {
            let expected = if index < 2 { delta } else { Vec2::ZERO };
            assert_eq!(vec2(after.x - before.x, after.y - before.y), expected);
        }
        studio.undo(false);
        assert!(studio.project == original);

        let anchor = start - origin;
        studio.zoom_at(anchor, 2.0);
        assert_eq!(studio.offset + anchor * studio.zoom, anchor);
        let offset = studio.offset;
        frame(&ctx, &mut studio, vec![Event::PointerMoved(start)]);
        frame(
            &ctx,
            &mut studio,
            vec![button(start, PointerButton::Secondary, true)],
        );
        frame(&ctx, &mut studio, vec![Event::PointerMoved(end)]);
        frame(
            &ctx,
            &mut studio,
            vec![button(end, PointerButton::Secondary, false)],
        );
        assert_eq!(studio.offset, offset + delta);
        assert!(studio.project == original);
    }
}
