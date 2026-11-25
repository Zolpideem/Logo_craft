mod editable;
pub(crate) use editable::open as open_editable;
mod java;
mod zip;

use std::{
    collections::{HashMap, HashSet},
    fs::File,
    io::{self, Read},
    path::Path,
};

use xml::reader::{ParserConfig, XmlEvent};

pub(crate) struct Network {
    pub name: String,
    pub devices: Vec<Device>,
    pub paths: Vec<NetworkPath>,
}

pub(crate) struct Device {
    pub name: String,
    pub profile: Option<u16>,
    pub address: String,
    pub position: Option<[f32; 2]>,
    pub external: bool,
    pub diagram: Option<(String, Result<Diagram, String>)>,
    pub connections: Vec<Connection>,
}

pub(crate) struct Connection {
    pub remote: String,
    pub protocol: &'static str,
    pub client: bool,
}

pub(crate) struct NetworkPath {
    pub points: Vec<[f32; 2]>,
    pub local: bool,
}

#[derive(Default)]
pub(crate) struct Diagram {
    pub blocks: Vec<Block>,
    pub wires: Vec<Vec<[f32; 2]>>,
    wire_pins: Vec<Option<(usize, usize)>>,
    pub connectors: Vec<(Vec<[f32; 2]>, String)>,
    cut_pairs: Vec<(usize, Option<usize>, bool)>,
    pub comments: Vec<([f32; 2], String)>,
    pub omitted: usize,
}

pub(crate) struct Block {
    kind: String,
    parameter_text: String,
    times: Vec<Option<f64>>,
    limit: Option<i32>,
    set_priority: bool,
    schedule: Option<crate::schedule::Schedule>,
    message: Option<String>,
    has_unconverted_parameters: bool,
    io_number: Option<u16>,
    network: Option<crate::model::NetworkPort>,
    pub position: [f32; 2],
    pub size: [f32; 2],
    pub number: String,
    pub symbol: String,
    pub description: String,
    pub inputs: Vec<InputPin>,
    pub outputs: Vec<[[f32; 2]; 2]>,
    output_figures: Vec<usize>,
}

pub(crate) struct InputPin {
    figure: usize,
    name: String,
    pub ends: [[f32; 2]; 2],
    pub inverted: bool,
}

pub(crate) const EXTENSION: &str = "mnp";
pub(crate) const DIAGRAM_EXTENSION: &str = "lsc";

pub(crate) fn is_project(path: &Path) -> bool {
    has_extension(path, EXTENSION)
}

pub(crate) fn is_diagram(path: &Path) -> bool {
    has_extension(path, DIAGRAM_EXTENSION)
}

fn has_extension(path: &Path, extension: &str) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .is_some_and(|value| value.eq_ignore_ascii_case(extension))
}

impl Network {
    pub(crate) fn open_diagram(path: &Path) -> io::Result<Self> {
        const LIMIT: usize = 8 * 1024 * 1024;
        let mut bytes = Vec::new();
        File::open(path)?
            .take((LIMIT + 1) as u64)
            .read_to_end(&mut bytes)?;
        if bytes.len() > LIMIT {
            return Err(invalid("Le fichier de schéma est trop volumineux."));
        }
        let diagram = Diagram::read(&bytes)?;
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        Ok(Self {
            name: name.clone(),
            devices: vec![Device {
                name: name.clone(),
                profile: None,
                address: String::new(),
                position: None,
                external: false,
                diagram: Some((name, Ok(diagram))),
                connections: Vec::new(),
            }],
            paths: Vec::new(),
        })
    }

