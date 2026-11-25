use std::{collections::HashMap, io, path::Path};

use crate::model::{
    BlockDrawing, Connection, ConnectorDrawing, Device, DiagramDrawing, Kind, NetworkDrawing,
    Project, Protocol, Sheet, Wire, WireDrawing,
};

use super::{Block, Diagram, InputPin, Network, invalid, is_project};

pub(crate) fn open(path: &Path) -> io::Result<Project> {
    let network = is_project(path);
    let source = if network {
        Network::open(path)?
    } else {
        Network::open_diagram(path)?
    };
    let mut project = Project::empty();
    project.name = source.name;
    let mut warnings = Vec::new();
    let mut by_address = HashMap::new();
    let mut positions = Vec::new();
    let mut external = Vec::new();
    if network {
        for (index, device) in source.devices.iter().enumerate() {
            let id = index as u32 + 1;
            let ip = device
                .address
                .parse()
                .unwrap_or(std::net::Ipv4Addr::UNSPECIFIED);
            if !ip.is_unspecified() {
                by_address.entry(ip).or_insert(id);
            }
            if device.external {
                external.push(id);
            }
            if let Some(position) = device.position {
                positions.push((id, position));
            }
            project.devices.push(Device {
                id,
                profile: device.profile,
                name: device.name.clone(),
                ip,
            });
        }
        for (index, device) in source.devices.iter().enumerate() {
            let id = index as u32 + 1;
            for link in &device.connections {
                let peer = link
                    .remote
                    .parse()
                    .ok()
                    .and_then(|ip| by_address.get(&ip))
                    .copied();
                let protocol = match link.protocol {
                    "S7" => Some(Protocol::S7),
                    "Modbus" => Some(Protocol::ModbusTcp),
                    _ => None,
                };
                let Some((peer, protocol)) = peer.filter(|peer| *peer != id).zip(protocol) else {
                    warnings.push(format!(
                        "{} : liaison {} vers {} à vérifier",
                        device.name, link.protocol, link.remote
                    ));
                    continue;
                };
                let (client, server) = if link.client { (id, peer) } else { (peer, id) };
                if !project.connections.iter().any(|link| {
                    link.client == client && link.server == server && link.protocol == protocol
                }) {
                    project.connections.push(Connection {
                        id: project.connections.len() as u32 + 1,
                        client,
                        server,
                        protocol,
                    });
                }
            }
        }
        let local = source
            .paths
            .iter()
            .find(|path| path.local)
            .and_then(|path| path.points.last())
            .copied()
            .unwrap_or([155.0, 115.0]);
        project.network_drawing = Some(NetworkDrawing {
            positions,
            external,
            local,
            paths: source
                .paths
                .into_iter()
                .map(|path| (path.local, path.points))
                .collect(),
        });
    }
    for (index, device) in source.devices.into_iter().enumerate() {
        let Some((name, diagram)) = device.diagram else {
            continue;
        };
        let diagram = match diagram {
            Ok(diagram) => diagram,
            Err(error) => {
                warnings.push(format!("{name} : {error}"));
                continue;
            }
        };
        let mut sheet = diagram.into_sheet(name, network.then_some(index as u32 + 1));
        for block in &mut sheet.blocks {
            block.network.device = block.network.remote_ip.and_then(|ip| by_address.get(&ip).copied());
            if block.kind == Kind::MessageText && block.message.is_empty() {
                warnings.push(format!("{} · {} : contenu de l’affichage non importé", sheet.name, block.name));
            }
            if block.network.source == crate::model::NetworkSource::Unsupported {
                warnings.push(format!("{} · {} : paramètre réseau à reconfigurer", sheet.name, block.name));
            }
        }
        if let Some(drawing) = &sheet.drawing {
            warnings.extend(
                drawing
                    .issues
                    .iter()
                    .map(|issue| format!("{} : {issue}", sheet.name)),
            );
        }
        project.sheets.push(sheet);
    }
    if project.sheets.is_empty() && !warnings.is_empty() {
        return Err(invalid(&warnings.join("\n")));
    }
    project.switch_sheet(0);
    project.document_valid().map_err(invalid)?;
    project.import_warnings = warnings;
    Ok(project)
}

