//! Private deterministic binary codec primitives for the V1 protocol.

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum WireError {
    UnexpectedEnd,
    InvalidUtf8,
    LengthOverflow { length: u64 },
    TrailingBytes { remaining: u32 },
}

pub(crate) struct Encoder {
    bytes: Vec<u8>,
}

impl Encoder {
    pub(crate) fn new() -> Self {
        Self { bytes: Vec::new() }
    }

    pub(crate) fn write_u8(&mut self, value: u8) {
        self.bytes.push(value);
    }

    pub(crate) fn write_u16(&mut self, value: u16) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }

    pub(crate) fn write_u32(&mut self, value: u32) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }

    pub(crate) fn write_u64(&mut self, value: u64) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }

    pub(crate) fn write_bytes(&mut self, value: &[u8]) {
        self.bytes.extend_from_slice(value);
    }

    pub(crate) fn write_text(&mut self, value: &str) {
        let length = u16::try_from(value.len()).expect("protocol text field exceeds u16 length");
        self.write_u16(length);
        self.write_bytes(value.as_bytes());
    }

    pub(crate) fn finish(self) -> Vec<u8> {
        self.bytes
    }
}

pub(crate) struct Decoder<'a> {
    bytes: &'a [u8],
    cursor: usize,
}

impl<'a> Decoder<'a> {
    pub(crate) fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, cursor: 0 }
    }

    pub(crate) fn read_u8(&mut self) -> Result<u8, WireError> {
        Ok(self.read_array::<1>()?[0])
    }

    pub(crate) fn read_u16(&mut self) -> Result<u16, WireError> {
        Ok(u16::from_be_bytes(self.read_array()?))
    }

    pub(crate) fn read_u32(&mut self) -> Result<u32, WireError> {
        Ok(u32::from_be_bytes(self.read_array()?))
    }

    pub(crate) fn read_u64(&mut self) -> Result<u64, WireError> {
        Ok(u64::from_be_bytes(self.read_array()?))
    }

    pub(crate) fn read_array<const N: usize>(&mut self) -> Result<[u8; N], WireError> {
        let end = self
            .cursor
            .checked_add(N)
            .ok_or(WireError::LengthOverflow { length: N as u64 })?;
        let slice = self
            .bytes
            .get(self.cursor..end)
            .ok_or(WireError::UnexpectedEnd)?;
        self.cursor = end;
        let mut output = [0_u8; N];
        output.copy_from_slice(slice);
        Ok(output)
    }

    pub(crate) fn read_bytes(&mut self, length: u32) -> Result<Vec<u8>, WireError> {
        let length = usize::try_from(length).map_err(|_| WireError::LengthOverflow {
            length: u64::from(length),
        })?;
        let end = self
            .cursor
            .checked_add(length)
            .ok_or(WireError::LengthOverflow {
                length: length as u64,
            })?;
        let slice = self
            .bytes
            .get(self.cursor..end)
            .ok_or(WireError::UnexpectedEnd)?;
        self.cursor = end;
        Ok(slice.to_vec())
    }

    pub(crate) fn read_text(&mut self) -> Result<String, WireError> {
        let length = u32::from(self.read_u16()?);
        String::from_utf8(self.read_bytes(length)?).map_err(|_| WireError::InvalidUtf8)
    }

    pub(crate) fn finish(self) -> Result<(), WireError> {
        let remaining = self.bytes.len().saturating_sub(self.cursor);
        if remaining == 0 {
            Ok(())
        } else {
            Err(WireError::TrailingBytes {
                remaining: u32::try_from(remaining).unwrap_or(u32::MAX),
            })
        }
    }
}