    pub(crate) fn open(path: &Path) -> io::Result<Self> {
        let archive = zip::Archive::open(path)?;
        let manifests = archive
            .names()
            .filter(|name| !name.contains('/') && name.ends_with(".xml"))
            .collect::<Vec<_>>();
        let manifest = match manifests.as_slice() {
            [manifest] => *manifest,
            _ => return Err(invalid("Le manifeste XML du projet est absent ou ambigu.")),
        };
        let root = xml(&archive.read(manifest, 2 * 1024 * 1024)?)?;
        let name = root
            .child("properties")
            .map_or("", |node| node.text("name"));
        let mut network = Network {
            name: if name.is_empty() {
                path.file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned()
            } else {
                name.into()
            },
            devices: Vec::new(),
            paths: Vec::new(),
        };
        let empty = Element::default();
        let figures = root.child("Figures").unwrap_or(&empty);
        let devices = root.child("Devices").unwrap_or(&empty);
        let entries = devices.children.iter().filter(|node| node.name == "Device");
        for record in entries {
            if network.devices.len() >= 64 {
                return Err(invalid("Ce projet contient trop d’appareils."));
            }
            let id = record.attr("id");
            let info = record.child("NetworkInfo").unwrap_or(&empty);
            let address = info.text("IP");
            let name = record.attr("name");
            let filename = record.attr("diagram");
            let position = figures
                .children
                .iter()
                .find(|figure| figure.attr("deviceId") == id)
                .and_then(|figure| figure.attr("position").split_once(','))
                .and_then(|(x, y)| Some([coordinate(x)?, coordinate(y)?]));
            let diagram = (!filename.is_empty()).then(|| {
                let member = format!("{id}/{filename}");
                let result = archive
                    .read(&member, 8 * 1024 * 1024)
                    .and_then(|data| Diagram::read(&data))
                    .map_err(|error| error.to_string());
                (filename.to_owned(), result)
            });
            let connections = info
                .child("EthernetConnections")
                .unwrap_or(&empty)
                .children
                .iter()
                .filter(|node| node.name == "EthernetConnection")
                .map(|node| Connection {
                    remote: node.text("remoteIP").into(),
                    protocol: match node.text("type") {
                        "0" => "S7",
                        "1" => "Modbus",
                        _ => "Ethernet",
                    },
                    client: node.text("isClient") == "true",
                })
                .collect();
            network.devices.push(Device {
                profile: record.attr("type").parse().ok(),
                name: if name.is_empty() {
                    "Appareil externe".into()
                } else {
                    name.into()
                },
                address: address.into(),
                position,
                external: record.attr("type") == "1000",
                diagram,
                connections,
            });
        }
        for figure in &figures.children {
            let kind = figure.attr("type");
            if !matches!(kind, "51" | "52") {
                continue;
            }
            let points = figure
                .child("Points")
                .unwrap_or(&empty)
                .children
                .iter()
                .filter_map(|point| {
                    Some([coordinate(point.attr("x"))?, coordinate(point.attr("y"))?])
                })
                .collect::<Vec<_>>();
            if points.len() >= 2 {
                network.paths.push(NetworkPath {
                    points,
                    local: kind == "52",
                });
            }
        }
        Ok(network)
    }
}

fn coordinate(value: &str) -> Option<f32> {
    let value = value.trim().parse::<i32>().ok()?;
    (-100_000..=100_000)
        .contains(&value)
        .then_some(value as f32)
}

