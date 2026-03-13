use either::{Either, Left, Right};
use std::fmt::{self, Display, Formatter};

#[derive(Debug)]
pub enum PclParserError {
    FileTooBig,
    UnknownCommand(u32),
    InvalidCommand(u32),
}

impl Display for PclParserError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        match self {
            PclParserError::FileTooBig => write!(f, "Too big file"),
            PclParserError::UnknownCommand(x) => write!(f, "Unknown command at {x:X}h"),
            PclParserError::InvalidCommand(x) => write!(f, "Invalid command at {x:X}h"),
        }
    }
}

#[allow(dead_code)]
#[derive(Debug)]
pub enum PclCommand {
    Char(u8),
    LineTermination(u8),
    EndOfLineWrap(u8),
    HorizontalMotionIndex(u16),
    VerticalMotionIndex(u16),
    ClearHorizontalMargins,
    RasterGraphicsPresentationMode(u8),
    SecondarySymbolSet(u16, u8),
    VerticalCursorPositioning(Either<u16, i16>),
    HorizontalCursorPositioning(Either<u16, i16>),
    EnableUnderline,
    DisableUnderline,
}

pub struct PclParser<'a> {
    bytes: &'a mut dyn Iterator<Item=u8>,
    read: u32,
    command_start: u32,
}

impl<'a> PclParser<'a> {
    fn read_first_byte(&mut self) -> Option<Result<u8, PclParserError>> {
        self.command_start = self.read;
        let c = self.bytes.next()?;
        let Some(read) = self.read.checked_add(1) else { return Some(Err(PclParserError::FileTooBig)); };
        self.read = read;
        Some(Ok(c))
    }

    fn read_byte(&mut self) -> Result<u8, PclParserError> {
        let c = self.bytes.next().ok_or_else(|| PclParserError::InvalidCommand(self.command_start))?;
        let Some(read) = self.read.checked_add(1) else { return Err(PclParserError::FileTooBig); };
        self.read = read;
        Ok(c)
    }

    fn read_i16_or_u16(&mut self) -> Result<(Either<u16, i16>, u8), PclParserError> {
        let mut c = self.read_byte()?;
        if c == b'+' || c == b'-' {
            let neg = c == b'-';
            let mut res = 0i16;
            c = self.read_byte()?;
            if c < b'0' || c > b'9' {
                return Err(PclParserError::InvalidCommand(self.command_start));
            }
            let t = loop {
                res = res
                    .checked_mul(10)
                    .and_then(|x| x.checked_add(i16::from(c - b'0')))
                    .ok_or_else(|| PclParserError::InvalidCommand(self.command_start))?;
                c = self.read_byte()?;
                if c < b'0' || c > b'9' {
                    break c;
                }
            };
            Ok((Right(if neg { -res } else { res }), t))
        } else {
            let mut res = 0u16;
            if c < b'0' || c > b'9' {
                return Err(PclParserError::InvalidCommand(self.command_start));
            }
            let t = loop {
                res = res
                    .checked_mul(10)
                    .and_then(|x| x.checked_add(u16::from(c - b'0')))
                    .ok_or_else(|| PclParserError::InvalidCommand(self.command_start))?;
                c = self.read_byte()?;
                if c < b'0' || c > b'9' {
                    break c;
                }
            };
            Ok((Left(res), t))
        }
    }

    fn read_u16(&mut self) -> Result<(u16, u8), PclParserError> {
        let mut res = 0u16;
        let mut c = self.read_byte()?;
        if c < b'0' || c > b'9' {
            return Err(PclParserError::InvalidCommand(self.command_start));
        }
        let t = loop {
            res = res
                .checked_mul(10)
                .and_then(|x| x.checked_add(u16::from(c - b'0')))
                .ok_or_else(|| PclParserError::InvalidCommand(self.command_start))?;
            c = self.read_byte()?;
            if c < b'0' || c > b'9' {
                break c;
            }
        };
        Ok((res, t))
    }

    fn read_u8(&mut self) -> Result<(u8, u8), PclParserError> {
        let mut res = 0u8;
        let mut c = self.read_byte()?;
        if c < b'0' || c > b'9' {
            return Err(PclParserError::InvalidCommand(self.command_start));
        }
        let t = loop {
            res = res
                .checked_mul(10)
                .and_then(|x| x.checked_add(c - b'0'))
                .ok_or_else(|| PclParserError::InvalidCommand(self.command_start))?;
            c = self.read_byte()?;
            if c < b'0' || c > b'9' {
                break c;
            }
        };
        Ok((res, t))
    }

