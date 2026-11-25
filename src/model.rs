use std::{
    collections::{HashMap, HashSet, VecDeque},
    net::Ipv4Addr,
};

use crate::schedule::{self, Schedule};
use serde::{Deserialize, Serialize};

pub const COUNTER_MAX: i32 = 99_999;
pub const DEFAULT_DELAY: f64 = 0.0;
pub const DEFAULT_LIMIT: i32 = 3;
pub const SCAN_MS: u64 = 20;
pub const ANALOG_IO_MAX: i32 = 1000;
pub const MAX_DELAY: f64 = 86_400.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Kind {
    Input,
    Output,
    And,
    Or,
    Not,
    Nand,
    Nor,
    Xor,
    OnDelay,
    OffDelay,
    Relay,
    Counter,
    OnOffDelay,
    LatchedDelay,
    PulseRelay,
    PulseGenerator,
    AnalogInput,
    AnalogOutput,
    Amplifier,
    Threshold,
    Comparator,
    AndRising,
    NandFalling,
    ConstantHigh,
    ConstantLow,
    ToggleRelay,
    CursorKey,
    FunctionKey,
    OpenPin,
    Marker,
    EdgePulse,
    Stopwatch,
    RandomDelay,
    HoursCounter,
    Frequency,
    AnalogMarker,
    Differential,
    Watchdog,
    Mux,
    Pwm,
    Math,
    AnalogFilter,
    NetInput,
    NetAnalogInput,
    NetOutput,
    NetAnalogOutput,
    ShiftBit,
    Stairway,
    ComfortSwitch,
    Ramp,
    MinMax,
    Average,
    ShiftRegister,
    MathError,
    MessageText,
    DataLog,
    WeeklyTimer,
    YearlyTimer,
    AstroClock,
    PiController,
    FloatToInteger,
    IntegerToFloat,
    SoftKey,
    Udf,
}