impl Diagram {
    fn read(data: &[u8]) -> io::Result<Self> {
        let (stream, roots) = java::Stream::read(data)?;
        let mut parameters = HashMap::new();
        for id in 0..stream.values.len() {
            if let Some((class, _)) = stream.object(id)
                && class.ends_with("Parameter")
                && let Some(owner) = stream.reference(id, "fBlock")
            {
                parameters.entry(owner).or_insert_with(Vec::new).push(id);
            }
        }
        let pin_at = |figure, end| {
            stream
                .reference(figure, end)
                .and_then(|connector| stream.reference(connector, "fOwner"))
        };
        let mut diagram = Self::default();
        let point = |id| {
            Some([
                stream.integer(id, "x")? as f32,
                stream.integer(id, "y")? as f32,
            ])
        };
        let points = |array| match stream.values.get(array) {
            Some(java::Value::Array(items)) => items.iter().filter_map(|&id| point(id)).collect(),
            _ => Vec::new(),
        };
        let vector = |owner, field| {
            let id = stream.reference(owner, field)?;
            let data = stream.reference(id, "elementData")?;
            let java::Value::Array(items) = stream.values.get(data)? else {
                return None;
            };
            Some(items.as_slice())
        };
        let attached_comments: HashSet<_> = roots
            .iter()
            .copied()
            .filter(|&figure| {
                stream
                    .object(figure)
                    .is_some_and(|(class, _)| class == "BlockFigure")
            })
            .filter_map(|figure| stream.reference(figure, "fCommentFigure"))
            .collect();
        for figure in roots {
            let Some((class, _)) = stream.object(figure) else {
                continue;
            };
            match class {
                "BlockFigure" => {
                    let Some((block, position)) = stream.reference(figure, "fBlock").zip(
                        stream
                            .reference(figure, "fDisplayBox100")
                            .or_else(|| stream.reference(figure, "fDisplayBox"))
                            .and_then(point),
                    ) else {
                        diagram.omitted += 1;
                        continue;
                    };
                    let Some((kind, _)) = stream.object(block) else {
                        diagram.omitted += 1;
                        continue;
                    };
                    let prefix = stream.string(block, "fBlockNumberPrefix").unwrap_or("");
                    let number = stream.integer(block, "fBlockNumber").unwrap_or_default();
                    let io_number = u16::try_from(number).ok().filter(|number| *number > 0);
                    let number = if prefix.eq_ignore_ascii_case("B") {
                        format!("{prefix}{number:03}")
                    } else if number > 0 {
                        format!("{prefix}{number}")
                    } else {
                        prefix.into()
                    };
                    let description = stream
                        .string(block, "fUserDefinedName")
                        .unwrap_or("")
                        .trim()
                        .to_owned();
                    let mut inputs = Vec::new();
                    let mut outputs = Vec::new();
                    let mut output_figures = Vec::new();
                    for &pin in vector(figure, "fFigures").unwrap_or_default() {
                        let Some((class, _)) = stream.object(pin) else {
                            continue;
                        };
                        if !matches!(class, "InputIfcFigure" | "OutputIfcFigure") {
                            continue;
                        }
                        let Some([first, second, ..]) = vector(pin, "fPoints") else {
                            continue;
                        };
                        let Some((first, second)) = point(*first).zip(point(*second)) else {
                            continue;
                        };
                        let ends = [first, second];
                        let connector = stream.reference(pin, "fBlockConnector");
                        if class == "InputIfcFigure" {
                            let inverted =
                                connector.and_then(|id| stream.integer(id, "fNegated")) == Some(1);
                            inputs.push(InputPin {
                                figure: pin,
                                name: connector
                                    .and_then(|id| stream.string(id, "fConnectorName"))
                                    .unwrap_or("")
                                    .into(),
                                ends,
                                inverted,
                            });
                        } else {
                            outputs.push(ends);
                            output_figures.push(pin);
                        }
                    }
                    let parameter = parameter(
                        &stream,
                        block,
                        kind,
                        parameters
                            .get(&block)
                            .map(Vec::as_slice)
                            .unwrap_or_default(),
                    );
                    let times = parameter.map_or_else(Vec::new, |id| {
                        let keys: &[_] = match kind {
                            "ComfortSwitchBlock" => &["fHTimer", "fLTimer", "fTimerToWarning"],
                            "StairsSwitchBlock" => &["fTimer", "", "fTimerToWarning"],
                            "OnDelayBlock"
                            | "OffDelayBlock"
                            | "RetentiveOnDelayBlock"
                            | "WipingRelayBlock" => &["fTimer"],
                            "OnOffDelayBlock" | "RandomBlock" => &["fHTimer", "fLTimer"],
                            "WipingRelayPECBlock" => &["fTimer", "fBreakTimer"],
                            "AsymPulseGeneratorBlock" => &["fImpulseLength", "fPauseLength"],
                            _ => &[],
                        };
                        keys.iter().map(|key| read_time(&stream, id, key)).collect()
                    });
                    let is_network = matches!(
                        kind,
                        "NetworkNodeBlock" | "AnalogNetworkNodeBlock" | "SInputBlock"
                            | "SAnalogInputBlock" | "SOutputBlock" | "SAnalogOutputBlock"
                    );
                    let network = is_network.then(|| {
                        parameter.map(|id| read_network_port(&stream, id))
                            .unwrap_or_default()
                    });
                    let limit = parameter.and_then(|id| match kind {
                        "WipingRelayPECBlock" => stream.integer(id, "fCycles").filter(|n| *n > 0),
                        "ComfortSwitchBlock" | "StairsSwitchBlock" => {
                            let ms = read_time(&stream, id, "fWarningTimer")? * 1000.0;
                            (0.0..=10_000.0).contains(&ms).then_some(ms as i32)
                        }
                        _ => None,
                    });
                    let set_priority = kind == "PulseRelayBlock"
                        && parameter.and_then(|id| stream.string(id, "fPreference")) == Some("SR");
                    let schedule = if kind == "ClockBlock" {
                        parameter.and_then(|id| weekly_schedule(&stream, id))
                    } else {
                        None
                    };
                    let message = if kind == "MessageBlock" {
                        parameter.and_then(|id| plain_message(&stream, id))
                    } else {
                        None
                    };
                    let parameter_text = stream
                        .reference(figure, "fCommentFigure")
                        .and_then(|comment| stream.string(comment, "fWholeText"))
                        .unwrap_or("")
                        .to_owned();
                    let has_unconverted_parameters = match kind {
                        "OnDelayBlock"
                        | "OffDelayBlock"
                        | "RetentiveOnDelayBlock"
                        | "OnOffDelayBlock"
                        | "WipingRelayBlock"
                        | "AsymPulseGeneratorBlock"
                        | "RandomBlock" => {
                            !times.first().is_some_and(Option::is_some)
                                || times.iter().any(Option::is_none)
                        }
                        "ClockBlock" => schedule.is_none(),
                        "ComfortSwitchBlock" | "WipingRelayPECBlock" | "StairsSwitchBlock" => {
                            !times.first().is_some_and(Option::is_some)
                        }
                        _ if is_network => false,
                        "InputBlock" | "OutputBlock" | "MarkerBlock" | "MessageBlock" => false,
                        _ => parameter.is_some() && kind != "PulseRelayBlock",
                    } || parameter
                        .and_then(|p| stream.reference(p, "fRemanence"))
                        .is_some_and(|r| stream.integer(r, "fRemanence") == Some(1));
                    diagram.blocks.push(Block {
                        kind: kind.into(),
                        parameter_text,
                        times,
                        limit,
                        set_priority,
                        schedule,
                        message,
                        has_unconverted_parameters,
                        io_number,
                        network,
                        position,
                        size: [
                            stream
                                .integer(figure, "fImageWidth100")
                                .filter(|size| *size > 0)
                                .unwrap_or(24)
                                .clamp(12, 200) as f32,
                            stream
                                .integer(figure, "fImageHeight100")
                                .filter(|size| *size > 0)
                                .unwrap_or(if prefix.eq_ignore_ascii_case("B") {
                                    38
                                } else {
                                    34
                                })
                                .clamp(12, 200) as f32,
                        ],
                        number,
                        symbol: symbol(kind, prefix),
                        description,
                        inputs,
                        outputs,
                        output_figures,
                    });
                }
                "FixedElbowConnection" | "LineConnection" => {
                    if let Some(array) = stream
                        .reference(figure, "fPoints")
                        .and_then(|vector| stream.reference(vector, "elementData"))
                    {
                        let path = points(array);
                        if path.len() >= 2 {
                            diagram.wires.push(path);
                            diagram
                                .wire_pins
                                .push(pin_at(figure, "fStart").zip(pin_at(figure, "fEnd")));
                            continue;
                        }
                    }
                    diagram.omitted += 1;
                }
                "InputCutConnectFigure" | "OutputCutConnectFigure" => {
                    let path = stream
                        .reference(figure, "fPoly")
                        .and_then(|polygon| {
                            let coordinates = |field| match stream
                                .values
                                .get(stream.reference(polygon, field)?)?
                            {
                                java::Value::Integers(values) => Some(values),
                                _ => None,
                            };
                            let count =
                                usize::try_from(stream.integer(polygon, "npoints")?).ok()?;
                            Some(
                                coordinates("xpoints")?
                                    .iter()
                                    .zip(coordinates("ypoints")?)
                                    .take(count)
                                    .map(|(&x, &y)| [x as f32, y as f32])
                                    .collect::<Vec<_>>(),
                            )
                        })
                        .unwrap_or_default();
                    if path.len() >= 3 {
                        let label = stream
                            .reference(figure, "fCCTextFigure")
                            .and_then(|text| stream.string(text, "fWholeText"))
                            .unwrap_or("");
                        diagram.connectors.push((path, label.trim().to_owned()));
                        diagram.cut_pairs.push((
                            figure,
                            stream.reference(figure, "fPartnerFigure"),
                            class == "InputCutConnectFigure",
                        ));
                    } else {
                        diagram.omitted += 1;
                    }
                }
                "CommentTextFigure" | "TextFigure" => {
                    if attached_comments.contains(&figure)
                        || stream.integer(figure, "fIsVisible") == Some(0)
                    {
                        continue;
                    }
                    let origin = |current, legacy| {
                        stream
                            .integer(figure, current)
                            .or_else(|| stream.integer(figure, legacy))
                            .unwrap_or_default() as f32
                    };
                    let x = origin("fOriginX100", "fOriginX");
                    let y = origin("fOriginY100", "fOriginY");
                    if let Some(text) = stream
                        .string(figure, "fWholeText")
                        .map(str::trim)
                        .filter(|text| !text.is_empty())
                    {
                        diagram.comments.push(([x, y], text.into()));
                    }
                }
                _ => diagram.omitted += 1,
            }
        }
        Ok(diagram)
    }
}