impl Diagram {
    fn into_sheet(self, name: String, device: Option<u32>) -> Sheet {
        let mut circuit = Project::empty();
        let mut drawing = DiagramDrawing {
            comments: self.comments,
            ..Default::default()
        };
        let mut inputs = HashMap::new();
        let mut outputs = HashMap::new();
        let mut unsupported = 0;
        for source in &self.blocks {
            let mapped = source.editable_kind();
            let kind = mapped.unwrap_or(Kind::OpenPin);
            let id = circuit.add(kind, source.position[0], source.position[1]);
            let block = circuit.blocks.last_mut().unwrap();
            block.io_number = source.io_number;
            if let Some(port) = source.network {
                block.network = port;
            }
            block.name = if source.description.is_empty() {
                source.number.clone()
            } else {
                format!("{} ({})", source.number, source.description)
            };
            if let Some(Some(seconds)) = source.times.first() {
                block.duration = *seconds;
            }
            if let Some(Some(seconds)) = source.times.get(1) {
                block.duration_low = *seconds;
            }
            if let Some(Some(seconds)) = source.times.get(2) {
                block.pulse_delay = *seconds;
            }
            if let Some(limit) = source.limit {
                block.limit = limit;
            }
            block.set_priority = source.set_priority;
            if let Some(schedule) = &source.schedule {
                block.schedule = Some(schedule.clone());
            }
            if let Some(message) = &source.message {
                block.message.clone_from(message);
            }
            let local =
                |point: [f32; 2]| [point[0] - source.position[0], point[1] - source.position[1]];
            let mut appearance = BlockDrawing {
                size: source.size,
                symbol: source.symbol.clone(),
                inputs: vec![None; kind.pins().len()],
                extra_inputs: Vec::new(),
                output: source.outputs.first().map(|ends| ends.map(local)),
                unsupported: mapped.is_none() || source.has_unconverted_parameters,
                parameter_text: source.parameter_text.clone(),
            };
            for input in &source.inputs {
                if let Some(pin) = mapped.and_then(|kind| pin_index(kind, input))
                    && appearance.inputs[pin].is_none()
                {
                    appearance.inputs[pin] = Some(input.ends.map(local));
                    inputs.insert(input.figure, (id, pin));
                    if input.inverted && kind.can_invert_pin(pin) {
                        block.inverted_pins |= 1 << pin;
                    } else if input.inverted {
                        appearance.unsupported = true;
                    }
                } else {
                    appearance
                        .extra_inputs
                        .push((input.name.clone(), input.ends.map(local)));
                }
            }
            if appearance.unsupported {
                unsupported += 1;
            }
            if let Some(&figure) = source.output_figures.first() {
                outputs.insert(figure, id);
            }
            block.drawing = Some(appearance);
        }

        let mut occupied = std::collections::HashSet::new();
        let anchor = |id| {
            let block = &circuit.blocks[id as usize - 1];
            [block.x, block.y]
        };
        let mut connectors: Vec<_> = self
            .connectors
            .into_iter()
            .map(|(path, label)| Some(ConnectorDrawing { path, label }))
            .collect();
        let cut_indices: HashMap<_, _> = self
            .cut_pairs
            .iter()
            .enumerate()
            .map(|(index, (figure, _, _))| (*figure, index))
            .collect();
        let mut to_marker = HashMap::new();
        let mut from_marker = HashMap::new();
        let mut loose = Vec::new();
        for (route, pins) in self.wires.into_iter().zip(self.wire_pins) {
            if let Some((start, end)) = pins {
                if let (Some(&from), Some(&(to, pin))) = (outputs.get(&start), inputs.get(&end))
                    && occupied.insert((to, pin))
                {
                    let wire = Wire::new(from, to, pin);
                    circuit.wires.push(wire);
                    drawing.routes.push(WireDrawing {
                        wire,
                        route,
                        anchors: [anchor(from), anchor(to)],
                        connectors: None,
                        cut_stubs: None,
                    });
                    continue;
                }
                if let Some(&from) = outputs.get(&start)
                    && cut_indices.contains_key(&end)
                {
                    to_marker.entry(end).or_insert((from, loose.len()));
                }
                if let Some(&(to, pin)) = inputs.get(&end)
                    && cut_indices.contains_key(&start)
                {
                    from_marker.entry(start).or_insert((to, pin, loose.len()));
                }
            }
            loose.push(Some(route));
        }

        // A cut link stores two independent paths and a pair of linked figures.
        for (index, &(figure, partner, enters_marker)) in self.cut_pairs.iter().enumerate() {
            if !enters_marker {
                continue;
            }
            let Some(partner) = partner else { continue };
            let (Some(&other), Some(&(from, start)), Some(&(to, pin, end))) = (
                cut_indices.get(&partner),
                to_marker.get(&figure),
                from_marker.get(&partner),
            ) else {
                continue;
            };
            if self.cut_pairs[other].1 != Some(figure)
                || self.cut_pairs[other].2
                || !occupied.insert((to, pin))
            {
                continue;
            }
            let wire = Wire {
                cut: true,
                ..Wire::new(from, to, pin)
            };
            circuit.wires.push(wire);
            drawing.routes.push(WireDrawing {
                wire,
                route: Vec::new(),
                anchors: [anchor(from), anchor(to)],
                connectors: Some([
                    connectors[index].take().unwrap(),
                    connectors[other].take().unwrap(),
                ]),
                cut_stubs: Some([loose[start].take().unwrap(), loose[end].take().unwrap()]),
            });
        }
        drawing.loose_wires = loose.into_iter().flatten().collect();
        drawing.loose_connectors = connectors.into_iter().flatten().collect();
        if !drawing.loose_wires.is_empty() || !drawing.loose_connectors.is_empty() {
            drawing.issues.push(format!(
                "{} fils et {} renvois à vérifier",
                drawing.loose_wires.len(),
                drawing.loose_connectors.len()
            ));
        }
        if self.omitted > 0 {
            drawing
                .issues
                .push(format!("{} éléments graphiques non importés", self.omitted));
        }
        if unsupported > 0 {
            drawing.issues.push(format!(
                "{unsupported} blocs avec des données d’origine à vérifier"
            ));
        }
        Sheet {
            name,
            device,
            blocks: circuit.blocks,
            wires: circuit.wires,
            drawing: Some(drawing),
        }
    }
}

