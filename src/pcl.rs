pub enum PclParserError {
    FileTooBig,
    UnknownCommand(u32),
    InvalidCommand(u32),
}

pub enum PclCommand {
    Char(u8),
    LineTermination(u8),
    HorizontalMotionIndex(u16),
    ClearHorizontalMargins,
}

struct PclParser<'a> {
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

    fn parse_amp(&mut self) -> Result<PclCommand, PclParserError> {
        match self.read_byte()? {
            b'k' => self.parse_amp_k(),
            _ => Err(PclParserError::UnknownCommand(self.command_start)),
        }
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
    
    fn parse(&mut self) -> Result<PclCommand, PclParserError> {
        match self.read_byte()? {
            b'9' => Ok(PclCommand::ClearHorizontalMargins),
            b'&' => self.parse_amp(),
            _ => Err(PclParserError::UnknownCommand(self.command_start)),
        }
    }
}

impl<'a> Iterator for PclParser<'a> {
    type Item = Result<PclCommand, PclParserError>;

    fn next(&mut self) -> Option<Self::Item> {
        let c = match self.read_first_byte()? {
            Ok(c) => c,
            Err(e) => return Some(Err(e)),
        };
        if c != 0x1b { return Some(Ok(PclCommand::Char(c))); }
        Some(self.parse())
    }
}

pub fn parse_pcl(
    bytes: &mut dyn Iterator<Item=u8>
) -> impl Iterator<Item=Result<PclCommand, PclParserError>> + '_ {
    PclParser { bytes, command_start: 0, read: 0 }
}