fn parameter(
    stream: &java::Stream<'_>,
    block: usize,
    kind: &str,
    owned: &[usize],
) -> Option<usize> {
    let class = format!("{}Parameter", kind.trim_end_matches("Block"));
    let (_, fields) = stream.object(block)?;
    stream
        .reference(block, "fParameter")
        .or_else(|| stream.reference(block, "fClockParameter"))
        .or_else(|| {
            fields
                .keys()
                .filter_map(|key| {
                    let reference = key.contains('#').then(|| stream.reference(block, key))??;
                    (stream.object(reference)?.0 == class).then_some(reference)
                })
                .min()
        })
        .or_else(|| {
            owned.iter().copied().find(|&id| {
                stream.object(id).is_some_and(|(name, _)| {
                    name == class || kind == "MessageBlock" && name == "AbstractMessageParameter"
                })
            })
        })
}

fn read_network_port(stream: &java::Stream<'_>, id: usize) -> crate::model::NetworkPort {
    use crate::model::{NetworkPort, NetworkSource, RemoteBlock};

    let selected = |key| stream.integer(id, key) == Some(1);
    let source = if selected("fVMSelected") {
        NetworkSource::LocalVm
    } else if selected("fRemoteDeviceSelected") || selected("fSlaveSelected") {
        match stream.integer(id, "fNumberType") {
            Some(-3) => NetworkSource::RemoteVm,
            Some(10) => NetworkSource::RemoteBlock(RemoteBlock::Marker),
            _ => NetworkSource::Unsupported,
        }
    } else if selected("fDiagnosticSelected") {
        NetworkSource::Diagnostic
    } else {
        NetworkSource::Unsupported
    };
    let field = match source {
        NetworkSource::RemoteBlock(_) => "fBlockNumber",
        NetworkSource::Diagnostic => "fDiagnosticType",
        _ => "fVMAddress",
    };
    NetworkPort {
        source,
        address: stream.integer(id, field).and_then(|value| u16::try_from(value).ok())
            .unwrap_or_default(),
        remote_ip: stream.string(id, "fIPAddress")
            .or_else(|| stream.string(id, "fSlaveAddress"))
            .and_then(|value| value.parse().ok()),
        bit: stream.integer(id, "fBitNumber").filter(|bit| (0..=7).contains(bit))
            .unwrap_or(0) as u8,
        ..Default::default()
    }
}

