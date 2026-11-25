use eframe::egui::{self, Align, Button, Layout, RichText, Sense, Stroke, Ui, Vec2, vec2};

use crate::model::{ANALOG_IO_MAX, COUNTER_MAX, Kind, MAX_DELAY, Operation};

use super::{FileAction, Studio, theme::*};

const ATELIER_FUNCTION_COUNT: usize = Kind::LOGIC.len()
    + Kind::TIMERS.len()
    + Kind::COUNTERS.len()
    + Kind::ANALOG.len()
    + Kind::OTHER.len();

impl Studio {
    pub(super) fn header(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("title")
            .frame(panel(DARK, 18, 11))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let (mark, _) = ui.allocate_exact_size(vec2(32.0, 32.0), Sense::hover());
                    let center = mark.center();
                    let left = center + vec2(-8.0, 4.0);
                    let middle = center + vec2(0.0, -4.0);
                    let right = center + vec2(8.0, 4.0);
                    ui.painter().rect_filled(mark, 9, ACCENT);
                    ui.painter().add(egui::Shape::line(
                        vec![left, middle, right],
                        Stroke::new(2.0_f32, DARK),
                    ));
                    for point in [left, middle, right] {
                        ui.painter().circle_filled(point, 2.5, DARK);
                    }
                    ui.label(
                        RichText::new("Atelier logique")
                            .color(INK)
                            .strong()
                            .size(20.0),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let (label, color, background) = if self.editing() {
                            ("Édition", ACCENT, SELECTED)
                        } else {
                            ("Simulation active", HIGH, LIVE)
                        };
                        panel(background, 10, 5).corner_radius(6).show(ui, |ui| {
                            ui.label(RichText::new(label).color(color).strong().size(12.0));
                            let (dot, _) = ui.allocate_exact_size(vec2(7.0, 7.0), Sense::hover());
                            ui.painter().circle_filled(dot.center(), 3.0, color);
                        });
                        ui.separator();
                        ui.toggle_value(&mut self.show_inspector, "Propriétés");
                        let palette_label = if self.editing() {
                            "Bibliothèque"
                        } else {
                            "Signaux"
                        };
                        ui.toggle_value(&mut self.show_palette, palette_label);
                    });
                });
            });
        egui::TopBottomPanel::top("toolbar")
            .frame(panel(SHELL, 14, 8))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.menu_button("Fichier", |ui| {
                        if ui.button("Nouveau   Ctrl+N").clicked() {
                            ui.close();
                            self.ask(FileAction::New, ctx);
                        }
                        if ui.button("Ouvrir…   Ctrl+O").clicked() {
                            ui.close();
                            self.open(ctx);
                        }
                        for (label, action) in [
                            ("Charger l’exemple", FileAction::Example),
                            ("Charger l’exemple réseau", FileAction::NetworkExample),
                        ] {
                            if ui.button(label).clicked() {
                                ui.close();
                                self.ask(action, ctx);
                            }
                        }
                        ui.separator();
                        if ui.button("Enregistrer   Ctrl+S").clicked() {
                            ui.close();
                            self.save(false);
                        }
                        if ui.button("Enregistrer sous…").clicked() {
                            ui.close();
                            self.save(true);
                        }
                    });
                    if ui.button("Enregistrer").on_hover_text("Ctrl+S").clicked() {
                        self.save(false);
                    }
                    if !self.project.import_warnings.is_empty()
                        && ui.button("Détails de l’import").clicked()
                    {
                        self.show_import_warnings = true;
                    }
                    ui.separator();
                    ui.selectable_value(&mut self.show_network, false, "Mode diagramme");
                    ui.selectable_value(&mut self.show_network, true, "Projet réseau");
                    if ui
                        .add_enabled(
                            self.editing() && !self.undo.is_empty(),
                            Button::new("Annuler"),
                        )
                        .on_hover_text("Ctrl+Z")
                        .clicked()
                    {
                        self.undo(false);
                    }
                    if ui
                        .add_enabled(
                            self.editing() && !self.redo.is_empty(),
                            Button::new("Rétablir"),
                        )
                        .on_hover_text("Ctrl+Y")
                        .clicked()
                    {
                        self.undo(true);
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let label = if self.editing() {
                            "▶  Simuler"
                        } else {
                            "■  Arrêter"
                        };
                        if ui
                            .add(
                                Button::new(RichText::new(label).strong().color(DARK))
                                    .fill(if self.editing() { ACCENT } else { HIGH }),
                            )
                            .on_hover_text("F5")
                            .clicked()
                        {
                            self.toggle_simulation();
                        }
                        if ui.selectable_label(self.grid, "Grille").clicked() {
                            self.grid = !self.grid;
                        }
                        if ui.button("Vue initiale").clicked() {
                            self.offset = Vec2::ZERO;
                            self.zoom = 1.0;
                        }
                    });
                });
            });
    }

    pub(super) fn status_bar(&self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("status").frame(panel(DARK, 15, 7)).show(ctx, |ui| {
            ui.horizontal(|ui| {
                let hint = if !self.status.is_empty() { self.status.as_str() }
                    else if !self.editing() { "Actionnez les entrées pour tester le circuit." }
                    else if self.cut_tool { "Cliquez sur un fil pour le couper ou sur un renvoi pour le rejoindre. Échap pour quitter l’outil." }
                    else if self.place.is_some() { "Cliquez sur la feuille pour poser le bloc. Échap pour annuler." }
                    else if let Some(start) = self.wire_start {
                        match start {
                            super::Port::Input(..) => "Choisissez une broche de sortie. Échap pour annuler.",
                            super::Port::Output(..) => "Choisissez une broche d’entrée. Échap pour annuler.",
                        }
                    }
                    else if self.selected_wire.is_some() { "Liaison sélectionnée · Suppr pour la retirer." }
                    else { "Cliquez sur deux broches ou glissez de l’une à l’autre pour les relier." };
                ui.add(egui::Label::new(RichText::new(hint).color(MUTED).size(12.0)).truncate());
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(RichText::new(format!("{} blocs   /   {} fils", self.project.blocks.len(), self.project.wires.len()))
                        .monospace().color(INK).size(12.0));
                });
            });
        });
    }

    pub(super) fn palette(&mut self, ui: &mut Ui) {
        heading(
            ui,
            if self.editing() {
                "Bibliothèque"
            } else {
                "Signaux"
            },
        );
        ui.separator();
        if self.editing() {
            egui::ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 1.0;
                    ui.label(RichText::new(format!("{ATELIER_FUNCTION_COUNT} blocs fonctionnels")).strong().size(12.0))
                        .on_hover_text("Nombre de types dans les familles Fonctions de base et Fonctions spéciales.");
                    ui.separator();
                    egui::CollapsingHeader::new(RichText::new("Constantes et E/S").strong())
                        .default_open(true)
                        .show(ui, |ui| {
                            self.palette_group(ui, "Numérique", Kind::IO, HIGH, true);
                            self.palette_group(ui, "Analogique", Kind::ANALOG_IO, ANALOG, false);
                            self.palette_group(
                                ui,
                                "Réseau",
                                Kind::NETWORK,
                                HIGH,
                                self.show_network,
                            );
                        });
                    self.palette_group(ui, "Fonctions de base", &Kind::LOGIC, ACCENT, true);
                    egui::CollapsingHeader::new(RichText::new("Fonctions spéciales").strong())
                        .default_open(true)
                        .show(ui, |ui| {
                            self.palette_group(ui, "Temporisations", Kind::TIMERS, OUTPUT, false);
                            self.palette_group(ui, "Compteurs", Kind::COUNTERS, OUTPUT, false);
                            self.palette_group(
                                ui,
                                "Fonctions analogiques",
                                Kind::ANALOG,
                                ANALOG,
                                false,
                            );
                            self.palette_group(ui, "Autres", Kind::OTHER, OUTPUT, false);
                        });
                    self.palette_group(
                        ui,
                        "Profil d’archive de variables",
                        Kind::ARCHIVE,
                        OUTPUT,
                        false,
                    );
                    self.palette_group(
                        ui,
                        "Fonctions utilisateur",
                        Kind::USER_DEFINED,
                        ACCENT,
                        false,
                    );
                });
        } else {
            egui::ScrollArea::vertical().show(ui, |ui| {
                for (index, block) in self.project.blocks.iter().enumerate() {
                    if block.kind == Kind::MessageText
                        && self.signal(index).high
                        && !block.message.is_empty()
                    {
                        ui.colored_label(OUTPUT, &block.message);
                    }
                }
                for (label, kind) in [
                    ("Entrées numériques", Kind::Input),
                    ("Touches fléchées", Kind::CursorKey),
                    ("Touches de fonction", Kind::FunctionKey),
                    ("Télérupteurs logiciels", Kind::SoftKey),
                    ("Sorties numériques", Kind::Output),
                    ("Entrées analogiques", Kind::AnalogInput),
                    ("Sorties analogiques", Kind::AnalogOutput),
                    ("Entrées réseau", Kind::NetInput),
                    ("Sorties réseau", Kind::NetOutput),
                    ("Entrées réseau analogiques", Kind::NetAnalogInput),
                    ("Sorties réseau analogiques", Kind::NetAnalogOutput),
                ] {
                    if !self.project.blocks.iter().any(|block| block.kind == kind) {
                        continue;
                    }
                    ui.label(RichText::new(label).color(MUTED).strong().size(12.0));
                    for i in 0..self.project.blocks.len() {
                        let block = &self.project.blocks[i];
                        if block.kind == kind {
                            let signal = self.signal(i);
                            let value = u8::from(if kind == Kind::SoftKey {
                                signal.input
                            } else {
                                signal.high
                            });
                            let caption = format!("{}   {value}", block.name);
                            if kind.simulated_key() {
                                if ui
                                    .add_sized(
                                        [ui.available_width(), 28.0],
                                        Button::new(caption).fill(if signal.high {
                                            LIVE
                                        } else {
                                            CARD
                                        }),
                                    )
                                    .clicked()
                                {
                                    self.switch_input(i);
                                }
                            } else if kind == Kind::AnalogInput {
                                let mut value = signal.analog_input;
                                let changed = ui
                                    .horizontal(|ui| {
                                        ui.label(&block.name);
                                        ui.add(
                                            egui::DragValue::new(&mut value)
                                                .range(0..=ANALOG_IO_MAX),
                                        )
                                        .on_hover_text("Valeur simulée de 0 à 1000")
                                        .changed()
                                    })
                                    .inner;
                                if changed && let Some((start, sim)) = &mut self.simulation {
                                    sim.signals()[i].analog_input = value;
                                    sim.scan(start.elapsed().as_secs_f64(), |ip, source, address| self.logo.digital_signal(ip, source, address));
                                }
                            } else if kind.analog_value() {
                                ui.label(format!("{}   {}", block.name, signal.analog));
                            } else {
                                ui.label(RichText::new(caption).color(if signal.high {
                                    HIGH
                                } else {
                                    MUTED
                                }));
                            }
                        }
                    }
                    ui.add_space(10.0);
                }
            });
        }
    }

    fn palette_group(
        &mut self,
        ui: &mut Ui,
        label: &str,
        kinds: &[Kind],
        color: egui::Color32,
        open: bool,
    ) {
        egui::CollapsingHeader::new(RichText::new(label).color(color).strong())
            .default_open(open)
            .show_unindented(ui, |ui| {
                ui.spacing_mut().button_padding.y = 1.0;
                ui.spacing_mut().interact_size.y = 14.0;
                for &kind in kinds {
                    let active = self.place == Some(kind);
                    if ui
                        .add_sized(
                            [ui.available_width(), 14.0],
                            Button::new(RichText::new(kind.label()).size(11.5))
                                .right_text(
                                    RichText::new(kind.symbol())
                                        .monospace()
                                        .size(11.5)
                                        .color(color),
                                )
                                .truncate()
                                .frame_when_inactive(false)
                                .selected(active),
                        )
                        .on_hover_text(kind.label())
                        .clicked()
                    {
                        self.place = if active { None } else { Some(kind) };
                        self.cut_tool = false;
                        self.cancel_wiring();
                        self.status.clear();
                    }
                }
            });
    }

    pub(super) fn inspector(&mut self, ui: &mut Ui) {
        heading(ui, "Propriétés");
        ui.separator();
        self.inspector_contents(ui);
    }

    pub(super) fn inspector_contents(&mut self, ui: &mut Ui) {
        egui::ScrollArea::vertical().id_salt("block-fields").show(ui, |ui| {
            if self.selected.len() > 1 {
                ui.label(format!("{} blocs sélectionnés", self.selected.len()));
                if self.editing()
                    && ui
                        .button("Supprimer les blocs")
                        .on_hover_text("Suppr")
                        .clicked()
                {
                    self.remove_selected();
                }
                return;
            }
            let Some(index) = self
                .selected
                .iter()
                .next()
                .and_then(|&id| self.project.blocks.iter().position(|b| b.id == id))
            else {
                let field = if self.project.sheets.len() > 1 { "Nom du projet" } else { "Nom du schéma" };
                ui.label(RichText::new(field).color(MUTED).size(12.0));
                let mut name = self.project.active_name().to_owned();
                if self.project.sheets.len() > 1 {
                    name.clone_from(&self.project.name);
                }
                if ui
                    .add_enabled(
                        self.editing(),
                        egui::TextEdit::singleline(&mut name).char_limit(100),
                    )
                    .changed()
                {
                    self.remember();
                    match self.project.active_sheet {
                        Some(index) if self.project.sheets.len() == 1 => self.project.sheets[index].name = name,
                        _ => self.project.name = name,
                    }
                }
                ui.add_space(8.0);
                ui.label(
                    RichText::new("Sélectionnez un bloc pour modifier ses paramètres.")
                        .color(MUTED),
                );
                return;
            };
            let block = self.project.blocks[index].clone();
            let id = block.id;
            let signal = self.signal(index);
            ui.label(
                RichText::new(block.kind.label())
                    .color(HIGH)
                    .strong()
                    .size(16.0),
            );
            ui.label(RichText::new("Nom du bloc").color(MUTED).size(12.0));
            let mut name = block.name.clone();
            if ui
                .add_enabled(
                    self.editing(),
                    egui::TextEdit::singleline(&mut name).char_limit(60),
                )
                .changed()
            {
                self.remember();
                self.project.blocks[index].name = name;
            }
            if block.kind.network_input() || block.kind.network_output() {
                self.network_block_summary(ui, &block);
            }
            let mut inverted_pins = block.inverted_pins;
            let mut inputs = block.kind.pins().iter().enumerate().filter(|(pin, _)| {
                block.kind.can_invert_pin(*pin)
                    && block.drawing.as_ref().is_none_or(|drawing| {
                        drawing.inputs.get(*pin).is_none_or(Option::is_some)
                    })
            }).peekable();
            if inputs.peek().is_some() {
                egui::CollapsingHeader::new("Inversion des entrées")
                    .id_salt(id)
                    .show(ui, |ui| {
                        ui.add_enabled_ui(self.editing(), |ui| {
                            for (pin, label) in inputs {
                                let mut inverted = block.pin_inverted(pin);
                                if ui.checkbox(&mut inverted, format!("Inverser {label}")).changed() {
                                    inverted_pins ^= 1 << pin;
                                }
                            }
                        });
                    });
            }
            if inverted_pins != block.inverted_pins {
                self.remember();
                self.project.blocks[index].inverted_pins = inverted_pins;
            }
            if block.kind.timer() || matches!(block.kind, Kind::Frequency | Kind::AnalogFilter) {
                let mut values = [block.duration, block.duration_low, block.pulse_delay];
                let labels: &[&str] = match block.kind {
                    Kind::OnOffDelay => &["Retard à la montée", "Retard à la descente"],
                    Kind::PulseGenerator => &["Durée de l’impulsion", "Durée de la pause"],
                    Kind::EdgePulse => &[
                        "Durée de l’impulsion",
                        "Durée de la pause",
                        "Retard initial",
                    ],
                    Kind::RandomDelay => {
                        &["Retard maximal à la montée", "Retard maximal à la descente"]
                    }
                    Kind::Pwm => &["Période"],
                    Kind::Frequency => &["Fenêtre de mesure"],
                    Kind::AnalogFilter => &["Constante de temps"],
                    Kind::Average => &["Fenêtre d’échantillonnage"],
                    Kind::PiController => &["Temps intégral"],
                    Kind::Stairway => &["Durée d’éclairage", "Préavis avant extinction"],
                    Kind::ComfortSwitch => &[
                        "Durée d’éclairage",
                        "Appui long",
                        "Préavis avant extinction",
                    ],
                    _ => &["Temporisation"],
                };
                if block.kind == Kind::Stairway {
                    values.swap(1, 2);
                }
                let mut changed = false;
                for (label, value) in labels.iter().zip(&mut values) {
                    ui.label(*label);
                    changed |= ui
                        .add_enabled(
                            self.editing(),
                            egui::DragValue::new(value)
                                .range(0.0..=MAX_DELAY)
                                .speed(0.1)
                                .suffix(" s"),
                        )
                        .changed();
                }
                if changed {
                    self.remember();
                    if block.kind == Kind::Stairway {
                        values.swap(1, 2);
                    }
                    [
                        self.project.blocks[index].duration,
                        self.project.blocks[index].duration_low,
                        self.project.blocks[index].pulse_delay,
                    ] = values;
                }
                if !self.editing() {
                    ui.label(format!("Écoulé  {:.2} s", signal.elapsed));
                }
            }
            if matches!(
                block.kind,
                Kind::Amplifier
                    | Kind::Threshold
                    | Kind::Comparator
                    | Kind::Differential
                    | Kind::Watchdog
                    | Kind::Pwm
                    | Kind::Frequency
                    | Kind::PiController
            ) {
                let mut params = block.analog;
                let mut changed = false;
                if block.kind != Kind::Frequency {
                    ui.label("Gain");
                    changed |= ui
                        .add_enabled(
                            self.editing(),
                            egui::DragValue::new(&mut params.gain)
                                .range(-10.0..=10.0)
                                .speed(0.01)
                                .max_decimals(2),
                        )
                        .changed();
                    ui.label("Décalage");
                    changed |= ui
                        .add_enabled(
                            self.editing(),
                            egui::DragValue::new(&mut params.offset).range(-10000..=10000),
                        )
                        .changed();
                }
                if !matches!(block.kind, Kind::Amplifier | Kind::Pwm | Kind::PiController) {
                    let labels = match block.kind {
                        Kind::Watchdog => ["Tolérance supérieure", "Tolérance inférieure"],
                        Kind::Differential => ["Seuil de désactivation", "Écart d’activation"],
                        Kind::Frequency => {
                            ["Seuil d’activation (Hz)", "Seuil de désactivation (Hz)"]
                        }
                        _ => ["Seuil d’activation", "Seuil de désactivation"],
                    };
                    for (label, value) in labels.into_iter().zip([&mut params.on, &mut params.off])
                    {
                        ui.label(label);
                        changed |= ui
                            .add_enabled(
                                self.editing(),
                                egui::DragValue::new(value).range(-20000..=20000),
                            )
                            .changed();
                    }
                }
                if changed {
                    self.remember();
                    self.project.blocks[index].analog = params;
                }
            }
            if let Some((label, range)) = match block.kind {
                Kind::HoursCounter => Some(("Intervalle d’entretien (h)", 1..=COUNTER_MAX)),
                Kind::EdgePulse => Some(("Nombre d’impulsions", 1..=COUNTER_MAX)),
                Kind::Counter => Some(("Seuil", 0..=COUNTER_MAX)),
                Kind::Stairway | Kind::ComfortSwitch => {
                    Some(("Coupure de préavis (ms)", 0..=10_000))
                }
                Kind::Average => Some(("Nombre d’échantillons", 1..=32_767)),
                Kind::Ramp => Some(("Vitesse (unités/s)", 1..=10_000)),
                Kind::PiController => Some(("Gain proportionnel (× 0,01)", 0..=9_999)),
                Kind::ShiftRegister | Kind::ShiftBit => Some(("Position du bit", 1..=8)),
                _ => None,
            } {
                let mut limit = block.limit;
                ui.label(label);
                if ui
                    .add_enabled(
                        self.editing(),
                        egui::DragValue::new(&mut limit).range(range),
                    )
                    .changed()
                {
                    self.remember();
                    self.project.blocks[index].limit = limit;
                }
                if !self.editing() {
                    match block.kind {
                        Kind::HoursCounter => {
                            ui.label(format!("Total  {:.3} h", signal.elapsed / 3600.0))
                        }
                        Kind::Counter => ui.label(format!("Compteur  {}", signal.count)),
                        Kind::EdgePulse => ui.label(format!("Écoulé  {:.2} s", signal.elapsed)),
                        _ => ui.allocate_response(egui::Vec2::ZERO, egui::Sense::hover()),
                    };
                }
            }
            if matches!(
                block.kind,
                Kind::Mux | Kind::Math | Kind::Ramp | Kind::PiController
            ) {
                let mut values = block.values;
                let mut operation = block.operation;
                ui.add_enabled_ui(self.editing(), |ui| {
                    if block.kind == Kind::Math {
                        egui::ComboBox::from_label("Opérateur")
                            .selected_text(operation_label(operation))
                            .show_ui(ui, |ui| {
                                for choice in [
                                    Operation::Add,
                                    Operation::Subtract,
                                    Operation::Multiply,
                                    Operation::Divide,
                                ] {
                                    ui.selectable_value(
                                        &mut operation,
                                        choice,
                                        operation_label(choice),
                                    );
                                }
                            });
                    }
                    let labels: &[&str] = match block.kind {
                        Kind::Ramp => &["Niveau 1", "Niveau 2", "Niveau départ/arrêt", "Plafond"],
                        Kind::PiController => &["Consigne", "Sortie manuelle"],
                        Kind::Mux => &["V1", "V2", "V3", "V4"],
                        _ => &["V1", "V2"],
                    };
                    for (label, value) in labels.iter().zip(&mut values) {
                        ui.horizontal(|ui| {
                            ui.label(*label);
                            ui.add(egui::DragValue::new(value));
                        });
                    }
                });
                if values != block.values || operation != block.operation {
                    self.remember();
                    self.project.blocks[index].values = values;
                    self.project.blocks[index].operation = operation;
                }
            }
            if let Some(label) = match block.kind {
                Kind::ToggleRelay => Some("Priorité à l’entrée S"),
                Kind::MinMax => Some("Réinitialiser si En = 0"),
                Kind::MathError => Some("Réinitialisation automatique"),
                Kind::PiController => Some("Action inverse"),
                Kind::SoftKey => Some("Mode bouton-poussoir"),
                _ => None,
            } {
                let mut set_priority = block.set_priority;
                if ui
                    .add_enabled(
                        self.editing(),
                        egui::Checkbox::new(&mut set_priority, label),
                    )
                    .changed()
                {
                    self.remember();
                    self.project.blocks[index].set_priority = set_priority;
                }
            }
            let modes: &[&str] = match block.kind {
                Kind::MinMax => &["Minimum", "Maximum", "Sélection par S1", "Valeur Ax"],
                Kind::MathError => &["Les deux erreurs", "Division par zéro", "Dépassement"],
                _ => &[],
            };
            if !modes.is_empty() {
                let mut mode = block.limit.clamp(0, modes.len() as i32 - 1);
                ui.add_enabled_ui(self.editing(), |ui| {
                    egui::ComboBox::from_label("Mode")
                        .selected_text(modes[mode as usize])
                        .show_ui(ui, |ui| {
                            for (i, &label) in modes.iter().enumerate() {
                                ui.selectable_value(&mut mode, i as i32, label);
                            }
                        });
                });
                if mode != block.limit && self.editing() {
                    self.remember();
                    self.project.blocks[index].limit = mode;
                }
            }
            if block.kind == Kind::MessageText {
                let mut message = block.message.clone();
                ui.label("Message affiché pendant la simulation");
                if ui
                    .add_enabled(
                        self.editing(),
                        egui::TextEdit::multiline(&mut message).char_limit(240),
                    )
                    .changed()
                {
                    self.remember();
                    self.project.blocks[index].message = message;
                }
            }
            if block.kind == Kind::Udf {
                if let Some(circuit) = &block.subcircuit {
                    ui.strong(&circuit.name);
                    for (pin, input) in circuit.blocks.iter().filter(|b| b.kind == Kind::Input).enumerate() {
                        ui.label(format!("I{} · {}", pin + 1, input.name));
                    }
                } else {
                    ui.label("Choisissez un schéma local avec jusqu’à quatre entrées numériques et une sortie numérique.");
                }
                if ui.add_enabled(self.editing(), Button::new("Associer un schéma…")).clicked() {
                    self.pick_subcircuit(index);
                }
            }
            if matches!(block.kind, Kind::FloatToInteger | Kind::IntegerToFloat) {
                let mut address = block.limit;
                let mut double = block.set_priority;
                let mut resolution = block.duration;
                let mut fallback = block.values[0];
                ui.add_enabled_ui(self.editing(), |ui| {
                    ui.checkbox(&mut double, "Double précision (8 octets)");
                    let last = if double { 1016 } else { 1020 };
                    ui.horizontal(|ui| {
                        ui.label("Adresse VM");
                        ui.add(egui::DragValue::new(&mut address).range(0..=last));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Résolution");
                        ui.add(
                            egui::DragValue::new(&mut resolution)
                                .range(0.000001..=MAX_DELAY)
                                .max_decimals(6)
                                .speed(0.01),
                        );
                    });
                    if block.kind == Kind::IntegerToFloat {
                        ui.horizontal(|ui| {
                            ui.label("Valeur si Ax est libre");
                            ui.add(egui::DragValue::new(&mut fallback));
                        });
                    }
                    ui.label(
                        RichText::new("Mémoire VM de la simulation locale")
                            .color(MUTED)
                            .small(),
                    );
                });
                if self.editing()
                    && (address, double, resolution, fallback)
                        != (
                            block.limit,
                            block.set_priority,
                            block.duration,
                            block.values[0],
                        )
                {
                    self.remember();
                    let updated = &mut self.project.blocks[index];
                    updated.limit = address.min(if double { 1016 } else { 1020 });
                    updated.set_priority = double;
                    updated.duration = resolution;
                    updated.values[0] = fallback;
                }
            }
            if matches!(
                block.kind,
                Kind::WeeklyTimer | Kind::YearlyTimer | Kind::AstroClock
            ) {
                let mut schedule = block.schedule.clone().unwrap_or_default();
                ui.add_enabled_ui(self.editing(), |ui| {
                    edit_schedule(ui, block.kind, &mut schedule)
                });
                if self.editing() && block.schedule.as_ref() != Some(&schedule) {
                    self.remember();
                    self.project.blocks[index].schedule = Some(schedule);
                }
            }
            if !self.editing() {
                if block.kind == Kind::DataLog
                    && let Some((_, sim)) = &self.simulation
                {
                    ui.strong("Relevés de cette simulation");
                    for (time, _, value) in sim
                        .archive()
                        .iter()
                        .rev()
                        .filter(|(_, id, _)| *id == block.id)
                    {
                        ui.label(format!("{time:.2} s    {value}"));
                    }
                } else if block.kind.analog_value() {
                    ui.label(format!("Valeur  {}", signal.analog));
                } else {
                    ui.label(format!("État  {}", u8::from(signal.high)));
                }
            }
            if !block.kind.pins().is_empty() {
                ui.add_space(5.0);
                ui.separator();
                ui.label(RichText::new("Connexions").strong());
                for (pin, label) in block.kind.pins().iter().enumerate() {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(*label).monospace().color(MUTED));
                        if let Some(wire) = self
                            .project
                            .wires
                            .iter()
                            .find(|w| w.to == id && w.pin == pin)
                            .copied()
                        {
                            let from = self.project.block(wire.from).unwrap();
                            ui.label(RichText::new(&from.name).size(12.0).color(INK));
                            if self.editing()
                                && ui
                                    .small_button("Débrancher")
                                    .clicked()
                            {
                                self.disconnect(wire);
                            }
                        } else {
                            ui.label(
                                RichText::new(format!(
                                    "Libre · {}",
                                    u8::from(block.kind.unconnected())
                                ))
                                .size(12.0)
                                .color(MUTED),
                            );
                        }
                    });
                }
            }
            ui.add_space(8.0);
            if self.editing()
                && ui
                    .button("Supprimer le bloc")
                    .on_hover_text("Suppr")
                    .clicked()
            {
                self.remove_selected();
            }
        });
    }
}