impl Kind {
    pub const IO: &[Self] = &[
        Self::Input,
        Self::CursorKey,
        Self::FunctionKey,
        Self::ShiftBit,
        Self::ConstantLow,
        Self::ConstantHigh,
        Self::Output,
        Self::OpenPin,
        Self::Marker,
    ];
    pub const LOGIC: [Self; 8] = [
        Self::And,
        Self::AndRising,
        Self::Nand,
        Self::NandFalling,
        Self::Or,
        Self::Nor,
        Self::Xor,
        Self::Not,
    ];
    pub const TIMERS: &[Self] = &[
        Self::OnDelay,
        Self::OffDelay,
        Self::OnOffDelay,
        Self::LatchedDelay,
        Self::PulseRelay,
        Self::EdgePulse,
        Self::PulseGenerator,
        Self::RandomDelay,
        Self::Stairway,
        Self::ComfortSwitch,
        Self::WeeklyTimer,
        Self::YearlyTimer,
        Self::AstroClock,
        Self::Stopwatch,
    ];
    pub const OTHER: &[Self] = &[
        Self::Relay,
        Self::ToggleRelay,
        Self::MessageText,
        Self::SoftKey,
        Self::ShiftRegister,
        Self::MathError,
        Self::FloatToInteger,
        Self::IntegerToFloat,
    ];
    pub const ARCHIVE: &[Self] = &[Self::DataLog];
    pub const USER_DEFINED: &[Self] = &[Self::Udf];
    pub const ANALOG_IO: &[Self] = &[Self::AnalogInput, Self::AnalogOutput, Self::AnalogMarker];
    pub const ANALOG: &[Self] = &[
        Self::Math,
        Self::Comparator,
        Self::Threshold,
        Self::Amplifier,
        Self::Watchdog,
        Self::Differential,
        Self::Mux,
        Self::Ramp,
        Self::PiController,
        Self::Pwm,
        Self::AnalogFilter,
        Self::MinMax,
        Self::Average,
    ];
    pub const COUNTERS: &[Self] = &[Self::Counter, Self::HoursCounter, Self::Frequency];
    pub const NETWORK: &[Self] = &[
        Self::NetInput,
        Self::NetAnalogInput,
        Self::NetOutput,
        Self::NetAnalogOutput,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Input => "Entrée numérique",
            Self::Output => "Sortie numérique",
            Self::And => "ET",
            Self::Or => "OU",
            Self::Not => "NON",
            Self::Nand => "NON-ET",
            Self::Nor => "NON-OU",
            Self::Xor => "OU exclusif",
            Self::OnDelay => "Retard à la montée",
            Self::OffDelay => "Retard à la descente",
            Self::Relay => "Relais mémorisé",
            Self::Counter => "Compteur",
            Self::OnOffDelay => "Retard montée et descente",
            Self::LatchedDelay => "Retard mémorisé",
            Self::PulseRelay => "Relais de passage temporisé",
            Self::PulseGenerator => "Générateur d’impulsions",
            Self::AnalogInput => "Entrée analogique",
            Self::AnalogOutput => "Sortie analogique",
            Self::Amplifier => "Amplificateur analogique",
            Self::Threshold => "Seuil analogique",
            Self::Comparator => "Comparateur analogique",
            Self::AndRising => "ET à front montant",
            Self::NandFalling => "NON-ET à front descendant",
            Self::ConstantHigh => "Constante 1",
            Self::ConstantLow => "Constante 0",
            Self::ToggleRelay => "Relais à impulsion",
            Self::CursorKey => "Touche fléchée",
            Self::FunctionKey => "Touche de fonction LOGO! TD",
            Self::OpenPin => "Borne ouverte",
            Self::Marker => "Mémento",
            Self::EdgePulse => "Relais de passage déclenché par front",
            Self::Stopwatch => "Chronomètre",
            Self::RandomDelay => "Générateur aléatoire",
            Self::HoursCounter => "Compteur d’heures de fonctionnement",
            Self::Frequency => "Détecteur de seuil",
            Self::AnalogMarker => "Mémento analogique",
            Self::Differential => "Détecteur de seuil différentiel analogique",
            Self::Watchdog => "Surveillance de la valeur analogique",
            Self::Mux => "MUX analogique",
            Self::Pwm => "Modulateur de largeur d’impulsion",
            Self::Math => "Opération mathématique",
            Self::AnalogFilter => "Filtre analogique",
            Self::NetInput => "Entrée réseau",
            Self::NetAnalogInput => "Entrée réseau analogique",
            Self::NetOutput => "Sortie réseau",
            Self::NetAnalogOutput => "Sortie réseau analogique",
            Self::ShiftBit => "Bit de registre à décalage",
            Self::Stairway => "Interrupteur d’éclairage d’escalier",
            Self::ComfortSwitch => "Interrupteur confort",
            Self::Ramp => "Rampe analogique",
            Self::MinMax => "Max/Min",
            Self::Average => "Valeur moyenne",
            Self::ShiftRegister => "Registre de décalage",
            Self::MathError => "Détection d’erreurs d’opération mathématique",
            Self::MessageText => "Texte de message",
            Self::DataLog => "Archive de variables",
            Self::WeeklyTimer => "Minuterie hebdomadaire",
            Self::YearlyTimer => "Horloge de programmation annuelle",
            Self::AstroClock => "Horloge astronomique",
            Self::PiController => "Régulateur PI",
            Self::FloatToInteger => "Conversion flottant/entier",
            Self::IntegerToFloat => "Conversion entier/flottant",
            Self::SoftKey => "Télérupteur logiciel",
            Self::Udf => "Fonction utilisateur",
        }
    }

    pub fn symbol(self) -> &'static str {
        match self {
            Self::Input => "I",
            Self::Output => "Q",
            Self::And => "&",
            Self::Or => "≥1",
            Self::Not => "¬",
            Self::Nand => "¬&",
            Self::Nor => "¬≥1",
            Self::Xor => "=1",
            Self::OnDelay => "TON",
            Self::OffDelay => "TOF",
            Self::Relay => "RS",
            Self::Counter => "CT",
            Self::OnOffDelay => "T/H",
            Self::LatchedDelay => "TM",
            Self::PulseRelay => "TP",
            Self::PulseGenerator => "GEN",
            Self::AnalogInput => "AI",
            Self::AnalogOutput => "AQ",
            Self::Amplifier => "AMP",
            Self::Threshold => "SE",
            Self::Comparator => "CMP",
            Self::AndRising => "&↑",
            Self::NandFalling => "&↓",
            Self::ConstantHigh => "1",
            Self::ConstantLow => "0",
            Self::ToggleRelay => "TLR",
            Self::CursorKey => "C",
            Self::FunctionKey => "F",
            Self::OpenPin => "x",
            Self::Marker => "M",
            Self::EdgePulse => "TP↑",
            Self::Stopwatch => "SW",
            Self::RandomDelay => "RND",
            Self::HoursCounter => "H",
            Self::Frequency => "Hz",
            Self::AnalogMarker => "AM",
            Self::Differential => "ΔA",
            Self::Watchdog => "MON",
            Self::Mux => "MUX",
            Self::Pwm => "PWM",
            Self::Math => "MAT",
            Self::AnalogFilter => "FLT",
            Self::NetInput => "NI",
            Self::NetAnalogInput => "NAI",
            Self::NetOutput => "NQ",
            Self::NetAnalogOutput => "NAQ",
            Self::ShiftBit => "S",
            Self::Stairway => "ESC",
            Self::ComfortSwitch => "IC",
            Self::Ramp => "RMP",
            Self::MinMax => "MIN",
            Self::Average => "MOY",
            Self::ShiftRegister => ">>",
            Self::MathError => "ERR",
            Self::MessageText => "TXT",
            Self::DataLog => "LOG",
            Self::WeeklyTimer => "HEB",
            Self::YearlyTimer => "ANN",
            Self::AstroClock => "AST",
            Self::PiController => "PI",
            Self::FloatToInteger => "F/I",
            Self::IntegerToFloat => "I/F",
            Self::SoftKey => "TL",
            Self::Udf => "UDF",
        }
    }

    pub fn pins(self) -> &'static [&'static str] {
        match self {
            Self::Input | Self::AnalogInput | Self::ConstantHigh | Self::ConstantLow => &[],
            Self::Output | Self::Not => &["1"],
            Self::And | Self::Or | Self::Nand | Self::Nor | Self::AndRising | Self::NandFalling => {
                &["1", "2", "3", "4"]
            }
            Self::Xor => &["1", "2"],
            Self::OnDelay | Self::OnOffDelay | Self::PulseRelay => &["Trg"],
            Self::OffDelay | Self::LatchedDelay => &["Trg", "R"],
            Self::PulseGenerator => &["En", "Inv"],
            Self::Relay => &["S", "R"],
            Self::Counter => &["R", "Cnt", "Dir"],
            Self::AnalogOutput | Self::Amplifier | Self::Threshold => &["Ax"],
            Self::Comparator => &["Ax", "Ay"],
            Self::ToggleRelay => &["Trg", "S", "R"],
            Self::CursorKey | Self::FunctionKey | Self::NetInput | Self::NetAnalogInput => &[],
            Self::OpenPin | Self::Marker | Self::NetOutput => &["1"],
            Self::EdgePulse => &["Trg", "R"],
            Self::Stopwatch => &["En", "Lap", "R"],
            Self::RandomDelay => &["En"],
            Self::HoursCounter => &["R", "En", "Ral"],
            Self::Frequency => &["Fre"],
            Self::AnalogMarker
            | Self::Differential
            | Self::AnalogFilter
            | Self::NetAnalogOutput => &["Ax"],
            Self::Watchdog | Self::Pwm => &["En", "Ax"],
            Self::Mux => &["En", "S1", "S2"],
            Self::Math => &["En", "V1", "V2"],
            Self::ShiftBit => &["Reg"],
            Self::Stairway | Self::MessageText => &["Trg"],
            Self::ComfortSwitch => &["Trg", "R"],
            Self::Ramp => &["En", "Sel", "St"],
            Self::MinMax => &["En", "S1", "Ax"],
            Self::Average => &["En", "R", "Ax"],
            Self::ShiftRegister => &["In", "Trg", "Dir"],
            Self::MathError => &["R", "Calc"],
            Self::DataLog => &["Trg", "Ax"],
            Self::WeeklyTimer | Self::YearlyTimer | Self::AstroClock | Self::FloatToInteger => &[],
            Self::PiController => &["A/M", "R", "PV"],
            Self::IntegerToFloat => &["Ax"],
            Self::SoftKey => &["En"],
            Self::Udf => &["I1", "I2", "I3", "I4"],
        }
    }

    pub fn output(self) -> bool {
        !matches!(
            self,
            Self::Output
                | Self::AnalogOutput
                | Self::OpenPin
                | Self::NetOutput
                | Self::NetAnalogOutput
                | Self::DataLog
        )
    }
    pub fn timer(self) -> bool {
        matches!(
            self,
            Self::OnDelay
                | Self::OffDelay
                | Self::OnOffDelay
                | Self::LatchedDelay
                | Self::PulseRelay
                | Self::PulseGenerator
                | Self::EdgePulse
                | Self::RandomDelay
                | Self::Pwm
                | Self::Stairway
                | Self::ComfortSwitch
                | Self::Average
                | Self::PiController
        )
    }
    pub fn two_times(self) -> bool {
        matches!(
            self,
            Self::OnOffDelay | Self::PulseGenerator | Self::EdgePulse | Self::RandomDelay
        )
    }
    pub fn analog_value(self) -> bool {
        matches!(
            self,
            Self::AnalogInput
                | Self::AnalogOutput
                | Self::Amplifier
                | Self::AnalogMarker
                | Self::Stopwatch
                | Self::Mux
                | Self::Math
                | Self::AnalogFilter
                | Self::NetAnalogInput
                | Self::NetAnalogOutput
                | Self::Ramp
                | Self::MinMax
                | Self::Average
                | Self::PiController
                | Self::FloatToInteger
                | Self::IntegerToFloat
        )
    }
    pub fn analog_pin(self, pin: usize) -> bool {
        match self {
            Self::Watchdog | Self::Pwm | Self::Math => pin != 0,
            Self::MinMax | Self::Average | Self::PiController => pin == 2,
            Self::MathError | Self::DataLog => pin == 1,
            Self::AnalogOutput
            | Self::Amplifier
            | Self::Threshold
            | Self::Comparator
            | Self::AnalogMarker
            | Self::Differential
            | Self::AnalogFilter
            | Self::IntegerToFloat
            | Self::NetAnalogOutput => true,
            _ => false,
        }
    }

    pub fn can_invert_pin(self, pin: usize) -> bool {
        pin < self.pins().len()
            && !matches!(
                self,
                Self::Output | Self::NetOutput | Self::OpenPin | Self::Udf | Self::ShiftBit
            )
            && !self.analog_pin(pin)
    }

    pub fn network_input(self) -> bool {
        matches!(self, Self::NetInput | Self::NetAnalogInput)
    }

    pub fn network_output(self) -> bool {
        matches!(self, Self::NetOutput | Self::NetAnalogOutput)
    }

    pub fn simulated_key(self) -> bool {
        matches!(
            self,
            Self::Input | Self::CursorKey | Self::FunctionKey | Self::SoftKey
        )
    }
    pub fn unconnected(self) -> bool {
        matches!(
            self,
            Self::And | Self::Nand | Self::Not | Self::AndRising | Self::NandFalling
        )
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AnalogParameters {
    pub gain: f64,
    pub offset: i16,
    pub on: i16,
    pub off: i16,
}

impl Default for AnalogParameters {
    fn default() -> Self {
        Self {
            gain: 1.0,
            offset: 0,
            on: 500,
            off: 400,
        }
    }
}

impl AnalogParameters {
    fn scale(&self, value: i32) -> i32 {
        (f64::from(value) * self.gain + f64::from(self.offset)).trunc() as i32
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Block {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub io_number: Option<u16>,
    pub id: u32,
    pub kind: Kind,
    pub name: String,
    pub x: f32,
    pub y: f32,
    pub duration: f64,
    pub limit: i32,
    #[serde(default)]
    pub duration_low: f64,
    #[serde(default)]
    pub analog: AnalogParameters,
    #[serde(default)]
    pub set_priority: bool,
    #[serde(default)]
    pub pulse_delay: f64,
    #[serde(default)]
    pub values: [i16; 4],
    #[serde(default)]
    pub operation: Operation,
    #[serde(default)]
    pub network: NetworkPort,
    #[serde(default)]
    pub message: String,
    #[serde(default, skip_serializing_if = "is_empty_pin_mask")]
    pub inverted_pins: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schedule: Option<Schedule>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subcircuit: Option<Box<Project>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drawing: Option<BlockDrawing>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BlockDrawing {
    pub size: [f32; 2],
    pub symbol: String,
    pub inputs: Vec<Option<[[f32; 2]; 2]>>,
    pub extra_inputs: Vec<(String, [[f32; 2]; 2])>,
    pub output: Option<[[f32; 2]; 2]>,
    pub unsupported: bool,
    pub parameter_text: String,
}

fn is_empty_pin_mask(mask: &u8) -> bool {
    *mask == 0
}

impl Block {
    fn address_number(&self) -> Option<u16> {
        self.io_number.or_else(|| {
            self.name.split_whitespace().next()?
                .strip_prefix(self.kind.symbol())?.parse().ok()
        })
    }
    pub fn has_output(&self) -> bool {
        self.kind.output()
            || self
                .drawing
                .as_ref()
                .is_some_and(|drawing| drawing.output.is_some())
    }

    pub fn pin_inverted(&self, pin: usize) -> bool {
        self.kind.can_invert_pin(pin) && self.inverted_pins & (1 << pin) != 0
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub enum Operation {
    #[default]
    Add,
    Subtract,
    Multiply,
    Divide,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct NetworkPort {
    pub device: Option<u32>,
    pub connection: Option<u32>,
    pub address: u16,
    #[serde(default)]
    pub source: NetworkSource,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remote_ip: Option<Ipv4Addr>,
    #[serde(default, skip_serializing_if = "is_zero_bit")]
    pub bit: u8,
}

fn is_zero_bit(bit: &u8) -> bool {
    *bit == 0
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum NetworkSource {
    #[default]
    Simulated,
    DiscreteInput,
    Coil,
    LocalVm,
    RemoteVm,
    RemoteBlock(RemoteBlock),
    Diagnostic,
    Unsupported,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RemoteBlock {
    Input,
    Output,
    Marker,
    AnalogInput,
    AnalogOutput,
    AnalogMarker,
}

impl RemoteBlock {
    pub fn kind(self) -> Kind {
        match self {
            Self::Input => Kind::Input,
            Self::Output => Kind::Output,
            Self::Marker => Kind::Marker,
            Self::AnalogInput => Kind::AnalogInput,
            Self::AnalogOutput => Kind::AnalogOutput,
            Self::AnalogMarker => Kind::AnalogMarker,
        }
    }
}

impl NetworkSource {
    pub fn live_count(self) -> Option<u16> {
        match self {
            Self::DiscreteInput => Some(24),
            Self::Coil => Some(20),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Simulated => "Connexion simulée",
            Self::DiscreteInput => "Entrée I · Modbus TCP",
            Self::Coil => "Sortie Q · Modbus TCP",
            Self::LocalVm => "Mémoire variable (VM) locale",
            Self::RemoteVm | Self::RemoteBlock(_) => "Appareil distant",
            Self::Diagnostic => "Diagnostic",
            Self::Unsupported => "Paramètre importé non pris en charge",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Device {
    pub id: u32,
    pub name: String,
    pub ip: Ipv4Addr,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile: Option<u16>,
}

impl Device {
    pub fn profile_label(&self) -> &str {
        match self.profile {
            Some(101) => "LOGO! 8 (0BA8.Standard)",
            Some(104) => "LOGO! 8.1 / 8.2 (FS4)",
            _ => "—",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Protocol {
    S7,
    ModbusTcp,
}

impl Protocol {
    pub fn label(self) -> &'static str {
        match self {
            Self::S7 => "S7",
            Self::ModbusTcp => "Modbus TCP",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Connection {
    pub id: u32,
    pub client: u32,
    pub server: u32,
    pub protocol: Protocol,
}

impl Connection {
    pub fn peer(self, device: u32) -> Option<u32> {
        if self.client == device {
            Some(self.server)
        } else if self.server == device {
            Some(self.client)
        } else {
            None
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Wire {
    pub from: u32,
    pub to: u32,
    pub pin: usize,
    #[serde(default)]
    pub cut: bool,
}

impl Wire {
    pub fn new(from: u32, to: u32, pin: usize) -> Self {
        Self {
            from,
            to,
            pin,
            cut: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Project {
    version: u32,
    pub name: String,
    pub blocks: Vec<Block>,
    pub wires: Vec<Wire>,
    #[serde(default)]
    pub devices: Vec<Device>,
    #[serde(default)]
    pub connections: Vec<Connection>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drawing: Option<DiagramDrawing>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sheets: Vec<Sheet>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_sheet: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub network_drawing: Option<NetworkDrawing>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub import_warnings: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct DiagramDrawing {
    pub comments: Vec<([f32; 2], String)>,
    pub routes: Vec<WireDrawing>,
    pub loose_wires: Vec<Vec<[f32; 2]>>,
    pub loose_connectors: Vec<ConnectorDrawing>,
    pub issues: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WireDrawing {
    pub wire: Wire,
    pub route: Vec<[f32; 2]>,
    pub anchors: [[f32; 2]; 2],
    pub connectors: Option<[ConnectorDrawing; 2]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cut_stubs: Option<[Vec<[f32; 2]>; 2]>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ConnectorDrawing {
    pub path: Vec<[f32; 2]>,
    pub label: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Sheet {
    pub name: String,
    pub device: Option<u32>,
    pub blocks: Vec<Block>,
    pub wires: Vec<Wire>,
    pub drawing: Option<DiagramDrawing>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NetworkDrawing {
    pub positions: Vec<(u32, [f32; 2])>,
    pub external: Vec<u32>,
    pub local: [f32; 2],
    pub paths: Vec<(bool, Vec<[f32; 2]>)>,
}

pub type ScanOrder = Vec<(usize, [Option<usize>; 4])>;

impl Project {
    pub fn empty() -> Self {
        Self {
            version: 1,
            name: "Nouveau schéma".into(),
            blocks: Vec::new(),
            wires: Vec::new(),
            devices: Vec::new(),
            connections: Vec::new(),
            drawing: None,
            sheets: Vec::new(),
            active_sheet: None,
            network_drawing: None,
            import_warnings: Vec::new(),
        }
    }

    pub fn switch_sheet(&mut self, next: usize) {
        if self.active_sheet == Some(next) || next >= self.sheets.len() {
            return;
        }
        if let Some(active) = self.active_sheet {
            let current = &mut self.sheets[active];
            current.blocks = std::mem::take(&mut self.blocks);
            current.wires = std::mem::take(&mut self.wires);
            current.drawing = self.drawing.take();
        }
        let next_sheet = &mut self.sheets[next];
        self.blocks = std::mem::take(&mut next_sheet.blocks);
        self.wires = std::mem::take(&mut next_sheet.wires);
        self.drawing = next_sheet.drawing.take();
        self.active_sheet = Some(next);
    }

    pub fn active_name(&self) -> &str {
        self.active_sheet
            .map_or(&self.name, |index| &self.sheets[index].name)
    }

    pub fn document_valid(&self) -> Result<(), &'static str> {
        if self.sheets.len() > 64
            || self.sheets.is_empty() != self.active_sheet.is_none()
            || self.active_sheet.is_some_and(|index| {
                self.sheets.get(index).is_none_or(|sheet| {
                    !sheet.blocks.is_empty() || !sheet.wires.is_empty() || sheet.drawing.is_some()
                })
            })
        {
            return Err("Liste de schémas invalide.");
        }
        if self.drawing.is_none() && self.sheets.is_empty() {
            return self.order().map(|_| ());
        }
        self.valid_layout(&self.blocks, &self.wires)?;
        for sheet in &self.sheets {
            self.valid_layout(&sheet.blocks, &sheet.wires)?;
        }
        Ok(())
    }

    fn valid_layout(&self, blocks: &[Block], wires: &[Wire]) -> Result<(), &'static str> {
        if self.version != 1
            || blocks.len() > 2000
            || wires.len() > 8000
            || self.devices.len() > 64
            || self.connections.len() > 512
        {
            return Err("Format ou taille du projet incompatible.");
        }
        let mut ids = HashMap::new();
        for block in blocks {
            if ids.insert(block.id, block).is_some()
                || !block.x.is_finite()
                || !block.y.is_finite()
                || block.drawing.as_ref().is_some_and(|drawing| {
                    drawing
                        .size
                        .iter()
                        .any(|value| !(12.0..=200.0).contains(value))
                })
            {
                return Err("Position ou géométrie de bloc invalide.");
            }
        }
        let mut targets = HashSet::new();
        for wire in wires {
            if !ids.contains_key(&wire.from)
                || !ids
                    .get(&wire.to)
                    .is_some_and(|target| wire.pin < target.kind.pins().len())
                || !targets.insert((wire.to, wire.pin))
            {
                return Err("Un fil référence une broche invalide ou déjà occupée.");
            }
        }
        if self.connections.iter().any(|link| {
            !self.devices.iter().any(|device| device.id == link.client)
                || !self.devices.iter().any(|device| device.id == link.server)
        }) {
            return Err("Une liaison réseau référence un appareil absent.");
        }
        Ok(())
    }

    pub fn example() -> Self {
        let mut p = Self::empty();
        p.name = "Autorisation de marche".into();
        let start = p.add(Kind::Input, 36.0, 38.0);
        let limit = p.add(Kind::Input, 36.0, 122.0);
        let gate = p.add(Kind::And, 144.0, 76.0);
        let out = p.add(Kind::Output, 254.0, 78.0);
        p.block_mut(start).unwrap().name = "I1 · Marche".into();
        p.block_mut(limit).unwrap().name = "I2 · Autorisation".into();
        p.block_mut(out).unwrap().name = "Q1 · Moteur".into();
        p.wires = vec![
            Wire::new(start, gate, 0),
            Wire::new(limit, gate, 1),
            Wire::new(gate, out, 0),
        ];
        p
    }

    pub fn network_example() -> Self {
        let mut p = Self::empty();
        p.name = "Signal entre deux automates".into();
        p.devices = vec![
            Device {
                id: 1,
                profile: None,
                name: "LOGO! A".into(),
                ip: Ipv4Addr::new(192, 168, 10, 10),
            },
            Device {
                id: 2,
                profile: None,
                name: "LOGO! B".into(),
                ip: Ipv4Addr::new(192, 168, 10, 11),
            },
        ];
        p.connections.push(Connection {
            id: 1,
            client: 1,
            server: 2,
            protocol: Protocol::ModbusTcp,
        });
        for (row, (input, send, receive, output)) in [
            (Kind::Input, Kind::NetOutput, Kind::NetInput, Kind::Output),
            (
                Kind::AnalogInput,
                Kind::NetAnalogOutput,
                Kind::NetAnalogInput,
                Kind::AnalogOutput,
            ),
        ]
        .into_iter()
        .enumerate()
        {
            let y = 48.0 + row as f32 * 96.0;
            let source = p.add(input, 36.0, y);
            let sender = p.add(send, 148.0, y);
            let receiver = p.add(receive, 262.0, y);
            let target = p.add(output, 374.0, y);
            p.block_mut(sender).unwrap().network = NetworkPort {
                device: Some(1),
                connection: Some(1),
                address: 1,
                ..Default::default()
            };
            p.block_mut(receiver).unwrap().network = NetworkPort {
                device: Some(2),
                connection: Some(1),
                address: 1,
                ..Default::default()
            };
            p.wires
                .extend([Wire::new(source, sender, 0), Wire::new(receiver, target, 0)]);
        }
        p
    }

    pub fn add(&mut self, kind: Kind, x: f32, y: f32) -> u32 {
        let id = (1..).find(|id| self.block(*id).is_none()).unwrap();
        let prefix = match kind {
            Kind::Input => "I",
            Kind::Output => "Q",
            Kind::AnalogInput => "AI",
            Kind::AnalogOutput => "AQ",
            Kind::CursorKey
            | Kind::FunctionKey
            | Kind::Marker
            | Kind::AnalogMarker
            | Kind::NetInput
            | Kind::NetOutput
            | Kind::NetAnalogInput
            | Kind::NetAnalogOutput => kind.symbol(),
            _ => "B",
        };
        let index = (1..)
            .find(|i| {
                let label = format!("{prefix}{i}");
                self.blocks
                    .iter()
                    .all(|b| b.name.split_whitespace().next() != Some(label.as_str())
                        && (b.kind != kind || b.io_number != u16::try_from(*i).ok()))
            })
            .unwrap();
        self.blocks.push(Block {
            io_number: u16::try_from(index).ok(),
            id,
            kind,
            name: format!("{prefix}{index}"),
            x,
            y,
            duration: match kind {
                Kind::AnalogFilter => 0.5,
                Kind::Frequency => 1.0,
                Kind::Stairway | Kind::ComfortSwitch => 30.0,
                Kind::Average => 1.0,
                Kind::PiController => 10.0,
                Kind::FloatToInteger | Kind::IntegerToFloat => 1.0,
                _ if kind.two_times() || matches!(kind, Kind::PulseRelay | Kind::Pwm) => 1.0,
                _ => DEFAULT_DELAY,
            },
            limit: match kind {
                Kind::EdgePulse => 1,
                Kind::HoursCounter => 100,
                Kind::Ramp => 100,
                Kind::ShiftBit | Kind::ShiftRegister => 1,
                Kind::MinMax => 0,
                Kind::MathError => 0,
                Kind::Average => 10,
                Kind::Stairway | Kind::ComfortSwitch => 0,
                Kind::PiController => 100,
                Kind::FloatToInteger | Kind::IntegerToFloat => 0,
                _ => DEFAULT_LIMIT,
            },
            duration_low: if kind.two_times() || kind == Kind::ComfortSwitch {
                1.0
            } else {
                0.0
            },
            analog: if kind == Kind::Frequency {
                AnalogParameters {
                    on: 4,
                    off: 2,
                    ..Default::default()
                }
            } else {
                AnalogParameters::default()
            },
            set_priority: false,
            pulse_delay: 0.0,
            values: if kind == Kind::Ramp {
                [250, 750, 0, 1000]
            } else {
                [0, 250, 500, 1000]
            },
            operation: Operation::default(),
            network: NetworkPort::default(),
            message: String::new(),
            inverted_pins: 0,
            schedule: matches!(
                kind,
                Kind::WeeklyTimer | Kind::YearlyTimer | Kind::AstroClock
            )
            .then(Schedule::default),
            subcircuit: None,
            drawing: None,
        });
        id
    }

    pub fn block(&self, id: u32) -> Option<&Block> {
        self.blocks.iter().find(|b| b.id == id)
    }
    pub fn block_mut(&mut self, id: u32) -> Option<&mut Block> {
        self.blocks.iter_mut().find(|b| b.id == id)
    }

    fn udf_ports(&self) -> Result<(Vec<usize>, usize), &'static str> {
        if !self.devices.is_empty()
            || !self.connections.is_empty()
            || self.blocks.iter().any(|b| {
                b.kind == Kind::Udf
                    || b.subcircuit.is_some()
                    || b.kind.network_input()
                    || b.kind.network_output()
            })
        {
            return Err(
                "Une fonction utilisateur accepte un schéma local sans fonction imbriquée.",
            );
        }
        let mut inputs = Vec::new();
        let mut output = None;
        for (index, block) in self.blocks.iter().enumerate() {
            if block.kind == Kind::Input {
                inputs.push(index);
            }
            if block.kind == Kind::Output && output.replace(index).is_some() {
                return Err("Une fonction utilisateur accepte un seul bloc Sortie numérique.");
            }
        }
        if inputs.len() > 4 {
            return Err("Une fonction utilisateur accepte jusqu’à quatre entrées numériques.");
        }
        Ok((
            inputs,
            output.ok_or("Ajoutez un bloc Sortie numérique au schéma de la fonction.")?,
        ))
    }

    pub fn order(&self) -> Result<ScanOrder, &'static str> {
        if self.version != 1
            || self.blocks.len() > 2000
            || self.wires.len() > 8000
            || self.devices.len() > 64
            || self.connections.len() > 512
        {
            return Err("Format ou taille du projet incompatible.");
        }
        let mut devices = HashSet::new();
        let mut ips = HashSet::new();
        if self.devices.iter().any(|d| {
            let external = self.network_drawing.as_ref().is_some_and(|drawing| drawing.external.contains(&d.id));
            !devices.insert(d.id)
                || (!d.ip.is_unspecified() && !ips.insert(d.ip))
                || d.name.trim().is_empty()
                || (d.ip.is_unspecified() && !external)
                || d.ip.is_broadcast()
                || d.ip.is_multicast()
        }) {
            return Err("Les appareils doivent avoir un nom et une adresse IP unique et valide.");
        }
        let mut links = HashMap::new();
        for &link in &self.connections {
            if links.insert(link.id, link).is_some()
                || link.client == link.server
                || !devices.contains(&link.client)
                || !devices.contains(&link.server)
            {
                return Err("Une liaison réseau référence un appareil absent ou invalide.");
            }
        }
        let mut ids = HashMap::new();
        let mut outputs = HashSet::new();
        let mut block_count = self.blocks.len();
        for (i, b) in self.blocks.iter().enumerate() {
            if ids.insert(b.id, i).is_some()
                || !b.x.is_finite()
                || !b.y.is_finite()
                || !b.duration.is_finite()
                || !(0.0..=MAX_DELAY).contains(&b.duration)
                || !(0.0..=MAX_DELAY).contains(&b.duration_low)
                || !(0.0..=MAX_DELAY).contains(&b.pulse_delay)
                || !(-10.0..=10.0).contains(&b.analog.gain)
                || !(-10_000..=10_000).contains(&b.analog.offset)
                || !(-20_000..=20_000).contains(&b.analog.on)
                || !(-20_000..=20_000).contains(&b.analog.off)
                || !(0..=COUNTER_MAX).contains(&b.limit)
            {
                return Err("Le projet contient des paramètres de bloc invalides.");
            }
            if let Some(subcircuit) = &b.subcircuit {
                block_count += subcircuit.blocks.len();
                if b.kind != Kind::Udf || block_count > 2000 {
                    return Err(
                        "Le projet contient trop de blocs ou une fonction utilisateur invalide.",
                    );
                }
                subcircuit.udf_ports()?;
                subcircuit.order()?;
            }
            if b.kind.network_input() || b.kind.network_output() {
                if b.network.device.is_some_and(|id| !devices.contains(&id)) {
                    return Err("Un bloc réseau référence un appareil absent.");
                }
                if let Some(count) = b.network.source.live_count()
                    && (b.kind != Kind::NetInput
                        || b.network.connection.is_some()
                        || !(1..=count).contains(&b.network.address))
                {
                    return Err(
                        "La source Modbus doit désigner une entrée I ou une sortie Q valide.",
                    );
                }
                if let Some(link) = b.network.connection {
                    let port = (
                        link,
                        b.network.device,
                        b.network.address,
                        b.kind.analog_value(),
                    );
                    if b.network
                        .device
                        .and_then(|id| links.get(&link)?.peer(id))
                        .is_none()
                    {
                        return Err("La liaison choisie ne dessert pas cet appareil.");
                    }
                    if b.kind.network_output() && !outputs.insert(port) {
                        return Err("Deux sorties partagent la même adresse sur cette liaison.");
                    }
                }
            }
        }
        let mut sources = vec![[None; 4]; self.blocks.len()];
        let mut successors = vec![Vec::new(); self.blocks.len()];
        let mut dependencies = vec![0; self.blocks.len()];
        for w in &self.wires {
            let (Some(&from), Some(&to)) = (ids.get(&w.from), ids.get(&w.to)) else {
                return Err("Un fil référence un bloc absent.");
            };
            if !self.blocks[from].has_output()
                || w.pin >= self.blocks[to].kind.pins().len()
                || sources[to][w.pin].replace(from).is_some()
            {
                return Err("Un fil référence une broche invalide ou déjà occupée.");
            }
            if self.blocks[from].kind.analog_value() != self.blocks[to].kind.analog_pin(w.pin) {
                return Err("Reliez deux broches de même type : analogique ou numérique.");
            }
            match self.blocks[to].kind {
                Kind::MathError if w.pin == 1 && self.blocks[from].kind != Kind::Math => {
                    return Err("Reliez Calc à un bloc Opération mathématique.");
                }
                Kind::ShiftBit if self.blocks[from].kind != Kind::ShiftRegister => {
                    return Err("Reliez Reg à un bloc Registre de décalage.");
                }
                Kind::Udf
                    if self.blocks[to].subcircuit.as_ref().is_some_and(|circuit| {
                        circuit
                            .blocks
                            .iter()
                            .filter(|block| block.kind == Kind::Input)
                            .count()
                            <= w.pin
                    }) =>
                {
                    return Err("Cette fonction utilisateur ne possède pas cette entrée.");
                }
                _ => {}
            }
            successors[from].push(to);
            dependencies[to] += 1;
        }
        let mut pending: VecDeque<_> = dependencies
            .iter()
            .enumerate()
            .filter_map(|(i, &count)| (count == 0).then_some(i))
            .collect();
        let mut order = Vec::with_capacity(self.blocks.len());
        let mut scheduled = vec![false; self.blocks.len()];
        while order.len() < self.blocks.len() {
            let i = if let Some(i) = pending.pop_front() {
                i
            } else if self.drawing.is_some()
                && let Some(i) = self.blocks.iter().enumerate().position(|(i, block)| {
                    !scheduled[i] && matches!(block.kind,
                        Kind::Output | Kind::AnalogOutput | Kind::Marker | Kind::AnalogMarker
                            | Kind::Relay | Kind::PulseRelay | Kind::ToggleRelay)
                        && in_feedback(i, &successors, &scheduled)
                })
            {
                // Feedback into an imported state or output reads the previous scan.
                i
            } else {
                return Err("Une boucle de rétroaction a été détectée.");
            };
            scheduled[i] = true;
            order.push((i, sources[i]));
            for &to in &successors[i] {
                if scheduled[to] {
                    continue;
                }
                dependencies[to] -= 1;
                if dependencies[to] == 0 {
                    pending.push_back(to);
                }
            }
        }
        Ok(order)
    }
}

fn in_feedback(start: usize, successors: &[Vec<usize>], scheduled: &[bool]) -> bool {
    let mut visited = scheduled.to_vec();
    let mut pending = successors[start].clone();
    while let Some(index) = pending.pop() {
        if index == start {
            return true;
        }
        if !std::mem::replace(&mut visited[index], true) {
            pending.extend(&successors[index]);
        }
    }
    false
}

#[derive(Clone, Copy, Default)]
pub struct Signal {
    pub high: bool,
    pub input: bool,
    pub count: i32,
    pub elapsed: f64,
    pub analog_input: i32,
    pub analog: i32,
    since: Option<f64>,
    previous: bool,
    reference: i32,
    lifetime: f64,
    remainder: f64,
    random: u32,
    wait: f64,
    shift_bits: u8,
    math_error: u8,
}

impl Signal {
    fn elapsed_since_start(&mut self, enabled: bool, time: f64) -> f64 {
        if enabled {
            self.elapsed = time - *self.since.get_or_insert(time);
        } else {
            self.since = None;
            self.elapsed = 0.0;
        }
        self.elapsed
    }
}

pub struct Simulation {
    pub signals: Vec<Signal>,
    pub archive: VecDeque<(f64, u32, i32)>,
    order: ScanOrder,
    scanned: bool,
    last_scan: f64,
    wall_time: i64,
    vm: [u8; 1024],
    network_sources: Vec<(usize, usize)>,
    functions: HashMap<usize, UserFunction>,
}

struct UserFunction {
    inputs: Vec<usize>,
    output: usize,
    simulation: Simulation,
}

pub struct ProjectSimulation {
    circuits: Vec<(Project, Result<Simulation, &'static str>)>,
    active: usize,
    remote: Vec<RemoteAccess>,
}

struct RemoteAccess {
    sheet: usize,
    block: usize,
    peer: Option<usize>,
    target: Option<usize>,
}

impl ProjectSimulation {
    pub fn new(project: &Project) -> Result<Self, &'static str> {
        let active = project.active_sheet.unwrap_or(0);
        let mut circuit = project.clone();
        let mut sheets = std::mem::take(&mut circuit.sheets);
        circuit.active_sheet = None;
        let mut devices = HashMap::new();
        let circuits = if sheets.is_empty() {
            let simulation = Simulation::new(&circuit);
            vec![(circuit, simulation)]
        } else {
            let current = &mut sheets[active];
            current.blocks = std::mem::take(&mut circuit.blocks);
            current.wires = std::mem::take(&mut circuit.wires);
            current.drawing = circuit.drawing.take();
            sheets.into_iter().enumerate().map(|(index, sheet)| {
                if let Some(device) = sheet.device {
                    devices.insert(device, index);
                }
                let mut circuit = circuit.clone();
                circuit.blocks = sheet.blocks;
                circuit.wires = sheet.wires;
                circuit.drawing = sheet.drawing;
                let simulation = Simulation::new(&circuit);
                (circuit, simulation)
            }).collect::<Vec<_>>()
        };
        circuits[active].1.as_ref().map_err(|error| *error)?;

        let mut remote = Vec::new();
        for (sheet, (circuit, _)) in circuits.iter().enumerate() {
            for (index, block) in circuit.blocks.iter().enumerate() {
                let port = block.network;
                if !matches!(port.source, NetworkSource::RemoteBlock(_) | NetworkSource::RemoteVm) {
                    continue;
                }
                let device = port.device.or_else(|| {
                    project.devices.iter().find(|device| Some(device.ip) == port.remote_ip)
                        .map(|device| device.id)
                });
                let peer = device.and_then(|device| devices.get(&device).copied());
                let target = match port.source {
                    NetworkSource::RemoteBlock(kind) => peer.and_then(|peer| {
                        circuits[peer].0.blocks.iter().position(|candidate| {
                            candidate.kind == kind.kind() && candidate.address_number() == Some(port.address)
                        })
                    }),
                    _ => None,
                };
                remote.push(RemoteAccess { sheet, block: index, peer, target });
            }
        }
        Ok(Self { circuits, active, remote })
    }

    pub fn select(&mut self, sheet: usize) -> Result<(), &'static str> {
        self.circuits[sheet].1.as_ref().map_err(|error| *error)?;
        self.active = sheet;
        Ok(())
    }

    pub fn signals(&mut self) -> &mut [Signal] {
        &mut self.circuits[self.active].1.as_mut().unwrap().signals
    }

    pub fn signal(&self, index: usize) -> Signal {
        self.circuits[self.active].1.as_ref().unwrap().signals[index]
    }

    pub fn archive(&self) -> &VecDeque<(f64, u32, i32)> {
        &self.circuits[self.active].1.as_ref().unwrap().archive
    }

    pub fn scan(&mut self, time: f64, mut read_live: impl FnMut(Ipv4Addr, NetworkSource, u16) -> Option<bool>) {
        for &RemoteAccess { sheet, block: index, peer, target } in &self.remote {
            let block = &self.circuits[sheet].0.blocks[index];
            if !block.kind.network_input() {
                continue;
            }
            let port = block.network;
            let analog = block.kind.analog_value();
            let value = peer.and_then(|peer| {
                let simulation = self.circuits[peer].1.as_ref().ok()?;
                match port.source {
                    NetworkSource::RemoteBlock(_) => target.map(|index| simulation.signals[index]),
                    NetworkSource::RemoteVm => Some(simulation.read_vm(port, analog)),
                    _ => None,
                }
            }).unwrap_or_default();
            if let Ok(simulation) = &mut self.circuits[sheet].1 {
                simulation.signals[index].input = value.high;
                simulation.signals[index].analog_input = value.analog;
            }
        }
        for (circuit, simulation) in &mut self.circuits {
            let Ok(simulation) = simulation else { continue };
            for (block, signal) in circuit.blocks.iter().zip(&mut simulation.signals) {
                let port = block.network;
                if block.kind == Kind::NetInput && port.source.live_count().is_some() {
                    signal.input = circuit.devices.iter().find(|d| Some(d.id) == port.device)
                        .and_then(|device| read_live(device.ip, port.source, port.address))
                        .unwrap_or_default();
                }
            }
            simulation.scan(circuit, time);
        }
        for &RemoteAccess { sheet, block: index, peer, .. } in &self.remote {
            let block = &self.circuits[sheet].0.blocks[index];
            let port = block.network;
            if port.source == NetworkSource::RemoteVm
                && block.kind.network_output()
                && let Some(peer) = peer
                && let Ok(simulation) = &self.circuits[sheet].1
            {
                let analog = block.kind.analog_value();
                let value = simulation.signals[index];
                if let Ok(target) = &mut self.circuits[peer].1 {
                    target.write_vm(port, analog, value);
                }
            }
        }
    }
}

impl Simulation {
    fn read_vm(&self, port: NetworkPort, analog: bool) -> Signal {
        let address = usize::from(port.address);
        let mut value = Signal::default();
        if analog {
            if let Some(bytes) = self.vm.get(address..address + 2) {
                value.analog = i32::from(i16::from_be_bytes(bytes.try_into().unwrap()));
            }
        } else if let Some(byte) = self.vm.get(address) {
            value.high = byte & (1 << port.bit.min(7)) != 0;
        }
        value
    }

    fn write_vm(&mut self, port: NetworkPort, analog: bool, value: Signal) {
        let address = usize::from(port.address);
        if analog {
            if let Some(bytes) = self.vm.get_mut(address..address + 2) {
                let word = value.analog.clamp(i32::from(i16::MIN), i32::from(i16::MAX)) as i16;
                bytes.copy_from_slice(&word.to_be_bytes());
            }
        } else if let Some(byte) = self.vm.get_mut(address) {
            let mask = 1 << port.bit.min(7);
            *byte = (*byte & !mask) | if value.high { mask } else { 0 };
        }
    }

    pub fn new(project: &Project) -> Result<Self, &'static str> {
        if project
            .drawing
            .as_ref()
            .is_some_and(|drawing| !drawing.issues.is_empty())
            || project.blocks.iter().any(|block| {
                block
                    .drawing
                    .as_ref()
                    .is_some_and(|drawing| drawing.unsupported)
            })
        {
            return Err(
                "La simulation est indisponible pour ce schéma. Consultez les détails de l’import.",
            );
        }
        let order = project.order()?;
        let outputs: HashMap<_, _> = project
            .blocks
            .iter()
            .enumerate()
            .filter(|(_, b)| b.kind.network_output())
            .filter_map(|(i, b)| {
                let p = b.network;
                Some((
                    (p.connection?, p.device?, p.address, b.kind.analog_value()),
                    i,
                ))
            })
            .collect();
        let links: HashMap<_, _> = project.connections.iter().map(|c| (c.id, *c)).collect();
        let network_sources = project
            .blocks
            .iter()
            .enumerate()
            .filter(|(_, b)| b.kind.network_input())
            .filter_map(|(i, b)| {
                let p = b.network;
                let connection = p.connection?;
                let peer = links.get(&connection)?.peer(p.device?)?;
                let output = outputs.get(&(connection, peer, p.address, b.kind.analog_value()))?;
                Some((i, *output))
            })
            .collect();
        let mut functions = HashMap::new();
        for (index, block) in project
            .blocks
            .iter()
            .enumerate()
            .filter(|(_, b)| b.kind == Kind::Udf)
        {
            let circuit = block
                .subcircuit
                .as_ref()
                .ok_or("Associez un schéma à chaque fonction utilisateur avant de simuler.")?;
            let (inputs, output) = circuit.udf_ports()?;
            functions.insert(
                index,
                UserFunction {
                    inputs,
                    output,
                    simulation: Self::new(circuit)?,
                },
            );
        }
        Ok(Self {
            signals: vec![Signal::default(); project.blocks.len()],
            archive: VecDeque::new(),
            order,
            scanned: false,
            last_scan: 0.0,
            wall_time: schedule::now(),
            vm: [0; 1024],
            network_sources,
            functions,
        })
    }

    pub fn scan(&mut self, project: &Project, time: f64) {
        let wall_time = schedule::now();
        let delta = if self.scanned {
            (time - self.last_scan).max(0.0)
        } else {
            0.0
        };
        self.last_scan = time;
        for (index, block) in project.blocks.iter().enumerate() {
            if block.kind.network_input() && block.network.source == NetworkSource::LocalVm {
                let value = self.read_vm(block.network, block.kind.analog_value());
                self.signals[index].input = value.high;
                self.signals[index].analog_input = value.analog;
            }
        }
        for &(input, output) in &self.network_sources {
            self.signals[input].input = self.signals[output].high;
            self.signals[input].analog_input = self.signals[output].analog;
        }
        for &(i, sources) in &self.order {
            let b = &project.blocks[i];
            let inputs = sources.map(|source| {
                source.map_or_else(
                    || Signal {
                        high: b.kind.unconnected(),
                        ..Signal::default()
                    },
                    |j| self.signals[j],
                )
            });
            let pins = std::array::from_fn(|pin| {
                inputs[pin].high ^ (sources[pin].is_some() && b.pin_inverted(pin))
            });
            let state = &mut self.signals[i];
            let [a, c, d, _] = pins;
            state.high = match b.kind {
                Kind::Input | Kind::CursorKey | Kind::FunctionKey | Kind::NetInput => state.input,
                Kind::ConstantHigh => true,
                Kind::ConstantLow => false,
                Kind::AnalogInput => {
                    state.analog = state.analog_input.clamp(0, ANALOG_IO_MAX);
                    false
                }
                Kind::Output | Kind::Marker | Kind::OpenPin | Kind::NetOutput => a,
                Kind::AnalogMarker | Kind::NetAnalogOutput => {
                    state.analog = inputs[0].analog;
                    false
                }
                Kind::NetAnalogInput => {
                    state.analog = state.analog_input;
                    false
                }
                Kind::And => pins.into_iter().all(|v| v),
                Kind::Or => pins.into_iter().any(|v| v),
                Kind::Not => !a,
                Kind::Nand => !pins.into_iter().all(|v| v),
                Kind::Nor => !pins.into_iter().any(|v| v),
                Kind::Xor => a ^ c,
                Kind::Relay => !c && (a || state.high),
                Kind::ToggleRelay => {
                    let rising = a && !state.previous;
                    state.previous = a;
                    if c || d {
                        c && (!d || b.set_priority)
                    } else {
                        state.high ^ rising
                    }
                }
                Kind::AndRising | Kind::NandFalling => {
                    let all = pins.into_iter().all(|v| v);
                    let level = all == (b.kind == Kind::AndRising);
                    let rising = self.scanned && level && !state.previous;
                    state.previous = level;
                    rising
                }
                Kind::OnDelay => {
                    if a {
                        state.elapsed = time - *state.since.get_or_insert(time);
                        state.elapsed >= b.duration
                    } else {
                        state.since = None;
                        state.elapsed = 0.0;
                        false
                    }
                }
                Kind::OffDelay => {
                    if c || a {
                        state.since = None;
                        state.elapsed = 0.0;
                        !c
                    } else if state.high {
                        let started = state.since.is_none();
                        state.elapsed = time - *state.since.get_or_insert(time);
                        started || state.elapsed < b.duration
                    } else {
                        false
                    }
                }
                Kind::Counter => {
                    if a {
                        state.count = 0;
                    } else if c && !state.previous {
                        state.count = (state.count + if d { -1 } else { 1 }).clamp(0, COUNTER_MAX);
                    }
                    state.previous = c;
                    !a && state.count >= b.limit
                }
                Kind::OnOffDelay => {
                    let changing = a != state.high;
                    let elapsed = state.elapsed_since_start(changing, time);
                    let wait = if a { b.duration } else { b.duration_low };
                    if changing && elapsed >= wait {
                        state.since = None;
                        a
                    } else {
                        state.high
                    }
                }
                Kind::LatchedDelay => {
                    let started = !c && (state.since.is_some() || (a && !state.previous));
                    state.previous = a;
                    let elapsed = state.elapsed_since_start(started, time);
                    started && elapsed >= b.duration
                }
                Kind::PulseRelay => {
                    let elapsed = state.elapsed_since_start(a, time);
                    a && elapsed < b.duration
                }
                Kind::PulseGenerator => {
                    let elapsed = state.elapsed_since_start(a, time);
                    let period = b.duration + b.duration_low;
                    a && period > 0.0 && ((elapsed % period < b.duration) ^ c)
                }
                Kind::EdgePulse => {
                    if c {
                        state.since = None;
                    } else if a && !state.previous {
                        state.since = Some(time);
                    }
                    state.previous = a;
                    let Some(start) = state.since else {
                        state.high = false;
                        state.elapsed = 0.0;
                        continue;
                    };
                    state.elapsed = time - start;
                    let elapsed = state.elapsed - b.pulse_delay;
                    let period = b.duration + b.duration_low;
                    elapsed >= 0.0
                        && period > 0.0
                        && elapsed < period * f64::from(b.limit)
                        && elapsed % period < b.duration
                }
                Kind::Stopwatch => {
                    if state.previous && !d {
                        state.elapsed += delta;
                    }
                    state.previous = a && !d;
                    if d {
                        state.elapsed = 0.0;
                    }
                    if !c || d {
                        state.analog = (state.elapsed * 100.0) as i32;
                    }
                    false
                }
                Kind::HoursCounter => {
                    if state.previous {
                        state.lifetime += delta;
                        state.remainder += delta;
                    }
                    state.previous = c;
                    if a || d {
                        state.remainder = 0.0;
                    }
                    if d {
                        state.lifetime = 0.0;
                    }
                    state.elapsed = state.lifetime;
                    !a && !d && state.remainder >= f64::from(b.limit) * 3600.0
                }
                Kind::Frequency => {
                    if a && !state.previous {
                        state.count = state.count.saturating_add(1);
                    }
                    state.previous = a;
                    let since = *state.since.get_or_insert(time);
                    if time - since < b.duration.max(0.02) {
                        state.high
                    } else {
                        let frequency =
                            f64::from(std::mem::take(&mut state.count)) / (time - since);
                        state.since = Some(time);
                        hysteresis(
                            frequency as i32,
                            i32::from(b.analog.on),
                            i32::from(b.analog.off),
                            state.high,
                        )
                    }
                }
                Kind::RandomDelay => {
                    if a != state.previous {
                        if state.random == 0 {
                            state.random = b.id.max(1);
                        }
                        state.random ^= state.random << 13;
                        state.random ^= state.random >> 17;
                        state.random ^= state.random << 5;
                        state.wait = f64::from(state.random) / f64::from(u32::MAX)
                            * if a { b.duration } else { b.duration_low };
                        state.since = Some(time);
                    }
                    state.previous = a;
                    let elapsed = state.elapsed_since_start(a != state.high, time);
                    if elapsed >= state.wait { a } else { state.high }
                }
                Kind::AnalogOutput => {
                    state.analog = inputs[0].analog.clamp(0, ANALOG_IO_MAX);
                    false
                }
                Kind::Amplifier => {
                    state.analog = b
                        .analog
                        .scale(inputs[0].analog)
                        .clamp(i32::from(i16::MIN), i32::from(i16::MAX));
                    false
                }
                Kind::Threshold | Kind::Comparator | Kind::Differential => {
                    let mut value = b.analog.scale(inputs[0].analog);
                    if b.kind == Kind::Comparator {
                        value = value.saturating_sub(b.analog.scale(inputs[1].analog));
                    }
                    let (on, off) = (i32::from(b.analog.on), i32::from(b.analog.off));
                    if b.kind == Kind::Differential {
                        let end = on + off;
                        if off >= 0 {
                            value >= end || (value >= on && state.high)
                        } else {
                            value <= end || (value <= on && state.high)
                        }
                    } else {
                        hysteresis(value, on, off, state.high)
                    }
                }
                Kind::Watchdog => {
                    let value = b.analog.scale(inputs[1].analog);
                    if a && !state.previous {
                        state.reference = value;
                    }
                    state.previous = a;
                    let offset = value.saturating_sub(state.reference);
                    a && (offset > i32::from(b.analog.on).abs()
                        || offset < -i32::from(b.analog.off).abs())
                }
                Kind::Mux => {
                    state.analog = if a {
                        i32::from(b.values[usize::from(c) * 2 + usize::from(d)])
                    } else {
                        0
                    };
                    false
                }
                Kind::Pwm => {
                    let elapsed = state.elapsed_since_start(a, time);
                    let ratio = f64::from(b.analog.scale(inputs[1].analog).clamp(0, ANALOG_IO_MAX))
                        / f64::from(ANALOG_IO_MAX);
                    a && b.duration > 0.0 && elapsed % b.duration < b.duration * ratio
                }
                Kind::Math => {
                    let x = if sources[1].is_some() {
                        inputs[1].analog
                    } else {
                        i32::from(b.values[0])
                    };
                    let y = if sources[2].is_some() {
                        inputs[2].analog
                    } else {
                        i32::from(b.values[1])
                    };
                    let result = if a {
                        match b.operation {
                            Operation::Add => x.checked_add(y),
                            Operation::Subtract => x.checked_sub(y),
                            Operation::Multiply => x.checked_mul(y),
                            Operation::Divide => x.checked_div(y),
                        }
                    } else {
                        Some(0)
                    };
                    state.math_error = if a && b.operation == Operation::Divide && y == 0 {
                        1
                    } else if result.and_then(|v| i16::try_from(v).ok()).is_none() {
                        2
                    } else {
                        0
                    };
                    state.analog = result
                        .unwrap_or(0)
                        .clamp(i32::from(i16::MIN), i32::from(i16::MAX));
                    false
                }
                Kind::AnalogFilter => {
                    let target = f64::from(inputs[0].analog);
                    if !self.scanned || b.duration == 0.0 {
                        state.remainder = target;
                    } else {
                        state.remainder +=
                            (target - state.remainder) * (1.0 - (-delta / b.duration).exp());
                    }
                    state.analog = state.remainder.round() as i32;
                    false
                }
                Kind::Stairway | Kind::ComfortSwitch => {
                    let rising = a && !state.previous;
                    let falling = !a && state.previous;
                    state.previous = a;
                    let comfort = b.kind == Kind::ComfortSwitch;
                    if comfort && (c || rising && state.reference != 0) {
                        state.since = None;
                        state.reference = 0;
                    } else if rising {
                        state.since = Some(time);
                        state.reference = 1;
                    } else if falling && state.reference == 1 {
                        let held = time - state.since.unwrap_or(time);
                        state.reference = if comfort && held >= b.duration_low {
                            3
                        } else {
                            2
                        };
                        state.since = Some(time);
                    }
                    state.elapsed = time - state.since.unwrap_or(time);
                    if state.reference == 2 && state.elapsed >= b.duration {
                        state.reference = 0;
                    }
                    let warning = state.reference == 2
                        && (b.duration - b.pulse_delay
                            ..b.duration - b.pulse_delay + f64::from(b.limit) / 1000.0)
                            .contains(&state.elapsed);
                    state.reference != 0 && !warning
                }
                Kind::Ramp => {
                    let [low, high, start, max] = b.values.map(f64::from);
                    if !a {
                        state.remainder = 0.0;
                        state.since = None;
                        state.reference = 0;
                    } else if !state.previous {
                        state.remainder = start.min(max);
                        state.since = Some(time);
                    } else if state.reference != 2 {
                        if d {
                            state.reference = 1;
                        }
                        if state.since.is_some_and(|t| time - t < 0.1) {
                            // Hold the start/stop level briefly.
                        } else if state.reference == 1 && state.remainder == start.min(max) {
                            state.reference = 2;
                            state.since = Some(time);
                        } else {
                            let target = if state.reference == 1 {
                                start
                            } else if c {
                                high
                            } else {
                                low
                            };
                            let step = f64::from(b.limit.max(1)) * delta;
                            state.remainder +=
                                (target.min(max) - state.remainder).clamp(-step, step);
                        }
                    } else if state.since.is_some_and(|t| time - t >= 0.1) {
                        state.remainder = 0.0;
                    }
                    state.previous = a;
                    state.analog = state.remainder.round() as i32;
                    false
                }
                Kind::MinMax => {
                    let value = inputs[2].analog;
                    if a {
                        if state.since.is_none() {
                            state.reference = value;
                            state.count = value;
                            state.since = Some(time);
                        }
                        state.reference = state.reference.min(value);
                        state.count = state.count.max(value);
                        state.analog = match b.limit {
                            0 => state.reference,
                            1 => state.count,
                            2 if c => state.count,
                            2 => state.reference,
                            _ => value,
                        };
                    } else if b.set_priority {
                        state.since = None;
                        state.analog = 0;
                    }
                    false
                }
                Kind::Average => {
                    if !a || c {
                        state.since = None;
                        state.count = 0;
                        state.remainder = 0.0;
                    }
                    if c {
                        state.analog = 0;
                    } else if a {
                        let start = *state.since.get_or_insert(time);
                        let samples = b.limit.clamp(1, 32_767);
                        let fraction = ((time - start) / b.duration.max(0.02)).clamp(0.0, 1.0);
                        let due = (fraction * f64::from(samples)).floor() as i32;
                        state.remainder +=
                            f64::from(inputs[2].analog) * f64::from(due - state.count);
                        state.count = due;
                        if due == samples {
                            state.analog = (state.remainder / f64::from(samples)).round() as i32;
                            state.since = Some(time);
                            state.remainder = 0.0;
                            state.count = 0;
                        }
                    }
                    false
                }
                Kind::ShiftRegister => {
                    if c && !state.previous {
                        state.shift_bits = if d {
                            (state.shift_bits >> 1) | (u8::from(a) << 7)
                        } else {
                            (state.shift_bits << 1) | u8::from(a)
                        };
                    }
                    state.previous = c;
                    state.shift_bits & (1 << (b.limit.clamp(1, 8) - 1)) != 0
                }
                Kind::ShiftBit => inputs[0].shift_bits & (1 << (b.limit.clamp(1, 8) - 1)) != 0,
                Kind::MathError => {
                    let mask = match b.limit {
                        1 => 1,
                        2 => 2,
                        _ => 3,
                    };
                    !a && (inputs[1].math_error & mask != 0 || !b.set_priority && state.high)
                }
                Kind::MessageText => a,
                Kind::SoftKey => {
                    let active = a && state.input;
                    let result = active && (!b.set_priority || !state.previous);
                    state.previous = active;
                    result
                }
                Kind::Udf => {
                    let function = self.functions.get_mut(&i).unwrap();
                    for (&index, value) in function.inputs.iter().zip(pins) {
                        function.simulation.signals[index].input = value;
                    }
                    function
                        .simulation
                        .scan(b.subcircuit.as_ref().unwrap(), time);
                    function.simulation.signals[function.output].high
                }
                Kind::DataLog => {
                    if a && !state.previous {
                        if self.archive.len() >= 1000 {
                            self.archive.pop_front();
                        }
                        self.archive.push_back((time, b.id, inputs[1].analog));
                    }
                    state.previous = a;
                    false
                }
                Kind::WeeklyTimer => b
                    .schedule
                    .as_ref()
                    .is_some_and(|s| s.weekly(wall_time, self.wall_time)),
                Kind::YearlyTimer => b
                    .schedule
                    .as_ref()
                    .is_some_and(|s| s.annual(wall_time, self.wall_time)),
                Kind::AstroClock => b.schedule.as_ref().is_some_and(|s| s.daylight(wall_time)),
                Kind::PiController => {
                    let error = (f64::from(b.values[0])
                        - f64::from(b.analog.scale(inputs[2].analog)))
                        * if b.set_priority { -1.0 } else { 1.0 };
                    let proportional = error * f64::from(b.limit) / 100.0;
                    if c || !a {
                        state.analog = if c {
                            0
                        } else {
                            i32::from(b.values[1]).clamp(0, 1000)
                        };
                        state.since = None;
                    } else if state.since.is_none() {
                        state.remainder = f64::from(state.analog) - proportional;
                        state.since = Some(time);
                    } else if let Some(start) = state.since
                        && time - start >= 0.5
                    {
                        state.remainder += proportional * (time - start) / b.duration.max(0.5);
                        let output = (proportional + state.remainder).clamp(0.0, 1000.0);
                        state.remainder = output - proportional;
                        state.analog = output.round() as i32;
                        state.since = Some(time);
                    }
                    false
                }
                Kind::FloatToInteger | Kind::IntegerToFloat => {
                    let width = if b.set_priority { 8 } else { 4 };
                    let address = (b.limit as usize).min(self.vm.len() - width);
                    let memory = &mut self.vm[address..address + width];
                    let resolution = b.duration.max(0.000001);
                    let value = if b.kind == Kind::IntegerToFloat {
                        let value = if sources[0].is_some() {
                            inputs[0].analog
                        } else {
                            i32::from(b.values[0])
                        };
                        let converted = f64::from(value) * resolution;
                        if b.set_priority {
                            memory.copy_from_slice(&converted.to_be_bytes());
                        } else {
                            memory.copy_from_slice(&(converted as f32).to_be_bytes());
                        }
                        value
                    } else {
                        let value = if b.set_priority {
                            f64::from_be_bytes(memory.try_into().unwrap())
                        } else {
                            f64::from(f32::from_be_bytes(memory.try_into().unwrap()))
                        };
                        (value / resolution) as i32
                    };
                    state.analog = value.clamp(i32::from(i16::MIN), i32::from(i16::MAX));
                    false
                }
            };
        }
        for (index, block) in project.blocks.iter().enumerate() {
            if block.kind.network_output() && block.network.source == NetworkSource::LocalVm {
                self.write_vm(block.network, block.kind.analog_value(), self.signals[index]);
            }
        }
        self.wall_time = wall_time;
        self.scanned = true;
    }
}

fn hysteresis(value: i32, on: i32, off: i32, active: bool) -> bool {
    if on >= off {
        value > on || (value > off && active)
    } else {
        (on..off).contains(&value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn imported_user_function_runs_and_survives_serialization() {
        let mut project = Project::empty();
        let a = project.add(Kind::ConstantHigh, 0.0, 0.0);
        let b = project.add(Kind::Input, 0.0, 0.0);
        let udf = project.add(Kind::Udf, 0.0, 0.0);
        project.block_mut(udf).unwrap().subcircuit = Some(Box::new(Project::example()));
        project.wires = vec![Wire::new(a, udf, 0), Wire::new(b, udf, 1)];
        let project: Project =
            serde_json::from_slice(&serde_json::to_vec(&project).unwrap()).unwrap();
        let mut sim = Simulation::new(&project).unwrap();
        sim.scan(&project, 0.0);
        assert!(!sim.signals[2].high);
        sim.signals[1].input = true;
        sim.scan(&project, 0.02);
        assert!(sim.signals[2].high);
    }

    #[test]
    fn shift_bit_and_math_error_track_their_wired_blocks() {
        let mut project = Project::empty();
        let one = project.add(Kind::ConstantHigh, 0.0, 0.0);
        let trigger = project.add(Kind::Input, 0.0, 0.0);
        let register = project.add(Kind::ShiftRegister, 0.0, 0.0);
        let bit = project.add(Kind::ShiftBit, 0.0, 0.0);
        let math = project.add(Kind::Math, 0.0, 0.0);
        let error = project.add(Kind::MathError, 0.0, 0.0);
        project.block_mut(bit).unwrap().limit = 2;
        let calc = project.block_mut(math).unwrap();
        calc.operation = Operation::Divide;
        calc.values[1] = 0;
        project.wires = [
            (one, register, 0),
            (trigger, register, 1),
            (register, bit, 0),
            (one, math, 0),
            (math, error, 1),
        ]
        .map(|(from, to, pin)| Wire::new(from, to, pin))
        .to_vec();
        let mut sim = Simulation::new(&project).unwrap();
        for (step, input) in [false, true, false, true].into_iter().enumerate() {
            sim.signals[1].input = input;
            sim.scan(&project, step as f64);
            assert_eq!(sim.signals[3].high, step == 3);
            assert!(sim.signals[5].high);
        }
    }

    #[test]
    fn converters_share_vm_and_archive_records_one_value_per_trigger() {
        let mut project = Project::empty();
        let writer = project.add(Kind::IntegerToFloat, 0.0, 0.0);
        let reader = project.add(Kind::FloatToInteger, 0.0, 0.0);
        let trigger = project.add(Kind::Input, 0.0, 0.0);
        let archive = project.add(Kind::DataLog, 0.0, 0.0);
        for (id, resolution) in [(writer, 0.5), (reader, 0.25)] {
            let block = project.block_mut(id).unwrap();
            block.set_priority = true;
            block.limit = 1016;
            block.duration = resolution;
        }
        project.block_mut(writer).unwrap().values[0] = -30;
        project.wires = vec![
            Wire::new(trigger, archive, 0),
            Wire::new(reader, archive, 1),
        ];
        let mut sim = Simulation::new(&project).unwrap();
        sim.scan(&project, 0.0);
        assert_eq!(sim.signals[1].analog, -60);
        sim.signals[2].input = true;
        sim.scan(&project, 1.0);
        sim.scan(&project, 2.0);
        assert_eq!(sim.archive, VecDeque::from([(1.0, archive, -60)]));
    }

    #[test]
    fn network_project_round_trip_and_scan_use_peer_address_and_type() {
        let project = Project::network_example();
        let saved = serde_json::to_string(&project).unwrap();
        let project: Project = serde_json::from_str(&saved).unwrap();
        let mut sim = Simulation::new(&project).unwrap();
        sim.signals[0].input = true;
        sim.signals[4].analog_input = 625;
        sim.scan(&project, 0.0);
        assert!(!sim.signals[3].high);
        assert_eq!(sim.signals[7].analog, 0);
        sim.scan(&project, 0.02);
        assert!(sim.signals[3].high);
        assert_eq!(sim.signals[7].analog, 625);

        let mut duplicate = project.clone();
        let sender = duplicate.blocks[1].clone();
        let id = duplicate.add(Kind::NetOutput, 0.0, 0.0);
        duplicate.block_mut(id).unwrap().network = sender.network;
        assert!(duplicate.order().is_err());

        let mut unmatched = project;
        unmatched.blocks[2].network.address = 2;
        let mut sim = Simulation::new(&unmatched).unwrap();
        sim.signals[0].input = true;
        sim.scan(&unmatched, 0.0);
        sim.scan(&unmatched, 0.02);
        assert!(!sim.signals[3].high);
    }

    #[test]
    fn network_blocks_exchange_vm_words_and_independent_bits() {
        let mut project = Project::empty();
        let bit0 = project.add(Kind::NetInput, 0.0, 0.0);
        let bit3 = project.add(Kind::NetInput, 0.0, 0.0);
        let word = project.add(Kind::NetAnalogInput, 0.0, 0.0);
        let digital_source = project.add(Kind::Input, 0.0, 0.0);
        let analog_source = project.add(Kind::AnalogInput, 0.0, 0.0);
        let bit_out = project.add(Kind::NetOutput, 0.0, 0.0);
        let word_out = project.add(Kind::NetAnalogOutput, 0.0, 0.0);
        for (id, address, bit) in [(bit0, 0, 0), (bit3, 0, 3), (word, 1, 0), (bit_out, 0, 3), (word_out, 1, 0)] {
            project.block_mut(id).unwrap().network = NetworkPort {
                source: NetworkSource::LocalVm,
                address,
                bit,
                ..Default::default()
            };
        }
        project.wires = vec![Wire::new(digital_source, bit_out, 0), Wire::new(analog_source, word_out, 0)];
        let mut sim = Simulation::new(&project).unwrap();
        sim.signals[3].input = true;
        sim.signals[4].analog_input = 625;
        sim.scan(&project, 0.0);
        sim.scan(&project, 0.02);
        assert!(!sim.signals[0].high);
        assert!(sim.signals[1].high);
        assert_eq!(sim.signals[2].analog, 625);
        sim.signals[3].input = false;
        sim.scan(&project, 0.04);
        sim.scan(&project, 0.06);
        assert!(!sim.signals[1].high);
    }

    #[test]
    fn arithmetic_accepts_mixed_pins_and_preserves_analog_type() {
        let mut p = Project::empty();
        let enable = p.add(Kind::ConstantHigh, 0.0, 0.0);
        let input = p.add(Kind::AnalogInput, 0.0, 0.0);
        let math = p.add(Kind::Math, 0.0, 0.0);
        p.block_mut(math).unwrap().values[1] = 25;
        p.wires
            .extend([Wire::new(enable, math, 0), Wire::new(input, math, 1)]);
        let mut sim = Simulation::new(&p).unwrap();
        sim.signals[1].analog_input = 300;
        sim.scan(&p, 0.0);
        assert_eq!(sim.signals[2].analog, 325);
        p.wires[1].from = enable;
        assert!(p.order().is_err());
    }

    #[test]
    fn example_requires_both_inputs() {
        let p = Project::example();
        let mut sim = Simulation::new(&p).unwrap();
        for (a, b) in [(false, false), (true, false), (true, true), (false, true)] {
            sim.signals[0].input = a;
            sim.signals[1].input = b;
            sim.scan(&p, 0.0);
            assert_eq!(sim.signals[3].high, a && b);
        }
    }

    #[test]
    fn timers_switch_on_their_configured_edges() {
        let mut p = Project::empty();
        let input = p.add(Kind::Input, 0.0, 0.0);
        let on = p.add(Kind::OnDelay, 0.0, 0.0);
        let off = p.add(Kind::OffDelay, 0.0, 0.0);
        p.block_mut(on).unwrap().duration = 1.0;
        p.block_mut(off).unwrap().duration = 1.0;
        p.wires = vec![Wire::new(input, on, 0), Wire::new(input, off, 0)];
        let mut sim = Simulation::new(&p).unwrap();
        for (t, input, on, off) in [
            (0.0, false, false, false),
            (0.5, true, false, true),
            (1.5, true, true, true),
            (1.75, false, false, true),
            (2.0, false, false, true),
            (2.75, false, false, false),
        ] {
            sim.signals[0].input = input;
            sim.scan(&p, t);
            assert_eq!((sim.signals[1].high, sim.signals[2].high), (on, off));
        }
    }

    #[test]
    fn counter_counts_edges_and_resets() {
        let mut p = Project::empty();
        let reset = p.add(Kind::Input, 0.0, 0.0);
        let pulse = p.add(Kind::Input, 0.0, 0.0);
        let dir = p.add(Kind::Input, 0.0, 0.0);
        let counter = p.add(Kind::Counter, 0.0, 0.0);
        p.wires = vec![
            Wire::new(reset, counter, 0),
            Wire::new(pulse, counter, 1),
            Wire::new(dir, counter, 2),
        ];
        let mut sim = Simulation::new(&p).unwrap();
        for (reset, pulse, dir, count) in [
            (false, true, false, 1),
            (false, true, false, 1),
            (false, false, false, 1),
            (false, true, false, 2),
            (false, false, true, 2),
            (false, true, true, 1),
            (true, true, false, 0),
        ] {
            sim.signals[0].input = reset;
            sim.signals[1].input = pulse;
            sim.signals[2].input = dir;
            sim.scan(&p, 0.0);
            assert_eq!(sim.signals[3].count, count);
        }
    }

    #[test]
    fn validates_wires() {
        let mut p = Project::empty();
        let a = p.add(Kind::Not, 0.0, 0.0);
        let b = p.add(Kind::Not, 0.0, 0.0);
        p.wires.push(Wire::new(a, b, 0));
        assert!(p.order().is_ok());
        p.wires.push(Wire::new(b, a, 0));
        assert!(p.order().is_err());
        p.wires[1] = Wire::new(a, b, 1);
        assert!(p.order().is_err());
    }

    #[test]
    fn analog_wiring_scales_values_and_applies_hysteresis() {
        let mut p = Project::empty();
        let input = p.add(Kind::AnalogInput, 0.0, 0.0);
        let amp = p.add(Kind::Amplifier, 0.0, 0.0);
        let output = p.add(Kind::AnalogOutput, 0.0, 0.0);
        let threshold = p.add(Kind::Threshold, 0.0, 0.0);
        let comparator = p.add(Kind::Comparator, 0.0, 0.0);
        let digital = p.add(Kind::Output, 0.0, 0.0);
        let params = &mut p.block_mut(amp).unwrap().analog;
        params.gain = 2.0;
        params.offset = -100;
        p.wires = [
            (input, amp, 0),
            (amp, output, 0),
            (amp, threshold, 0),
            (amp, comparator, 0),
            (input, comparator, 1),
            (threshold, digital, 0),
        ]
        .map(|(from, to, pin)| Wire::new(from, to, pin))
        .to_vec();
        let saved = serde_json::to_vec(&p).unwrap();
        let restored = serde_json::from_slice(&saved).unwrap();
        assert_eq!(p, restored);
        let mut sim = Simulation::new(&restored).unwrap();
        for (input, amp, output, threshold, comparator) in [
            (0, -100, 0, false, false),
            (300, 500, 500, false, false),
            (700, 1300, 1000, true, true),
            (275, 450, 450, true, false),
            (250, 400, 400, false, false),
        ] {
            sim.signals[0].analog_input = input;
            sim.scan(&restored, 0.0);
            assert_eq!(sim.signals[1].analog, amp);
            assert_eq!(sim.signals[2].analog, output);
            assert_eq!(sim.signals[3].high, threshold);
            assert_eq!(sim.signals[4].high, comparator);
            assert_eq!(sim.signals[5].high, threshold);
        }
        p.wires[5].from = amp;
        assert!(p.order().is_err());
        p.wires[5].from = threshold;
        p.wires[0].from = threshold;
        assert!(p.order().is_err());
    }

    #[test]
    fn extended_timers_handle_short_triggers_reset_and_invert() {
        let mut p = Project::empty();
        let trigger = p.add(Kind::Input, 0.0, 0.0);
        let control = p.add(Kind::Input, 0.0, 0.0);
        for kind in [
            Kind::OnOffDelay,
            Kind::LatchedDelay,
            Kind::PulseRelay,
            Kind::PulseGenerator,
        ] {
            let to = p.add(kind, 0.0, 0.0);
            p.block_mut(to).unwrap().duration = 1.0;
            p.wires.push(Wire::new(trigger, to, 0));
            if kind.pins().len() == 2 {
                p.wires.push(Wire::new(control, to, 1));
            }
        }
        let mut sim = Simulation::new(&p).unwrap();
        for (time, trigger, control, expected) in [
            (0.0, true, false, [false, false, true, true]),
            (0.5, false, false, [false, false, false, false]),
            (1.0, false, false, [false, true, false, false]),
            (1.5, true, true, [false, false, true, false]),
            (2.5, true, false, [true, false, false, false]),
            (3.5, true, false, [true, false, false, true]),
            (4.0, false, true, [true, false, false, false]),
            (5.0, false, false, [false, false, false, false]),
        ] {
            sim.signals[0].input = trigger;
            sim.signals[1].input = control;
            sim.scan(&p, time);
            assert_eq!(
                sim.signals[2..].iter().map(|v| v.high).collect::<Vec<_>>(),
                expected
            );
        }
    }

    #[test]
    fn edge_blocks_emit_one_scan_and_toggle_relay_stays_latched() {
        let mut p = Project::empty();
        let input = p.add(Kind::Input, 0.0, 0.0);
        for kind in [Kind::AndRising, Kind::NandFalling, Kind::ToggleRelay] {
            let to = p.add(kind, 0.0, 0.0);
            p.wires.push(Wire::new(input, to, 0));
        }
        let mut sim = Simulation::new(&p).unwrap();
        for (input, expected) in [
            (false, [false, false, false]),
            (true, [true, false, true]),
            (true, [false, false, true]),
            (false, [false, true, true]),
            (false, [false, false, true]),
            (true, [true, false, false]),
        ] {
            sim.signals[0].input = input;
            sim.scan(&p, 0.0);
            assert_eq!(
                sim.signals[1..].iter().map(|v| v.high).collect::<Vec<_>>(),
                expected
            );
        }
    }

    #[test]
    fn reads_projects_saved_before_analog_and_extended_timers() {
        let data = r#"{
            "version": 1, "name": "Ancien", "blocks": [
                {"id": 1, "kind": "OnDelay", "name": "B1", "x": 0, "y": 0, "duration": 3, "limit": 3}
            ], "wires": []
        }"#;
        let project: Project = serde_json::from_str(data).unwrap();
        assert_eq!(project.blocks[0].duration, 3.0);
        assert_eq!(project.blocks[0].analog, AnalogParameters::default());
        assert!(Simulation::new(&project).is_ok());
    }
}