fn read_time(stream: &java::Stream<'_>, parameter: usize, key: &str) -> Option<f64> {
    let timer = stream.reference(parameter, key)?;
    let id = stream.reference(timer, "fParamTime").unwrap_or(timer);
    if let Some(milliseconds) = stream.long(id, "fTime") {
        let seconds = milliseconds as f64 / 1000.0;
        return (0.0..=crate::model::MAX_DELAY)
            .contains(&seconds)
            .then_some(seconds);
    }
    let value = f64::from(stream.integer(id, "fTime")?);
    let seconds = match stream.integer(id, "fTimeBase")? {
        0 => value / 100.0,
        1 => value,
        2 => value * 60.0,
        _ => return None,
    };
    (0.0..=crate::model::MAX_DELAY)
        .contains(&seconds)
        .then_some(seconds)
}

fn weekly_schedule(
    stream: &java::Stream<'_>,
    parameter: usize,
) -> Option<crate::schedule::Schedule> {
    let array = |id, field| match stream.values.get(stream.reference(id, field)?)? {
        java::Value::Array(items) => Some(items),
        _ => None,
    };
    let cams = array(parameter, "fCam")?;
    if cams.len() != 3 {
        return None;
    }
    let mut schedule = crate::schedule::Schedule {
        pulse: stream.integer(parameter, "fPulse") == Some(1),
        ..Default::default()
    };
    for (&cam, target) in cams.iter().zip(&mut schedule.cams) {
        let on = stream.reference(cam, "fOnTime")?;
        let off = stream.reference(cam, "fOffTime")?;
        let inactive = |time| stream.integer(time, "fInactive") == Some(1);
        if inactive(on) && inactive(off) {
            continue;
        }
        if !schedule.pulse && inactive(on) != inactive(off) {
            return None;
        }
        let mode = usize::try_from(stream.integer(cam, "fCurrentMode")?).ok()?;
        let &mode = array(cam, "fAvailableModes")?.get(mode)?;
        let java::Value::Integers(days) = stream.values.get(stream.reference(mode, "fDays")?)?
        else {
            return None;
        };
        for &day in days {
            if !(1..=7).contains(&day) {
                return None;
            }
            // Calendar enumerates Sunday first. The editor starts its week on Monday.
            target.days |= 1 << ((day + 5) % 7);
        }
        let minute = |time| {
            let value = stream.long(time, "fTime")? / 60_000;
            (0..1440).contains(&value).then_some(value as u16)
        };
        target.start_minute = minute(on)?;
        target.end_minute = if inactive(off) { 0 } else { minute(off)? };
    }
    Some(schedule)
}

