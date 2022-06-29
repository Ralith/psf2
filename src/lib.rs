//! Parser for v2 [PC Screen Fonts](https://www.win.tue.nl/~aeb/linux/kbd/font-formats-1.html),
//! bitmap fonts which are simple and fast to draw.

#![no_std]

#[cfg(feature = "std")]
extern crate std;

#[cfg(feature = "unicode")]
extern crate alloc;

#[cfg(feature = "unicode")]
use alloc::string::String;
#[cfg(feature = "unicode")]
use hashbrown::HashMap;
#[cfg(feature = "unicode")]
use rustc_hash::FxHasher;

/// A well-formed PSF2 font
#[derive(Clone)]
pub struct Font<Data> {
    data: Data,
    #[cfg(feature = "unicode")]
    unicode: HashMap<String, u32, core::hash::BuildHasherDefault<FxHasher>>,
}

impl<Data: AsRef<[u8]>> Font<Data> {
    /// Try to parse `data` as a PSF2 font
    pub fn new(data: Data) -> Result<Self, ParseError> {
        let bytes = data.as_ref();
        let header = bytes.get(0..8 * 4).ok_or(ParseError::UnexpectedEnd)?;
        if &header[0..4] != &[0x72, 0xb5, 0x4a, 0x86] {
            return Err(ParseError::BadMagic);
        }

        let mut result = Self {
            data,
            #[cfg(feature = "unicode")]
            unicode: HashMap::default(),
        };

        let glyphs_size = result
            .charsize()
            .checked_mul(result.length())
            .ok_or(ParseError::UnexpectedEnd)?;
        let glyphs_end = result
            .headersize()
            .checked_add(glyphs_size)
            .ok_or(ParseError::UnexpectedEnd)? as usize;

        if glyphs_end > result.data.as_ref().len() {
            return Err(ParseError::UnexpectedEnd);
        }

        #[cfg(feature = "unicode")]
        if result.flags() & 0x01 != 0 {
            let table = &result.data.as_ref()[glyphs_end..];
            let mut index = 0;
            let mut start = 0;
            let mut in_sequence = false;
            for (i, &x) in table.iter().enumerate() {
                if x == 0xFF || x == 0xFE {
                    let slice = &table[start..i];
                    if let Ok(s) = core::str::from_utf8(slice) {
                        if in_sequence {
                            result.unicode.insert(s.into(), index);
                        } else {
                            for c in s.chars() {
                                result.unicode.insert(c.into(), index);
                            }
                        }
                    }

                    start = i + 1;
                    in_sequence = true;
                }
                if x == 0xFF {
                    index += 1;
                    in_sequence = false;
                }
            }
        }

        Ok(result)
    }

    #[inline]
    fn headersize(&self) -> u32 {
        u32::from_le_bytes(self.data.as_ref()[8..12].try_into().unwrap())
    }

    #[inline]
    fn flags(&self) -> u32 {
        u32::from_le_bytes(self.data.as_ref()[12..16].try_into().unwrap())
    }

    #[inline]
    fn length(&self) -> u32 {
        u32::from_le_bytes(self.data.as_ref()[16..20].try_into().unwrap())
    }

    #[inline]
    fn charsize(&self) -> u32 {
        u32::from_le_bytes(self.data.as_ref()[20..24].try_into().unwrap())
    }

    /// Number of rows in a glyph
    #[inline]
    pub fn height(&self) -> u32 {
        u32::from_le_bytes(self.data.as_ref()[24..28].try_into().unwrap())
    }

    /// Number of columns in a glyph
    #[inline]
    pub fn width(&self) -> u32 {
        u32::from_le_bytes(self.data.as_ref()[28..32].try_into().unwrap())
    }

    /// Get an iterator over the rows of the glyph bitmap for ASCII char `c`, if present
    #[inline]
    pub fn get_ascii(&self, c: u8) -> Option<RowIter<'_>> {
        self.get_index(c as u32)
    }

    /// Like [`get_ascii`](Self::get_ascii), but for a unicode scalar value
    #[cfg(feature = "unicode")]
    pub fn get_unicode(&self, c: char) -> Option<RowIter<'_>> {
        // Encode UTF-8
        let c = c as u32;
        let mut buf = [0u8; 4];
        let len = if c <= 0x7F {
            return self.get_ascii(c as u8);
        } else if c <= 0x07FF {
            buf[0] = 0xC0 | ((c >> 6) as u8 & 0x1F);
            buf[1] = 0x80 | (c & 0x3F) as u8;
            2
        } else if c <= 0xFFFF {
            buf[0] = 0xE0 | ((c >> 12) as u8 & 0x0F);
            buf[1] = 0x80 | ((c >> 6) as u8 & 0x3F);
            buf[2] = 0x80 | (c as u8 & 0x3F);
            3
        } else if c <= 0x10FFFF {
            buf[0] = 0xF0 | ((c >> 18) as u8 & 0x07);
            buf[1] = 0x80 | ((c >> 12) as u8 & 0x3F);
            buf[2] = 0x80 | ((c >> 6) as u8 & 0x3F);
            buf[3] = 0x80 | (c as u8 & 0x3F);
            4
        } else {
            // Invalid Unicode; unreachable?
            return None;
        };
        self.get_unicode_composed(core::str::from_utf8(&buf[..len]).unwrap())
    }

    /// Like [`get_unicode`](Self::get_unicode), but for one or more Unicode codepoints corresponding to a single glyph
    #[cfg(feature = "unicode")]
    pub fn get_unicode_composed(&self, seq: &str) -> Option<RowIter<'_>> {
        let index = self.unicode.get(seq).copied().or_else(|| {
            if seq.is_ascii() && seq.len() == 1 {
                seq.chars().next().map(|x| x as u32)
            } else {
                None
            }
        })?;
        self.get_index(index)
    }

    #[inline]
    fn get_index(&self, i: u32) -> Option<RowIter<'_>> {
        let offset = self.headersize() + i * self.charsize();
        let data = self
            .data
            .as_ref()
            .get(offset as usize..(offset + self.charsize()) as usize)?;
        Some(RowIter {
            data,
            width: self.width() as usize,
        })
    }
}

