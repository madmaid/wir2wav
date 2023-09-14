use byteorder::{LittleEndian, ReadBytesExt};
use std::fmt;
use std::io::{Cursor, Read};
use std::path::PathBuf;
use std::string::FromUtf8Error;

pub struct Wir {
    pub header: WirHeader,
    pub body: WirBody,
}
#[cfg(feature = "convert_to_wav")]
impl Wir {
    pub fn write_to_wav<P: Into<PathBuf>>(
        &mut self,
        path: P,
        spec: hound::WavSpec,
    ) -> hound::Result<()> {
        let mut writer = hound::WavWriter::create(path.into(), spec)?;
        while (&mut self.body).into_iter().last().unwrap().len() > 0 {
            for channel in &mut self.body {
                writer.write_sample(channel.remove(0))?;
            }
        }
        writer.finalize().unwrap();
        Ok(())
    }
}

#[derive(Debug)]
pub struct WirHeader {
    pub magic: String,
    pub file_size: u32,
    pub version: String,
    pub header_size: u32,
    pub i3: u16,
    pub channels: u16,
    pub sample_rate: u32,
    pub fs2: u32,
    pub i4: u16,
    pub i5: u16,
    pub data: String,
}

#[cfg(feature = "convert_to_wav")]
impl WirHeader {
    pub fn to_wavspec(&mut self) -> hound::WavSpec {
        hound::WavSpec {
            channels: self.channels,
            sample_rate: self.sample_rate,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        }
    }
}

type WirChannel = Vec<f32>;
type WirBody = Vec<WirChannel>;

#[derive(Debug, Clone)]
pub struct ParseError;
impl fmt::Display for ParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "Parse Failed")
    }
}

#[derive(Debug)]
pub enum ParserError {
    IoError(std::io::Error),
    InvalidCharacterError(FromUtf8Error),
}

pub type ParseResult<T> = std::result::Result<T, ParserError>;

pub struct Parser {
    reader: Cursor<Vec<u8>>,
}

impl Parser {
    pub fn new(bytes: Vec<u8>) -> Parser {
        Parser {
            reader: Cursor::new(bytes),
        }
    }
    pub fn parse(&mut self) -> Result<Wir, ParseError> {
        let header = self.parse_header().unwrap();
        let body = self.parse_body(&header);
        Ok(Wir { header, body })
    }
    pub fn parse_body(&mut self, header: &WirHeader) -> WirBody {
        let mut body: WirBody = vec![];
        for _ in 0..header.channels {
            body.push(vec![]);
        }

        while (self.reader.position() as u32) < header.file_size {
            for channel in 0..header.channels {
                let data = self.reader.read_f32::<LittleEndian>().unwrap();
                body[channel as usize].push(data);
            }
        }
        body
    }
    pub fn parse_header(&mut self) -> Result<WirHeader, ParseError> {
        let magic = self.parse_magic().unwrap();
        let file_size = self.parse_file_size().unwrap();
        let version = self.parse_version().unwrap();
        let header_size = self.parse_header_size().unwrap();
        let i3 = self.parse_i3_variable().unwrap();
        let channels = self.parse_channels().unwrap();
        let sample_rate = self.parse_sample_rate().unwrap();
        let fs2 = self.parse_fs2_variable().unwrap();
        let i4 = self.parse_i4_channels().unwrap();
        let i5 = self.parse_i5_variable().unwrap();
        let data = self.parse_end_of_header().unwrap();

        let header = WirHeader {
            magic,
            file_size,
            version,
            header_size,
            i3,
            channels,
            sample_rate,
            fs2,
            i4,
            i5,
            data,
        };
        Ok(header)
    }
    fn parse_magic(&mut self) -> ParseResult<String> {
        let mut magic: [u8; 4] = [0; 4];
        self.reader
            .read_exact(&mut magic)
            .map_err(ParserError::IoError)?;
        String::from_utf8(magic.to_vec()).map_err(ParserError::InvalidCharacterError)
    }
    fn parse_file_size(&mut self) -> std::io::Result<u32> {
        self.reader.read_u32::<LittleEndian>()
    }
    fn parse_version(&mut self) -> ParseResult<String> {
        let mut version: [u8; 8] = [0; 8];
        self.reader
            .read_exact(&mut version)
            .map_err(ParserError::IoError)?;
        String::from_utf8(version.to_vec()).map_err(ParserError::InvalidCharacterError)
    }
    fn parse_header_size(&mut self) -> std::io::Result<u32> {
        self.reader.read_u32::<LittleEndian>()
    }
    fn parse_i3_variable(&mut self) -> std::io::Result<u16> {
        self.reader.read_u16::<LittleEndian>()
    }
    fn parse_channels(&mut self) -> std::io::Result<u16> {
        self.reader.read_u16::<LittleEndian>()
    }
    fn parse_sample_rate(&mut self) -> std::io::Result<u32> {
        self.reader.read_u32::<LittleEndian>()
    }
    fn parse_fs2_variable(&mut self) -> std::io::Result<u32> {
        self.reader.read_u32::<LittleEndian>()
    }
    fn parse_i4_channels(&mut self) -> std::io::Result<u16> {
        self.reader.read_u16::<LittleEndian>()
    }
    fn parse_i5_variable(&mut self) -> std::io::Result<u16> {
        self.reader.read_u16::<LittleEndian>()
    }
    fn parse_end_of_header(&mut self) -> ParseResult<String> {
        let mut data: [u8; 4] = [0; 4];
        self.reader.read_exact(&mut data).unwrap();
        String::from_utf8(data.to_vec()).map_err(ParserError::InvalidCharacterError)
    }
}