fn plain_message(stream: &java::Stream<'_>, parameter: usize) -> Option<String> {
    let java::Value::Array(lines) = stream
        .values
        .get(stream.reference(parameter, "fMessage")?)?
    else {
        return None;
    };
    let text = lines
        .iter()
        .filter_map(|&line| stream.string(line, "fText"))
        .collect::<Vec<_>>()
        .join("\n");
    (!text.trim().is_empty()).then_some(text)
}

fn symbol(kind: &str, prefix: &str) -> String {
    match kind {
        "AndBlock" => "&",
        "NandBlock" => "!&",
        "OrBlock" => "≥1",
        "NorBlock" => "!≥1",
        "XorBlock" => "=1",
        "NotBlock" => "¬",
        "InputBlock" => "I",
        "OutputBlock" | "SOutputBlock" | "VirtualOutputBlock" => "Q",
        "MarkerBlock" => "M",
        "HighBlock" => "1",
        "LowBlock" => "0",
        "OnDelayBlock" => "TON",
        "OffDelayBlock" => "TOF",
        "PulseRelayBlock" => "RS",
        "LatchingRelayBlock" => "SR",
        "CounterBlock" => "±1",
        "WipingRelayPECBlock" => "TP",
        "ComfortSwitchBlock" => "CONF",
        "ClockBlock" => "H",
        "MessageBlock" => "TXT",
        "NetworkNodeBlock" if !prefix.is_empty() => prefix,
        _ if !prefix.is_empty() && !prefix.eq_ignore_ascii_case("B") => prefix,
        _ => return kind.trim_end_matches("Block").into(),
    }
    .into()
}

