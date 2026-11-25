use std::{
    collections::HashMap,
    io::{self, Cursor, Read},
};

#[derive(Clone)]
struct Class {
    name: String,
    flags: u8,
    fields: Vec<(u8, String)>,
    parent: Option<usize>,
}

pub(super) enum Value {
    Empty,
    Class,
    Text(String),
    Object(String, HashMap<String, Field>),
    Array(Vec<usize>),
    Integers(Vec<i32>),
}

pub(super) enum Field {
    Int(i32),
    Long(i64),
    Ref(Option<usize>),
}

pub(super) struct Stream<'a> {
    input: Cursor<&'a [u8]>,
    pub values: Vec<Value>,
    classes: HashMap<usize, Class>,
}

fn invalid() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        "Schéma illisible ou format non pris en charge.",
    )
}

// Earlier writers serialize some connections through the neighboring blocks.
const MAX_DEPTH: u16 = 512;

impl<'a> Stream<'a> {
    pub fn object(&self, id: usize) -> Option<(&str, &HashMap<String, Field>)> {
        match self.values.get(id)? {
            Value::Object(name, fields) => Some((name.rsplit('.').next()?, fields)),
            _ => None,
        }
    }

    pub fn reference(&self, id: usize, field: &str) -> Option<usize> {
        match self.object(id)?.1.get(field)? {
            Field::Ref(reference) => *reference,
            _ => None,
        }
    }

    pub fn integer(&self, id: usize, field: &str) -> Option<i32> {
        match self.object(id)?.1.get(field)? {
            Field::Int(number) => Some(*number),
            _ => None,
        }
    }

    pub fn long(&self, id: usize, field: &str) -> Option<i64> {
        match self.object(id)?.1.get(field)? {
            Field::Long(number) => Some(*number),
            _ => None,
        }
    }

    pub fn string(&self, id: usize, field: &str) -> Option<&str> {
        match self.values.get(self.reference(id, field)?)? {
            Value::Text(text) => Some(text),
            _ => None,
        }
    }

    pub fn read(bytes: &'a [u8]) -> io::Result<(Self, Vec<usize>)> {
        let mut stream = Self {
            input: Cursor::new(bytes),
            values: Vec::new(),
            classes: HashMap::new(),
        };
        if stream.bytes::<4>()? != [0xac, 0xed, 0, 5]
            || stream.byte()? != 0x77
            || stream.byte()? != 16
        {
            return Err(invalid());
        }
        let header = stream.bytes::<16>()?;
        let count = i32::from_be_bytes(header[12..].try_into().unwrap());
        if !(0..=20_000).contains(&count) {
            return Err(invalid());
        }
        let mut roots = Vec::with_capacity(count as usize);
        for _ in 0..count {
            roots.push(stream.value(0)?.ok_or_else(invalid)?);
        }
        Ok((stream, roots))
    }

    fn bytes<const N: usize>(&mut self) -> io::Result<[u8; N]> {
        let mut data = [0; N];
        self.input.read_exact(&mut data)?;
        Ok(data)
    }

    fn byte(&mut self) -> io::Result<u8> {
        Ok(self.bytes::<1>()?[0])
    }
    fn int(&mut self) -> io::Result<i32> {
        Ok(i32::from_be_bytes(self.bytes()?))
    }
    fn short(&mut self) -> io::Result<u16> {
        Ok(u16::from_be_bytes(self.bytes()?))
    }