fn heading(ui: &mut Ui, text: &str) {
    ui.label(RichText::new(text).size(18.0).strong().color(INK));
}

fn edit_schedule(ui: &mut Ui, kind: Kind, schedule: &mut crate::schedule::Schedule) {
    use crate::schedule::{Recurrence, month_days};
    if kind == Kind::AstroClock {
        for (label, angle, bound) in [
            ("Latitude (°)", &mut schedule.latitude, 90.0),
            ("Longitude (° est)", &mut schedule.longitude, 180.0),
        ] {
            ui.horizontal(|ui| {
                ui.label(label);
                ui.add(
                    egui::DragValue::new(angle)
                        .range(-bound..=bound)
                        .speed(0.01),
                );
            });
        }
        for (label, minutes) in [
            ("Décalage lever", &mut schedule.sunrise_offset),
            ("Décalage coucher", &mut schedule.sunset_offset),
        ] {
            ui.horizontal(|ui| {
                ui.label(label);
                ui.add(
                    egui::DragValue::new(minutes)
                        .range(-180..=180)
                        .suffix(" min"),
                );
            });
        }
        return;
    }
    ui.horizontal(|ui| {
        ui.label("Décalage fixe UTC");
        ui.add(
            egui::DragValue::new(&mut schedule.utc_offset)
                .range(-720..=840)
                .suffix(" min"),
        );
    });
    ui.checkbox(&mut schedule.pulse, "Impulsion d’un cycle");
    if kind == Kind::WeeklyTimer {
        for (i, cam) in schedule.cams.iter_mut().enumerate() {
            ui.separator();
            ui.label(format!("Créneau {}", i + 1));
            ui.horizontal_wrapped(|ui| {
                for (day, label) in ["Lu", "Ma", "Me", "Je", "Ve", "Sa", "Di"]
                    .into_iter()
                    .enumerate()
                {
                    let bit = 1 << day;
                    if ui.selectable_label(cam.days & bit != 0, label).clicked() {
                        cam.days ^= bit;
                    }
                }
            });
            for (label, minute) in [
                ("Début", &mut cam.start_minute),
                ("Fin", &mut cam.end_minute),
            ] {
                if schedule.pulse && label == "Fin" {
                    continue;
                }
                ui.horizontal(|ui| {
                    let (mut hour, mut minutes) = (*minute / 60, *minute % 60);
                    ui.label(label);
                    ui.add(egui::DragValue::new(&mut hour).range(0..=23).suffix(" h"));
                    ui.add(
                        egui::DragValue::new(&mut minutes)
                            .range(0..=59)
                            .suffix(" min"),
                    );
                    *minute = hour * 60 + minutes;
                });
            }
        }
    } else {
        ui.horizontal_wrapped(|ui| {
            for (label, recurrence) in [
                ("Unique", Recurrence::Once),
                ("Mensuelle", Recurrence::Monthly),
                ("Annuelle", Recurrence::Yearly),
            ] {
                ui.selectable_value(&mut schedule.recurrence, recurrence, label);
            }
        });
        for (label, [year, month, day]) in [
            ("Début", &mut schedule.start_date),
            ("Fin", &mut schedule.end_date),
        ] {
            ui.label(label);
            ui.horizontal_wrapped(|ui| {
                ui.add(
                    egui::DragValue::new(year)
                        .range(2000..=2099)
                        .suffix(" (année)"),
                );
                if schedule.recurrence != Recurrence::Monthly {
                    ui.add(egui::DragValue::new(month).range(1..=12).suffix(" (mois)"));
                }
                let last = if schedule.recurrence == Recurrence::Monthly {
                    31
                } else {
                    month_days(
                        if schedule.recurrence == Recurrence::Yearly {
                            2000
                        } else {
                            *year
                        },
                        *month,
                    )
                };
                ui.add(egui::DragValue::new(day).range(1..=last).suffix(" (jour)"));
            });
        }
    }
}

fn operation_label(operation: Operation) -> &'static str {
    match operation {
        Operation::Add => "Addition",
        Operation::Subtract => "Soustraction",
        Operation::Multiply => "Multiplication",
        Operation::Divide => "Division entière",
    }
}
