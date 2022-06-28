#![no_std]

#[cfg(feature = "std")]
extern crate std;

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

    pub fn get(&self, c: char) -> Option<RowIter<'_>> {
        // TODO: Unicode translation
        let index = c as u32;
        let offset = self.headersize() + index * self.charsize();
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

#[derive(Clone)]
pub struct RowIter<'a> {
    data: &'a [u8],
    width: usize,
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

#[derive(Clone)]
pub struct ColumnIter<'a> {
    data: &'a [u8],
    bit: usize,
    width: usize,
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