#[derive(Default)]
struct Element {
    name: String,
    attributes: HashMap<String, String>,
    children: Vec<Element>,
    text: String,
}

impl Element {
    fn child(&self, name: &str) -> Option<&Self> {
        self.children.iter().find(|node| node.name == name)
    }

    fn text(&self, name: &str) -> &str {
        self.child(name).map_or("", |node| node.text.trim())
    }

    fn attr(&self, name: &str) -> &str {
        self.attributes.get(name).map_or("", String::as_str)
    }
}

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

fn xml(bytes: &[u8]) -> io::Result<Element> {
    let mut stack = vec![Element::default()];
    for event in ParserConfig::new().create_reader(bytes) {
        match event.map_err(io::Error::other)? {
            XmlEvent::StartElement {
                name, attributes, ..
            } => {
                if stack.len() >= 64 {
                    return Err(invalid("Le XML du projet est trop imbriqué."));
                }
                stack.push(Element {
                    name: name.local_name,
                    attributes: attributes
                        .into_iter()
                        .map(|attribute| (attribute.name.local_name, attribute.value))
                        .collect(),
                    ..Default::default()
                });
            }
            XmlEvent::Characters(text) | XmlEvent::CData(text) => {
                stack.last_mut().unwrap().text.push_str(&text)
            }
            XmlEvent::EndElement { .. } => {
                let node = stack.pop().unwrap();
                stack.last_mut().unwrap().children.push(node);
            }
            _ => {}
        }
    }
    stack
        .pop()
        .and_then(|root| root.children.into_iter().next())
        .filter(|root| root.name == "Project")
        .ok_or_else(|| invalid("Structure de projet inconnue."))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_mnp_and_rejects_invalid_project_data() {
        assert!(is_project(Path::new("Projet.MNP")));
        assert!(is_diagram(Path::new("Schéma.LSC")));
        assert!(!is_diagram(Path::new("Projet.MNP")));
        assert!(!is_project(Path::new("Projet.logique")));
        assert!(xml(b"<Unrelated />").is_err());
        assert!(Diagram::read(b"not a Siemens diagram").is_err());
    }

    #[test]
    #[ignore = "nécessite le projet Siemens de référence via ATELIER_MNP_TEST"]
    fn opens_reference_project() {
        let path = std::env::var_os("ATELIER_MNP_TEST").expect("ATELIER_MNP_TEST");
        let project = Network::open(Path::new(&path)).unwrap();
        use crate::model::{Kind, NetworkSource, ProjectSimulation, RemoteBlock, Wire};
        let mut editable = open_editable(Path::new(&path)).unwrap();
        let receiver = editable.blocks.iter().position(|block| block.name == "N1").unwrap();
        let port = editable.blocks[receiver].network;
        assert_eq!(port.source, NetworkSource::RemoteBlock(RemoteBlock::Marker));
        assert_eq!(port.address, 4);
        assert_eq!(port.remote_ip, Some("192.168.10.11".parse().unwrap()));
        assert_eq!(editable.devices.iter().find(|device| Some(device.id) == port.device).unwrap().name, "PLC_Cellier");
        let mut original = ProjectSimulation::new(&editable).unwrap();
        original.select(1).unwrap();
        original.scan(0.0, |_, _, _| None);

        // Drive Cellier's existing M4 from a known input and check that Salon receives it.
        editable.switch_sheet(1);
        let marker = editable.blocks.iter().find(|block| block.name == "M4").unwrap().id;
        let input = editable.add(Kind::Input, 0.0, 0.0);
        let input_index = editable.blocks.len() - 1;
        editable.wires.retain(|wire| wire.to != marker);
        editable.wires.push(Wire::new(input, marker, 0));
        let mut sim = ProjectSimulation::new(&editable).unwrap();
        sim.signals()[input_index].input = true;
        sim.scan(0.0, |_, _, _| None);
        sim.scan(0.02, |_, _, _| None);
        sim.select(0).unwrap();
        assert!(sim.signal(receiver).high);
        sim.select(1).unwrap();
        sim.signals()[input_index].input = false;
        sim.scan(0.04, |_, _, _| None);
        sim.scan(0.06, |_, _, _| None);
        sim.select(0).unwrap();
        assert!(!sim.signal(receiver).high);
        assert_eq!(project.name, "Project_Maison");
        assert_eq!(project.paths.len(), 4);
        for (device, file) in [
            ("PLC_Salon", "Domotique_Salon.lsc"),
            ("PLC_Cellier", "Domotique_Cellier.lsc"),
        ] {
            let device = project
                .devices
                .iter()
                .find(|item| item.name == device)
                .unwrap();
            let (name, diagram) = device.diagram.as_ref().unwrap();
            assert_eq!(name, file);
            let diagram = diagram.as_ref().unwrap();
            assert!(
                diagram.blocks.len() > 50,
                "{name}: {} blocs",
                diagram.blocks.len()
            );
            assert!(
                diagram.wires.len() > 50,
                "{name}: {} fils",
                diagram.wires.len()
            );
            assert!(diagram.connectors.iter().any(|(path, _)| path[0][0] > 24.0));
            assert_eq!(diagram.omitted, 0);
            if file == "Domotique_Salon.lsc" {
                let gate = diagram
                    .blocks
                    .iter()
                    .find(|block| block.number == "B043")
                    .unwrap();
                assert_eq!(gate.inputs.len(), 4);
                let mut pins: Vec<_> = gate.inputs.iter().collect();
                pins.sort_by(|a, b| a.ends[0][1].total_cmp(&b.ends[0][1]));
                assert!(
                    pins.iter()
                        .all(|pin| pin.ends[0][0].max(pin.ends[1][0]) == gate.position[0])
                );
            }
        }
    }

    #[cfg(windows)]
    #[test]
    #[ignore = "nécessite les exemples de l’installation Siemens via ATELIER_LSC_TEST"]
    fn opens_installed_cutting_example_as_a_standalone_diagram() {
        let path = std::env::var_os("ATELIER_LSC_TEST").expect("ATELIER_LSC_TEST");
        let network = Network::open_diagram(Path::new(&path)).unwrap();
        let (name, diagram) = network.devices[0].diagram.as_ref().unwrap();
        assert_eq!(name, "Cutting tool.lsc");
        let diagram = diagram.as_ref().unwrap();
        assert!(!diagram.blocks.is_empty());
        assert!(!diagram.wires.is_empty());
        assert!(
            diagram
                .blocks
                .iter()
                .any(|block| block.inputs.iter().any(|pin| pin.inverted))
        );
        assert!(diagram.blocks.iter().any(|block| !block.outputs.is_empty()));
        assert!(
            diagram
                .blocks
                .iter()
                .any(|block| !block.parameter_text.is_empty())
        );
        assert!(
            diagram
                .comments
                .iter()
                .filter(|(point, _)| *point == [0.0, 0.0])
                .count()
                <= 1
        );
    }
}
