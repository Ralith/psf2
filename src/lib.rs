#![no_std]

#[derive(Copy, Clone)]
pub struct Font<Data> {
    data: Data,
}

impl<Data: AsRef<[u8]>> Font<Data> {
    pub fn new(data: Data) -> Result<Self, ParseError> {
        let bytes = data.as_ref();
        let header = bytes.get(0..8 * 4).ok_or(ParseError::UnexpectedEnd)?;
        if &header[0..4] != &[0x72, 0xb5, 0x4a, 0x86] {
            return Err(ParseError::BadMagic);
        }

        let result = Self { data };

        let glyphs_size = result
            .charsize()
            .checked_mul(result.length())
            .ok_or(ParseError::UnexpectedEnd)?;
        let glyphs_end = result
            .headersize()
            .checked_add(glyphs_size)
            .ok_or(ParseError::UnexpectedEnd)?;

        if glyphs_end as usize > result.data.as_ref().len() {
            return Err(ParseError::UnexpectedEnd);
        }

        Ok(result)
    }

    #[inline]
    fn headersize(&self) -> u32 {
        u32::from_le_bytes(self.data.as_ref()[8..12].try_into().unwrap())
    }

    // #[inline]
    // fn flags(&self) -> u32 {
    //     u32::from_le_bytes(self.data.as_ref()[12..16].try_into().unwrap())
    // }

    #[inline]
    fn length(&self) -> u32 {
        u32::from_le_bytes(self.data.as_ref()[16..20].try_into().unwrap())
    }

    #[inline]
    fn charsize(&self) -> u32 {
        u32::from_le_bytes(self.data.as_ref()[20..24].try_into().unwrap())
    }

    #[inline]
    pub fn height(&self) -> u32 {
        u32::from_le_bytes(self.data.as_ref()[24..28].try_into().unwrap())
    }

    #[inline]
    pub fn width(&self) -> u32 {
        u32::from_le_bytes(self.data.as_ref()[28..32].try_into().unwrap())
    }

    pub fn get(&self, c: char) -> Option<Glyph<'_>> {
        // TODO: Unicode translation
        let index = c as u32;
        if index >= self.length() {
            return None;
        }
        Some(Glyph {
            font: Font {
                data: self.data.as_ref(),
            },
            index,
        })
    }
}

#[derive(Debug, Copy, Clone)]
pub enum ParseError {
    /// Input data ended prematurely
    UnexpectedEnd,
    /// Missing magic number; probably not PSF data.
    BadMagic,
}

#[derive(Copy, Clone)]
pub struct Glyph<'a> {
    font: Font<&'a [u8]>,
    index: u32,
}

impl<'a> Glyph<'a> {
    #[inline]
    pub fn rows(&self) -> impl Iterator<Item = impl Iterator<Item = bool> + 'a> + 'a {
        let font = self.font;
        let start = font.headersize() + self.index * font.charsize();
        let data = &font.data[start as usize..(start + font.charsize()) as usize];
        data.chunks_exact(((font.width() + 7) / 8) as usize)
            .map(move |row| {
                row.iter()
                    .flat_map(|row_byte| (0..8).rev().map(move |bit| row_byte & (1 << bit) != 0))
                    .take(font.width() as usize)
            })
    }
}