    fn parse_amp_k(&mut self) -> Result<PclCommand, PclParserError> {
        let (n, c) = self.read_u16()?;
        match c {
            b'G' => {
                let n = if n <= 3 { u8::try_from(n).unwrap() } else {
                    return Err(PclParserError::InvalidCommand(self.command_start));
                };
                Ok(PclCommand::LineTermination(n))
            },
            b'H' => Ok(PclCommand::HorizontalMotionIndex(n)),
            _ => Err(PclParserError::UnknownCommand(self.command_start)),
        }
    }
    
    fn parse_amp_l(&mut self) -> Result<PclCommand, PclParserError> {
        let (n, c) = self.read_u16()?;
        match c {
            b'C' => Ok(PclCommand::VerticalMotionIndex(n)),
            _ => Err(PclParserError::UnknownCommand(self.command_start)),
        }
    }

    fn parse_amp_s(&mut self) -> Result<PclCommand, PclParserError> {
        let (n, c) = self.read_u8()?;
        match c {
            b'C' => {
                if n > 1 {
                    return Err(PclParserError::InvalidCommand(self.command_start));
                }
                Ok(PclCommand::EndOfLineWrap(n))
            },
            _ => Err(PclParserError::UnknownCommand(self.command_start)),
        }
    }
    
    fn parse_amp_d(&mut self) -> Result<PclCommand, PclParserError> {
        match self.read_byte()? {
            b'D' => Ok(PclCommand::EnableUnderline),
            b'@' => Ok(PclCommand::DisableUnderline),
            _ => Err(PclParserError::UnknownCommand(self.command_start)),
        }
    }

    fn parse_star_r(&mut self) -> Result<PclCommand, PclParserError> {
        let (n, c) = self.read_u8()?;
        match c {
            b'F' => {
                if n != 0 && n != 3 {
                    return Err(PclParserError::InvalidCommand(self.command_start));
                }
                Ok(PclCommand::RasterGraphicsPresentationMode(n))
            },
            _ => Err(PclParserError::UnknownCommand(self.command_start)),
        }
    }
    
    fn parse_star_p(&mut self) -> Result<PclCommand, PclParserError> {
        let (n, c) = self.read_i16_or_u16()?;
        match c {
            b'Y' => Ok(PclCommand::VerticalCursorPositioning(n)),
            b'X' => Ok(PclCommand::HorizontalCursorPositioning(n)),
            _ => Err(PclParserError::UnknownCommand(self.command_start)),
        }
    }

    fn parse_amp(&mut self) -> Result<PclCommand, PclParserError> {
        match self.read_byte()? {
            b'k' => self.parse_amp_k(),
            b'l' => self.parse_amp_l(),
            b's' => self.parse_amp_s(),
            b'd' => self.parse_amp_d(),
            _ => Err(PclParserError::UnknownCommand(self.command_start)),
        }
    }

    fn parse_star(&mut self) -> Result<PclCommand, PclParserError> {
        match self.read_byte()? {
            b'r' => self.parse_star_r(),
            b'p' => self.parse_star_p(),
            _ => Err(PclParserError::UnknownCommand(self.command_start)),
        }
    }

    fn parse_rparen(&mut self) -> Result<PclCommand, PclParserError> {
        let (n, c) = self.read_u16()?;
        if c < 64 || c >= 96 { return Err(PclParserError::InvalidCommand(self.command_start)); }
        Ok(PclCommand::SecondarySymbolSet(n, c))
    }

    fn parse(&mut self) -> Result<PclCommand, PclParserError> {
        match self.read_byte()? {
            b'9' => Ok(PclCommand::ClearHorizontalMargins),
            b'&' => self.parse_amp(),
            b'*' => self.parse_star(),
            b')' => self.parse_rparen(),
            _ => Err(PclParserError::UnknownCommand(self.command_start)),
        }
    }
}

impl<'a> Iterator for PclParser<'a> {
    type Item = Result<(PclCommand, u32), PclParserError>;

    fn next(&mut self) -> Option<Self::Item> {
        let c = match self.read_first_byte()? {
            Ok(c) => c,
            Err(e) => return Some(Err(e)),
        };
        if c != 0x1b { return Some(Ok((PclCommand::Char(c), self.command_start))); }
        match self.parse() {
            Err(e) => Some(Err(e)),
            Ok(command) => Some(Ok((command, self.command_start))),
        }
    }
}

pub fn parse_pcl(bytes: &mut dyn Iterator<Item=u8>) -> PclParser<'_> {
    PclParser { bytes, command_start: 0, read: 0 }
}
