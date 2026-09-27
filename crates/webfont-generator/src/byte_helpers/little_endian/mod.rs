pub(crate) struct LittleEndian<T>(T);

impl<T> LittleEndian<T> {
    pub(crate) fn new(bytes: T) -> Self {
        Self(bytes)
    }
}

impl<T: AsMut<[u8]>> LittleEndian<T> {
    pub(crate) fn write_u16_at(&mut self, offset: usize, value: u16) {
        self.0.as_mut()[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
    }

    pub(crate) fn write_u32_at(&mut self, offset: usize, value: u32) {
        self.0.as_mut()[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }
}

#[cfg(test)]
mod tests {
    use super::LittleEndian;

    #[test]
    fn writes_u16_and_u32_at_offsets() {
        let mut bytes = [0; 6];
        {
            let mut writer = LittleEndian::new(&mut bytes);
            writer.write_u16_at(0, 0x1234);
            writer.write_u32_at(2, 0x5678_9abc);
        }

        assert_eq!(bytes, [0x34, 0x12, 0xbc, 0x9a, 0x78, 0x56]);
    }
}