fn pin_index(kind: Kind, pin: &InputPin) -> Option<usize> {
    let pins = kind.pins();
    pins.iter()
        .position(|name| name.eq_ignore_ascii_case(&pin.name))
        .or_else(|| {
            (pins == ["1"] && matches!(pin.name.as_str(), "In" | "I" | "Trg" | "S")).then_some(0)
        })
        .or_else(|| (kind == Kind::MessageText && pin.name == "En").then_some(0))
}

impl Block {
    fn editable_kind(&self) -> Option<Kind> {
        Some(match self.kind.as_str() {
            "InputBlock" => Kind::Input,
            "OutputBlock" => Kind::Output,
            "AnalogInputBlock" => Kind::AnalogInput,
            "AnalogOutputBlock" => Kind::AnalogOutput,
            "MarkerBlock" => Kind::Marker,
            "AnalogMarkerBlock" => Kind::AnalogMarker,
            "HighBlock" => Kind::ConstantHigh,
            "LowBlock" => Kind::ConstantLow,
            "VirtualOutputBlock" | "OpenConnectorBlock" => Kind::OpenPin,
            "AnalogNetworkNodeBlock" => Kind::NetAnalogInput,
            "NetworkNodeBlock" | "SInputBlock" => Kind::NetInput,
            "SOutputBlock" => Kind::NetOutput,
            "SAnalogInputBlock" => Kind::NetAnalogInput,
            "SAnalogOutputBlock" => Kind::NetAnalogOutput,
            "AndBlock" => Kind::And,
            "NandBlock" => Kind::Nand,
            "OrBlock" => Kind::Or,
            "NorBlock" => Kind::Nor,
            "XorBlock" => Kind::Xor,
            "NotBlock" => Kind::Not,
            "OnDelayBlock" => Kind::OnDelay,
            "OffDelayBlock" => Kind::OffDelay,
            "OnOffDelayBlock" => Kind::OnOffDelay,
            "RetentiveOnDelayBlock" => Kind::LatchedDelay,
            "WipingRelayBlock" => Kind::PulseRelay,
            "AsymPulseGeneratorBlock" => Kind::PulseGenerator,
            "RandomBlock" => Kind::RandomDelay,
            "StairsSwitchBlock" => Kind::Stairway,
            "PecAndBlock" => Kind::AndRising,
            "PecNandBlock" => Kind::NandFalling,
            "KeyInputBlock" => Kind::CursorKey,
            "TDFunctionKeyInputBlock" => Kind::FunctionKey,
            "ShiftRegisterInputBlock" => Kind::ShiftBit,
            "CounterBlock" => Kind::Counter,
            "HoursCounterBlock" => Kind::HoursCounter,
            "TriggerBlock" => Kind::Frequency,
            "AmplifierBlock" => Kind::Amplifier,
            "AnalogTriggerBlock" => Kind::Threshold,
            "AnalogComparatorBlock" => Kind::Comparator,
            "AnalogDeltaTriggerBlock" => Kind::Differential,
            "AnalogWatchdogBlock" => Kind::Watchdog,
            "AnalogMuxBlock" => Kind::Mux,
            "AnalogMathsBlock" => Kind::Math,
            "PWMBlock" => Kind::Pwm,
            "AnalogFilterBlock" => Kind::AnalogFilter,
            "RampControlBlock" => Kind::Ramp,
            "MaxMinBlock" => Kind::MinMax,
            "AverageValueBlock" => Kind::Average,
            "ShiftRegisterBlock" => Kind::ShiftRegister,
            "MathDetectionBlock" => Kind::MathError,
            "DataLogBlock" => Kind::DataLog,
            "YearClockBlock" => Kind::YearlyTimer,
            "AstronomicalClockBlock" => Kind::AstroClock,
            "StopWatchBlock" => Kind::Stopwatch,
            "PIDBlock" => Kind::PiController,
            "Float2IntegerBlock" => Kind::FloatToInteger,
            "Integer2FloatBlock" => Kind::IntegerToFloat,
            "SoftkeyBlock" => Kind::SoftKey,
            "PulseRelayBlock" => Kind::ToggleRelay,
            "LatchingRelayBlock" => Kind::Relay,
            "ComfortSwitchBlock" => Kind::ComfortSwitch,
            "WipingRelayPECBlock" => Kind::EdgePulse,
            "ClockBlock" => Kind::WeeklyTimer,
            "MessageBlock" => Kind::MessageText,
            _ => return None,
        })
    }
}