pub fn check_magic<S: Into<String>>(magic_word: S) -> bool {
    &magic_word.into() == "wvIR"
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::path::Path;

    #[test]
    fn test_parse_magic() {
        let input = b"wvIR";
        let mut parser = Parser::new(input.to_vec());
        let result = parser.parse_magic().unwrap();
        assert!(check_magic(result), "magic is right");
    }
    #[test]
    fn test_parse_file_size() {
        let input = [0x78, 0x88, 0x11, 0x00].to_vec();
        let mut parser = Parser::new(input);
        assert_eq!(
            parser.parse_file_size().unwrap(),
            u32::from_le_bytes([0x78, 0x88, 0x11, 0x00])
        );
    }
    #[test]
    fn test_parse_version() {
        let input = b"ver1fmt ";
        let mut parser = Parser::new(input.to_vec());
        let result = parser.parse_version().unwrap();
        assert_eq!(result, "ver1fmt ".to_string());
    }
    #[test]
    fn test_parse_header_size() {
        let input = [0x10, 0x00, 0x00, 0x00].to_vec();
        let mut parser = Parser::new(input);
        assert_eq!(
            parser.parse_header_size().unwrap(),
            u32::from_le_bytes([0x10, 0x00, 0x00, 0x00])
        );
    }
    #[test]
    fn test_parse_i3_variable() {
        let input = [0x03, 0x00].to_vec();
        let mut parser = Parser::new(input);
        assert_eq!(
            parser.parse_i3_variable().unwrap(),
            u16::from_le_bytes([0x03, 0x00])
        );
    }
    #[test]
    fn test_parse_channels() {
        let input = [0x03, 0x00].to_vec();
        let mut parser = Parser::new(input);
        assert_eq!(
            parser.parse_channels().unwrap(),
            u16::from_le_bytes([0x03, 0x00])
        );
    }
    #[test]
    fn test_parse_sample_rate() {
        let input = [0x00, 0x77, 0x01, 0x00].to_vec();
        let mut parser = Parser::new(input);
        assert_eq!(
            parser.parse_sample_rate().unwrap(),
            u32::from_le_bytes([0x00, 0x77, 0x01, 0x00])
        );
    }
    #[test]
    fn test_parse_fs2_variable() {
        let input = [0x00, 0xdc, 0x05, 0x00].to_vec();
        let mut parser = Parser::new(input);
        assert_eq!(
            parser.parse_fs2_variable().unwrap(),
            u32::from_le_bytes([0x00, 0xdc, 0x05, 0x00])
        );
    }
    #[test]
    fn test_parse_i4_channels() {
        let input = [0x04, 0x00].to_vec();
        let mut parser = Parser::new(input);
        assert_eq!(
            parser.parse_i4_channels().unwrap(),
            u16::from_le_bytes([0x04, 0x00])
        );
    }
    #[test]
    fn test_parse_i5_variable() {
        let input = [0x17, 0x00].to_vec();
        let mut parser = Parser::new(input);
        assert_eq!(
            parser.parse_i5_variable().unwrap(),
            u16::from_le_bytes([0x17, 0x00])
        );
    }
    #[test]
    fn test_parse_end_of_header() {
        let bytes = b"data";
        let mut parser = Parser::new(bytes.to_vec());
        assert_eq!(parser.parse_end_of_header().unwrap(), "data".to_string());
    }
    #[test]
    fn test_parsing_is_sequencial() {
        let input = [0x77, 0x76, 0x49, 0x52, 0x78, 0x88, 0x11, 0x00].to_vec();
        let mut parser = Parser::new(input);
        let magic = parser.parse_magic().unwrap();
        assert!(check_magic(magic), "magic is right");
        assert_eq!(
            parser.parse_file_size().unwrap(),
            u32::from_ne_bytes([0x78, 0x88, 0x11, 0x00])
        );
    }
    #[test]
    fn test_parse_file() {
        let path = Path::new("./data/mono.wir");
        let mut file = File::open(&path).unwrap();
        let mut buf = vec![];
        file.read_to_end(&mut buf).unwrap();
        let mut parser = Parser::new(buf);
        let header = parser.parse_header().unwrap();
        assert_eq!(header.version, "ver1fmt ".to_string())
    }
}