    fn take(&mut self, count: usize) -> io::Result<&'a [u8]> {
        let start = self.input.position() as usize;
        let end = start.checked_add(count).ok_or_else(invalid)?;
        let bytes = self.input.get_ref().get(start..end).ok_or_else(invalid)?;
        self.input.set_position(end as u64);
        Ok(bytes)
    }

    fn text(&mut self, count: usize) -> io::Result<String> {
        let bytes = self.take(count)?;
        if let Ok(text) = std::str::from_utf8(bytes) {
            return Ok(text.into());
        }
        let mut units = Vec::new();
        let mut offset = 0;
        while let Some(&first) = bytes.get(offset) {
            let width = match first {
                0..=0x7f => 1,
                0xc0..=0xdf => 2,
                0xe0..=0xef => 3,
                _ => return Err(invalid()),
            };
            let mut unit = u16::from(first & [0, 0x7f, 0x1f, 0x0f][width]);
            for &next in bytes.get(offset + 1..offset + width).ok_or_else(invalid)? {
                if next & 0xc0 != 0x80 {
                    return Err(invalid());
                }
                unit = (unit << 6) | u16::from(next & 0x3f);
            }
            units.push(unit);
            offset += width;
        }
        String::from_utf16(&units).map_err(|_| invalid())
    }

    fn utf(&mut self) -> io::Result<String> {
        let count = self.short()? as usize;
        self.text(count)
    }

    fn add(&mut self, value: Value) -> io::Result<usize> {
        let id = self.values.len();
        if id >= 200_000 {
            return Err(invalid());
        }
        self.values.push(value);
        Ok(id)
    }

    fn value(&mut self, depth: u16) -> io::Result<Option<usize>> {
        let tag = self.byte()?;
        self.tag(tag, depth)
    }

    fn tag(&mut self, tag: u8, depth: u16) -> io::Result<Option<usize>> {
        if depth >= MAX_DEPTH {
            return Err(invalid());
        }
        let next = depth + 1;
        let id = match tag {
            0x70 => return Ok(None),
            0x71 => {
                let id = self.int()?.checked_sub(0x7e0000).ok_or_else(invalid)? as usize;
                self.values.get(id).ok_or_else(invalid)?;
                id
            }
            0x72 => {
                let name = self.utf()?;
                self.take(8)?;
                let id = self.add(Value::Class)?;
                let flags = self.byte()?;
                let count = self.short()?;
                let mut fields = Vec::new();
                for _ in 0..count {
                    let kind = self.byte()?;
                    let name = self.utf()?;
                    if matches!(kind, b'L' | b'[') {
                        self.value(next)?;
                    }
                    fields.push((kind, name));
                }
                self.annotation(next)?;
                let parent = self.value(next)?;
                self.classes.insert(
                    id,
                    Class {
                        name,
                        flags,
                        fields,
                        parent,
                    },
                );
                id
            }
            0x73 => {
                let class = self.value(next)?.ok_or_else(invalid)?;
                let id = self.add(Value::Empty)?;
                let name = self.classes.get(&class).ok_or_else(invalid)?.name.clone();
                let mut fields = HashMap::new();
                self.data(class, &mut fields, next)?;
                self.values[id] = Value::Object(name, fields);
                id
            }
            0x74 | 0x7c => {
                let count = if tag == 0x74 {
                    self.short()? as usize
                } else {
                    usize::try_from(u64::from_be_bytes(self.bytes()?)).map_err(|_| invalid())?
                };
                let text = self.text(count)?;
                self.add(Value::Text(text))?
            }
            0x75 => {
                let class = self.value(next)?.ok_or_else(invalid)?;
                let id = self.add(Value::Empty)?;
                let count = usize::try_from(self.int()?).map_err(|_| invalid())?;
                let kind = *self
                    .classes
                    .get(&class)
                    .and_then(|c| c.name.as_bytes().get(1))
                    .ok_or_else(invalid)?;
                self.values[id] = match kind {
                    b'L' | b'[' => {
                        if count > 200_000 {
                            return Err(invalid());
                        }
                        let mut refs = Vec::new();
                        for _ in 0..count {
                            if let Some(reference) = self.value(next)? {
                                refs.push(reference);
                            }
                        }
                        Value::Array(refs)
                    }
                    b'I' => {
                        let bytes = self.take(count.checked_mul(4).ok_or_else(invalid)?)?;
                        Value::Integers(
                            bytes
                                .chunks_exact(4)
                                .map(|chunk| i32::from_be_bytes(chunk.try_into().unwrap()))
                                .collect(),
                        )
                    }
                    _ => {
                        self.take(count.checked_mul(width(kind)?).ok_or_else(invalid)?)?;
                        Value::Empty
                    }
                };
                id
            }
            0x76 | 0x7e => {
                self.value(next)?;
                let id = self.add(Value::Empty)?;
                if tag == 0x7e {
                    self.value(next)?;
                }
                id
            }
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "Donnée de schéma inconnue ({tag:#04x}, octet {}).",
                        self.input.position() - 1
                    ),
                ));
            }
        };
        Ok(Some(id))
    }

    fn annotation(&mut self, depth: u16) -> io::Result<Vec<usize>> {
        let mut objects = Vec::new();
        loop {
            match self.byte()? {
                0x78 => return Ok(objects),
                0x77 => {
                    let count = self.byte()?;
                    self.take(count as usize)?;
                }
                0x7a => {
                    let count = usize::try_from(self.int()?).map_err(|_| invalid())?;
                    self.take(count)?;
                }
                tag => {
                    objects.extend(self.tag(tag, depth)?);
                }
            }
        }
    }

    fn data(
        &mut self,
        class: usize,
        fields: &mut HashMap<String, Field>,
        depth: u16,
    ) -> io::Result<()> {
        if depth >= MAX_DEPTH {
            return Err(invalid());
        }
        let Class {
            flags,
            name,
            fields: declared,
            parent,
            ..
        } = self.classes.get(&class).ok_or_else(invalid)?.clone();
        if flags & 4 != 0 {
            if flags & 8 == 0 {
                return Err(invalid());
            }
            self.annotation(depth + 1)?;
            return Ok(());
        }
        if let Some(parent) = parent {
            self.data(parent, fields, depth + 1)?;
        }
        for (kind, field) in declared {
            let value = match kind {
                b'L' | b'[' => Field::Ref(self.value(depth + 1)?),
                b'I' => Field::Int(self.int()?),
                b'B' | b'Z' => Field::Int(i32::from(self.byte()?)),
                b'C' => Field::Int(i32::from(self.short()?)),
                b'S' => Field::Int(i32::from(i16::from_be_bytes(self.bytes()?))),
                b'J' => Field::Long(i64::from_be_bytes(self.bytes()?)),
                _ => {
                    self.take(width(kind)?)?;
                    continue;
                }
            };
            fields.insert(field, value);
        }
        if flags & 1 != 0 {
            for (index, object) in self.annotation(depth + 1)?.into_iter().enumerate() {
                fields.insert(format!("{name}#{index}"), Field::Ref(Some(object)));
            }
        }
        Ok(())
    }
}

fn width(kind: u8) -> io::Result<usize> {
    match kind {
        b'B' | b'Z' => Ok(1),
        b'C' | b'S' => Ok(2),
        b'I' | b'F' => Ok(4),
        b'J' | b'D' => Ok(8),
        _ => Err(invalid()),
    }
}