/// Why data might not be a valid PSF2 font
#[derive(Debug, Copy, Clone)]
pub enum ParseError {
    /// Input data ended prematurely
    UnexpectedEnd,
    /// Missing magic number; probably not PSF data.
    BadMagic,
}

#[cfg(feature = "std")]
impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        f.pad(match *self {
            ParseError::UnexpectedEnd => "unexpected end",
            ParseError::BadMagic => "bad magic number",
        })
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ParseError {}

/// Iterator over each row of a glyph
#[derive(Clone)]
pub struct RowIter<'a> {
    data: &'a [u8],
    width: usize,
}

impl<'a> RowIter<'a> {
    /// The raw data defining the glyph, minus any portions already iterated through
    ///
    /// Initially [`Font::height`] rows of [`Font::width`] bits, each row padded to a whole number
    /// of bytes.
    pub fn data(&self) -> &'a [u8] {
        self.data
    }
}

impl<'a> Iterator for RowIter<'a> {
    type Item = ColumnIter<'a>;
    #[inline]
    fn next(&mut self) -> Option<ColumnIter<'a>> {
        let advance = (self.width + 7) / 8;
        if self.data.len() < advance {
            return None;
        }
        let (next, rest) = self.data.split_at(advance);
        self.data = rest;
        Some(ColumnIter {
            data: next,
            bit: 0,
            width: self.width,
        })
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len(), Some(self.len()))
    }
}

impl ExactSizeIterator for RowIter<'_> {
    #[inline]
    fn len(&self) -> usize {
        self.data.len() / self.width
    }
}

impl<'a> DoubleEndedIterator for RowIter<'a> {
    #[inline]
    fn next_back(&mut self) -> Option<ColumnIter<'a>> {
        let advance = (self.width + 7) / 8;
        if self.data.len() < advance {
            return None;
        }
        let (rest, next) = self.data.split_at(self.data.len() - advance);
        self.data = rest;
        Some(ColumnIter {
            data: next,
            bit: 0,
            width: self.width,
        })
    }
}

/// Iterator over each column within a single row of a glyph
///
/// Yields whether the pixel at each position should be filled.
#[derive(Clone)]
pub struct ColumnIter<'a> {
    data: &'a [u8],
    bit: usize,
    width: usize,
}

impl<'a> ColumnIter<'a> {
    /// A bitfield defining the filled pixels in this row of the glyph
    ///
    /// The most significant bit corresponds to the leftmost pixel. Only the first [`Font::width`]
    /// bits are meaningful.
    pub fn data(&self) -> &'a [u8] {
        self.data
    }
}

impl<'a> Iterator for ColumnIter<'a> {
    type Item = bool;

    #[inline]
    fn next(&mut self) -> Option<bool> {
        if self.bit >= self.width {
            return None;
        }

        let byte = unsafe { self.data.get_unchecked(self.bit >> 3) };
        let result = byte & BITS[self.bit & 7] != 0;

        self.bit += 1;

        Some(result)
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len(), Some(self.len()))
    }
}

impl ExactSizeIterator for ColumnIter<'_> {
    #[inline]
    fn len(&self) -> usize {
        self.width - self.bit
    }
}

impl<'a> DoubleEndedIterator for ColumnIter<'a> {
    #[inline]
    fn next_back(&mut self) -> Option<bool> {
        if self.bit >= self.width {
            return None;
        }

        let bit = self.width - 1;

        let byte = unsafe { self.data.get_unchecked(bit >> 3) };
        let result = byte & BITS[bit & 7] != 0;

        self.width = bit;

        Some(result)
    }
}

const BITS: [u8; 8] = [
    1 << 7,
    1 << 6,
    1 << 5,
    1 << 4,
    1 << 3,
    1 << 2,
    1 << 1,
    1 << 0,
];

#[cfg(all(test, feature = "std"))]
mod tests {
    use std::vec::Vec;

    use super::*;

    #[test]
    fn column_correctness() {
        let it = ColumnIter {
            data: &[3, 0],
            bit: 0,
            width: 9,
        };
        assert_eq!(it.len(), 9);
        assert_eq!(
            it.collect::<Vec<_>>(),
            &[false, false, false, false, false, false, true, true, false]
        );
    }

    #[test]
    fn reverse_column() {
        let it = ColumnIter {
            data: &[3, 0],
            bit: 0,
            width: 9,
        };
        let mut naive = it.clone().collect::<Vec<_>>();
        naive.reverse();
        assert_eq!(naive, it.rev().collect::<Vec<_>>());
    }

    #[test]
    fn row_correctness() {
        let it = RowIter {
            data: &[128, 0],
            width: 1,
        };
        assert_eq!(it.len(), 2);
        assert_eq!(it.flat_map(|x| x).collect::<Vec<_>>(), &[true, false]);
    }

    #[test]
    fn reverse_row() {
        let it = RowIter {
            data: &[128, 0],
            width: 1,
        };
        let mut naive = it.clone().flat_map(|x| x).collect::<Vec<_>>();
        naive.reverse();
        assert_eq!(naive, it.rev().flat_map(|x| x).collect::<Vec<_>>());
    }
}
