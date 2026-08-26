use std::io::{Error, ErrorKind};

pub(crate) struct BigEndian<T>(T);

impl<T> BigEndian<T> {
    pub(crate) fn new(bytes: T) -> Self {
        Self(bytes)
    }
}

impl<T: AsRef<[u8]>> BigEndian<T> {
    pub(crate) fn read_u16(&self, offset: usize) -> Result<u16, Error> {
        let bytes = self.0.as_ref().get(offset..offset + 2).ok_or_else(|| {
            Error::new(
                ErrorKind::UnexpectedEof,
                "Unexpected EOF while reading u16.",
            )
        })?;
        Ok(u16::from_be_bytes([bytes[0], bytes[1]]))
    }

    pub(crate) fn read_u32(&self, offset: usize) -> Result<u32, Error> {
        let bytes = self.0.as_ref().get(offset..offset + 4).ok_or_else(|| {
            Error::new(
                ErrorKind::UnexpectedEof,
                "Unexpected EOF while reading u32.",
            )
        })?;
        Ok(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }
}

impl<T: AsMut<[u8]>> BigEndian<T> {
    pub(crate) fn write_i16_at(&mut self, offset: usize, value: i16) {
        self.0.as_mut()[offset..offset + 2].copy_from_slice(&value.to_be_bytes());
    }

    pub(crate) fn write_u16_at(&mut self, offset: usize, value: u16) {
        self.0.as_mut()[offset..offset + 2].copy_from_slice(&value.to_be_bytes());
    }

    pub(crate) fn write_u32_at(&mut self, offset: usize, value: u32) {
        self.0.as_mut()[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
    }
}

impl BigEndian<&mut Vec<u8>> {
    pub(crate) fn push_i16(&mut self, value: i16) {
        self.0.extend_from_slice(&value.to_be_bytes());
    }

    pub(crate) fn push_u16(&mut self, value: u16) {
        self.0.extend_from_slice(&value.to_be_bytes());
    }

    pub(crate) fn push_u32(&mut self, value: u32) {
        self.0.extend_from_slice(&value.to_be_bytes());
    }
}

#[cfg(test)]
mod tests {
    use std::io::ErrorKind;

    use super::BigEndian;

    #[test]
    fn reads_u16_and_u32_at_offsets() {
        let reader = BigEndian::new([0x12, 0x34, 0x56, 0x78, 0x9a]);

        assert_eq!(reader.read_u16(1).unwrap(), 0x3456);
        assert_eq!(reader.read_u32(1).unwrap(), 0x3456_789a);
    }

    #[test]
    fn rejects_reads_past_the_end() {
        let reader = BigEndian::new([0; 4]);

        assert_eq!(
            reader.read_u16(3).unwrap_err().kind(),
            ErrorKind::UnexpectedEof
        );
        assert_eq!(
            reader.read_u32(1).unwrap_err().kind(),
            ErrorKind::UnexpectedEof
        );
    }

    #[test]
    fn writes_at_offsets_and_appends() {
        let mut bytes = [0; 6];
        {
            let mut writer = BigEndian::new(&mut bytes);
            writer.write_u16_at(0, 0x1234);
            writer.write_u32_at(2, 0x5678_9abc);
        }
        assert_eq!(bytes, [0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc]);

        let mut output = Vec::new();
        {
            let mut writer = BigEndian::new(&mut output);
            writer.push_u16(0x1234);
            writer.push_u32(0x5678_9abc);
        }
        assert_eq!(output, bytes);

        let mut signed = [0; 2];
        BigEndian::new(&mut signed).write_i16_at(0, -2);
        assert_eq!(signed, [0xff, 0xfe]);

        let mut signed = Vec::new();
        BigEndian::new(&mut signed).push_i16(-2);
        assert_eq!(signed, [0xff, 0xfe]);
    }
}
